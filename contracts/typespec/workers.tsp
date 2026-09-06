import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Workers;

enum WorkerStatus {
  Registering: "registering",
  Idle: "idle",
  Leased: "leased",
  Draining: "draining",
  Offline: "offline",
}

@doc("What a capability row asserts. profile is an operator-reviewed profile name, feature is a free capability tag.")
enum CapabilityKind {
  Profile: "profile",
  Arch: "arch",
  Os: "os",
  Feature: "feature",
}

@doc("A fixed-profile worker pinned to an immutable image digest. Workers never receive caller-selected images.")
@Ores.table("workers")
@Ores.unique("name")
@Ores.index("status,lastHeartbeatAt")
model Worker {
  @key id: uuid;
  orgId?: uuid;

  @minLength(1)
  @maxLength(120) name: string;

  status: WorkerStatus;

  @minLength(1)
  @maxLength(32) arch: string;

  @minLength(1)
  @maxLength(32) os: string;

  @minLength(71)
  @maxLength(71)
  @pattern("^sha256-[0-9a-f]+$") imageDigest: string;

  @minValue(1)
  @maxValue(64) maxConcurrency: int32;

  registeredAt: utcDateTime;
  lastHeartbeatAt?: utcDateTime;
}

@doc("One asserted capability of a worker. Disabled rows are retained so scheduling decisions stay auditable.")
@Ores.table("worker_capabilities")
@Ores.unique("workerId,kind,value")
@Ores.index("kind,value")
model Capability {
  @key id: uuid;
  @Ores.references("Worker.id") workerId: uuid;
  kind: CapabilityKind;

  @minLength(1)
  @maxLength(120) value: string;

  enabled: boolean;
}

@doc("A periodic liveness and load sample from a worker. Absence past the lease window marks the worker offline.")
@Ores.table("worker_heartbeats")
@Ores.unique("workerId,observedAt")
@Ores.index("observedAt")
model Heartbeat {
  @key id: uuid;
  @Ores.references("Worker.id") workerId: uuid;
  observedAt: utcDateTime;

  @minValue(0)
  @maxValue(64) activeJobs: int32;

  @minValue(0) queueDepth: int32;

  @minValue(0) loadAverage: float64;

  @minLength(1)
  @maxLength(32) version: string;
}

@doc("An operator-reviewed execution profile. Commands and artifact paths are compiled in, never caller-supplied.")
@Ores.table("worker_profiles")
@Ores.unique("name")
@Ores.index("reviewedAt")
model Profile {
  @key id: uuid;

  @minLength(1)
  @maxLength(64)
  @pattern("^[a-z0-9][a-z0-9-]*[a-z0-9]$") name: string;

  @minLength(1)
  @maxLength(255) image: string;

  @minLength(71)
  @maxLength(71)
  @pattern("^sha256-[0-9a-f]+$") imageDigest: string;

  @maxItems(64) commands: string[];

  @maxItems(32) artifactPaths: string[];

  @minValue(100)
  @maxValue(64000) cpuMillis: int32;

  @minValue(128)
  @maxValue(262144) memoryMebibytes: int32;

  @minValue(1)
  @maxValue(86400) timeoutSeconds: int32;

  reviewedAt: utcDateTime;
}
