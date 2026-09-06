import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Transport;

@doc("Client to server commands on the websocket avenue. The kind field is the union discriminator.")
enum WsCommandKind {
  SubscribeRunLogs: "subscribe-run-logs",
  UnsubscribeRunLogs: "unsubscribe-run-logs",
  Presence: "presence",
  ChatEvent: "chat-event",
  Heartbeat: "heartbeat",
}

@doc("Server to client events on the websocket avenue. The kind field is the union discriminator.")
enum WsEventKind {
  RunLog: "run-log",
  Presence: "presence",
  ChatEvent: "chat-event",
  HeartbeatAck: "heartbeat-ack",
  Error: "error",
}

@doc("Frame kinds on the stateful TCP avenue. Every frame is length-prefixed JSON.")
enum TcpFrameKind {
  Hello: "hello",
  Command: "command",
  Event: "event",
  Ping: "ping",
  Pong: "pong",
  Bye: "bye",
}

// Discriminated by `kind`. The peer JSON Schema authority states the same table as
// a oneOf over `kind` consts; TypeSpec has no sum type inside the contract subset,
// so the requirement is stated here and enforced by the runtime validator:
//
//   subscribe-run-logs   requires runId
//   unsubscribe-run-logs requires runId
//   presence             requires channel
//   chat-event           requires sessionId, payload
//   heartbeat            requires nothing beyond the envelope
@doc("One websocket command as received. The union is tagged by kind; fields not named by that kind are absent.")
@Ores.table("ws_command_audit")
@Ores.unique("connectionId,clientSequence")
@Ores.index("kind,sentAt")
model WsCommand {
  @key id: uuid;
  kind: WsCommandKind;
  connectionId: uuid;
  correlationId: uuid;

  @minValue(0) clientSequence: int64;

  runId?: uuid;
  jobId?: uuid;
  sessionId?: uuid;

  @maxLength(120) channel?: string;

  @minValue(0) cursor?: int64;

  payload?: json;
  sentAt: utcDateTime;
}

// Discriminated by `kind`:
//
//   run-log        requires runId, jobId, payload
//   presence       requires payload
//   chat-event     requires sessionId, payload
//   heartbeat-ack  requires nothing beyond the envelope
//   error          requires message
@doc("One websocket event as emitted. The union is tagged by kind; fields not named by that kind are absent.")
@Ores.table("ws_event_audit")
@Ores.unique("connectionId,serverSequence")
@Ores.index("kind,emittedAt")
model WsEvent {
  @key id: uuid;
  kind: WsEventKind;
  connectionId: uuid;
  correlationId?: uuid;

  @minValue(0) serverSequence: int64;

  runId?: uuid;
  jobId?: uuid;
  sessionId?: uuid;

  @maxLength(200) message?: string;

  payload?: json;
  emittedAt: utcDateTime;
}

// Discriminated by `kind`: hello, command and event require payload; ping, pong
// and bye are payload-free. The codec bounds payloadBytes at 1 MiB.
@doc("One frame on the stateful TCP avenue. payloadBytes is the decoded length and is bounded by the codec.")
@Ores.table("tcp_frame_audit")
@Ores.unique("connectionId,sequence")
@Ores.index("kind,receivedAt")
model TcpFrame {
  @key id: uuid;
  kind: TcpFrameKind;
  connectionId: uuid;

  @minValue(0) sequence: int64;

  @minValue(0)
  @maxValue(1048576) payloadBytes: int32;

  payload?: json;
  receivedAt: utcDateTime;
}
