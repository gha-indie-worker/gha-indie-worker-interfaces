package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;
import org.junit.jupiter.api.Timeout;

@Timeout(20)
final class ActorImmutableSharedTransportTest {
    // Test probes are host-only, not guest-shared mutable state.
    private static volatile Object expected;
    private static volatile CountDownLatch bothRunning;
    private static final AtomicInteger delivered = new AtomicInteger();

    private static final class Observer implements ActorRuntime.Behavior<ActorRuntime.Shared<List<String>>> {
        @Override
        public void onMessage(
                ActorRuntime.Shared<List<String>> message,
                ActorRuntime.ActorContext<ActorRuntime.Shared<List<String>>> context)
                throws Exception {
            List<String> frozen = message.value();
            assertSame(expected, frozen, "shared actors must see the same frozen graph");
            assertEquals(List.of("original"), frozen);
            assertThrows(UnsupportedOperationException.class, () -> frozen.add("bad"));
            delivered.incrementAndGet();
            bothRunning.countDown();
            assertTrue(bothRunning.await(5, TimeUnit.SECONDS),
                    "two different shared actors should not be globally serialized");
            context.self().stop();
        }
    }

    @Test
    void sharedActorsReceiveSameDeeplyImmutableSnapshotWithoutAnotherCopy() throws Exception {
        bothRunning = new CountDownLatch(2);
        delivered.set(0);
        try (var runtime = new ActorRuntime(
                IsolatePolicy.developer(),
                new ActorRuntime.DispatcherConfig(2, 2, 8, 32))) {
            ArrayList<String> mutable = new ArrayList<>(List.of("original"));
            ActorRuntime.Shared<List<String>> published = runtime.shareReadonly(mutable);
            expected = published.value();
            mutable.set(0, "mutated");
            var actorA = runtime.spawnShared(() -> new Observer());
            var actorB = runtime.spawnShared(() -> new Observer());
            actorA.send(published);
            actorB.send(published);
            assertTrue(bothRunning.await(5, TimeUnit.SECONDS));
            assertTrue(actorA.awaitTermination(5, TimeUnit.SECONDS));
            assertTrue(actorB.awaitTermination(5, TimeUnit.SECONDS));
            assertEquals(2, delivered.get());
            assertTrue(actorA.failure().isEmpty());
            assertTrue(actorB.failure().isEmpty());
        } finally {
            expected = null;
        }
    }

    @Test
    void recursivelyFrozenMutableGraphContainsNoWritableNestedAliases() {
        ArrayList<Object> inner = new ArrayList<>();
        inner.add("before");
        var source = new java.util.LinkedHashMap<String, Object>();
        source.put("nested", List.of(inner));
        Object frozen = ActorRuntime.freeze(source);
        inner.add("after");
        source.put("new", 3);
        @SuppressWarnings("unchecked")
        var frozenMap = (java.util.Map<String, Object>) frozen;
        assertFalse(frozenMap.containsKey("new"));
        @SuppressWarnings("unchecked")
        var nested = (List<List<String>>) frozenMap.get("nested");
        assertEquals(List.of("before"), nested.getFirst());
        assertThrows(UnsupportedOperationException.class, () -> nested.getFirst().add("bad"));
        assertThrows(UnsupportedOperationException.class, () -> frozenMap.put("bad", 1));
    }
}
