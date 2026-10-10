package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertFalse;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertNotSame;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicLong;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.Test;

/**
 * Concrete ROOT-lane refinement of the Rust abstract timer/await model.
 *
 * This uses an OresFuture as a simulated timer-expiry notification until
 * Oreslang's typed source-level timer primitive is implemented.
 */
final class RootEventLoopRefinementTest {
    private static void nestedSynchronousCall(long owningDispatchId) {
        // Inner user functions belong to the same top-level guest turn.
        assertNotEquals(0L, owningDispatchId);
        assertEquals(owningDispatchId, OresScheduler.currentDispatchId());
    }

    @Test
    void timerExpirySchedulesCallbackWhileMainAwaitsWithoutOverlappingGuestStacks()
            throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            OresFuture<Integer> external = new OresFuture<>();
            OresFuture<Integer> timerExpiry = new OresFuture<>();
            AtomicInteger guestStacks = new AtomicInteger();
            AtomicInteger maxGuestStacks = new AtomicInteger();
            AtomicLong firstDispatch = new AtomicLong();
            AtomicReference<Thread> producerThread = new AtomicReference<>();
            CountDownLatch mainReachedAwait = new CountDownLatch(1);

            OresFuture<Integer> main = vm.rootScheduler().start(new OresScheduler.Task<>() {
                private int pc;
                @Override
                public OresScheduler.Step<Integer> resume(OresScheduler.Resume resume) {
                    int active = guestStacks.incrementAndGet();
                    maxGuestStacks.accumulateAndGet(active, Math::max);
                    try {
                        if (pc++ == 0) {
                            firstDispatch.set(OresScheduler.currentDispatchId());
                            nestedSynchronousCall(firstDispatch.get());
                            mainReachedAwait.countDown();
                            return OresScheduler.await(external);
                        }
                        assertNotEquals(firstDispatch.get(), OresScheduler.currentDispatchId(),
                                "await must enter a fresh logical dispatch");
                        return OresScheduler.done((Integer) resume.value() + 1);
                    } finally {
                        guestStacks.decrementAndGet();
                    }
                }
            });

            assertTrue(mainReachedAwait.await(5, TimeUnit.SECONDS));
            OresFuture<Integer> callback = vm.rootScheduler().start(new OresScheduler.Task<>() {
                private int pc;
                @Override
                public OresScheduler.Step<Integer> resume(OresScheduler.Resume resume) {
                    int active = guestStacks.incrementAndGet();
                    maxGuestStacks.accumulateAndGet(active, Math::max);
                    try {
                        if (pc++ == 0) return OresScheduler.await(timerExpiry);
                        assertNotSame(producerThread.get(), Thread.currentThread(),
                                "timer/producer carrier must not execute Oreslang callback");
                        return OresScheduler.done((Integer) resume.value());
                    } finally {
                        guestStacks.decrementAndGet();
                    }
                }
            });

            Thread producer = Thread.ofPlatform().start(() -> {
                producerThread.set(Thread.currentThread());
                timerExpiry.completeFromRuntime(55);
            });
            producer.join();
            assertEquals(55, callback.get(5, TimeUnit.SECONDS));
            assertFalse(main.isDone(), "unrelated timer cannot settle main's await");
            external.completeFromRuntime(41);
            assertEquals(42, main.get(5, TimeUnit.SECONDS));
            assertEquals(1, maxGuestStacks.get(),
                    "ROOT user callbacks and async continuations may never overlap");
        }
    }

    @Test
    void closingRootCannotResurrectAnAwaitingCallbackWithALateTimer()
            throws Exception {
        OresVM vm = OresVM.create(Runnable::run);
        OresFuture<Integer> expiry = new OresFuture<>();
        CountDownLatch awaiting = new CountDownLatch(1);
        AtomicInteger callbackExecutions = new AtomicInteger();
        try {
            OresFuture<Integer> callback = vm.rootScheduler().start(
                    new OresScheduler.Task<>() {
                        private boolean first = true;
                        @Override
                        public OresScheduler.Step<Integer> resume(OresScheduler.Resume resume) {
                            if (first) {
                                first = false;
                                awaiting.countDown();
                                return OresScheduler.await(expiry);
                            }
                            callbackExecutions.incrementAndGet();
                            return OresScheduler.done((Integer) resume.value());
                        }
                    });
            assertTrue(awaiting.await(5, TimeUnit.SECONDS));
            vm.close();
            expiry.completeFromRuntime(55);
            assertTrue(callback.isDone(), "runtime close settles suspended task");
            assertEquals(0, callbackExecutions.get(),
                    "late timer completion cannot execute callback after ROOT teardown");
        } finally {
            if (vm.started()) vm.close();
        }
    }
}
