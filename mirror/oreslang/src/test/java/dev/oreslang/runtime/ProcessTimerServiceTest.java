package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.*;

import java.time.Duration;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.RejectedExecutionException;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import java.util.concurrent.atomic.AtomicReference;
import org.junit.jupiter.api.Test;

final class ProcessTimerServiceTest {

    @Test
    void timerCallbackUsesOwningProcessLaneNeverTimerCarrier() throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            AtomicReference<Thread> userCarrier = new AtomicReference<>();
            AtomicInteger calls = new AtomicInteger();
            var timer = vm.scheduleTimer(vm.rootScheduler(), Duration.ZERO, () -> {
                userCarrier.set(Thread.currentThread());
                assertEquals(vm.rootScheduler(), OresScheduler.current());
                calls.incrementAndGet();
            });
            timer.fired().get(5, TimeUnit.SECONDS);
            assertNotNull(timer.callbackCompletion());
            timer.callbackCompletion().get(5, TimeUnit.SECONDS);
            assertNotNull(userCarrier.get());
            assertFalse(userCarrier.get().getName().startsWith("ores-process-timer"));
            assertEquals(1, calls.get());
            ProcessTurnTrace.verify(vm.rootScheduler().turnTrace());
        }
    }

    @Test
    void rootExitCancelsPendingTimersWithoutKeepingMainAlive() throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            AtomicInteger called = new AtomicInteger();
            var timer = vm.scheduleTimer(vm.rootScheduler(),
                    Duration.ofMinutes(10), called::incrementAndGet);
            assertEquals(1, vm.activeTimers(vm.rootScheduler()));
            vm.finishRootMain();
            assertTrue(timer.isCancelled());
            assertTrue(timer.fired().isCancelled());
            assertEquals(0, vm.activeTimers(vm.rootScheduler()));
            assertEquals(0, called.get());
            assertThrows(RejectedExecutionException.class,
                    () -> vm.scheduleTimer(vm.rootScheduler(), Duration.ZERO));
        }
    }

    @Test
    void anotherSupervisorTimerOutlivesRootButNotItsOwnScope() throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            var daddy = vm.createSupervisorScheduler("daddy");
            var tick = vm.scheduleTimer(daddy, Duration.ZERO);
            vm.finishRootMain();
            tick.fired().get(5, TimeUnit.SECONDS);
            assertFalse(tick.isCancelled());
            assertThrows(RejectedExecutionException.class,
                    () -> vm.scheduleTimer(vm.rootScheduler(), Duration.ofSeconds(1)));
        }
    }

    @Test
    void timerExpiryNeverReentersAnAlreadyRunningGuestStack() throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            CountDownLatch entered = new CountDownLatch(1);
            CountDownLatch release = new CountDownLatch(1);
            AtomicInteger active = new AtomicInteger();
            AtomicInteger peak = new AtomicInteger();
            var first = vm.rootScheduler().startSync(() -> {
                int n = active.incrementAndGet();
                peak.accumulateAndGet(n, Math::max);
                entered.countDown();
                try {
                    if (!release.await(5, TimeUnit.SECONDS)) throw new AssertionError("timeout");
                } finally {
                    active.decrementAndGet();
                }
                return 1;
            });
            try {
                assertTrue(entered.await(5, TimeUnit.SECONDS));
                var timer = vm.scheduleTimer(vm.rootScheduler(), Duration.ZERO, () -> {
                    int n = active.incrementAndGet();
                    peak.accumulateAndGet(n, Math::max);
                    active.decrementAndGet();
                });
                timer.fired().get(5, TimeUnit.SECONDS);
                assertEquals(0, timer.callbackCompletion().isDone() ? 1 : 0,
                        "queued callback must not run while ROOT is occupied");
                release.countDown();
                first.get(5, TimeUnit.SECONDS);
                timer.callbackCompletion().get(5, TimeUnit.SECONDS);
                assertEquals(1, peak.get());
                ProcessTurnTrace.verify(vm.rootScheduler().turnTrace());
            } finally {
                release.countDown();
            }
        }
    }

    @Test
    void explicitTimerCancellationRejectsLateCallback() throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            AtomicInteger called = new AtomicInteger();
            var timer = vm.scheduleTimer(vm.rootScheduler(),
                    Duration.ofMinutes(1), called::incrementAndGet);
            assertTrue(timer.cancel());
            assertFalse(timer.cancel());
            assertTrue(timer.fired().isCancelled());
            assertEquals(0, called.get());
        }
    }
}
