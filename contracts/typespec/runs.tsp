import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Runs;

@doc("Lifecycle of a run and of every job inside it. queued and planning are pre-execution states.")
enum RunStatus {
  Queued: "queued",
  Planning: "planning",
  Running: "running",
  Succeeded: "succeeded",
  Failed: "failed",
  Cancelled: "cancelled",
}

enum StepConclusion {
  Succeeded: "succeeded",
  Failed: "failed",
  Skipped: "skipped",
  Cancelled: "cancelled",
}

@doc("Which continuity lane executed the work. arc is native GitHub parity, independent is this fleet's compiler.")
enum ParityLane {
  Arc: "arc",
  Independent: "independent",
}

enum CancellationReason {
  UserRequested: "user-requested",
  Policy: "policy",
  Timeout: "timeout",
  Superseded: "superseded",
}

@doc("A parsed, classified workflow at an immutable revision. A plan never carries caller-selected commands.")
@Ores.table("run_plans")
@Ores.unique("repository,revision,workflowPath")
@Ores.index("repository,createdAt")
model Plan {
  @key id: uuid;

  @minLength(3)
  @maxLength(140)
  @pattern("^[A-Za-z0-9._-]+/[A-Za-z0-9._-]+$") repository: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") revision: string;

  @minLength(1)
  @maxLength(255) workflowPath: string;

  parityLane: ParityLane;

  @minValue(0)
  @maxValue(256) jobCount: int32;

  supported: boolean;

  @maxItems(128) exclusions: string[];

  @maxLength(64) profile?: string;

  createdAt: utcDateTime;
}

@doc("One execution of a plan for one org. Terminal states are succeeded, failed and cancelled.")
@Ores.table("runs")
@Ores.unique("planId,attempt")
@Ores.index("orgId,queuedAt")
@Ores.index("status")
model Run {
  @key id: uuid;
  @Ores.references("Plan.id") planId: uuid;
  orgId: uuid;
  status: RunStatus;

  @minValue(1)
  @maxValue(50) attempt: int32;

  queuedAt: utcDateTime;
  startedAt?: utcDateTime;
  finishedAt?: utcDateTime;
  requestedBy?: uuid;
}

@doc("One job of a run. needs holds the static dependency names validated for unknown nodes and cycles.")
@Ores.table("run_jobs")
@Ores.unique("runId,name")
@Ores.index("status")
model Job {
  @key id: uuid;
  @Ores.references("Run.id") runId: uuid;

  @minLength(1)
  @maxLength(120) name: string;

  status: RunStatus;

  @minLength(1)
  @maxLength(64) profile: string;

  @maxItems(64) needs: string[];
  startedAt?: utcDateTime;
  finishedAt?: utcDateTime;
  exitCode?: int32;
}

@doc("One ordered step of a job. Steps are declarative and never carry caller-selected shell.")
@Ores.table("run_steps")
@Ores.unique("jobId,ordinal")
model Step {
  @key id: uuid;
  @Ores.references("Job.id") jobId: uuid;

  @minValue(0)
  @maxValue(512) ordinal: int32;

  @minLength(1)
  @maxLength(120) name: string;

  conclusion?: StepConclusion;
  startedAt?: utcDateTime;
  finishedAt?: utcDateTime;
}

@doc("A contiguous slice of a job log. Chunks are dense and ordered by sequence for websocket replay.")
@Ores.table("run_log_chunks")
@Ores.unique("jobId,sequence")
@Ores.index("emittedAt")
model LogChunk {
  @key id: uuid;
  @Ores.references("Job.id") jobId: uuid;

  @minValue(0) sequence: int64;

  @minValue(0) offsetBytes: int64;

  @maxLength(65536) body: string;

  truncated: boolean;
  emittedAt: utcDateTime;
}

@doc("A cancellation claim against a run. At most one per run; effectiveAt is set once workers acknowledge.")
@Ores.table("run_cancellations")
@Ores.unique("runId")
@Ores.index("reason,requestedAt")
model RunCancellation {
  @key id: uuid;
  @Ores.references("Run.id") runId: uuid;
  reason: CancellationReason;
  requestedAt: utcDateTime;
  requestedBy?: uuid;
  effectiveAt?: utcDateTime;

  @maxLength(200) note?: string;
}
