import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Queue;

@doc("Durable lifecycle of one independently executable CI job. superseded is terminal and must never publish green.")
enum QueueJobStatus {
  Queued: "queued",
  Claimed: "claimed",
  Running: "running",
  Succeeded: "succeeded",
  Failed: "failed",
  Cancelled: "cancelled",
  Expired: "expired",
  Superseded: "superseded",
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

@doc("Canonical durable queue row. Cloudflare and NATS may wake consumers but are not competing queue authorities.")
@Ores.table("ci_queue_jobs")
@Ores.unique("idempotency_key")
@Ores.index("status,priority,available_at")
@Ores.index("repository,revision_sha")
model QueueJob {
  @key id: uuid;

  @minLength(3)
  @maxLength(140)
  @pattern("^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$") repository: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision_sha: string;

  @minLength(1)
  @maxLength(255) workflow_path: string;

  @minLength(1)
  @maxLength(64) profile: string;

  @minValue(1)
  @maxValue(50) attempt: int32;

  @minLength(64)
  @maxLength(64)
  @pattern("^[0-9a-f]+$") idempotency_key: string;

  status: QueueJobStatus;

  @minValue(0)
  @maxValue(1000) priority: int32;

  @maxLength(32) required_os?: string;
  @maxLength(32) required_arch?: string;
  @maxLength(32) required_runtime?: string;
  @maxItems(64) required_capabilities: string[];

  created_at: utcDateTime;
  available_at: utcDateTime;
  claimed_at?: utcDateTime;
  started_at?: utcDateTime;
  finished_at?: utcDateTime;
  superseded_by_job_id?: uuid;
}

@doc("One fenced worker claim. generation is monotonic per job; only the current generation may heartbeat or publish a terminal result.")
@Ores.table("ci_job_leases")
@Ores.unique("job_id,generation")
@Ores.index("worker_id,expires_at")
@Ores.index("job_id,released_at")
model JobLease {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;

  @doc("Foreign identity from the workers slice; intentionally not a cross-slice SQL foreign key here.")
  worker_id: uuid;

  @minValue(1) generation: int64;

  @minLength(64)
  @maxLength(64)
  @pattern("^[0-9a-f]+$") lease_token_hash: string;

  leased_at: utcDateTime;
  heartbeat_at: utcDateTime;
  expires_at: utcDateTime;
  released_at?: utcDateTime;
}

@doc("Observed hosted GitHub job evidence kept separate from independent execution evidence.")
@Ores.table("ci_hosted_run_observations")
@Ores.index("repository,revision_sha,observed_at")
@Ores.index("classification,observed_at")
model HostedRunObservation {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;

  @minLength(3)
  @maxLength(140) repository: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision_sha: string;

  @minValue(1) github_run_id: int64;
  @minValue(1) github_job_id?: int64;
  status: HostedJobStatus;
  conclusion?: HostedJobConclusion;
  @minValue(0) step_count?: int32;
  classification: HostedEvidenceClassification;
  admission_reason?: RunnerAdmissionReason;
  observed_at: utcDateTime;
}

@doc("Independent execution evidence. Secret values are forbidden; only redacted summaries and opaque artifact identifiers belong here.")
@Ores.table("ci_execution_evidence")
@Ores.unique("job_id,lease_generation")
@Ores.index("worker_id,finished_at")
model ExecutionEvidence {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;
  worker_id: uuid;
  @minValue(1) lease_generation: int64;

  @minLength(1)
  @maxLength(32) runtime_backend: string;
  @minLength(1)
  @maxLength(32) os: string;
  @minLength(1)
  @maxLength(32) arch: string;

  @maxLength(120) ores_compose_project?: string;
  @maxLength(120) ores_compose_session?: string;
  @maxItems(64) artifact_ids: string[];
  @maxLength(2048) redacted_summary?: string;

  started_at: utcDateTime;
  finished_at: utcDateTime;
}

@doc("One external check/status publication attempt. Exact SHA plus lease generation prevents stale workers from publishing green.")
@Ores.table("ci_check_publications")
@Ores.unique("job_id,lease_generation,context")
@Ores.index("revision_sha,published_at")
model CheckPublication {
  @key id: uuid;
  @Ores.references("QueueJob.id") job_id: uuid;
  @minValue(1) lease_generation: int64;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision_sha: string;

  @minLength(1)
  @maxLength(120) context: string;
  state: CheckState;
  @maxLength(160) external_id?: string;
  published_at: utcDateTime;
}
