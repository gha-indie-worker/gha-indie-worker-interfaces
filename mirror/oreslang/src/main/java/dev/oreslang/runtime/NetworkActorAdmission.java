package dev.oreslang.runtime;

import java.util.LinkedHashMap;
import java.util.Map;
import java.util.Objects;
import java.util.UUID;

/**
 * Transport-side admission accounting for a long-lived protocol actor.
 *
 * This is CONTROL/broker state, not guest actor code. In particular, opening
 * or closing a lease never invokes a receive handler or acquires an actor's
 * native execution lease. Delivery still has to enter ActorRuntime's mailbox.
 *
 * A connection scope has a stable identity across HTTP requests / TCP chunks /
 * WebSocket events, and every message has a distinct identity and optional
 * request identity. Neither the scope nor the lease wraps an OS descriptor or
 * is allowed through ordinary actor message serialization.
 *
 * The transport MUST supply a real connection scope; e.g. a JDK HttpExchange
 * alone is not evidence of a persistent HTTP/1.1 connection or HTTP/2 stream.
 */
public final class NetworkActorAdmission implements AutoCloseable {
    public enum Protocol { HTTP, TCP, WS }

    public record Limits(int connections, int messagesPerConnection) {
        public Limits {
            if (connections < 1 || connections > 65536) {
                throw new IllegalArgumentException("connections must be 1..65536");
            }
            if (messagesPerConnection < 1 || messagesPerConnection > 65536) {
                throw new IllegalArgumentException("messages_per_connection must be 1..65536");
            }
        }
    }

    public record Identity(Protocol protocol, UUID connectionId, String requestId,
                           long messageSequence, UUID messageId) {
        public Identity {
            Objects.requireNonNull(protocol);
            Objects.requireNonNull(connectionId);
            Objects.requireNonNull(requestId);
            Objects.requireNonNull(messageId);
            if (messageSequence < 1) throw new IllegalArgumentException("invalid message sequence");
        }
    }

    private final Limits limits;
    private final Map<UUID, Connection> connections = new LinkedHashMap<>();
    private boolean closed;

    public NetworkActorAdmission(Limits limits) {
        this.limits = Objects.requireNonNull(limits);
    }

    public synchronized int activeConnections() { return connections.size(); }
    public synchronized boolean closed() { return closed; }

    public synchronized Connection open(Protocol protocol) {
        Objects.requireNonNull(protocol);
        if (closed) throw new IllegalStateException("network actor admission is closed");
        if (connections.size() >= limits.connections()) {
            throw new IllegalStateException("network actor connection capacity exceeded");
        }
        Connection connection = new Connection(protocol, UUID.randomUUID());
        connections.put(connection.id, connection);
        return connection;
    }

    public final class Connection implements AutoCloseable {
        private final Protocol protocol;
        private final UUID id;
        private final Map<Long, Message> activeMessages = new LinkedHashMap<>();
        private long nextSequence = 1;
        private boolean retired;

        private Connection(Protocol protocol, UUID id) {
            this.protocol = protocol;
            this.id = id;
        }

        public Protocol protocol() { return protocol; }
        public UUID id() { return id; }

        public int inFlightMessages() {
            synchronized (NetworkActorAdmission.this) { return activeMessages.size(); }
        }

        /**
         * A HTTP request can produce several distinct events, all correlated by
         * requestId; different requests can coexist on the same connection.
         * TCP/WS callers may use an empty requestId when no request exists.
         */
        public Message admit(String requestId) {
            Objects.requireNonNull(requestId, "requestId");
            if (requestId.length() > 256) throw new IllegalArgumentException("request id too long");
            synchronized (NetworkActorAdmission.this) {
                if (closed || retired || connections.get(id) != this) {
                    throw new IllegalStateException("connection admission revoked");
                }
                if (activeMessages.size() >= limits.messagesPerConnection()) {
                    throw new IllegalStateException("per-connection message capacity exceeded");
                }
                if (nextSequence == Long.MAX_VALUE) {
                    throw new IllegalStateException("connection message sequence exhausted");
                }
                long sequence = nextSequence++;
                Identity identity = new Identity(protocol, id, requestId, sequence, UUID.randomUUID());
                Message message = new Message(this, identity);
                activeMessages.put(sequence, message);
                return message;
            }
        }

        @Override public void close() {
            synchronized (NetworkActorAdmission.this) {
                if (retired) return;
                retired = true;
                connections.remove(id, this);
                // Revoke each outstanding message (including suspended work)
                // before releasing the connection slot. Late completions fail.
                for (Message message : activeMessages.values()) message.retired = true;
                activeMessages.clear();
            }
        }
    }

    public final class Message implements AutoCloseable {
        private final Connection connection;
        private final Identity identity;
        private boolean retired;

        private Message(Connection connection, Identity identity) {
            this.connection = connection;
            this.identity = identity;
        }

        public Identity identity() { return identity; }

        public boolean live() {
            synchronized (NetworkActorAdmission.this) {
                return !closed && !retired && !connection.retired
                        && connections.get(connection.id) == connection
                        && connection.activeMessages.get(identity.messageSequence()) == this;
            }
        }

        @Override public void close() {
            synchronized (NetworkActorAdmission.this) {
                if (retired) return;
                retired = true;
                connection.activeMessages.remove(identity.messageSequence(), this);
            }
        }
    }

    @Override public synchronized void close() {
        if (closed) return;
        closed = true;
        for (Connection connection : java.util.List.copyOf(connections.values())) {
            connection.close();
        }
    }
}
