package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executors;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

final class SerialProcessExecutorTest {
    @Test
    void manyIndependentSubmissionsStillExecuteOneAtATime() throws Exception {
        try (var carriers = Executors.newFixedThreadPool(4)) {
            var process = new SerialProcessExecutor(carriers, 1024);
            var done = new CountDownLatch(200);
            var active = new AtomicInteger();
            var maxActive = new AtomicInteger();
            var completed = new AtomicInteger();
            for (int i = 0; i < 200; i++) {
                process.execute(() -> {
                    int inside = active.incrementAndGet();
                    maxActive.accumulateAndGet(inside, Math::max);
                    try { completed.incrementAndGet(); }
                    finally {
                        active.decrementAndGet();
                        done.countDown();
                    }
                });
            }
            assertTrue(done.await(5, TimeUnit.SECONDS));
            assertEquals(200, completed.get());
            assertEquals(1, maxActive.get());
        }
    }

    @Test
    void pendingTurnsAreBoundedAndRemainFifo() {
        var admitted = new ArrayList<Runnable>();
        var order = new ArrayList<Integer>();
        var process = new SerialProcessExecutor(admitted::add, 2);
        process.execute(() -> order.add(1));
        process.execute(() -> order.add(2));
        assertThrows(RejectedExecutionException.class,
                () -> process.execute(() -> order.add(3)));
        assertEquals(1, admitted.size(), "one physical drain submission");
        admitted.removeFirst().run();
        assertEquals(List.of(1, 2), order);
    }

    @Test
    void fatalTurnDoesNotOrphanAlreadyAcceptedTurns() {
        var admitted = new ArrayList<Runnable>();
        var process = new SerialProcessExecutor(admitted::add, 2);
        var completed = new AtomicInteger();
        process.execute(() -> { throw new LinkageError("fatal test"); });
        process.execute(completed::incrementAndGet);
        assertThrows(LinkageError.class, () -> admitted.removeFirst().run());
        assertEquals(1, admitted.size(), "remaining drain was resubmitted");
        admitted.removeFirst().run();
        assertEquals(1, completed.get());
    }
}
