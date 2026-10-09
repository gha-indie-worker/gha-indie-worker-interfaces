package dev.oreslang.runtime;

import java.util.Objects;
import java.util.concurrent.ArrayBlockingQueue;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.ConcurrentHashMap;
import java.util.Set;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.Executor;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * VM-owned asynchronous root runtime.
 *
 * <p>The CONTROL pool is deliberately prestarted during VM construction. Guest
 * execution is not admitted until this barrier is complete, so the first
 * {@code main} turn can safely use Futures, timers, channel/select wakeups and
 * root continuations without lazily creating runtime infrastructure.</p>
 */
public final class OresVM implements AutoCloseable {
    private static final int DEFAULT_CONTROL_PARALLELISM =
            Math.max(2, Math.min(4, Runtime.getRuntime().availableProcessors()));
    private static final int DEFAULT_QUEUE_CAPACITY = 65_536;

    private enum StartupState { NEW, STARTING, STARTED, FAILED, CLOSED }

    private final ThreadPoolExecutor controlExecutor;
    private final ProcessTimerService processTimers = new ProcessTimerService();
    private final OresScheduler rootScheduler;
    private final OresScheduler controlScheduler;
    private final Executor processAdmission;
    private final OresScheduler.TurnExecutor guestTurnAdmission;
    private final Set<OresScheduler> supervisorSchedulers = ConcurrentHashMap.newKeySet();
    private final AtomicLong nextSupervisorId = new AtomicLong();
    private final AtomicBoolean closed = new AtomicBoolean();
    private volatile StartupState startupState = StartupState.NEW;

    private OresVM(int controlParallelism, OresScheduler.TurnExecutor turnExecutor, Executor rootAdmission) {
        if (controlParallelism <= 0) {
            throw new IllegalArgumentException("control parallelism must be positive");
        }

        AtomicInteger carrierId = new AtomicInteger();
        ThreadFactory factory = runnable -> Thread.ofPlatform()
                .daemon(true)
                .name("ores-control-plane-" + carrierId.getAndIncrement())
                .unstarted(runnable);

        this.controlExecutor = new ThreadPoolExecutor(
                controlParallelism,
                controlParallelism,
                0L,
                TimeUnit.MILLISECONDS,
                new ArrayBlockingQueue<>(DEFAULT_QUEUE_CAPACITY),
                factory,
                new ThreadPoolExecutor.AbortPolicy());

        this.processAdmission = rootAdmission == null ? controlExecutor : rootAdmission;
        this.guestTurnAdmission = turnExecutor;
        // The ROOT process serializes user code, not the CONTROL carrier pool.
        // It can migrate carriers between non-overlapping source turns.
        this.rootScheduler = OresScheduler.runtimeOwned(
                "ores-root-scheduler",
                1,
                new SerialProcessExecutor(processAdmission, DEFAULT_QUEUE_CAPACITY),
                turnExecutor);
        // ActorGroup mailmen have individual execution leases and can run in
        // parallel with ROOT and with mailmen from other groups.
        this.controlScheduler = OresScheduler.runtimeOwned(
                "ores-control-scheduler",
                rootAdmission == null ? controlParallelism : 1,
                // Strict/untrusted contexts have a caller-thread Truffle/JNI
                // admission contract. Control/mailman turns must not escape
                // that isolated queue onto arbitrary CONTROL carriers.
                rootAdmission == null ? controlExecutor : rootAdmission,
                turnExecutor);
        try {
            processTimers.register(rootScheduler);
            rootScheduler.onClose(() -> processTimers.closeScope(rootScheduler));
            startup();
        } catch (RuntimeException | Error failure) {
            processTimers.close();
            rootScheduler.close();
            controlScheduler.close();
            controlExecutor.shutdownNow();
            throw failure;
        }
    }

    static OresVM create(OresScheduler.TurnExecutor turnExecutor) {
        return create(turnExecutor, null);
    }

    static OresVM create(OresScheduler.TurnExecutor turnExecutor, Executor rootAdmission) {
        return new OresVM(
                Integer.getInteger(
                        "ores.runtime.control.parallelism",
                        DEFAULT_CONTROL_PARALLELISM),
                turnExecutor,
                rootAdmission);
    }

    /**
     * VM bootstrap barrier. The STARTED state is visible only after every
     * configured CONTROL core carrier has been successfully prestarted.
     */
    synchronized void startup() {
        if (startupState == StartupState.CLOSED) {
            throw new IllegalStateException("Oreslang VM is closed");
        }
        switch (startupState) {
            case STARTED -> { return; }
            case STARTING -> throw new IllegalStateException(
                    "recursive Oreslang VM scheduler bootstrap");
            case FAILED -> throw new IllegalStateException(
                    "Oreslang VM scheduler bootstrap previously failed");
            case NEW -> startupState = StartupState.STARTING;
            default -> throw new AssertionError(startupState);
        }

        try {
            controlExecutor.allowCoreThreadTimeOut(false);
            int required = controlExecutor.getCorePoolSize();
            int newlyStarted = controlExecutor.prestartAllCoreThreads();
            int live = controlExecutor.getPoolSize();
            if (live != required) {
                throw new IllegalStateException(
                        "CONTROL dispatcher bootstrap incomplete: required="
                                + required + ", live=" + live
                                + ", newlyStarted=" + newlyStarted);
            }
            startupState = StartupState.STARTED;
        } catch (RuntimeException | Error failure) {
            startupState = StartupState.FAILED;
            throw failure;
        }
    }

    boolean started() {
        return startupState == StartupState.STARTED;
    }

    int controlCarrierCount() {
        return controlExecutor.getPoolSize();
    }

    int controlParallelism() {
        return controlScheduler.parallelism();
    }

    public OresScheduler rootScheduler() {
        if (!started()) {
            throw new IllegalStateException(
                    "Oreslang VM root scheduler was requested before startup");
        }
        return rootScheduler;
    }

    /**
     * Create an independent supervisor (daddy) process in this VM.
     * The scheduler has its own single-flight guest lane and identity.
     * Caller must keep supervisor-owned mutable state isolated from siblings;
     * mailbox/process ownership checking is a separate language boundary.
     */
    public synchronized OresScheduler createSupervisorScheduler(String name) {
        Objects.requireNonNull(name, "name");
        if (!name.matches("[A-Za-z0-9._-]{1,64}")) {
            throw new IllegalArgumentException("invalid supervisor process name");
        }
        if (!started() || closed.get()) {
            throw new RejectedExecutionException("VM closed before supervisor creation");
        }
        OresScheduler supervisor = OresScheduler.runtimeOwned(
                "ores-supervisor-" + name + "-" + nextSupervisorId.incrementAndGet(),
                1,
                new SerialProcessExecutor(processAdmission, DEFAULT_QUEUE_CAPACITY),
                guestTurnAdmission);
        try {
            processTimers.register(supervisor);
            supervisor.onClose(() -> processTimers.closeScope(supervisor));
        } catch (RuntimeException | Error failed) {
            supervisor.close();
            throw failed;
        }
        supervisorSchedulers.add(supervisor);
        return supervisor;
    }

    /** Schedule process-owned timeouts; timer workers never execute user code. */
    public ProcessTimerService.TimerHandle scheduleTimer(
            OresScheduler owner, java.time.Duration delay, Runnable callback) {
        if (!started() || closed.get()) {
            throw new RejectedExecutionException("timer scheduling requires a live VM");
        }
        return processTimers.schedule(owner, delay, callback);
    }

    public ProcessTimerService.TimerHandle scheduleTimer(
            OresScheduler owner, java.time.Duration delay) {
        return scheduleTimer(owner, delay, null);
    }

    /** End the short-lived ROOT main scope without keeping it alive for timers. */
    void finishRootMain() {
        processTimers.closeScope(rootScheduler);
    }

    int activeTimers(OresScheduler owner) {
        return processTimers.activeTimerCount(owner);
    }

    public int supervisorCount() {
        return supervisorSchedulers.size();
    }

    /**
     * Admit runtime CONTROL work through the already-owned root scheduler.
     *
     * Mailmen and other control services share the VM root/CONTROL admission
     * boundary. In a strict isolate this preserves the isolate's JNI/root-turn
     * admission executor rather than hopping guest work onto an unadmitted
     * native carrier.
     */
    OresFuture<Void> executeControl(Runnable task) {
        Objects.requireNonNull(task, "task");
        if (!started()) {
            throw new RejectedExecutionException(
                    "Oreslang VM CONTROL dispatcher is not available");
        }
        return controlScheduler.startSync(() -> {
            task.run();
            return null;
        });
    }

    public boolean isRootSchedulerCurrent() {
        return OresScheduler.current() == rootScheduler;
    }

    /** ROOT or a known daddy, never a free-floating user thread pool. */
    boolean isControlSchedulerCurrent() {
        return OresScheduler.current() == controlScheduler;
    }

    public boolean isGuestProcessSchedulerCurrent() {
        OresScheduler executing = OresScheduler.current();
        return executing == rootScheduler || supervisorSchedulers.contains(executing);
    }

    @Override
    public synchronized void close() {
        if (!closed.compareAndSet(false, true)) return;
        startupState = StartupState.CLOSED;
        processTimers.close();
        try {
            rootScheduler.close();
        } finally {
            try {
                for (OresScheduler supervisor : supervisorSchedulers) {
                    supervisor.close();
                }
                supervisorSchedulers.clear();
            } finally {
                try {
                    controlScheduler.close();
                } finally {
                    controlExecutor.shutdownNow();
                    try {
                        if (!controlExecutor.awaitTermination(5, TimeUnit.SECONDS)) {
                            throw new IllegalStateException(
                                    "Oreslang CONTROL scheduler carriers did not terminate");
                        }
                    } catch (InterruptedException interrupted) {
                        Thread.currentThread().interrupt();
                        throw new IllegalStateException(
                                "interrupted while closing Oreslang CONTROL scheduler",
                                interrupted);
                    }
                }
            }
        }
    }
}
