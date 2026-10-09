package dev.oreslang.runtime;

import java.util.ArrayList;
import java.util.List;
import java.util.Objects;
import java.util.concurrent.AbstractExecutorService;
import java.util.concurrent.ExecutorService;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.Semaphore;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicBoolean;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReferenceArray;
import java.util.concurrent.ThreadPoolExecutor;
import java.util.function.IntFunction;

/** Stable actor lanes. CPU binding, when requested, is mandatory rather than a hint. */
final class ActorAffinityDispatcher extends AbstractExecutorService {
    // Metadata is allocated eagerly, but carrier pools are allocated per lane
    // only after the first actor is actually scheduled on that lane.
    private final AtomicReferenceArray<ExecutorService> lanes;
    private final IntFunction<ExecutorService> factory;
    private final Object creationLock = new Object();
    private final int[] cpus;
    private final Semaphore queueSlots;
    private final AtomicInteger nextLane = new AtomicInteger();
    private final AtomicBoolean closed = new AtomicBoolean();

    ActorAffinityDispatcher(int parallelism, int capacity, int[] cpus,
                            IntFunction<ExecutorService> factory) {
        if (parallelism < 1 || capacity < 1) throw new IllegalArgumentException("invalid lane limits");
        this.cpus = cpus.clone();
        for (int cpu : cpus) NativeCarrierExecutor.requireAllowedCpu(cpu);
        this.queueSlots = new Semaphore(capacity);
        this.factory = Objects.requireNonNull(factory, "factory");
        this.lanes = new AtomicReferenceArray<>(parallelism);
    }

    int reserveLane() { return Math.floorMod(nextLane.getAndIncrement(), lanes.length()); }
    int cpuForLane(int lane) { return cpus.length == 0 ? -1 : cpus[lane % cpus.length]; }
    boolean nativeCarriers() {
        boolean any = false;
        for (int i = 0; i < lanes.length(); i++) {
            ExecutorService pool = lanes.get(i);
            if (pool != null) {
                any = true;
                if (!(pool instanceof NativeCarrierExecutor)) return false;
            }
        }
        return any;
    }

    int instantiatedLaneCount() {
        int count = 0;
        for (int i = 0; i < lanes.length(); i++) {
            if (lanes.get(i) != null) count++;
        }
        return count;
    }

    private ExecutorService lane(int index) {
        if (index < 0 || index >= lanes.length()) {
            throw new IllegalArgumentException("actor dispatch lane out of range");
        }
        ExecutorService pool = lanes.get(index);
        if (pool != null) return pool;
        synchronized (creationLock) {
            if (closed.get()) throw new RejectedExecutionException("actor lanes closed");
            pool = lanes.get(index);
            if (pool == null) {
                pool = Objects.requireNonNull(factory.apply(index), "actor lane factory returned null");
                if (closed.get()) {
                    pool.shutdownNow();
                    throw new RejectedExecutionException("actor lanes closed");
                }
                lanes.set(index, pool);
            }
            return pool;
        }
    }

    /** Optional warmup: exactly ONE lane, never the whole configured pool. */
    void warmOne() {
        ExecutorService pool = lane(0);
        if (pool instanceof ThreadPoolExecutor javaPool) {
            javaPool.prestartCoreThread();
        }
        // NativeCarrierExecutor(1,1) already starts its one carrier here.
    }

    void executeOnLane(int lane, Runnable command) {
        Objects.requireNonNull(command);
        if (closed.get() || !queueSlots.tryAcquire()) throw new RejectedExecutionException("actor lane queue unavailable");
        QueuedTurn task = new QueuedTurn(command);
        try {
            if (closed.get()) throw new RejectedExecutionException("actor lanes closed");
            lane(lane).execute(task);
        } catch (RuntimeException | Error failure) {
            task.releaseSlot();
            throw failure;
        }
    }

    private final class QueuedTurn implements Runnable {
        private final Runnable command;
        private final AtomicBoolean released = new AtomicBoolean();
        private QueuedTurn(Runnable command) { this.command = command; }
        private void releaseSlot() { if (released.compareAndSet(false, true)) queueSlots.release(); }
        @Override public void run() {
            releaseSlot();
            command.run();
        }
    }

    @Override public void execute(Runnable command) { executeOnLane(reserveLane(), command); }
    @Override public void shutdown() { shutdownNow(); }
    @Override public List<Runnable> shutdownNow() {
        closed.set(true);
        var abandoned = new ArrayList<Runnable>();
        synchronized (creationLock) {
            // No lazy lane may appear after this shutdown fence.
            for (int i = 0; i < lanes.length(); i++) {
                ExecutorService pool = lanes.get(i);
                if (pool == null) continue;
                for (Runnable pending : pool.shutdownNow()) {
                    QueuedTurn task = (QueuedTurn) pending;
                    task.releaseSlot();
                    abandoned.add(task.command);
                }
            }
        }
        return List.copyOf(abandoned);
    }
    @Override public boolean isShutdown() { return closed.get(); }
    @Override public boolean isTerminated() {
        if (!closed.get()) return false;
        for (int i = 0; i < lanes.length(); i++) {
            ExecutorService pool = lanes.get(i);
            if (pool != null && !pool.isTerminated()) return false;
        }
        return true;
    }
    @Override public boolean awaitTermination(long timeout, TimeUnit unit) throws InterruptedException {
        long budget = unit.toNanos(timeout);
        long start = System.nanoTime();
        if (!closed.get()) return false;
        for (int i = 0; i < lanes.length(); i++) {
            ExecutorService pool = lanes.get(i);
            if (pool != null && !pool.awaitTermination(
                    Math.max(0, budget - (System.nanoTime() - start)), TimeUnit.NANOSECONDS)) return false;
        }
        return true;
    }
}
