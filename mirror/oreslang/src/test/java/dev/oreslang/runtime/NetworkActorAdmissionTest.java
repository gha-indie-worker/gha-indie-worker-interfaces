package dev.oreslang.runtime;

import org.junit.jupiter.api.Test;

import java.util.ArrayList;
import java.util.HashSet;
import java.util.List;
import java.util.Set;
import java.util.concurrent.CountDownLatch;
import java.util.concurrent.Executors;
import java.util.concurrent.TimeUnit;
import java.util.concurrent.atomic.AtomicInteger;

import static org.junit.jupiter.api.Assertions.*;

class NetworkActorAdmissionTest {
    @Test
    void twoIndependentLimitsAndDistinctRequestAndMessageIdentities() {
        try (var admission = new NetworkActorAdmission(new NetworkActorAdmission.Limits(2, 2))) {
            var http = admission.open(NetworkActorAdmission.Protocol.HTTP);
            var ws = admission.open(NetworkActorAdmission.Protocol.WS);
            assertThrows(IllegalStateException.class,
                    () -> admission.open(NetworkActorAdmission.Protocol.TCP));

            var first = http.admit("request-A");
            var second = http.admit("request-A");
            assertEquals(first.identity().connectionId(), second.identity().connectionId());
            assertEquals(first.identity().requestId(), second.identity().requestId());
            assertNotEquals(first.identity().messageId(), second.identity().messageId());
            assertEquals(1, first.identity().messageSequence());
            assertEquals(2, second.identity().messageSequence());
            assertThrows(IllegalStateException.class, () -> http.admit("request-B"));
            assertEquals(0, ws.inFlightMessages(), "other connection's quota is independent");

            first.close();
            assertFalse(first.live());
            var third = http.admit("request-B");
            assertEquals(3, third.identity().messageSequence(), "message IDs cannot be reused");
            assertNotEquals(second.identity().requestId(), third.identity().requestId());
            assertEquals(2, http.inFlightMessages());
            ws.close();
            assertEquals(1, admission.activeConnections());
            http.close();
            assertFalse(second.live());
            assertFalse(third.live());
            second.close();
            third.close();
            http.close();
            assertEquals(0, admission.activeConnections());
        }
    }

    @Test
    void cancellationRevokesSuspendedMessagesAndNeverResurrectsThem() {
        var admission = new NetworkActorAdmission(new NetworkActorAdmission.Limits(1, 3));
        var conn = admission.open(NetworkActorAdmission.Protocol.TCP);
        var pending = conn.admit("");
        var pending2 = conn.admit("");
        assertTrue(pending.live());
        conn.close();
        assertFalse(pending.live());
        assertFalse(pending2.live());
        assertThrows(IllegalStateException.class, () -> conn.admit(""));
        var replacement = admission.open(NetworkActorAdmission.Protocol.TCP);
        assertNotEquals(conn.id(), replacement.id());
        pending.close();
        pending2.close();
        assertEquals(0, replacement.inFlightMessages());
        admission.close();
        assertFalse(replacement.id().equals(conn.id()));
        assertThrows(IllegalStateException.class,
                () -> admission.open(NetworkActorAdmission.Protocol.WS));
        replacement.close();
    }

    @Test
    void concurrentProducersCannotOverAdmitOneConnection() throws Exception {
        try (var admission = new NetworkActorAdmission(new NetworkActorAdmission.Limits(1, 5));
             var pool = Executors.newFixedThreadPool(16)) {
            var conn = admission.open(NetworkActorAdmission.Protocol.WS);
            var start = new CountDownLatch(1);
            var done = new CountDownLatch(32);
            var accepted = new AtomicInteger();
            var rejected = new AtomicInteger();
            List<NetworkActorAdmission.Message> held =
                    java.util.Collections.synchronizedList(new ArrayList<>());
            for (int i = 0; i < 32; i++) {
                pool.execute(() -> {
                    try {
                        if (!start.await(5, TimeUnit.SECONDS)) return;
                        try {
                            held.add(conn.admit(""));
                            accepted.incrementAndGet();
                        } catch (IllegalStateException atLimit) {
                            rejected.incrementAndGet();
                        }
                    } catch (InterruptedException interrupted) {
                        Thread.currentThread().interrupt();
                    } finally {
                        done.countDown();
                    }
                });
            }
            start.countDown();
            assertTrue(done.await(10, TimeUnit.SECONDS));
            assertEquals(5, accepted.get());
            assertEquals(27, rejected.get());
            assertEquals(5, conn.inFlightMessages());
            Set<java.util.UUID> ids = new HashSet<>();
            for (var m : held) {
                assertTrue(m.live());
                assertTrue(ids.add(m.identity().messageId()));
                m.close();
            }
            assertEquals(0, conn.inFlightMessages());
            assertTrue(conn.admit("").live());
        }
    }

    @Test
    void requestIdsAreBoundedAndMessageCloseIsIdempotent() {
        try (var admission = new NetworkActorAdmission(new NetworkActorAdmission.Limits(1, 1))) {
            var conn = admission.open(NetworkActorAdmission.Protocol.HTTP);
            assertThrows(IllegalArgumentException.class, () -> conn.admit("x".repeat(257)));
            var message = conn.admit("stream-1");
            assertEquals(NetworkActorAdmission.Protocol.HTTP, message.identity().protocol());
            assertEquals("stream-1", message.identity().requestId());
            message.close();
            message.close();
            assertFalse(message.live());
            assertEquals(0, conn.inFlightMessages());
        }
    }
}
