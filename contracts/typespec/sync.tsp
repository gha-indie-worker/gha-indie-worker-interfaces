import "./ores.tsp";
import "@typespec/json-schema";
using Ores;

namespace GhaIndieWorker.V1.Sync;

enum SyncOp {
  Upsert: "upsert",
  Delete: "delete",
  Merge: "merge",
}

@doc("How a receiver settles a concurrent write. causal-merge requires the entity to have a registered merge function.")
enum ConflictPolicy {
  LastWriteWins: "last-write-wins",
  CausalMerge: "causal-merge",
  Reject: "reject",
}

@doc("One opto-sync causal envelope. vectorClock maps node id to a monotonic counter and is the only ordering authority. The runtime JSON Schema constrains each counter to a non-negative integer.")
@Ores.table("sync_causal_envelopes")
@Ores.unique("originNode,originSequence")
@Ores.index("entity,entityId")
@Ores.index("recordedAt")
model CausalEnvelope {
  @key id: uuid;

  @minLength(1)
  @maxLength(120) entity: string;

  @minLength(1)
  @maxLength(128) entityId: string;

  `op`: SyncOp;
  @TypeSpec.JsonSchema.extension("additionalProperties", #{ type: "integer", minimum: 0 })
  vectorClock: Record<unknown>;
  payload: json;
  conflictPolicy: ConflictPolicy;

  @minLength(1)
  @maxLength(64) originNode: string;

  @minValue(1) originSequence: int64;

  recordedAt: utcDateTime;
  appliedAt?: utcDateTime;
}

@doc("The high-water mark a node has durably applied for one entity family. Pull resumes from lastSequence.")
@Ores.table("sync_cursors")
@Ores.unique("nodeId,entity")
@Ores.index("updatedAt")
model SyncCursor {
  @key id: uuid;

  @minLength(1)
  @maxLength(64) nodeId: string;

  @minLength(1)
  @maxLength(120) entity: string;

  @minValue(0) lastSequence: int64;

  updatedAt: utcDateTime;
}
