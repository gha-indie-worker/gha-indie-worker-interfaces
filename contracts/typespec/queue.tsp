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
