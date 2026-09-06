#![forbid(unsafe_code)]

//! Serde mirror of the `webhooks` slice.

use serde::{Deserialize, Serialize};
use serde_json::Value;

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum DeliveryStatus {
    Received,
    Verified,
    Rejected,
    Dispatched,
    Duplicate,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RegistryEventAction {
    Push,
    Delete,
    Mirror,
}

/// The envelope for one GitHub webhook delivery.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct GitHubDelivery {
    pub id: String,
    pub delivery_id: String,
    pub event: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub action: Option<String>,
    pub repository: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub head_sha: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub workflow_path: Option<String>,
    pub signature_valid: bool,
    pub status: DeliveryStatus,
    pub received_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub rejection_reason: Option<String>,
    pub payload: Value,
}

/// A normalized container-registry image event.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RegistryImageEvent {
    pub id: String,
    pub registry: String,
    pub repository: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub tag: Option<String>,
    pub digest: String,
    pub action: RegistryEventAction,
    pub occurred_at: String,
    pub received_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub source_delivery_id: Option<String>,
    pub payload: Value,
}

impl DeliveryStatus {
    /// `rejected` and `duplicate` never dispatch work.
    #[must_use]
    pub const fn dispatches_work(self) -> bool {
        matches!(self, DeliveryStatus::Dispatched)
    }

    /// A delivery in a terminal status is never retried.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            DeliveryStatus::Rejected | DeliveryStatus::Dispatched | DeliveryStatus::Duplicate
        )
    }
}

/// `owner/repo` with no path traversal and no empty half.
#[must_use]
pub fn is_repository_slug(value: &str) -> bool {
    let mut parts = value.split('/');
    let (Some(owner), Some(repo), None) = (parts.next(), parts.next(), parts.next()) else {
        return false;
    };
    let ok = |s: &str| {
        !s.is_empty()
            && s.len() <= 100
            && s.bytes()
                .all(|b| b.is_ascii_alphanumeric() || b == b'.' || b == b'_' || b == b'-')
    };
    ok(owner) && ok(repo)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn repository_slug_rejects_traversal_and_bare_names() {
        assert!(is_repository_slug("gha-indie-worker/gha-clone-server.rs"));
        assert!(!is_repository_slug("gha-clone-server.rs"));
        assert!(!is_repository_slug("owner/repo/extra"));
        assert!(!is_repository_slug("owner/"));
        assert!(!is_repository_slug("../etc/passwd"));
    }

    #[test]
    fn only_dispatched_deliveries_start_work() {
        assert!(DeliveryStatus::Dispatched.dispatches_work());
        assert!(!DeliveryStatus::Verified.dispatches_work());
        assert!(!DeliveryStatus::Duplicate.dispatches_work());
        assert!(DeliveryStatus::Duplicate.is_terminal());
        assert!(!DeliveryStatus::Received.is_terminal());
    }
}
