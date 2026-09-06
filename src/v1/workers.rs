#![forbid(unsafe_code)]

//! Serde mirror of the `workers` slice.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum WorkerStatus {
    Registering,
    Idle,
    Leased,
    Draining,
    Offline,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CapabilityKind {
    Profile,
    Arch,
    Os,
    Feature,
}

/// A fixed-profile worker pinned to an immutable image digest.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Worker {
    pub id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_id: Option<String>,
    pub name: String,
    pub status: WorkerStatus,
    pub arch: String,
    pub os: String,
    pub image_digest: String,
    pub max_concurrency: i32,
    pub registered_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_heartbeat_at: Option<String>,
}

/// One asserted capability of a worker.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Capability {
    pub id: String,
    pub worker_id: String,
    pub kind: CapabilityKind,
    pub value: String,
    pub enabled: bool,
}

/// A periodic liveness and load sample from a worker.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Heartbeat {
    pub id: String,
    pub worker_id: String,
    pub observed_at: String,
    pub active_jobs: i32,
    pub queue_depth: i32,
    pub load_average: f64,
    pub version: String,
}

/// An operator-reviewed execution profile.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub id: String,
    pub name: String,
    pub image: String,
    pub image_digest: String,
    pub commands: Vec<String>,
    pub artifact_paths: Vec<String>,
    pub cpu_millis: i32,
    pub memory_mebibytes: i32,
    pub timeout_seconds: i32,
    pub reviewed_at: String,
}

/// `sha256-` followed by 64 lowercase hex characters.
pub const IMAGE_DIGEST_LEN: usize = 71;

/// Whether a digest string is the pinned, immutable form the fleet accepts.
/// A tag such as `latest` is never a digest.
#[must_use]
pub fn is_pinned_digest(value: &str) -> bool {
    value.len() == IMAGE_DIGEST_LEN
        && value.starts_with("sha256-")
        && value[7..]
            .bytes()
            .all(|b| b.is_ascii_hexdigit() && !b.is_ascii_uppercase())
}

impl WorkerStatus {
    pub const ALL: [WorkerStatus; 5] = [
        WorkerStatus::Registering,
        WorkerStatus::Idle,
        WorkerStatus::Leased,
        WorkerStatus::Draining,
        WorkerStatus::Offline,
    ];

    /// Whether the scheduler may hand this worker a new job.
    #[must_use]
    pub const fn is_schedulable(self) -> bool {
        matches!(self, WorkerStatus::Idle)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn only_idle_workers_are_schedulable() {
        let schedulable: Vec<_> = WorkerStatus::ALL
            .into_iter()
            .filter(|s| s.is_schedulable())
            .collect();
        assert_eq!(schedulable, vec![WorkerStatus::Idle]);
    }

    #[test]
    fn digest_must_be_pinned() {
        let good = format!("sha256-{}", "ab".repeat(32));
        assert!(is_pinned_digest(&good));
        assert!(!is_pinned_digest("latest"));
        assert!(!is_pinned_digest(&format!("sha256-{}", "AB".repeat(32))));
        assert!(!is_pinned_digest(&format!("sha256-{}", "ab".repeat(31))));
    }
}
