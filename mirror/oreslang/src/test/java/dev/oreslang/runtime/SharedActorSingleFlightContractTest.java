package dev.oreslang.runtime;

import static org.junit.jupiter.api.Assertions.*;

import java.util.ArrayList;
import java.util.List;
import java.util.Map;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;
import org.junit.jupiter.api.Test;

final class SharedActorSingleFlightContractTest {
    @Test
    void manyProducerThreadsCannotEnterOneSharedActorConcurrently()
            throws Exception {
        try (ActorRuntime runtime = new ActorRuntime(
                IsolatePolicy.developer(),
                new ActorRuntime.DispatcherConfig(4, 4, 8, 128))) {
            AtomicInteger inGuestCode = new AtomicInteger();
            AtomicInteger maximum = new AtomicInteger();
            AtomicInteger delivered = new AtomicInteger();
            var actor = runtime.spawnShared(() -> (Integer message, ActorRuntime.ActorContext<Integer> ctx) -> {
                int active = inGuestCode.incrementAndGet();
                maximum.accumulateAndGet(active, Math::max);
                try {
                    Thread.yield();
                    delivered.incrementAndGet();
                } finally {
                    inGuestCode.decrementAndGet();
                }
                if (delivered.get() == 64) ctx.self().stop();
            });
            int producers = 8;
            var ready = new CountDownLatch(producers);
            var release = new CountDownLatch(1);
            List<Thread> senders = new ArrayList<>();
            for (int p = 0; p < producers; p++) {
                int producer = p;
                senders.add(Thread.ofPlatform().start(() -> {
                    ready.countDown();
                    try {
                        release.await(5, TimeUnit.SECONDS);
                        for (int i = 0; i < 8; i++) actor.send(producer * 8 + i);
                    } catch (Exception failure) {
                        throw new AssertionError(failure);
                    }
                }));
            }
            assertTrue(ready.await(5, TimeUnit.SECONDS));
            release.countDown();
            for (Thread sender : senders) sender.join();
            assertTrue(actor.awaitTermination(10, TimeUnit.SECONDS));
            assertEquals(64, delivered.get(), "mailbox must never drop admitted messages");
            assertEquals(1, maximum.get(),
                    "one shared actor must never have two concurrent guest stacks");
            assertTrue(actor.failure().isEmpty());
        }
    }

    @Test
    void mailboxTransportFreezesDataNotMutableHostAliases() throws Exception {
        var mutable = new ArrayList<String>();
        mutable.add("before");
        CountDownLatch observed = new CountDownLatch(1);
        java.util.concurrent.atomic.AtomicReference<List<String>> delivered =
                new java.util.concurrent.atomic.AtomicReference<>();
        try (ActorRuntime runtime = new ActorRuntime(IsolatePolicy.developer())) {
            var actor = runtime.spawnShared(() -> (List<String> message,
                    ActorRuntime.ActorContext<List<String>> ctx) -> {
                delivered.set(message);
                observed.countDown();
                ctx.self().stop();
            });
            actor.send(mutable);
            mutable.set(0, "after");
            mutable.add("extra");
            assertTrue(observed.await(5, TimeUnit.SECONDS));
            assertEquals(List.of("before"), delivered.get());
            assertThrows(UnsupportedOperationException.class,
                    () -> delivered.get().add("guest-mutation"));
            assertTrue(actor.awaitTermination(5, TimeUnit.SECONDS));
        }
    }

    @Test
    void arbitraryOpaqueMutableHostObjectsCannotCrossMailboxBoundary() throws Exception {
        try (ActorRuntime runtime = new ActorRuntime(IsolatePolicy.developer())) {
            var actor = runtime.spawnShared(() -> (Object message,
                    ActorRuntime.ActorContext<Object> ctx) -> ctx.self().stop());
            assertThrows(IllegalArgumentException.class,
                    () -> actor.send(new StringBuilder("unowned")));
            actor.stop();
            assertTrue(actor.awaitTermination(5, TimeUnit.SECONDS));
        }
    }

    @Test
    void immutableSharedValueHasStablePayloadDuringRuntimeClose() {
        ActorRuntime runtime = new ActorRuntime(IsolatePolicy.developer());
        // A data-only immutable snapshot cannot be replaced with null on teardown.
        Object immutable = ActorRuntime.freeze(Map.of("version", 1));
        assertEquals(Map.of("version", 1), immutable);
        runtime.close();
        assertEquals(Map.of("version", 1), immutable);
    }
}
