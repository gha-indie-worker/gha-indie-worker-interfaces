#![forbid(unsafe_code)]

//! Serde mirror of the `chat` slice (ores-chat surfaces).

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatSurface {
    Visitor,
    Customer,
    Internal,
    Owner,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatRole {
    User,
    Agent,
    System,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ChatSessionStatus {
    Open,
    Assigned,
    Resolved,
    Abandoned,
}

/// One chat conversation.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatSession {
    pub id: String,
    pub surface: ChatSurface,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub visitor_token: Option<String>,
    pub status: ChatSessionStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub subject: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_to: Option<String>,
    pub started_at: String,
    pub last_activity_at: String,
}

/// One message in a session.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ChatMessage {
    pub id: String,
    pub session_id: String,
    pub sequence: i64,
    pub role: ChatRole,
    pub body: String,
    pub redacted: bool,
    pub attachments: Vec<String>,
    pub sent_at: String,
}

impl ChatSurface {
    pub const ALL: [ChatSurface; 4] = [
        ChatSurface::Visitor,
        ChatSurface::Customer,
        ChatSurface::Internal,
        ChatSurface::Owner,
    ];

    /// Whether the surface is reachable without an authenticated actor.
    #[must_use]
    pub const fn is_anonymous(self) -> bool {
        matches!(self, ChatSurface::Visitor)
    }

    /// Whether the surface is staff-only and never rendered to a customer.
    #[must_use]
    pub const fn is_staff_only(self) -> bool {
        matches!(self, ChatSurface::Internal | ChatSurface::Owner)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn surfaces_partition_into_anonymous_customer_and_staff() {
        for surface in ChatSurface::ALL {
            assert!(!(surface.is_anonymous() && surface.is_staff_only()));
        }
        assert!(ChatSurface::Visitor.is_anonymous());
        assert!(!ChatSurface::Customer.is_anonymous());
        assert!(ChatSurface::Owner.is_staff_only());
    }
}
