#![forbid(unsafe_code)]

//! Serde mirror of the `transport` slice: the websocket and stateful-TCP frames.
//!
//! Both authorities express the frames as sealed objects with a `kind`
//! discriminator, because the contract subset carries no sum type. The real
//! tagged unions are [`WsCommandBody`] and [`WsEventBody`]: they are derived from
//! the flat frame with `TryFrom`, and that conversion is exactly the per-kind
//! requirement the JSON Schema authority states as `oneOf`.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WsCommandKind {
    SubscribeRunLogs,
    UnsubscribeRunLogs,
    Presence,
    ChatEvent,
    Heartbeat,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WsEventKind {
    RunLog,
    Presence,
    ChatEvent,
    HeartbeatAck,
    Error,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum TcpFrameKind {
    Hello,
    Command,
    Event,
    Ping,
    Pong,
    Bye,
}

/// One websocket command as received.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WsCommand {
    pub id: String,
    pub kind: WsCommandKind,
    pub connection_id: String,
    pub correlation_id: String,
    pub client_sequence: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub channel: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cursor: Option<i64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    pub sent_at: String,
}

/// One websocket event as emitted.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct WsEvent {
    pub id: String,
    pub kind: WsEventKind,
    pub connection_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub correlation_id: Option<String>,
    pub server_sequence: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub run_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub session_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub message: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    pub emitted_at: String,
}

/// One frame on the stateful TCP avenue.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct TcpFrame {
    pub id: String,
    pub kind: TcpFrameKind,
    pub connection_id: String,
    pub sequence: i64,
    pub payload_bytes: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub payload: Option<Value>,
    pub received_at: String,
}

/// A field the frame's `kind` requires but that was absent.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct MissingField {
    pub kind: &'static str,
    pub field: &'static str,
}

impl core::fmt::Display for MissingField {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        write!(f, "{} frame requires {}", self.kind, self.field)
    }
}

impl std::error::Error for MissingField {}

/// The websocket command union, once the per-kind requirements hold.
#[derive(Clone, Debug, PartialEq)]
pub enum WsCommandBody {
    SubscribeRunLogs { run_id: String, cursor: Option<i64> },
    UnsubscribeRunLogs { run_id: String },
    Presence { channel: String },
    ChatEvent { session_id: String, payload: Value },
    Heartbeat,
}

impl TryFrom<&WsCommand> for WsCommandBody {
    type Error = MissingField;

    fn try_from(frame: &WsCommand) -> Result<Self, Self::Error> {
        let need = |value: Option<&String>, kind, field| {
            value.cloned().ok_or(MissingField { kind, field })
        };
        Ok(match frame.kind {
            WsCommandKind::SubscribeRunLogs => WsCommandBody::SubscribeRunLogs {
                run_id: need(frame.run_id.as_ref(), "subscribe-run-logs", "runId")?,
                cursor: frame.cursor,
            },
            WsCommandKind::UnsubscribeRunLogs => WsCommandBody::UnsubscribeRunLogs {
                run_id: need(frame.run_id.as_ref(), "unsubscribe-run-logs", "runId")?,
            },
            WsCommandKind::Presence => WsCommandBody::Presence {
                channel: need(frame.channel.as_ref(), "presence", "channel")?,
            },
            WsCommandKind::ChatEvent => WsCommandBody::ChatEvent {
                session_id: need(frame.session_id.as_ref(), "chat-event", "sessionId")?,
                payload: frame.payload.clone().ok_or(MissingField {
                    kind: "chat-event",
                    field: "payload",
                })?,
            },
            WsCommandKind::Heartbeat => WsCommandBody::Heartbeat,
        })
    }
}

/// The websocket event union, once the per-kind requirements hold.
#[derive(Clone, Debug, PartialEq)]
pub enum WsEventBody {
    RunLog {
        run_id: String,
        job_id: String,
        payload: Value,
    },
    Presence {
        payload: Value,
    },
    ChatEvent {
        session_id: String,
        payload: Value,
    },
    HeartbeatAck,
    Error {
        message: String,
    },
}

impl TryFrom<&WsEvent> for WsEventBody {
    type Error = MissingField;

    fn try_from(frame: &WsEvent) -> Result<Self, MissingField> {
        let need = |value: Option<&String>, kind, field| {
            value.cloned().ok_or(MissingField { kind, field })
        };
        let payload = |kind| {
            frame.payload.clone().ok_or(MissingField {
                kind,
                field: "payload",
            })
        };
        Ok(match frame.kind {
            WsEventKind::RunLog => WsEventBody::RunLog {
                run_id: need(frame.run_id.as_ref(), "run-log", "runId")?,
                job_id: need(frame.job_id.as_ref(), "run-log", "jobId")?,
                payload: payload("run-log")?,
            },
            WsEventKind::Presence => WsEventBody::Presence {
                payload: payload("presence")?,
            },
            WsEventKind::ChatEvent => WsEventBody::ChatEvent {
                session_id: need(frame.session_id.as_ref(), "chat-event", "sessionId")?,
                payload: payload("chat-event")?,
            },
            WsEventKind::HeartbeatAck => WsEventBody::HeartbeatAck,
            WsEventKind::Error => WsEventBody::Error {
                message: need(frame.message.as_ref(), "error", "message")?,
            },
        })
    }
}

impl TcpFrameKind {
    pub const ALL: [TcpFrameKind; 6] = [
        TcpFrameKind::Hello,
        TcpFrameKind::Command,
        TcpFrameKind::Event,
        TcpFrameKind::Ping,
        TcpFrameKind::Pong,
        TcpFrameKind::Bye,
    ];

    /// `hello`, `command` and `event` carry a payload; the rest are bare.
    #[must_use]
    pub const fn carries_payload(self) -> bool {
        matches!(
            self,
            TcpFrameKind::Hello | TcpFrameKind::Command | TcpFrameKind::Event
        )
    }
}

impl TcpFrame {
    /// Whether the payload presence agrees with the frame kind.
    #[must_use]
    pub const fn payload_matches_kind(&self) -> bool {
        self.kind.carries_payload() == self.payload.is_some()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn command(kind: WsCommandKind) -> WsCommand {
        WsCommand {
            id: "c1".into(),
            kind,
            connection_id: "conn-1".into(),
            correlation_id: "corr-1".into(),
            client_sequence: 1,
            run_id: None,
            job_id: None,
            session_id: None,
            channel: None,
            cursor: None,
            payload: None,
            sent_at: "2026-03-14T09:26:53Z".into(),
        }
    }

    #[test]
    fn subscribe_without_run_id_is_rejected() {
        let err = WsCommandBody::try_from(&command(WsCommandKind::SubscribeRunLogs)).unwrap_err();
        assert_eq!(err.field, "runId");
        assert_eq!(err.kind, "subscribe-run-logs");
    }

    #[test]
    fn heartbeat_needs_nothing_beyond_the_envelope() {
        assert_eq!(
            WsCommandBody::try_from(&command(WsCommandKind::Heartbeat)).unwrap(),
            WsCommandBody::Heartbeat
        );
    }

    #[test]
    fn chat_event_needs_session_and_payload() {
        let mut frame = command(WsCommandKind::ChatEvent);
        assert!(WsCommandBody::try_from(&frame).is_err());
        frame.session_id = Some("sess-1".into());
        assert_eq!(
            WsCommandBody::try_from(&frame).unwrap_err().field,
            "payload"
        );
        frame.payload = Some(json!({ "body": "hi" }));
        assert!(WsCommandBody::try_from(&frame).is_ok());
    }

    #[test]
    fn payload_presence_matches_frame_kind() {
        for kind in TcpFrameKind::ALL {
            let frame = TcpFrame {
                id: "f1".into(),
                kind,
                connection_id: "conn-1".into(),
                sequence: 0,
                payload_bytes: 0,
                payload: kind.carries_payload().then_some(json!({})),
                received_at: "2026-03-14T09:26:53Z".into(),
            };
            assert!(frame.payload_matches_kind(), "{kind:?}");
        }
    }
}
