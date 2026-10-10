# Network actors: connection scopes, request identities, and exclusive native execution

Status: **DRAFT design + partial runtime implementation** (PR #519).
Do not claim `define http actor`, `receive_http`, `receive_tcp`, or `receive_ws`
compile yet. Do not merge until executable JVM/socket/AOT proof is green.

## Invariants

1. For every SHARED/PRIVATE/UNTRUSTED actor, at most **one native carrier
   thread** may execute **any** actor code at a time (including startup,
   continuations, native-backed callbacks, actor-owned state, cleanup).
   Native I/O workers perform transport work, not actor receive callbacks.
2. Message *concurrency* is unfinished lifetimes, not native parallelism.
   `await` releases the lease; each continuation must reenter the actor's
   mailbox and reacquire exclusive carrier authority. Physical carrier may
   change between turns.
3. Network actors have **two independent admission controls**:
   `connections` (active transport connection scopes per actor) and
   `messages_per_connection` (unfinished message/event handlers per scope).
   These are not an execution-thread count. A protocol adapter must identify
   a real connection, not invent one from a request.
4. HTTP adds a distinct *request* identity beneath connection identity:
   `(actor incarnation, connection ID, request/stream ID, event ID)`.
   Several events may belong to one request, and HTTP/2 permits several
   concurrent requests on one connection. For HTTP/1.1, keepalive may
   reuse one connection for many sequential requests. Thus a separately
   configurable `requests_per_connection` may become necessary. An HTTP
   request must never be inferred from an OS file descriptor alone.
5. For TCP, `messages_per_connection` counts decoded application messages
   or bounded input-chunk events, NOT arbitrary `read()` syscall counts.
   For WS, it counts logical messages after fragmentation/reassembly; control
   frames and close signals need an independent priority/control budget.
6. Response output must be scoped to the **request** (or stream), never
   resolved from a mutable actor-wide `current_response` slot. A later
   message on the same actor must not redirect an older continuation's output.
   TCP/WS outbound operations bind to the connection generation and message/
   stream scope as appropriate. Cancellation/hot reload revokes stale grants.
7. The parent/daddy remains the transport broker. It alone owns listener,
   TLS state, HTTP/2 multiplexing, kernel client socket and descriptor
   lifecycle. **No client FD or raw JVM pointer** enters UNTRUSTED actors.
   Untrusted receives bounded copied/serialized request/message fields and
   produces bounded response-command frames; the broker validates and writes.
   Any future worker-specific FD transfer must be separately proven.

## Proposed source syntax (NOT YET IMPLEMENTED)

```oreslang
define http actor Pages as
  int connections: 100
  int messages_per_connection: 8

  receive_http(m: HttpMessage) {
    // m.connection_id / m.request_id / m.message_id are immutable.
    const req = m.get_request()
    const res = m.get_response()  // request-scoped, revocable response stream
    await res.start(200, "text/plain")
    await res.write(req.path)
    await res.finish()
  }
end

define tcp actor Ingest as
  int connections: 1000
  int messages_per_connection: 4

  receive_tcp(m: TcpMessage) {
    const conn = m.get_tcp_conn()
    // No conn.on("message", fn) callback: receive_tcp is the sole
    // guest-code entrypoint; the broker emits further mailbox messages.
    await conn.write(m.bytes)
  }
end

define ws actor Chat as
  int connections: 200
  int messages_per_connection: 16

  receive_ws(m: WsMessage) {
    const conn = m.get_ws_conn()
    await conn.send_text(m.text)
  }
end
```

Source-level `connection` means a broker-authenticated session, not a copied
socket pointer. `get_response()` is a request-scope capability, not ambient
actor state, and is available only on messages belonging to the same request.
For an HTTP request with multiple chunk/events, the binder checks that the
current message's **request ID matches** the response stream's request ID.
The response should serialize writes in-order under backpressure; per-request
simultaneous writer races must reject or use explicit sequencing. Event
callbacks must never run inline on transport worker threads.

## Current implemented substrate in PR #519

- `NetworkActorAdmission` enforces connection and per-connection unfinished
  message limits independently, issues unique connection/message identities,
  correlates multiple messages to one request ID, and revokes late messages
  on close. It does **not** yet bind transport sockets, actor dispatch, or
  source syntax. It stores no FDs/host object pointers.
- `NativeHttp` still uses the existing JDK HTTP/1.1 exchange broker.
  `move_to_reusable(actor)` and `handler.dispatch_to(exchange, actor)`
  allow multiple independent requests to flow into one existing SHARED/PRIVATE
  actor. They do **not** imply the JDK backend exposes connection identity or
  offers true physical FD transfer. Reusable requests release server admission
  when transport closes, not when the long-lived actor dies; deadlines abort
  only the affected request rather than cancelling the whole reusable actor.
- `http.claim(mail.value)` now requires the current *admitted* actor mail's
  transfer ticket. Once claimed, all exchange operations require the same
  mail sequence even when its async continuation resumes on another native
  carrier. This is **per-message authority**, deliberately stricter than
  a fully realized *multi-event per-request* capability. Generalized typed
  request-keyed events and cross-event response grants are pending.
- Existing `http.handler(...).dispatch(exchange)` still creates one actor
  per HTTP request and waits for that actor's retirement; it is unchanged.
  No untrusted exchange or physical client FD transfer is permitted.

## Missing required work / threat cases

- Implement parsing, typechecking, AOT ABI and method lowering for protocol
  actor declarations and typed `HttpMessage/TcpMessage/WsMessage` envelopes.
- Wire actual transport connection IDs to `NetworkActorAdmission` and bind
  every actor-dispatched event to a broker-issued, unforgeable request/session
  principal (not a mutable user-supplied token). Current string-ticket legacy
  path is **not** sufficient to prove semantic association against a caller
  intentionally misrouting valid tickets.
- Support request-scoped multi-event response authority while prohibiting
  cross-request aliasing, stale generation access, and guessed-token claims.
- Implement native TCP/WS framing adapters, socket close/half-close,
  backpressure and cancellation; do not mistake JDK HttpExchange for FD
  passing, HTTP/2, WS upgrade, or secure cross-process isolation.
- UNTRUSTED: enforce byte/event/fuel budgets per connection and per request;
  broker-only response command validation; no pointers, reflection or ambient
  I/O; isolate/sandbox implementation and proof.
- Validate HTTP connection reuse/pipelining/multiplexing, write sequencing,
  per-message fairness, producer floods, actor termination with 5+ suspended
  requests, stream closure races, actor hot reload and generation fencing.
  Run JVM + Rust bounded models and actual socket/JIT/AOT end-to-end tests.
