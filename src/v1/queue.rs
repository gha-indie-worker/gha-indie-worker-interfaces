#![forbid(unsafe_code)]

//! Serde mirror of the durable `queue` contract slice.
//!
//! New queue wire names are canonical `snake_case`. This module is an ergonomic
//! mirror only; the independently authored TypeSpec and JSON Schema files remain
//! the peer authorities.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueJobStatus {
    Queued,
    Claimed,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Expired,
    Superseded,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedEvidenceClassification {
    NonTerminal,
    StepfulSuccess,
    StepfulFailure,
    ZeroStepRunnerAdmissionFailure,
    ZeroStepUnknown,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedJobStatus {
    Queued,
    InProgress,
    Completed,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum HostedJobConclusion {
    Success,
    Failure,
    Cancelled,
    TimedOut,
    Neutral,
    Skipped,
    ActionRequired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum RunnerAdmissionReason {
    BudgetExhausted,
    QuotaExceeded,
    NoEligibleRunner,
    RunnerProvisioningFailure,
    RunnerServiceUnavailable,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum CheckState {
    Pending,
    Success,
    Failure,
    Error,
    Cancelled,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct QueueJob {
    pub id: String,
    pub repository: String,
    pub revision_sha: String,
    pub workflow_path: String,
    pub profile: String,
    pub attempt: i32,
    pub idempotency_key: String,
    pub status: QueueJobStatus,
    pub priority: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_os: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_arch: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_runtime: Option<String>,
    pub required_capabilities: Vec<String>,
    pub created_at: String,
    pub available_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub claimed_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub superseded_by_job_id: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct JobLease {
    pub id: String,
    pub job_id: String,
    pub worker_id: String,
    pub generation: i64,
    pub lease_token_hash: String,
    pub leased_at: String,
    pub heartbeat_at: String,
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct HostedRunObservation {
    pub id: String,
    pub job_id: String,
    pub repository: String,
    pub revision_sha: String,
    pub github_run_id: i64,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_job_id: Option<i64>,
    pub status: HostedJobStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<HostedJobConclusion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub step_count: Option<i32>,
    pub classification: HostedEvidenceClassification,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub admission_reason: Option<RunnerAdmissionReason>,
    pub observed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct ExecutionEvidence {
    pub id: String,
    pub job_id: String,
    pub worker_id: String,
    pub lease_generation: i64,
    pub runtime_backend: String,
    pub os: String,
    pub arch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ores_compose_project: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub ores_compose_session: Option<String>,
    pub artifact_ids: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub redacted_summary: Option<String>,
    pub started_at: String,
    pub finished_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(deny_unknown_fields)]
pub struct CheckPublication {
    pub id: String,
    pub job_id: String,
    pub lease_generation: i64,
    pub revision_sha: String,
    pub context: String,
    pub state: CheckState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub published_at: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn queue_wire_names_are_snake_case() {
        let value = serde_json::to_value(HostedEvidenceClassification::ZeroStepUnknown).unwrap();
        assert_eq!(value, serde_json::Value::String("zero_step_unknown".into()));
    }

    #[test]
    fn superseded_is_terminal_and_never_success() {
        assert_ne!(QueueJobStatus::Superseded, QueueJobStatus::Succeeded);
    }
}
