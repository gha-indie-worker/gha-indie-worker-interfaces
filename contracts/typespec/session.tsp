// ============================================================================
// TypeSpec authority — live session protocol.
//
// PEER of contracts/json-schema/session.schema.json. Neither generated.
// Mirrors lib-core `runtime::session`: one Frame vocabulary, two encodings.
//
//   TCP        4-byte big-endian length prefix + the binary encoding.
//   WebSocket  the same frames as JSON text — this file describes that JSON.
//
// Three invariants are load-bearing and are expressed here, not just in prose:
//   1. Backpressure is explicit — the server may only send as many Event frames
//      as the subscriber has granted via Subscribe.credit + Credit.additional.
//   2. Resumption is exact — every Event carries a monotonic per-stream
//      `sequence`, and Hello.resume names where the client got to.
//   3. Liveness is a state machine — Ping/Pong carry a nonce, and a missed
//      heartbeat has a defined outcome rather than an open-ended wait.
//
// Integer bound: `runtime::session` uses u64 for nonces, sequences and command
// ids. The JSON encoding cannot carry a u64 exactly, so the JSON transport is
// bounded at 2^53-1 (IEEE-754 exact integers) in BOTH authorities. The TCP
// binary encoding carries the full u64; a peer that would exceed the JSON
// bound must close with `protocol` rather than round-trip a wrong number.
// ============================================================================

namespace GhaIndieWorker.Session;

// Wire protocol version. Bumped only for an incompatible change; Hello negotiates it.
const PROTOCOL_VERSION = 1;

// Largest frame encoded or accepted on either transport. An unbounded length prefix is a trivial memory exhaustion.
const MAX_FRAME_BYTES = 1048576;

@doc("Client's opening frame. `resume` carries a session id from a previous connection.")
model HelloFrame {
  kind: "hello";

  @minValue(1) @maxValue(65535) version: int32;

  @doc("Session id to resume. Absent on a fresh connection.")
  @minLength(1)
  @maxLength(128)
  resume?: string;
}

@doc("Server's answer to Hello. `resumed` is false when the named session could not be resumed.")
model WelcomeFrame {
  kind: "welcome";
  @minLength(1) @maxLength(128) session: string;
  resumed: boolean;

  @doc("How often the peer should expect a Ping. Zero disables heartbeating.")
  @minValue(0)
  @maxValue(3600)
  heartbeatSeconds: int32;
}

@doc("Liveness probe. The peer must answer with Pong carrying the same nonce.")
model PingFrame {
  kind: "ping";
  @minValue(0) @maxValue(9007199254740991) nonce: int64;
}

@doc("Answer to Ping. A Pong with an unexpected nonce is a protocol error.")
model PongFrame {
  kind: "pong";
  @minValue(0) @maxValue(9007199254740991) nonce: int64;
}

@doc("Subscribe to a stream from a sequence, granting an initial credit.")
model SubscribeFrame {
  kind: "subscribe";
  @minLength(1) @maxLength(256) stream: string;

  @doc("First sequence wanted. 0 means from the beginning of what is retained.")
  @minValue(0)
  @maxValue(9007199254740991)
  from: int64;

  @doc("Events the server may send before it must wait for more credit.")
  @minValue(0)
  @maxValue(65535)
  credit: int32;
}

@doc("Stop delivery of a stream. Outstanding credit is discarded.")
model UnsubscribeFrame {
  kind: "unsubscribe";
  @minLength(1) @maxLength(256) stream: string;
}

@doc("Grant more credit on an existing subscription. This is the only way the server earns permission to send.")
model CreditFrame {
  kind: "credit";
  @minLength(1) @maxLength(256) stream: string;
  @minValue(1) @maxValue(65535) additional: int32;
}

@doc("One event on a stream. Consumes exactly one unit of credit.")
model EventFrame {
  kind: "event";
  @minLength(1) @maxLength(256) stream: string;

  @doc("Monotonic per stream. A client resumes by naming the next sequence it wants.")
  @minValue(0)
  @maxValue(9007199254740991)
  sequence: int64;

  @doc("Opaque payload, base64 in the JSON encoding.")
  payload: bytes;
}

@doc("The subscriber fell too far behind and the server dropped events. Told, never hidden.")
model LaggedFrame {
  kind: "lagged";
  @minLength(1) @maxLength(256) stream: string;

  @doc("The sequence delivery jumped to. Everything between the last Event and this is gone.")
  @minValue(0)
  @maxValue(9007199254740991)
  skippedTo: int64;
}

@doc("A request from the client. Answered by exactly one Ack or Error with the same id.")
model CommandFrame {
  kind: "command";
  @minValue(0) @maxValue(9007199254740991) id: int64;
  @minLength(1) @maxLength(64) @pattern("^[a-z][a-z0-9_.]*$") verb: string;
  payload: bytes;
}

@doc("Successful answer to a Command.")
model AckFrame {
  kind: "ack";
  @minValue(0) @maxValue(9007199254740991) id: int64;
  payload: bytes;
}

@doc("Failed answer to a Command, or a connection-level failure when `id` is absent.")
model ErrorFrame {
  kind: "error";

  @doc("The Command this answers. Absent for a connection-level error.")
  @minValue(0)
  @maxValue(9007199254740991)
  id?: int64;

  code: GhaIndieWorker.ErrorCode;
  @minLength(1) @maxLength(1024) message: string;
}

@doc("The sender is going away. No further frames are sent or read.")
model CloseFrame {
  kind: "close";
  code: GhaIndieWorker.ErrorCode;
  @minLength(0) @maxLength(1024) message: string;
}

@doc("Every frame on the wire. Discriminated by `kind`; the JSON encoding is a closed union.")
@discriminated(#{ envelope: "none", discriminatorPropertyName: "kind" })
union Frame {
  hello: HelloFrame,
  welcome: WelcomeFrame,
  ping: PingFrame,
  pong: PongFrame,
  subscribe: SubscribeFrame,
  unsubscribe: UnsubscribeFrame,
  credit: CreditFrame,
  event: EventFrame,
  lagged: LaggedFrame,
  command: CommandFrame,
  ack: AckFrame,
  error: ErrorFrame,
  close: CloseFrame,
}
