package dev.oreslang.runtime;

import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.ConcurrentHashMap;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;

final class ActorModuleStateOwnershipTest {
    // Deliberately test-only host probe; no captured state enters guest factories.
    private static volatile Probe probe;

    private static final class Probe {
        final ActorRuntime runtime;
        final CountDownLatch bothEntered = new CountDownLatch(2);
        final CountDownLatch release = new CountDownLatch(1);
        final AtomicInteger allocations = new AtomicInteger();
        final Map<ActorRuntime.ActorId, List<String>> perActor = new ConcurrentHashMap<>();

        Probe(ActorRuntime runtime) { this.runtime = runtime; }
    }

    private static final class ModuleReadingFactory
            implements ActorRuntime.BehaviorFactory<String> {
        @Override public ActorRuntime.Behavior<String> create(
                ActorRuntime.ActorContext<String> context) {
            return (message, turn) -> {
                Probe p = probe;
                @SuppressWarnings("unchecked")
                List<String> state = (List<String>) p.runtime.currentActorModuleState(
                        "pkg/shared.ores", "M", () -> {
                            p.allocations.incrementAndGet();
                            return new ArrayList<String>();
                        });
                if ("first".equals(message)) {
                    state.add(turn.self().id().toString());
                    assertNull(p.perActor.putIfAbsent(turn.self().id(), state));
                    p.bothEntered.countDown();
                    if (!p.release.await(5, TimeUnit.SECONDS)) {
                        throw new IllegalStateException("parallel module state test timed out");
                    }
                } else if ("second".equals(message)) {
                    assertSame(p.perActor.get(turn.self().id()), state,
                            "module state must survive actor turns");
                    assertEquals(List.of(turn.self().id().toString()), state);
                    // Unit qualification is part of state identity.
                    Object other = p.runtime.currentActorModuleState(
                            "pkg/other.ores", "M", ArrayList<String>::new);
                    assertNotSame(state, other);
                    Object nextModule = p.runtime.currentActorModuleState(
                            "pkg/shared.ores", "Other", ArrayList<String>::new);
                    assertNotSame(state, nextModule);
                    turn.self().stop();
                } else {
                    throw new IllegalArgumentException("unexpected test message");
                }
            };
        }
    }

    @Test
    void isolatedAndSharedActorsUseOneCodeUnitButPrivateMutableModuleInstances()
            throws Exception {
        try (ActorRuntime runtime = new ActorRuntime(IsolatePolicy.developer())) {
            Probe p = new Probe(runtime);
            probe = p;
            ActorRuntime.ActorRef<String> privateActor =
                    runtime.spawnPrivate(new ModuleReadingFactory());
            ActorRuntime.ActorRef<String> sharedActor =
                    runtime.spawnShared(new ModuleReadingFactory());
            try {
                privateActor.send("first");
                sharedActor.send("first");
                assertTrue(p.bothEntered.await(5, TimeUnit.SECONDS),
                        "independent actors must enter simultaneously");
            } finally {
                p.release.countDown();
            }
            privateActor.send("second");
            sharedActor.send("second");
            assertTrue(privateActor.awaitTermination(5, TimeUnit.SECONDS));
            assertTrue(sharedActor.awaitTermination(5, TimeUnit.SECONDS));
            assertTrue(privateActor.failure().isEmpty(), privateActor.failure().toString());
            assertTrue(sharedActor.failure().isEmpty(), sharedActor.failure().toString());
            assertEquals(2, p.allocations.get(), "each actor initializes M once");
            assertEquals(2, p.perActor.size());
            assertNotSame(p.perActor.get(privateActor.id()), p.perActor.get(sharedActor.id()),
                    "same immutable code must never imply aliased writable module state");
            probe = null;
        }
    }

    @Test
    void hostThreadCannotImpersonateActorModuleStateOwner() {
        try (ActorRuntime runtime = new ActorRuntime(IsolatePolicy.developer())) {
            assertThrows(SecurityException.class,
                    () -> runtime.currentActorModuleState(
                            "pkg/shared.ores", "M", ArrayList<String>::new));
        }
    }
}
