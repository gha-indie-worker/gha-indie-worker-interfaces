import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Queue;

@doc("Durable lifecycle for one exact-revision CI job. Terminal states are succeeded, failed, cancelled and expired.")
enum QueueState {
  Queued: "queued",
  Claimed: "claimed",
  Running: "running",
  Succeeded: "succeeded",
  Failed: "failed",
  Cancelled: "cancelled",
  Expired: "expired",
}

@doc("Worker trust level required before a job may be leased.")
enum TrustTier {
  Untrusted: "untrusted",
  Standard: "standard",
  Trusted: "trusted",
  Privileged: "privileged",
}

@doc("Why a queued or in-flight job was cancelled before normal completion.")
enum CancellationReason {
  UserRequested: "user-requested",
  Policy: "policy",
  Superseded: "superseded",
  LeaseExpired: "lease-expired",
}

@doc("Structural classification of hosted GitHub evidence. Zero-step observations are never source-level success evidence.")
enum HostedEvidenceClassification {
  NonTerminal: "non_terminal",
  StepfulSuccess: "stepful_success",
  StepfulFailure: "stepful_failure",
  ZeroStepRunnerAdmissionFailure: "zero_step_runner_admission_failure",
  ZeroStepUnknown: "zero_step_unknown",
  Cancelled: "cancelled",
}

enum HostedJobStatus {
  Queued: "queued",
  InProgress: "in_progress",
  Completed: "completed",
}

enum HostedJobConclusion {
  Success: "success",
  Failure: "failure",
  Cancelled: "cancelled",
  TimedOut: "timed_out",
  Neutral: "neutral",
  Skipped: "skipped",
  ActionRequired: "action_required",
}

@doc("Typed admission reasons derived only from stable machine-readable evidence, never free-form log substring matching.")
enum RunnerAdmissionReason {
  BudgetExhausted: "budget_exhausted",
  QuotaExceeded: "quota_exceeded",
  NoEligibleRunner: "no_eligible_runner",
  RunnerProvisioningFailure: "runner_provisioning_failure",
  RunnerServiceUnavailable: "runner_service_unavailable",
}

enum CheckState {
  Pending: "pending",
  Success: "success",
  Failure: "failure",
  Error: "error",
  Cancelled: "cancelled",
}

@doc("One durable queue row for an immutable repository revision. No secret values or caller-selected shell commands are stored here.")
@Ores.table("ci_queue_jobs")
@Ores.unique("repository,revision,workflow_path,profile,attempt,salt")
@Ores.index("state,available_at")
@Ores.index("repository,revision")
@Ores.index("priority,enqueued_at")
model QueueJob {
  @key id: uuid;

  @minLength(3)
  @maxLength(140)
  @pattern("^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$") repository: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision: string;

  @minLength(1)
  @maxLength(255) workflow_path: string;

  @minLength(1)
  @maxLength(64) profile: string;

  state: QueueState;

  @minValue(0)
  @maxValue(100) priority: int32;

  enqueued_at: utcDateTime;
  available_at: utcDateTime;

  @minValue(1)
  @maxValue(50) attempt: int32;

  @minLength(1)
  @maxLength(64)
  @pattern("^[A-Za-z0-9._-]+$") salt: string;

  @minLength(1)
  @maxLength(32) required_os: string;

  @minLength(1)
  @maxLength(32) required_arch: string;

  @minLength(1)
  @maxLength(32) required_runtime?: string;

  @minLength(1)
  @maxLength(64) required_profile: string;

  @maxItems(32) required_features: string[];
  minimum_trust_tier: TrustTier;

  supersedes_job_id?: uuid;
  cancelled_at?: utcDateTime;
  cancellation_reason?: CancellationReason;
}

@doc("A renewable lease granting one worker authority over one queue job generation. generation and fencing_token monotonically increase on reclaim.")
@Ores.table("ci_job_leases")
@Ores.unique("job_id,generation")
@Ores.unique("job_id,fencing_token")
@Ores.index("worker_id,expires_at")
@Ores.index("expires_at")
model JobLease {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;
  worker_id: uuid;

  @minValue(1) generation: int64;
  @minValue(1) fencing_token: int64;

  @minValue(1)
  @maxValue(50) attempt: int32;

  claimed_at: utcDateTime;
  heartbeat_at: utcDateTime;
  expires_at: utcDateTime;
  released_at?: utcDateTime;
}

@doc("Immutable evidence describing why a worker was eligible for a claim. This is safe to retain because it contains capability names, not secrets.")
@Ores.table("ci_claim_receipts")
@Ores.unique("lease_id")
@Ores.index("worker_id,claimed_at")
model ClaimReceipt {
  @key id: uuid;
  @Ores.references("JobLease.id") lease_id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;
  worker_id: uuid;

  @minValue(1) generation: int64;
  @minValue(1) fencing_token: int64;

  @minLength(1)
  @maxLength(32) matched_os: string;

  @minLength(1)
  @maxLength(32) matched_arch: string;

  @minLength(1)
  @maxLength(32) matched_runtime?: string;

  @minLength(1)
  @maxLength(64) matched_profile: string;

  @maxItems(32) matched_features: string[];
  matched_trust_tier: TrustTier;

  @minValue(1)
  @maxValue(64) worker_max_concurrency: int32;

  @minValue(0)
  @maxValue(64) worker_active_jobs: int32;

  claimed_at: utcDateTime;
}

@doc("Audit record linking a cancelled stale job to the immutable replacement that superseded it.")
@Ores.table("ci_job_supersessions")
@Ores.unique("superseded_job_id")
@Ores.unique("replacement_job_id")
model JobSupersession {
  @key id: uuid;
  @Ores.references("QueueJob.id") superseded_job_id: uuid;
  @Ores.references("QueueJob.id") replacement_job_id: uuid;
  created_at: utcDateTime;
}

@doc("Observed hosted GitHub job evidence kept separate from independent worker execution evidence.")
@Ores.table("ci_hosted_run_observations")
@Ores.index("repository,revision,observed_at")
@Ores.index("classification,observed_at")
model HostedRunObservation {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;

  @minLength(3)
  @maxLength(140)
  @pattern("^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$") repository: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision: string;

  @minValue(1) github_run_id: int64;
  @minValue(1) github_job_id?: int64;
  status: HostedJobStatus;
  conclusion?: HostedJobConclusion;
  @minValue(0) step_count?: int32;
  classification: HostedEvidenceClassification;
  admission_reason?: RunnerAdmissionReason;
  observed_at: utcDateTime;
}

@doc("Independent exact-lease execution evidence. Secret values are forbidden; only redacted summaries and opaque artifact identifiers belong here.")
@Ores.table("ci_execution_evidence")
@Ores.unique("lease_id")
@Ores.index("job_id,lease_generation")
@Ores.index("worker_id,finished_at")
model ExecutionEvidence {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;
  @Ores.references("JobLease.id") lease_id: uuid;
  worker_id: uuid;
  @minValue(1) lease_generation: int64;
  @minValue(1) fencing_token: int64;

  @minLength(1)
  @maxLength(32) runtime_backend: string;
  @minLength(1)
  @maxLength(32) os: string;
  @minLength(1)
  @maxLength(32) arch: string;

  @minLength(1)
  @maxLength(120) ores_compose_project?: string;
  @minLength(1)
  @maxLength(120) ores_compose_session?: string;
  @maxItems(64) artifact_ids: string[];
  @maxLength(2048) redacted_summary?: string;

  started_at: utcDateTime;
  finished_at: utcDateTime;
}

@doc("One external check/status publication. Exact revision plus lease generation and fencing token prevent stale workers from publishing green.")
@Ores.table("ci_check_publications")
@Ores.unique("job_id,lease_generation,fencing_token,context")
@Ores.index("revision,published_at")
model CheckPublication {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;
  @Ores.references("JobLease.id") lease_id: uuid;
  @minValue(1) lease_generation: int64;
  @minValue(1) fencing_token: int64;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision: string;

  @minLength(1)
  @maxLength(120) context: string;
  state: CheckState;
  @maxLength(160) external_id?: string;
  published_at: utcDateTime;
}
