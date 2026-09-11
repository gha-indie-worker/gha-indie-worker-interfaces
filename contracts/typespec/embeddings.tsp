import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Embeddings;

@doc("Whether a vector was produced as a query or as a stored document. Query and document vectors are never comparable across roles.")
enum EmbeddingRole {
  Query: "query",
  Document: "document",
}

@doc("How the ANN index is built and searched. binary-full-rerank stores a binary sketch and reranks with the full vector.")
enum AnnStrategy {
  HalfvecExact: "halfvec-exact",
  HalfvecMrl: "halfvec-mrl",
  BinaryFullRerank: "binary-full-rerank",
}

enum DistanceMetric {
  Cosine: "cosine",
  L2: "l2",
  InnerProduct: "inner-product",
}

enum Normalization {
  None: "none",
  L2: "l2",
}

@doc("What a regression run found. decoupling means two entities that used to co-move no longer do.")
enum RegressionKind {
  Drift: "drift",
  Collapse: "collapse",
  Decoupling: "decoupling",
  Saturation: "saturation",
}

enum AlertSeverity {
  Info: "info",
  Warning: "warning",
  Critical: "critical",
}

enum MatchOutcome {
  Accepted: "accepted",
  Rejected: "rejected",
  Deferred: "deferred",
}

@doc("The twelve-field identity of a comparison space. Two vectors may only be compared when every one of these fields is equal.")
@Ores.table("embedding_comparison_spaces")
@Ores.unique("tenantId,entityKind,entityId,modelFamily,modelName,modelVersion,dimsNative,role,normalization,distanceMetric,annStrategy,generationProfile")
@Ores.index("tenantId,entityKind")
model ComparisonSpace {
  @key id: uuid;
  tenantId: uuid;

  @minLength(1)
  @maxLength(64) entityKind: string;

  @minLength(1)
  @maxLength(128) entityId: string;

  @minLength(1)
  @maxLength(64) modelFamily: string;

  @minLength(1)
  @maxLength(128) modelName: string;

  @minLength(1)
  @maxLength(64) modelVersion: string;

  @minValue(1)
  @maxValue(4096) dimsNative: int32;

  role: EmbeddingRole;
  normalization: Normalization;
  distanceMetric: DistanceMetric;
  annStrategy: AnnStrategy;

  @minLength(1)
  @maxLength(64) generationProfile: string;

  createdAt: utcDateTime;
}

@doc("One stored vector inside a comparison space. dims must equal the space dimsNative unless the strategy is halfvec-mrl.")
@Ores.table("embedding_records")
@Ores.unique("spaceId,entityId")
@Ores.index("spaceId,createdAt")
model EmbeddingRecord {
  @key id: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;

  @minLength(1)
  @maxLength(128) entityId: string;

  @minItems(1)
  @maxItems(4100) vector: float64[];

  @minValue(1)
  @maxValue(4096) dims: int32;

  @minLength(64)
  @maxLength(64)
  @pattern("^[0-9a-f]+$") checksum: string;

  @minLength(40)
  @maxLength(40)
  @pattern("^[0-9a-f]+$") sourceRevision?: string;

  createdAt: utcDateTime;
}

@doc("A request to (re)index a bounded set of entities into one comparison space.")
@Ores.table("embedding_index_requests")
@Ores.unique("idempotencyKey")
@Ores.index("spaceId,requestedAt")
model IndexRequest {
  @key id: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;

  @minItems(1)
  @maxItems(1000) entityIds: string[];

  @minValue(0)
  @maxValue(9) priority: int32;

  @minLength(8)
  @maxLength(128) idempotencyKey: string;

  requestedAt: utcDateTime;
  requestedBy?: uuid;
}

@doc("A vector search inside one comparison space. lexicalPrefilter narrows candidates before the ANN probe.")
@Ores.table("embedding_search_requests")
@Ores.index("spaceId,requestedAt")
model SearchRequest {
  @key id: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;

  @maxLength(4000) queryText?: string;

  @minItems(1)
  @maxItems(4100) queryVector: float64[];

  @maxLength(400) lexicalPrefilter?: string;

  @minValue(1)
  @maxValue(1000) topK: int32;

  @minValue(-1)
  @maxValue(1) minScore?: float64;

  requestedAt: utcDateTime;
}

@doc("The settled result of a SearchRequest. matchedEntityIds and scores are positionally aligned.")
@Ores.table("embedding_search_responses")
@Ores.unique("requestId")
@Ores.index("respondedAt")
model SearchResponse {
  @key id: uuid;
  @Ores.references("SearchRequest.id") requestId: uuid;

  @maxItems(1000) matchedEntityIds: string[];

  @maxItems(1000) scores: float64[];

  @minValue(0) prefilterHits: int32;

  @minValue(0) elapsedMillis: int32;

  truncated: boolean;
  respondedAt: utcDateTime;
}

@doc("A statistical relationship detected between two entities in a space. coefficient is bounded to -1 through 1 and pValue to 0 through 1.")
@Ores.table("embedding_regression_findings")
@Ores.unique("spaceId,kind,xEntity,yEntity,window,detectedAt")
@Ores.index("spaceId,detectedAt")
model RegressionFinding {
  @key id: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;
  kind: RegressionKind;

  @minLength(1)
  @maxLength(128) xEntity: string;

  @minLength(1)
  @maxLength(128) yEntity: string;

  @minValue(-1)
  @maxValue(1) coefficient: float64;

  @minValue(0)
  @maxValue(1) pValue: float64;

  @minLength(1)
  @maxLength(32) window: string;

  @minValue(1) sampleN: int32;

  detectedAt: utcDateTime;
}

@doc("A standing rule that turns regression findings into alerts. Thresholds are evaluated only inside one space.")
@Ores.table("embedding_alert_rules")
@Ores.unique("spaceId,name")
@Ores.index("enabled,severity")
model AlertRule {
  @key id: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;

  @minLength(1)
  @maxLength(120) name: string;

  kind: RegressionKind;
  severity: AlertSeverity;

  @minValue(-1)
  @maxValue(1) thresholdCoefficient: float64;

  @minValue(0)
  @maxValue(1) maxPValue: float64;

  @minValue(1) minSampleN: int32;

  enabled: boolean;

  @maxItems(16) notifyChannels: string[];

  createdAt: utcDateTime;
}

@doc("A human or policy decision about one retrieved match. This is the supervision signal the regression jobs read.")
@Ores.table("embedding_match_events")
@Ores.index("spaceId,occurredAt")
@Ores.index("outcome")
model MatchEvent {
  @key id: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;

  @minLength(1)
  @maxLength(128) queryEntityId: string;

  @minLength(1)
  @maxLength(128) matchedEntityId: string;

  @minValue(-1)
  @maxValue(1) score: float64;

  outcome: MatchOutcome;
  occurredAt: utcDateTime;
  actorUserId?: uuid;
}
