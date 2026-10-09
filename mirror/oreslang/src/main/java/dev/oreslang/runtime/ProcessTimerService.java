package dev.oreslang.runtime;

import java.time.Duration;
import java.util.Objects;
import java.util.Set;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.ScheduledFuture;
import java.util.concurrent.ScheduledThreadPoolExecutor;
import java.util.concurrent.ThreadFactory;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;

/**
 * One VM-owned timer service, with independent logical process scopes.
 *
 * The timer thread only settles OresFuture and enqueues wakeups. User callbacks
 * are always resumable OresScheduler turns on their owning process.
 * Timer registration by itself never keeps main/process lifetime alive.
 */
public final class ProcessTimerService implements AutoCloseable {
    private final ScheduledThreadPoolExecutor clock;
    private final Map<OresScheduler, Scope> scopes = new ConcurrentHashMap<>();
    private final AtomicBoolean closed = new AtomicBoolean();

    public ProcessTimerService() {
        ThreadFactory factory = task -> Thread.ofPlatform().daemon(true)
                .name("ores-process-timer").unstarted(task);
        clock = new ScheduledThreadPoolExecutor(1, factory);
        clock.setRemoveOnCancelPolicy(true);
        clock.setExecuteExistingDelayedTasksAfterShutdownPolicy(false);
        clock.prestartAllCoreThreads();
    }

    void register(OresScheduler owner) {
        Objects.requireNonNull(owner);
        if (closed.get() || scopes.putIfAbsent(owner, new Scope(owner)) != null) {
            throw new RejectedExecutionException("timer process scope unavailable");
        }
        if (closed.get()) closeScope(owner);
    }

    public TimerHandle schedule(OresScheduler owner, Duration delay, Runnable callback) {
        Objects.requireNonNull(owner, "owner");
        Objects.requireNonNull(delay, "delay");
        if (delay.isNegative()) throw new IllegalArgumentException("timer delay cannot be negative");
        Scope scope = scopes.get(owner);
        if (scope == null) throw new RejectedExecutionException("unknown timer process");
        return scope.schedule(delay, callback);
    }

    public TimerHandle signal(OresScheduler owner, Duration delay) {
        return schedule(owner, delay, null);
    }

    void closeScope(OresScheduler owner) {
        Scope scope = scopes.remove(owner);
        if (scope != null) scope.close();
    }

    int activeTimerCount(OresScheduler owner) {
        Scope scope = scopes.get(owner);
        return scope == null ? 0 : scope.pending.size();
    }

    @Override public void close() {
        if (!closed.compareAndSet(false, true)) return;
        for (OresScheduler owner : Set.copyOf(scopes.keySet())) closeScope(owner);
        clock.shutdownNow();
    }

    public static final class TimerHandle implements AutoCloseable {
        private static final int PENDING = 0;
        private static final int FIRED = 1;
        private static final int CANCELLED = 2;

        private final Scope scope;
        private final OresFuture<Void> signal = new OresFuture<>();
        private final AtomicInteger state = new AtomicInteger(PENDING);
        private volatile ScheduledFuture<?> alarm;
        private volatile OresFuture<Void> callbackTask;

        private TimerHandle(Scope scope) { this.scope = scope; }

        /** Completion signals deadline expiration, not callback completion. */
        public OresFuture<Void> fired() { return signal; }
        public OresFuture<Void> callbackCompletion() { return callbackTask; }
        public boolean isCancelled() { return state.get() == CANCELLED; }

        public boolean cancel() {
            if (!state.compareAndSet(PENDING, CANCELLED)) return false;
            ScheduledFuture<?> scheduled = alarm;
            if (scheduled != null) scheduled.cancel(false);
            signal.cancel(false);
            OresFuture<Void> callback = callbackTask;
            if (callback != null) callback.cancel(false);
            scope.pending.remove(this);
            return true;
        }

        @Override public void close() {
            if (!cancel()) {
                OresFuture<Void> callback = callbackTask;
                if (callback != null) callback.cancel(false);
                scope.pending.remove(this);
            }
        }
    }

    private final class Scope {
        private final OresScheduler owner;
        private final Set<TimerHandle> pending = ConcurrentHashMap.newKeySet();
        private boolean open = true;

        private Scope(OresScheduler owner) { this.owner = owner; }

        private synchronized TimerHandle schedule(Duration delay, Runnable callback) {
            if (closed.get() || !open || owner.isClosed()) {
                throw new RejectedExecutionException("timer owner process already exited");
            }
            TimerHandle handle = new TimerHandle(this);
            pending.add(handle);
            try {
                if (callback != null) {
                    OresFuture<Void> task = owner.start(new OresScheduler.Task<>() {
                        private boolean awaiting = true;

                        @Override public OresScheduler.Step<Void> resume(OresScheduler.Resume resumed) {
                            if (awaiting) {
                                awaiting = false;
                                return OresScheduler.await(handle.signal);
                            }
                            if (resumed.failure() == null && isOpen() && !handle.isCancelled()) {
                                callback.run();
                            }
                            return OresScheduler.done(null);
                        }
                    });
                    handle.callbackTask = task;
                    task.whenCompleteRuntime((ignored, failure) -> pending.remove(handle));
                }

                long nanoseconds;
                try {
                    nanoseconds = delay.toNanos();
                } catch (ArithmeticException overflow) {
                    nanoseconds = Long.MAX_VALUE;
                }
                handle.alarm = clock.schedule(() -> expire(handle),
                        nanoseconds, TimeUnit.NANOSECONDS);
            } catch (RuntimeException | Error failure) {
                handle.close();
                throw failure;
            }
            return handle;
        }

        private synchronized boolean isOpen() { return open; }

        private void expire(TimerHandle handle) {
            // This callback runs only on the timer carrier. It cannot call any
            // guest/user code and cannot use the process's Truffle context.
            if (!handle.state.compareAndSet(TimerHandle.PENDING, TimerHandle.FIRED)) return;
            if (!isOpen()) {
                handle.signal.cancel(false);
            } else {
                handle.signal.completeFromRuntime(null);
            }
            if (handle.callbackTask == null) pending.remove(handle);
        }

        private synchronized void close() {
            if (!open) return;
            open = false;
            for (TimerHandle timer : Set.copyOf(pending)) timer.close();
            pending.clear();
        }
    }
}
