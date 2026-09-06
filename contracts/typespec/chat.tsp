import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Chat;

@doc("Which ores-chat surface a session belongs to. visitor is anonymous marketing, owner is the internal owner console.")
enum ChatSurface {
  Visitor: "visitor",
  Customer: "customer",
  Internal: "internal",
  Owner: "owner",
}

enum ChatRole {
  User: "user",
  Agent: "agent",
  System: "system",
}

enum ChatSessionStatus {
  Open: "open",
  Assigned: "assigned",
  Resolved: "resolved",
  Abandoned: "abandoned",
}

@doc("One chat conversation. A visitor session has no org or user until it is claimed by a sign-in.")
@Ores.table("chat_sessions")
@Ores.unique("visitorToken")
@Ores.index("surface,status")
@Ores.index("orgId,lastActivityAt")
model ChatSession {
  @key id: uuid;
  surface: ChatSurface;
  orgId?: uuid;
  userId?: uuid;

  @minLength(16)
  @maxLength(128) visitorToken?: string;

  status: ChatSessionStatus;

  @maxLength(200) subject?: string;

  assignedTo?: uuid;
  startedAt: utcDateTime;
  lastActivityAt: utcDateTime;
}

@doc("One message in a session. Sequence is dense per session so websocket replay is exact.")
@Ores.table("chat_messages")
@Ores.unique("sessionId,sequence")
@Ores.index("sentAt")
model ChatMessage {
  @key id: uuid;
  @Ores.references("ChatSession.id") sessionId: uuid;

  @minValue(0) sequence: int64;

  role: ChatRole;

  @maxLength(8000) body: string;

  redacted: boolean;

  @maxItems(16) attachments: string[];

  sentAt: utcDateTime;
}
