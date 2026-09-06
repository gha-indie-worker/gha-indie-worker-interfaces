#![forbid(unsafe_code)]

//! Serde mirror of the `runs` slice: plans, runs, jobs, steps, log chunks and
//! cancellation.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum RunStatus {
    Queued,
    Planning,
    Running,
    Succeeded,
    Failed,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum StepConclusion {
    Succeeded,
    Failed,
    Skipped,
    Cancelled,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum ParityLane {
    Arc,
    Independent,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum CancellationReason {
    UserRequested,
    Policy,
    Timeout,
    Superseded,
}

/// A parsed, classified workflow at an immutable revision.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Plan {
    pub id: String,
    pub repository: String,
    pub revision: String,
    pub workflow_path: String,
    pub parity_lane: ParityLane,
    pub job_count: i32,
    pub supported: bool,
    pub exclusions: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub profile: Option<String>,
    pub created_at: String,
}

/// One execution of a plan for one org.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Run {
    pub id: String,
    pub plan_id: String,
    pub org_id: String,
    pub status: RunStatus,
    pub attempt: i32,
    pub queued_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
}

/// One job of a run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Job {
    pub id: String,
    pub run_id: String,
    pub name: String,
    pub status: RunStatus,
    pub profile: String,
    pub needs: Vec<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub exit_code: Option<i32>,
}

/// One ordered step of a job.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Step {
    pub id: String,
    pub job_id: String,
    pub ordinal: i32,
    pub name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub conclusion: Option<StepConclusion>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub started_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub finished_at: Option<String>,
}

/// A contiguous slice of a job log.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogChunk {
    pub id: String,
    pub job_id: String,
    pub sequence: i64,
    pub offset_bytes: i64,
    pub body: String,
    pub truncated: bool,
    pub emitted_at: String,
}

/// A cancellation claim against a run.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RunCancellation {
    pub id: String,
    pub run_id: String,
    pub reason: CancellationReason,
    pub requested_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub effective_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub note: Option<String>,
}

impl RunStatus {
    pub const ALL: [RunStatus; 6] = [
        RunStatus::Queued,
        RunStatus::Planning,
        RunStatus::Running,
        RunStatus::Succeeded,
        RunStatus::Failed,
        RunStatus::Cancelled,
    ];

    /// A terminal status never transitions again.
    #[must_use]
    pub const fn is_terminal(self) -> bool {
        matches!(
            self,
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled
        )
    }

    /// The complete edge set of the run lifecycle. Mirrors `x-ores-transitions`
    /// in `contracts/json-schema/runs.schema.json`.
    #[must_use]
    pub const fn allowed_next(self) -> &'static [RunStatus] {
        match self {
            RunStatus::Queued => &[RunStatus::Planning, RunStatus::Cancelled],
            RunStatus::Planning => &[RunStatus::Running, RunStatus::Failed, RunStatus::Cancelled],
            RunStatus::Running => &[
                RunStatus::Succeeded,
                RunStatus::Failed,
                RunStatus::Cancelled,
            ],
            RunStatus::Succeeded | RunStatus::Failed | RunStatus::Cancelled => &[],
        }
    }

    #[must_use]
    pub fn may_advance_to(self, next: RunStatus) -> bool {
        self.allowed_next().contains(&next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn terminal_states_have_no_outgoing_edges() {
        for status in RunStatus::ALL {
            assert_eq!(status.is_terminal(), status.allowed_next().is_empty());
        }
    }

    #[test]
    fn cancellation_is_reachable_from_every_non_terminal_state() {
        for status in RunStatus::ALL {
            if !status.is_terminal() {
                assert!(status.may_advance_to(RunStatus::Cancelled), "{status:?}");
            }
        }
    }

    #[test]
    fn queued_cannot_jump_straight_to_running() {
        assert!(!RunStatus::Queued.may_advance_to(RunStatus::Running));
    }
}
