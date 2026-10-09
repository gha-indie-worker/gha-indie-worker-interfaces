package dev.oreslang.runtime;

import java.util.ArrayDeque;
import java.util.Objects;
import java.util.concurrent.Executor;
import java.util.concurrent.RejectedExecutionException;

/**
 * Bounded FIFO single-flight admission for one logical process.
 *
 * <p>The runtime's carrier pool can be parallel: only the turns on this lane
 * are serial. A task's await/cooperate step returns from its guest turn before
 * the process runs another task. On resume, its carrier may differ.</p>
 */
final class SerialProcessExecutor implements Executor {
    private final Executor carriers;
    private final int capacity;
    private final ArrayDeque<Runnable> ready = new ArrayDeque<>();
    private boolean draining;

    SerialProcessExecutor(Executor carriers, int capacity) {
        this.carriers = Objects.requireNonNull(carriers, "carriers");
        if (capacity < 1) throw new IllegalArgumentException("capacity must be positive");
        this.capacity = capacity;
    }

    @Override
    public void execute(Runnable turn) {
        Objects.requireNonNull(turn, "turn");
        synchronized (ready) {
            if (ready.size() >= capacity) {
                throw new RejectedExecutionException("process ready queue full");
            }
            ready.addLast(turn);
            if (draining) return;
            draining = true;
            try {
                // A carrier must be admitted before any other submission can
                // be accepted. Runtime carriers are async executors; strict
                // isolated-root admission is an external bounded turn queue.
                carriers.execute(this::drain);
            } catch (RuntimeException | Error failure) {
                draining = false;
                ready.removeLastOccurrence(turn);
                throw failure;
            }
        }
    }

    private void drain() {
        try {
            for (;;) {
                Runnable next;
                synchronized (ready) {
                    next = ready.pollFirst();
                    if (next == null) return;
                }
                next.run();
            }
        } finally {
            // A fatal carrier failure must not leave an accepted queue orphaned.
            // New admissions cannot start another drain until this handoff.
            synchronized (ready) {
                draining = false;
                if (!ready.isEmpty()) {
                    draining = true;
                    try {
                        carriers.execute(this::drain);
                    } catch (RuntimeException | Error rejected) {
                        draining = false;
                        // The owning scheduler closes/cancels outstanding tasks
                        // on runtime shutdown; do not execute guest code inline.
                    }
                }
            }
        }
    }
}
