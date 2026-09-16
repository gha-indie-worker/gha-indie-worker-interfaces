import "./ores.tsp";
import "@typespec/json-schema";
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

@doc("One websocket command as received. The union is tagged by kind; fields not named by that kind are absent.")
@TypeSpec.JsonSchema.extension("oneOf", #[
  #{ properties: #{ kind: #{ `const`: "subscribe-run-logs" } }, required: #["kind", "runId"] },
  #{ properties: #{ kind: #{ `const`: "unsubscribe-run-logs" } }, required: #["kind", "runId"] },
  #{ properties: #{ kind: #{ `const`: "presence" } }, required: #["kind", "channel"] },
  #{ properties: #{ kind: #{ `const`: "chat-event" } }, required: #["kind", "sessionId", "payload"] },
  #{ properties: #{ kind: #{ `const`: "heartbeat" } }, required: #["kind"] }
])
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

@doc("One websocket event as emitted. The union is tagged by kind; fields not named by that kind are absent.")
@TypeSpec.JsonSchema.extension("oneOf", #[
  #{ properties: #{ kind: #{ `const`: "run-log" } }, required: #["kind", "runId", "jobId", "payload"] },
  #{ properties: #{ kind: #{ `const`: "presence" } }, required: #["kind", "payload"] },
  #{ properties: #{ kind: #{ `const`: "chat-event" } }, required: #["kind", "sessionId", "payload"] },
  #{ properties: #{ kind: #{ `const`: "heartbeat-ack" } }, required: #["kind"] },
  #{ properties: #{ kind: #{ `const`: "error" } }, required: #["kind", "message"] }
])
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

@doc("One frame on the stateful TCP avenue. payloadBytes is the decoded length and is bounded by the codec.")
@TypeSpec.JsonSchema.extension("oneOf", #[
  #{ properties: #{ kind: #{ `const`: "hello" } }, required: #["kind", "payload"] },
  #{ properties: #{ kind: #{ `const`: "command" } }, required: #["kind", "payload"] },
  #{ properties: #{ kind: #{ `const`: "event" } }, required: #["kind", "payload"] },
  #{ properties: #{ kind: #{ `const`: "ping" } }, required: #["kind"] },
  #{ properties: #{ kind: #{ `const`: "pong" } }, required: #["kind"] },
  #{ properties: #{ kind: #{ `const`: "bye" } }, required: #["kind"] }
])
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
