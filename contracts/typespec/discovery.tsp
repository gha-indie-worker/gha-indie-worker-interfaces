// ============================================================================
// TypeSpec authority — embedding and discovery surface.
//
// PEER of contracts/json-schema/discovery.schema.json.
// Mirrors lib-core `analysis::embedding`. Two rules are contract, not policy:
//
//   1. A search never crosses a ComparisonSpace. The space id is a required
//      path parameter, so there is no call shape that omits it.
//   2. A space ships disabled and an approximate strategy needs measured
//      recall evidence, so `Evidence` is part of the enable request body —
//      you cannot turn a space on without saying what you measured.
// ============================================================================
import "@typespec/http";

using Http;

namespace GhaIndieWorker;

@doc("What has been measured about a comparison space. Every field must be true before an approximate strategy may be enabled. `analysis::embedding::Evidence`.")
model Evidence {
  @doc("The space survives a replay of every migration that touches it.")
  migrationReplay: boolean;

  @doc("No query in the space can return another tenant's row.")
  tenantIsolation: boolean;

  @doc("Retrieval still works with vectors switched off, so a regression is not silent.")
  lexicalOnlyRetrieval: boolean;

  @doc("Exact scan and the approximate index were compared and the recall recorded.")
  exactVsAnnRecall: boolean;

  @doc("The change can be rolled back without dropping the column.")
  rollback: boolean;

  @doc("A backup taken before the change restores cleanly.")
  backupRestore: boolean;
}

@doc("Turn a comparison space on. Refused unless the evidence supports the requested strategy.")
model EnableSpaceRequest {
  annStrategy: AnnStrategy;
  evidence: Evidence;

  @doc("Measured recall against exact scan, in [0, 1]. Required for anything but `halfvec_exact`.")
  @minValue(0)
  @maxValue(1)
  measuredRecall?: float64;
}

@doc("A nearest-neighbour query inside one comparison space.")
model SearchRequest {
  @doc("Query text. Encoded with the space's own prompt profile, never a different one.")
  @minLength(1)
  @maxLength(8192)
  text: string;

  @minValue(1) @maxValue(100) topK: int32;

  @doc("Reject neighbours below this similarity. Absent means return the top K regardless.")
  @minValue(0)
  @maxValue(1)
  minScore?: float64;
}

@doc("One neighbour. `score` is in the space's own metric, normalized to [0, 1].")
model Neighbour {
  @minLength(1) @maxLength(256) entityId: string;
  @minValue(0) @maxValue(1) score: float64;

  @doc("The chunk that matched, for a human to read. Never the whole document.")
  @minLength(1)
  @maxLength(4000)
  excerpt: string;
}

@doc("Answer to a search. `exact` is false when an approximate index served the query, so a caller can tell a measured answer from an estimated one.")
model SearchResponse {
  @maxItems(100) neighbours: Neighbour[];
  exact: boolean;
  strategy: AnnStrategy;
}

@route("/v1/orgs/{orgId}/spaces")
@tag("discovery")
interface Spaces {
  @get @route("") list(@path orgId: uuid, @query enabled?: boolean, ...ListQuery): Page<ComparisonSpace> | ApiError;
  @get @route("/{spaceId}") get(@path orgId: uuid, @path spaceId: uuid): ComparisonSpace | ApiError;

  @doc("Enable a space. 409 with `forbidden` when the evidence does not support the strategy.")
  @post
  @route("/{spaceId}/enable")
  enable(@path orgId: uuid, @path spaceId: uuid, @body body: EnableSpaceRequest): ComparisonSpace | ApiError;

  @doc("Search inside one space. There is no cross-space search: vectors from different spaces are not comparable.")
  @post
  @route("/{spaceId}/search")
  search(@path orgId: uuid, @path spaceId: uuid, @body body: SearchRequest): SearchResponse | ApiError;
}

@route("/v1/orgs/{orgId}/findings")
@tag("discovery")
interface Findings {
  @get
  @route("")
  list(
    @path orgId: uuid,
    @query severity?: FindingSeverity,
    @query runId?: uuid,
    ...ListQuery,
  ): Page<Finding> | ApiError;

  @get @route("/{findingId}") get(@path orgId: uuid, @path findingId: uuid): Finding | ApiError;
}
