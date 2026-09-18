#![forbid(unsafe_code)]

//! Serde mirror of the `queue` slice: durable jobs, leases, claim receipts,
//! supersession audit records, hosted observations, exact-lease execution
//! evidence and check publication. The authored TypeSpec and JSON Schema
//! documents remain the two peer authorities; these Rust types are an ergonomic
//! wire mirror.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum QueueState {
    Queued,
    Claimed,
    Running,
    Succeeded,
    Failed,
    Cancelled,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum TrustTier {
    Untrusted,
    Standard,
    Trusted,
    Privileged,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CancellationReason {
    UserRequested,
    Policy,
    Superseded,
    LeaseExpired,
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct QueueJob {
    pub id: String,
    pub repository: String,
    pub revision: String,
    pub workflow_path: String,
    pub profile: String,
    pub state: QueueState,
    pub priority: i32,
    pub enqueued_at: String,
    pub available_at: String,
    pub attempt: i32,
    pub salt: String,
    pub required_os: String,
    pub required_arch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub required_runtime: Option<String>,
    pub required_profile: String,
    pub required_features: Vec<String>,
    pub minimum_trust_tier: TrustTier,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub supersedes_job_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancelled_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cancellation_reason: Option<CancellationReason>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct JobLease {
    pub id: String,
    pub job_id: String,
    pub worker_id: String,
    pub generation: i64,
    pub fencing_token: i64,
    pub attempt: i32,
    pub claimed_at: String,
    pub heartbeat_at: String,
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ClaimReceipt {
    pub id: String,
    pub lease_id: String,
    pub job_id: String,
    pub worker_id: String,
    pub generation: i64,
    pub fencing_token: i64,
    pub matched_os: String,
    pub matched_arch: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub matched_runtime: Option<String>,
    pub matched_profile: String,
    pub matched_features: Vec<String>,
    pub matched_trust_tier: TrustTier,
    pub worker_max_concurrency: i32,
    pub worker_active_jobs: i32,
    pub claimed_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct JobSupersession {
    pub id: String,
    pub superseded_job_id: String,
    pub replacement_job_id: String,
    pub created_at: String,
}

#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct HostedRunObservation {
    pub id: String,
    pub job_id: String,
    pub repository: String,
    pub revision: String,
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct ExecutionEvidence {
    pub id: String,
    pub job_id: String,
    pub lease_id: String,
    pub worker_id: String,
    pub lease_generation: i64,
    pub fencing_token: i64,
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
#[serde(rename_all = "snake_case", deny_unknown_fields)]
pub struct CheckPublication {
    pub id: String,
    pub job_id: String,
    pub lease_id: String,
    pub lease_generation: i64,
    pub fencing_token: i64,
    pub revision: String,
    pub context: String,
    pub state: CheckState,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub external_id: Option<String>,
    pub published_at: String,
}

impl QueueState {
    pub const ALL: [QueueState; 7] = [
        QueueState::Queued,
        QueueState::Claimed,
        QueueState::Running,
        QueueState::Succeeded,
        QueueState::Failed,
        QueueState::Cancelled,
        QueueState::Expired,
    ];

    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            QueueState::Succeeded
                | QueueState::Failed
                | QueueState::Cancelled
                | QueueState::Expired
        )
    }

    /// Mirrors `x-ores-transitions.queue_state` in the JSON Schema authority.
    #[must_use]
    pub const fn allowed_next(self) -> &'static [QueueState] {
        match self {
            QueueState::Queued => &[
                QueueState::Claimed,
                QueueState::Cancelled,
                QueueState::Expired,
            ],
            QueueState::Claimed => &[
                QueueState::Running,
                QueueState::Queued,
                QueueState::Cancelled,
                QueueState::Expired,
            ],
            QueueState::Running => &[
                QueueState::Succeeded,
                QueueState::Failed,
                QueueState::Cancelled,
                QueueState::Expired,
            ],
            QueueState::Succeeded
            | QueueState::Failed
            | QueueState::Cancelled
            | QueueState::Expired => &[],
        }
    }

    #[must_use]
    pub fn may_advance_to(self, next: QueueState) -> bool {
        self.allowed_next().contains(&next)
    }
}

impl HostedEvidenceClassification {
    #[must_use]
    pub const fn is_source_level_success(self) -> bool {
        matches!(self, Self::StepfulSuccess)
    }

    #[must_use]
    pub const fn is_zero_step(self) -> bool {
        matches!(
            self,
            Self::ZeroStepRunnerAdmissionFailure | Self::ZeroStepUnknown
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_have_no_outgoing_edges() {
        for state in QueueState::ALL {
            assert_eq!(state.is_terminal(), state.allowed_next().is_empty());
        }
    }

    #[test]
    fn claimed_is_the_only_inflight_state_that_can_return_to_queued() {
        assert!(QueueState::Claimed.may_advance_to(QueueState::Queued));
        assert!(!QueueState::Running.may_advance_to(QueueState::Queued));
    }

    #[test]
    fn zero_step_hosted_evidence_is_never_source_success() {
        for classification in [
            HostedEvidenceClassification::ZeroStepRunnerAdmissionFailure,
            HostedEvidenceClassification::ZeroStepUnknown,
        ] {
            assert!(classification.is_zero_step());
            assert!(!classification.is_source_level_success());
        }
        assert!(HostedEvidenceClassification::StepfulSuccess.is_source_level_success());
    }

    #[test]
    fn snake_case_fields_round_trip_without_aliases() {
        let raw = r#"{
            "id":"10000001-1111-4222-8333-444455556666",
            "repository":"gha-indie-worker/gha-indie-worker.rs",
            "revision":"0123456789abcdef0123456789abcdef01234567",
            "workflow_path":".github/workflows/ci.yml",
            "profile":"rust-ci",
            "state":"queued",
            "priority":50,
            "enqueued_at":"2026-09-13T20:00:00Z",
            "available_at":"2026-09-13T20:00:00Z",
            "attempt":1,
            "salt":"pr-55-attempt-1",
            "required_os":"linux",
            "required_arch":"x86_64",
            "required_profile":"rust-ci",
            "required_features":["rust"],
            "minimum_trust_tier":"standard"
        }"#;
        let job: QueueJob = serde_json::from_str(raw).expect("canonical queue job");
        let value = serde_json::to_value(job).expect("serialize queue job");
        assert!(value.get("workflow_path").is_some());
        assert!(value.get("workflowPath").is_none());
    }

    #[test]
    fn check_publication_preserves_fencing_identity() {
        let raw = r#"{
            "id":"66666666-6666-4666-8666-666666666666",
            "job_id":"11111111-1111-4111-8111-111111111111",
            "lease_id":"22222222-2222-4222-8222-222222222222",
            "lease_generation":3,
            "fencing_token":9,
            "revision":"e947a78f0b3e1c91ce930c02cf25b57f3e919e8f",
            "context":"indiebuild.dev/ci",
            "state":"success",
            "published_at":"2026-09-14T21:34:05Z"
        }"#;
        let publication: CheckPublication = serde_json::from_str(raw).expect("publication");
        assert_eq!(publication.lease_generation, 3);
        assert_eq!(publication.fencing_token, 9);
    }
}
