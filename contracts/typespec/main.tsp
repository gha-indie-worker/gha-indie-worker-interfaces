// ============================================================================
// TypeSpec authority — persisted vocabulary of GHA Indie Worker.
//
// This file is HUMAN-AUTHORED and is a PEER of contracts/json-schema/*.
// It is not generated from the JSON Schema, and the JSON Schema is not
// generated from it. Neither is canonical. `just contracts` compiles each
// independently and compares the results; a discrepancy fails closed.
//
// Scope of THIS file: the persisted models only — the subset
// @oresoftware/ores-contracts can express (see its docs/subset.md), so that
// `ores-contracts check` can drive SQL / SeaORM / Diesel / Rust / TS / Dart
// from it. Wire-only types live in the sibling files, which `tsp compile`
// reads through the imports below and ores-contracts deliberately does not:
//
//   session.tsp    live session frames  (peer: json-schema/session.schema.json)
//   http.tsp       operations + errors  (peer: json-schema/errors.schema.json)
//   discovery.tsp  embedding surface    (peer: json-schema/discovery.schema.json)
//
// Field names are camelCase in both authorities. SQL/Rust/Dart emitters
// snake_case; TypeScript keeps camelCase, matching the JSON wire format.
// ============================================================================
import "./ores.tsp";
import "./session.tsp";
import "./http.tsp";
import "./discovery.tsp";
using Ores;

namespace GhaIndieWorker;

// ---------------------------------------------------------------------------
// Identity and tenancy — mirrors lib-core `runtime::tenancy` and `runtime::authz`.
// ---------------------------------------------------------------------------

@doc("Whether an account is a single person or an organization. `runtime::tenancy::AccountKind`.")
enum AccountKind {
  Individual: "individual",
  Organization: "organization",
}

@doc("Membership role, ordered least to most privileged. `runtime::tenancy::Role`.")
enum Role {
  Viewer: "viewer",
  Member: "member",
  Admin: "admin",
  Owner: "owner",
}

@doc("Lifecycle of an invitation. `runtime::tenancy::InvitationState`.")
enum InvitationState {
  Open: "open",
  Accepted: "accepted",
  Revoked: "revoked",
  Expired: "expired",
}

@doc("A public surface of the product, by host label. `runtime::surface::Surface`.")
enum Surface {
  App: "app",
  User: "user",
  Org: "org",
  Mobile: "m",
  Api: "api",
  Auth: "auth",
  Admin: "admin",
  AdminApi: "admin-api",
  Marketing: "www",
}

// ---------------------------------------------------------------------------
// CI/CD — vocabulary of docs/parity-matrix.json.
// ---------------------------------------------------------------------------

@doc("Where a run, job or step is in its lifecycle.")
enum RunStatus {
  Queued: "queued",
  InProgress: "in_progress",
  Completed: "completed",
}

@doc("How a completed run, job or step ended. Absent while status is not `completed`.")
enum RunConclusion {
  Success: "success",
  Failure: "failure",
  Cancelled: "cancelled",
  Skipped: "skipped",
  TimedOut: "timed_out",
  ActionRequired: "action_required",
  Neutral: "neutral",
}

@doc("Liveness of a registered runner.")
enum RunnerState {
  Offline: "offline",
  Idle: "idle",
  Busy: "busy",
}

@doc("Runner CPU architecture. Both k8s clusters are aarch64; Cloud Run is x64.")
enum RunnerArch {
  X64: "x64",
  Arm64: "arm64",
}

@doc("Runner operating system.")
enum RunnerOs {
  Linux: "linux",
  Macos: "macos",
  Windows: "windows",
}

@doc("Job isolation, weakest to strongest. Dimension `iso` of the parity matrix.")
enum IsolationMode {
  Process: "process",
  Rootless: "rootless",
  Microvm: "microvm",
  PerJobVm: "per_job_vm",
}

@doc("Blast radius of a cache entry. Dimension `cache.scoping-eviction`.")
enum CacheScope {
  Repository: "repository",
  Organization: "organization",
  Branch: "branch",
}

@doc("Kind of supply-chain attestation attached to a run. Dimension `supply`.")
enum AttestationKind {
  SlsaProvenance: "slsa_provenance",
  Sbom: "sbom",
  SigstoreBundle: "sigstore_bundle",
}

@doc("Protocol-level failure code. Byte-tagged on the wire; `runtime::session::ErrorCode`.")
enum ErrorCode {
  UnsupportedVersion: "unsupported_version",
  Unauthorized: "unauthorized",
  Forbidden: "forbidden",
  Protocol: "protocol",
  RateLimited: "rate_limited",
  NotFound: "not_found",
  GoingAway: "going_away",
  Timeout: "timeout",
  Internal: "internal",
}

// ---------------------------------------------------------------------------
// Embedding and discovery — mirrors lib-core `analysis::embedding`.
// ---------------------------------------------------------------------------

@doc("Vector similarity metric. `analysis::embedding::Metric`.")
enum Metric {
  Dot: "dot",
  Cosine: "cosine",
  L2: "l2",
}

@doc("Which side of a retrieval a vector was encoded for. `analysis::embedding::Role` — renamed here because the tenancy `Role` already owns that name in this namespace.")
enum EmbeddingRole {
  Query: "query",
  Document: "document",
}

@doc("Index strategy for a comparison space. `analysis::embedding::AnnStrategy`.")
enum AnnStrategy {
  HalfvecExact: "halfvec_exact",
  HalfvecMrl: "halfvec_mrl",
  BinaryFullRerank: "binary_full_rerank",
}

@doc("How much a finding matters.")
enum FindingSeverity {
  Info: "info",
  Low: "low",
  Medium: "medium",
  High: "high",
  Critical: "critical",
}

// ---------------------------------------------------------------------------
// Models
// ---------------------------------------------------------------------------

@doc("A tenant. Every other row in this contract is reachable from exactly one org.")
@Ores.table("orgs")
@Ores.unique("slug")
model Org {
  @key id: uuid;

  @doc("URL label. GitHub's own limit is 39 characters.")
  @minLength(1)
  @maxLength(39)
  @pattern("^[a-z0-9][a-z0-9-]*$")
  slug: string;

  @minLength(1) @maxLength(120) displayName: string;
  kind: AccountKind;

  @doc("Surface an org member lands on when signed in.")
  primarySurface: Surface;

  @doc("Verified apex domain, when the org has proven one.")
  @maxLength(253)
  apexDomain?: string;

  createdAt: utcDateTime;
}

@doc("A person. One row per human across every org they belong to.")
@Ores.table("users")
@Ores.unique("email")
model User {
  @key id: uuid;

  @doc("Normalized (lowercased, trimmed) address. RFC 5321 caps the path at 320 octets.")
  @minLength(3)
  @maxLength(320)
  @pattern("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")
  email: string;

  @minLength(1) @maxLength(120) displayName?: string;

  @doc("Authenticator assurance level from `shared-auth` introspection. `runtime::authz::Introspection::assurance_level`.")
  @minValue(0)
  @maxValue(4)
  assuranceLevel: int32;

  createdAt: utcDateTime;
}

@doc("A user's role in one org. Deleting the last Owner is refused by `runtime::tenancy`.")
@Ores.table("memberships")
@Ores.unique("orgId,userId")
@Ores.index("orgId,role")
model Membership {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @Ores.references("User.id") userId: uuid;
  role: Role;
  createdAt: utcDateTime;
}

@doc("Seat accounting for one org. `runtime::tenancy::Seats`: a seat is available when purchased > occupied + reserved.")
@Ores.table("org_seats")
model Seats {
  @key @Ores.references("Org.id") orgId: uuid;
  @minValue(0) @maxValue(1000000) purchased: int32;
  @minValue(0) @maxValue(1000000) occupied: int32;

  @doc("Seats held by open invitations, so two invitations cannot spend one seat.")
  @minValue(0)
  @maxValue(1000000)
  reserved: int32;
}

@doc("An outstanding invitation to join an org. `runtime::tenancy::Invitation`.")
@Ores.table("invitations")
@Ores.unique("orgId,email")
@Ores.index("state,expiresAt")
model Invitation {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;

  @minLength(3)
  @maxLength(320)
  @pattern("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")
  email: string;

  @doc("Role granted on acceptance. Never above the inviter's own role.")
  role: Role;

  state: InvitationState;
  expiresAt: utcDateTime;
  createdAt: utcDateTime;
}

@doc("How strangers may join one org. `runtime::tenancy::JoinPolicy`.")
@Ores.table("join_policies")
model JoinPolicy {
  @key @Ores.references("Org.id") orgId: uuid;

  @doc("Domains the org has proven it controls, lowercased and without a leading dot.")
  @maxItems(64) verifiedDomains: string[];

  domainJoinEnabled: boolean;
  restrictInvitationsToVerifiedDomains: boolean;
  defaultRole: Role;
}

@doc("A workflow definition tracked in a repository.")
@Ores.table("workflows")
@Ores.unique("orgId,repository,path")
model Workflow {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;

  @doc("`owner/name`, as GitHub spells it.")
  @minLength(3)
  @maxLength(140)
  @pattern("^[A-Za-z0-9_.-]+/[A-Za-z0-9_.-]+$")
  repository: string;

  @doc("Path within the repository, conventionally `.github/workflows/<file>.yml`.")
  @minLength(1)
  @maxLength(512)
  path: string;

  @minLength(1) @maxLength(200) name: string;
  @minLength(1) @maxLength(255) defaultBranch: string;
  enabled: boolean;
}

@doc("One execution of a workflow.")
@Ores.table("runs")
@Ores.index("workflowId,createdAt")
@Ores.index("status")
model Run {
  @key id: uuid;
  @Ores.references("Workflow.id") workflowId: uuid;

  @doc("Denormalized tenant, so every run query is tenant-scoped without a join.")
  @Ores.references("Org.id")
  orgId: uuid;

  @minValue(1) runNumber: int64;
  @minValue(1) @maxValue(1000) attempt: int32;
  status: RunStatus;

  @doc("Set exactly when `status` is `completed`.")
  conclusion?: RunConclusion;

  @minLength(40) @maxLength(40) @pattern("^[0-9a-f]+$") headSha: string;

  @doc("Triggering event: `push`, `pull_request`, `schedule`, `workflow_dispatch`, …")
  @minLength(1)
  @maxLength(64)
  event: string;

  createdAt: utcDateTime;
  startedAt?: utcDateTime;
  completedAt?: utcDateTime;
}

@doc("One job of a run. Jobs of a run form the `needs` DAG; the edges live in `Job.needs`.")
@Ores.table("jobs")
@Ores.index("runId,startedAt")
model Job {
  @key id: uuid;
  @Ores.references("Run.id") runId: uuid;
  @minLength(1) @maxLength(200) name: string;
  status: RunStatus;
  conclusion?: RunConclusion;

  @doc("Runner that claimed the job. Absent while queued.")
  @Ores.references("Runner.id")
  runnerId?: uuid;

  isolation: IsolationMode;

  @doc("Names of jobs in the same run that must finish first.")
  @maxItems(256) needs: string[];

  startedAt?: utcDateTime;
  completedAt?: utcDateTime;
}

@doc("One step of a job, numbered from 1 in declaration order.")
@Ores.table("steps")
@Ores.unique("jobId,number")
model Step {
  @key id: uuid;
  @Ores.references("Job.id") jobId: uuid;
  @minValue(1) @maxValue(1000) number: int32;
  @minLength(1) @maxLength(200) name: string;
  status: RunStatus;
  conclusion?: RunConclusion;

  @doc("Action reference, which org policy requires be pinned to a 40-hex commit sha.")
  @maxLength(300)
  @pattern("^[A-Za-z0-9_.-]+/[A-Za-z0-9_./-]+@[0-9a-f]+$")
  usesRef?: string;
}

@doc("A pool of runners with its own repository scoping. Dimension `gov.runner-groups`.")
@Ores.table("runner_groups")
@Ores.unique("orgId,name")
model RunnerGroup {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @minLength(1) @maxLength(100) name: string;
  allowsPublicRepositories: boolean;

  @doc("`owner/name` repositories allowed to target this group. Empty means every repository in the org.")
  @maxItems(500) selectedRepositories: string[];
}

@doc("A registered runner. Ephemeral runners execute one job and are destroyed.")
@Ores.table("runners")
@Ores.unique("orgId,name")
@Ores.index("state")
model Runner {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @Ores.references("RunnerGroup.id") groupId?: uuid;
  @minLength(1) @maxLength(100) name: string;
  os: RunnerOs;
  arch: RunnerArch;
  state: RunnerState;
  ephemeral: boolean;
  isolation: IsolationMode;
  @maxItems(64) labels: string[];
  lastSeenAt?: utcDateTime;
}

@doc("A dependency-cache entry. `key` + `version` is the actions/cache restore identity.")
@Ores.table("cache_entries")
@Ores.unique("orgId,scope,key,version")
@Ores.index("orgId,lastAccessedAt")
model CacheEntry {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  scope: CacheScope;
  @minLength(1) @maxLength(512) key: string;

  @doc("Hash of the cache's declared inputs. 64 hex characters.")
  @minLength(64)
  @maxLength(64)
  @pattern("^[0-9a-f]+$")
  version: string;

  @minValue(0) sizeBytes: int64;
  createdAt: utcDateTime;
  lastAccessedAt: utcDateTime;
}

@doc("A workflow artifact stored in the object-store backend. Dimension `art`.")
@Ores.table("artifacts")
@Ores.unique("runId,name")
@Ores.index("expiresAt")
model Artifact {
  @key id: uuid;
  @Ores.references("Run.id") runId: uuid;
  @minLength(1) @maxLength(200) name: string;
  @minValue(0) sizeBytes: int64;

  @doc("Content digest, `sha256:` + 64 hex characters.")
  @minLength(71)
  @maxLength(71)
  @pattern("^sha256:[0-9a-f]+$")
  digest: string;

  @minValue(1) @maxValue(400) retentionDays: int32;
  expiresAt: utcDateTime;
}

@doc("A supply-chain attestation over an artifact or image produced by a run.")
@Ores.table("attestations")
@Ores.index("subjectDigest")
model Attestation {
  @key id: uuid;
  @Ores.references("Run.id") runId: uuid;
  kind: AttestationKind;

  @minLength(71)
  @maxLength(71)
  @pattern("^sha256:[0-9a-f]+$")
  subjectDigest: string;

  @doc("The DSSE envelope or Sigstore bundle, verbatim.")
  bundle: bytes;

  createdAt: utcDateTime;
}

@doc("The twelve axes that make two vectors comparable. `analysis::embedding::ComparisonSpace`. Vectors from different spaces are never compared; every space ships disabled.")
@Ores.table("comparison_spaces")
@Ores.unique("tenant,entity,model,modelRevision,dimensions,metric,embeddingRole,promptProfile,chunkProfile,normalization,truncatedTo,contractVersion")
model ComparisonSpace {
  @key id: uuid;

  @doc("Axis 1. Vectors never cross a tenant boundary, even within one model.")
  @minLength(1)
  @maxLength(64)
  tenant: string;

  @doc("Axis 2. What is embedded: `run-log`, `workflow`, `ticket`.")
  @minLength(1)
  @maxLength(64)
  entity: string;

  @doc("Axis 3. Model identifier, exactly as the provider names it.")
  @minLength(1)
  @maxLength(200)
  model: string;

  @doc("Axis 4. A silently re-trained model is a different space.")
  @minLength(1)
  @maxLength(100)
  modelRevision: string;

  @doc("Axis 5. Native dimension before padding to the storage slot.")
  @minValue(1)
  @maxValue(4096)
  dimensions: int32;

  @doc("Axis 6.")
  metric: Metric;

  @doc("Axis 7. Query and document vectors are compared against each other, never within.")
  embeddingRole: EmbeddingRole;

  @doc("Axis 8. Instruction/prefix template applied before encoding.")
  @minLength(1)
  @maxLength(100)
  promptProfile: string;

  @doc("Axis 9. Different chunking is different content.")
  @minLength(1)
  @maxLength(100)
  chunkProfile: string;

  @doc("Axis 10. Post-encoding normalization: `l2`, `none`.")
  @minLength(1)
  @maxLength(32)
  normalization: string;

  @doc("Axis 11. Matryoshka truncation width, when the stored vector is truncated.")
  @minValue(1)
  @maxValue(4096)
  truncatedTo?: int32;

  @doc("Axis 12. Contract version that produced the row.")
  @minLength(1)
  @maxLength(32)
  contractVersion: string;

  @doc("Enabling anything but `halfvec_exact` requires measured recall evidence.")
  annStrategy: AnnStrategy;

  enabled: boolean;
}

@doc("Something discovery surfaced about a run. Findings are evidence, never an action.")
@Ores.table("findings")
@Ores.index("orgId,severity")
model Finding {
  @key id: uuid;
  @Ores.references("Org.id") orgId: uuid;
  @Ores.references("ComparisonSpace.id") spaceId: uuid;
  @Ores.references("Run.id") runId?: uuid;
  severity: FindingSeverity;
  @minLength(1) @maxLength(200) title: string;
  @minLength(1) @maxLength(4000) detail: string;

  @doc("Stable identity so the same finding is not reported twice.")
  @minLength(16)
  @maxLength(32)
  @pattern("^[0-9a-f]+$")
  fingerprint: string;

  @doc("Similarity that produced the finding, in [0, 1].")
  @minValue(0)
  @maxValue(1)
  score: float64;

  createdAt: utcDateTime;
}
