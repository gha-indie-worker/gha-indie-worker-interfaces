package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.assertEquals;
import static org.junit.jupiter.api.Assertions.assertNotEquals;
import static org.junit.jupiter.api.Assertions.assertTrue;

import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;

final class OresVMTest {
    @Test
    void onlyRootOrRegisteredSupervisorsAreRecognizedAsGuestProcessLanes()
            throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run);
             OresScheduler unrelated = new OresScheduler(1)) {
            assertTrue(!vm.isGuestProcessSchedulerCurrent(),
                    "host/test thread is not an Oreslang guest lane");
            OresScheduler daddy = vm.createSupervisorScheduler("daddy");
            assertTrue(vm.rootScheduler().startSync(vm::isGuestProcessSchedulerCurrent)
                    .get(5, TimeUnit.SECONDS));
            assertTrue(daddy.startSync(vm::isGuestProcessSchedulerCurrent)
                    .get(5, TimeUnit.SECONDS));
            assertTrue(!unrelated.startSync(vm::isGuestProcessSchedulerCurrent)
                    .get(5, TimeUnit.SECONDS));
            assertTrue(!vm.executeControl(() -> {}).isCancelled());
        }
    }

    @Test
    void rootGuestAdmissionCanStayOnCallerAcrossPendingAwait() throws Exception {
        var turns = new java.util.concurrent.LinkedBlockingQueue<Runnable>();
        try (OresVM vm = OresVM.create(Runnable::run, turns::add)) {
            Thread caller = Thread.currentThread();
            OresFuture<Integer> source = new OresFuture<>();
            var pc = new java.util.concurrent.atomic.AtomicInteger();
            OresFuture<Integer> task = vm.rootScheduler().start(resume -> {
                assertEquals(caller, Thread.currentThread());
                assertEquals(vm.rootScheduler(), OresScheduler.current());
                if (pc.getAndIncrement() == 0) return OresScheduler.await(source);
                return OresScheduler.done((Integer) resume.value() + 1);
            });
            assertTrue(vm.controlCarrierCount() > 0);
            assertEquals(1, vm.rootScheduler().parallelism());
            turns.poll(5, TimeUnit.SECONDS).run();
            assertTrue(!task.isDone());
            Thread producer = Thread.ofPlatform().start(() -> source.completeFromRuntime(41));
            producer.join();
            turns.poll(5, TimeUnit.SECONDS).run();
            assertEquals(42, task.get(5, TimeUnit.SECONDS));
            assertEquals(2, pc.get());
        }
    }

    @Test
    void controlCarriersAreLiveBeforeFirstRootTaskTurn() throws Exception {
        OresVM vm = OresVM.create(Runnable::run);
        try {
            assertTrue(vm.started());
            assertEquals(1, vm.rootScheduler().parallelism(),
                    "ROOT is one logical user-code process");
            assertEquals(1, vm.controlCarrierCount(),
                    "only one CONTROL carrier should be eager on bootstrap");
            assertEquals(1, vm.guestCarrierCount(),
                    "only one guest-process carrier should be eager on bootstrap");

            OresFuture<String> first =
                    vm.rootScheduler().start(resume -> {
                        assertTrue(resume.initial());
                        assertTrue(OresScheduler.isSchedulerCarrierThread());
                        assertEquals(
                                vm.rootScheduler(),
                                OresScheduler.current());
                        return OresScheduler.done(Thread.currentThread().getName());
                    });

            String threadName = first.get(5, TimeUnit.SECONDS);
            assertTrue(threadName.startsWith("ores-guest-process-"));
        } finally {
            vm.close();
        }
    }

    @Test
    void startupIsIdempotentAndSchedulerDispatchIdsRemainFresh() throws Exception {
        OresVM vm = OresVM.create(Runnable::run);
        try {
            OresFuture<Long> task = vm.rootScheduler().start(new OresScheduler.Task<>() {
                private int pc;
                @Override
                public OresScheduler.Step<Long> resume(OresScheduler.Resume resume) {
                    if (pc++ == 0) {
                        long first = OresScheduler.currentDispatchId();
                        return OresScheduler.await(OresFuture.completed(first));
                    }
                    long resumed = OresScheduler.currentDispatchId();
                    assertNotEquals(0L, resumed);
                    assertEquals(OresScheduler.currentTaskDomain(), OresScheduler.currentTaskDomain());
                    return OresScheduler.done(resumed);
                }
            });

            long dispatch = task.get(5, TimeUnit.SECONDS);
            assertTrue(dispatch > 0L);
            vm.startup();
            vm.startup();
            assertTrue(vm.controlCarrierCount() >= 1);
            assertTrue(vm.guestCarrierCount() >= 1);
        } finally {
            vm.close();
        }
    }

    @Test
    void rootTasksAreSingleFlightWhileControlPoolRemainsParallel() throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            var entered = new java.util.concurrent.CountDownLatch(1);
            var release = new java.util.concurrent.CountDownLatch(1);
            var active = new java.util.concurrent.atomic.AtomicInteger();
            var maximum = new java.util.concurrent.atomic.AtomicInteger();

            OresFuture<Integer> first = vm.rootScheduler().startSync(() -> {
                int now = active.incrementAndGet();
                maximum.accumulateAndGet(now, Math::max);
                entered.countDown();
                try {
                    if (!release.await(5, TimeUnit.SECONDS)) {
                        throw new IllegalStateException("test gate timed out");
                    }
                    return 1;
                } finally {
                    active.decrementAndGet();
                }
            });
            assertTrue(entered.await(5, TimeUnit.SECONDS));

            var rest = new java.util.ArrayList<OresFuture<Integer>>();
            for (int i = 0; i < 64; i++) {
                rest.add(vm.rootScheduler().startSync(() -> {
                    int now = active.incrementAndGet();
                    maximum.accumulateAndGet(now, Math::max);
                    try { return now; }
                    finally { active.decrementAndGet(); }
                }));
            }

            // This is a separate logical control/mailman domain, not ROOT.
            vm.executeControl(() -> {}).get(5, TimeUnit.SECONDS);
            release.countDown();
            assertEquals(1, first.get(5, TimeUnit.SECONDS));
            for (var future : rest) {
                assertEquals(1, future.get(5, TimeUnit.SECONDS));
            }
            assertEquals(1, maximum.get(), "no two ROOT tasks may overlap");
        }
    }

    @Test
    void rootAwaitReleasesProcessLaneAndContinuationReturnsThroughIt()
            throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            OresFuture<Integer> gate = new OresFuture<>();
            var turns = new java.util.concurrent.atomic.AtomicInteger();
            OresFuture<Integer> waiting = vm.rootScheduler().start(resume -> {
                if (turns.getAndIncrement() == 0) return OresScheduler.await(gate);
                return OresScheduler.done((Integer) resume.value() + 1);
            });
            OresFuture<Integer> other = vm.rootScheduler().startSync(() -> 7);
            assertEquals(7, other.get(5, TimeUnit.SECONDS));
            assertTrue(!waiting.isDone());
            Thread producer = Thread.ofPlatform().start(() -> gate.completeFromRuntime(41));
            producer.join();
            assertEquals(42, waiting.get(5, TimeUnit.SECONDS));
            assertEquals(2, turns.get());
        }
    }

    @Test
    void independentRootProcessesCanRunInParallel() throws Exception {
        try (OresVM grandaddy = OresVM.create(Runnable::run);
             OresVM daddy = OresVM.create(Runnable::run)) {
            var bothEntered = new java.util.concurrent.CountDownLatch(2);
            var release = new java.util.concurrent.CountDownLatch(1);
            java.util.concurrent.Callable<Integer> body = () -> {
                bothEntered.countDown();
                if (!release.await(5, TimeUnit.SECONDS)) {
                    throw new IllegalStateException("parallel process test timed out");
                }
                return 1;
            };
            OresFuture<Integer> parent = grandaddy.rootScheduler().startSync(body);
            OresFuture<Integer> child = daddy.rootScheduler().startSync(body);
            try {
                assertTrue(bothEntered.await(5, TimeUnit.SECONDS),
                        "different process lanes must not share a global lock");
            } finally {
                release.countDown();
            }
            assertEquals(1, parent.get(5, TimeUnit.SECONDS));
            assertEquals(1, child.get(5, TimeUnit.SECONDS));
        }
    }

    @Test
    void siblingSupervisorsInSameVmHaveIndependentSingleFlightLanes()
            throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            OresScheduler daddyA = vm.createSupervisorScheduler("daddy-A");
            OresScheduler daddyB = vm.createSupervisorScheduler("daddy-B");
            assertEquals(2, vm.supervisorCount());
            assertNotEquals(daddyA.name(), daddyB.name());
            var bothEntered = new java.util.concurrent.CountDownLatch(2);
            var release = new java.util.concurrent.CountDownLatch(1);
            var daddyAActive = new java.util.concurrent.atomic.AtomicInteger();
            var daddyAMax = new java.util.concurrent.atomic.AtomicInteger();

            OresFuture<Integer> aFirst = daddyA.startSync(() -> {
                int now = daddyAActive.incrementAndGet();
                daddyAMax.accumulateAndGet(now, Math::max);
                bothEntered.countDown();
                try {
                    if (!release.await(5, TimeUnit.SECONDS)) {
                        throw new IllegalStateException("daddy A gate timed out");
                    }
                    return 10;
                } finally {
                    daddyAActive.decrementAndGet();
                }
            });
            OresFuture<Integer> aSecond = daddyA.startSync(() -> {
                int now = daddyAActive.incrementAndGet();
                daddyAMax.accumulateAndGet(now, Math::max);
                try { return 20; }
                finally { daddyAActive.decrementAndGet(); }
            });
            OresFuture<Integer> b = daddyB.startSync(() -> {
                bothEntered.countDown();
                if (!release.await(5, TimeUnit.SECONDS)) {
                    throw new IllegalStateException("daddy B gate timed out");
                }
                return 30;
            });
            try {
                assertTrue(bothEntered.await(5, TimeUnit.SECONDS),
                        "sibling supervisors must not share one global lane");
            } finally {
                release.countDown();
            }
            assertEquals(10, aFirst.get(5, TimeUnit.SECONDS));
            assertEquals(20, aSecond.get(5, TimeUnit.SECONDS));
            assertEquals(30, b.get(5, TimeUnit.SECONDS));
            assertEquals(1, daddyAMax.get(),
                    "two tasks owned by daddy A cannot overlap");
        }
    }
}
