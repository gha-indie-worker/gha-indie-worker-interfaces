package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.*;

import java.util.List;
import java.util.concurrent.TimeUnit;
import org.junit.jupiter.api.Test;

final class ProcessTurnTraceTest {
    @Test
    void concreteSchedulerAwaitAndCallbackTurnsRefineSingleFlightProtocol()
            throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            OresFuture<Integer> awaiting = new OresFuture<>();
            OresFuture<Integer> resumed = vm.rootScheduler().start(new OresScheduler.Task<>() {
                private boolean first = true;
                @Override public OresScheduler.Step<Integer> resume(OresScheduler.Resume input) {
                    if (first) {
                        first = false;
                        return OresScheduler.await(awaiting);
                    }
                    return OresScheduler.done((Integer) input.value());
                }
            });
            assertEquals(7, vm.rootScheduler().startSync(() -> 7)
                    .get(5, TimeUnit.SECONDS));
            awaiting.completeFromRuntime(42);
            assertEquals(42, resumed.get(5, TimeUnit.SECONDS));
            var trace = vm.rootScheduler().turnTrace();
            ProcessTurnTrace.verify(trace);
            assertTrue(trace.singleFlight());
            assertEquals(0, trace.active());
            assertTrue(trace.events().stream()
                    .anyMatch(e -> e.kind() == ProcessTurnTrace.Kind.AWAIT));
            assertTrue(trace.events().stream()
                    .anyMatch(e -> e.kind() == ProcessTurnTrace.Kind.DONE));
            assertEquals(trace.events().stream()
                    .filter(e -> e.kind() == ProcessTurnTrace.Kind.ENTER).count(),
                    trace.events().stream()
                    .filter(e -> e.kind() == ProcessTurnTrace.Kind.EXIT).count());
        }
    }

    @Test
    void badTraceWithTwoOverlappingGuestStacksIsRejected() {
        var events = List.of(
                new ProcessTurnTrace.Event(1, ProcessTurnTrace.Kind.ENTER, 11, 100),
                new ProcessTurnTrace.Event(2, ProcessTurnTrace.Kind.ENTER, 12, 101));
        var trace = new ProcessTurnTrace.Snapshot("daddy", true, 0, 2, events);
        assertThrows(IllegalStateException.class, () -> ProcessTurnTrace.verify(trace));
    }

    @Test
    void truncatedTraceCannotBeMisrepresentedAsProof() {
        var incomplete = new ProcessTurnTrace.Snapshot("root", true, 1, 0, List.of());
        assertThrows(IllegalStateException.class, () -> ProcessTurnTrace.verify(incomplete));
    }

    @Test
    void distinctSupervisorSchedulerTracesDoNotShareOneExecutionDomain()
            throws Exception {
        try (OresVM vm = OresVM.create(Runnable::run)) {
            OresScheduler a = vm.createSupervisorScheduler("daddyA");
            OresScheduler b = vm.createSupervisorScheduler("daddyB");
            assertEquals(1, a.startSync(() -> 1).get(5, TimeUnit.SECONDS));
            assertEquals(2, b.startSync(() -> 2).get(5, TimeUnit.SECONDS));
            assertNotEquals(a.name(), b.name());
            ProcessTurnTrace.verify(a.turnTrace());
            ProcessTurnTrace.verify(b.turnTrace());
        }
    }
}
