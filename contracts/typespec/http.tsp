// ============================================================================
// TypeSpec authority — HTTP surface: operations, bindings and the error model.
//
// PEER of contracts/json-schema/errors.schema.json (the error model and the
// pagination envelope; the operations themselves are TypeSpec-only, because a
// JSON Schema document cannot express an HTTP binding and pretending otherwise
// would make the comparison dishonest — see contracts/README.md).
//
// Every operation is tenant-scoped by path. There is no ambient "current org":
// a handler that forgets the tenant cannot compile against these routes.
// ============================================================================
import "@typespec/http";

using Http;

namespace GhaIndieWorker;

// ---------------------------------------------------------------------------
// Error model
// ---------------------------------------------------------------------------

@doc("Which identity realm a token came from. A token is only ever valid in one. `runtime::authz::Realm`.")
enum Realm {
  @doc("End users and organizations. Never accepted by the admin plane.")
  Customer: "customer",

  @doc("Super-admins. Never accepted by the product plane.")
  Admin: "admin",
}

@doc("The audience a service introspects FOR. Introspecting without an audience is how a token minted for one service gets replayed against another, so every call names one. `runtime::authz::Audience`.")
enum Audience {
  Web: "indiebuild-web",
  Api: "indiebuild-api",
  AdminWeb: "indiebuild-admin-web",
  AdminApi: "indiebuild-admin",
}

@doc("The caller a handler actually sees, after introspection. `runtime::authz::VerifiedActor`. The realm/audience pair is checked before this value exists, so a handler holding one cannot be serving the wrong plane.")
model Principal {
  @doc("Subject claim, stable for the life of the account.")
  @minLength(1)
  @maxLength(255)
  subject: string;

  realm: Realm;
  audience: Audience;

  @doc("Authenticator assurance level. `runtime::authz::Introspection::assurance_level`.")
  @minValue(0)
  @maxValue(4)
  assuranceLevel: int32;

  @doc("Org the token is scoped to. Absent for an individual account with no org.")
  orgId?: uuid;

  @doc("Role in `orgId`. Present exactly when `orgId` is.")
  role?: Role;
}

@doc("Why an onboarding step was refused. One variant per `runtime::tenancy::OnboardingError`, so a client can branch on the same set the domain does instead of matching on prose.")
enum OnboardingErrorCode {
  @doc("Every seat is taken; the org must buy more before inviting.")
  NoSeatsAvailable: "no_seats_available",

  @doc("The email is not in a domain this organization has verified, and the org requires it.")
  DomainNotAllowed: "domain_not_allowed",

  @doc("An invitation was already accepted, revoked, or has expired.")
  InvitationNotOpen: "invitation_not_open",

  @doc("The invitation was minted for a different address.")
  InvitationAddressMismatch: "invitation_address_mismatch",

  @doc("The last owner cannot be demoted or removed; an org without an owner is unadministrable.")
  LastOwner: "last_owner",

  @doc("A member cannot grant a role above their own.")
  RoleEscalation: "role_escalation",

  @doc("The address is not shaped like an email we can send to.")
  InvalidEmail: "invalid_email",
}

@doc("The structured half of an onboarding refusal. `OnboardingError` carries payload on three of its variants; those are the optional fields here, and a client reads only the ones its `reason` defines.")
model OnboardingFailure {
  reason: OnboardingErrorCode;

  @doc("Set when `reason` is `no_seats_available`.")
  @minValue(0)
  @maxValue(1000000)
  purchased?: int32;

  @doc("Set when `reason` is `no_seats_available`.")
  @minValue(0)
  @maxValue(1000000)
  occupied?: int32;

  @doc("Set when `reason` is `no_seats_available`.")
  @minValue(0)
  @maxValue(1000000)
  reserved?: int32;

  @doc("Set when `reason` is `domain_not_allowed`.")
  @minLength(1)
  @maxLength(253)
  domain?: string;

  @doc("Set when `reason` is `role_escalation`: the role the caller holds.")
  actorRole?: Role;

  @doc("Set when `reason` is `role_escalation`: the role they tried to grant.")
  grantedRole?: Role;
}

@doc("One machine-readable detail of a failure, naming the offending field.")
model ErrorDetail {
  @doc("JSON Pointer to the offending member of the request, e.g. `/labels/3`.")
  @minLength(1)
  @maxLength(512)
  pointer: string;

  @minLength(1) @maxLength(1024) message: string;
}

@doc("The single error shape every operation returns. `code` is the same vocabulary the session protocol closes with, so a client has one error table for both transports.")
@error
model ApiError {
  @statusCode statusCode: 400 | 401 | 403 | 404 | 409 | 429 | 500;

  code: ErrorCode;

  @doc("Human-readable, safe to log. Never contains credentials or request bodies.")
  @minLength(1)
  @maxLength(1024)
  message: string;

  @doc("Correlates with the `request-id` response header and the trace. `runtime` puts it in every log line.")
  @minLength(1)
  @maxLength(64)
  requestId: string;

  @maxItems(64) details?: ErrorDetail[];

  @doc("Present on a 409 from an onboarding operation. Lets a client tell `no_seats_available` from `last_owner` without parsing `message`.")
  onboarding?: OnboardingFailure;
}

// ---------------------------------------------------------------------------
// Pagination — opaque cursors only. Offsets drift under concurrent writes.
// ---------------------------------------------------------------------------

@doc("Cursor pagination envelope. Cursors are opaque; a client that parses one is relying on an implementation detail.")
model PageInfo {
  @doc("Cursor for the next page. Absent on the last page.")
  @minLength(1)
  @maxLength(512)
  nextCursor?: string;

  @doc("True when a further page exists. Equivalent to `nextCursor` being present, stated so a client need not infer it.")
  hasMore: boolean;
}

model Page<T> {
  @maxItems(200) items: T[];
  page: PageInfo;
}

model ListQuery {
  @query @minValue(1) @maxValue(200) limit?: int32;
  @query @minLength(1) @maxLength(512) cursor?: string;
}

// ---------------------------------------------------------------------------
// Operations
// ---------------------------------------------------------------------------

@route("/v1")
@tag("identity")
interface Session {
  @doc("The caller as this service resolved them. The only operation that is not tenant-scoped by path, because its answer is what tells you which tenants you may name.")
  @get
  @route("/whoami")
  whoami(): Principal | ApiError;
}

@route("/v1/orgs/{orgId}")
@tag("identity")
interface Orgs {
  @get @route("") get(@path orgId: uuid): Org | ApiError;

  @get @route("/members") listMembers(@path orgId: uuid, ...ListQuery): Page<Membership> | ApiError;

  @get @route("/seats") seats(@path orgId: uuid): Seats | ApiError;

  @doc("Invite an address. Reserves a seat, so two invitations cannot spend the same one.")
  @post
  @route("/invitations")
  invite(@path orgId: uuid, @body body: InviteRequest): Invitation | ApiError;

  @doc("Revoke an open invitation and release its reserved seat.")
  @delete
  @route("/invitations/{invitationId}")
  revokeInvitation(@path orgId: uuid, @path invitationId: uuid): void | ApiError;

  @get @route("/join-policy") joinPolicy(@path orgId: uuid): JoinPolicy | ApiError;
}

model InviteRequest {
  @minLength(3)
  @maxLength(320)
  @pattern("^[^@\\s]+@[^@\\s]+\\.[^@\\s]+$")
  email: string;

  @doc("Never above the caller's own role; `runtime::tenancy` refuses escalation.")
  role: Role;
}

@route("/v1/orgs/{orgId}/workflows")
@tag("ci")
interface Workflows {
  @get @route("") list(@path orgId: uuid, @query repository?: string, ...ListQuery): Page<Workflow> | ApiError;
  @get @route("/{workflowId}") get(@path orgId: uuid, @path workflowId: uuid): Workflow | ApiError;

  @get
  @route("/{workflowId}/runs")
  listRuns(
    @path orgId: uuid,
    @path workflowId: uuid,
    @query status?: RunStatus,
    ...ListQuery,
  ): Page<Run> | ApiError;
}

@route("/v1/orgs/{orgId}/runs")
@tag("ci")
interface Runs {
  @get @route("/{runId}") get(@path orgId: uuid, @path runId: uuid): Run | ApiError;
  @get @route("/{runId}/jobs") listJobs(@path orgId: uuid, @path runId: uuid, ...ListQuery): Page<Job> | ApiError;

  @get
  @route("/{runId}/jobs/{jobId}/steps")
  listSteps(@path orgId: uuid, @path runId: uuid, @path jobId: uuid): Page<Step> | ApiError;

  @doc("Cancel a run. Idempotent: cancelling an already-terminal run is a 409, never a partial cancel.")
  @post
  @route("/{runId}/cancel")
  cancel(@path orgId: uuid, @path runId: uuid): Run | ApiError;

  @doc("Re-run only the failed jobs, as a new attempt of the same run. Dimension `ux.rerun-failed`.")
  @post
  @route("/{runId}/rerun-failed")
  rerunFailed(@path orgId: uuid, @path runId: uuid): Run | ApiError;

  @get @route("/{runId}/artifacts") listArtifacts(@path orgId: uuid, @path runId: uuid, ...ListQuery): Page<Artifact> | ApiError;
  @get @route("/{runId}/attestations") listAttestations(@path orgId: uuid, @path runId: uuid): Page<Attestation> | ApiError;
}

@route("/v1/orgs/{orgId}/runners")
@tag("ci")
interface Runners {
  @get @route("") list(@path orgId: uuid, @query state?: RunnerState, ...ListQuery): Page<Runner> | ApiError;
  @get @route("/{runnerId}") get(@path orgId: uuid, @path runnerId: uuid): Runner | ApiError;

  @doc("Deregister a runner. An ephemeral runner deregisters itself after one job.")
  @delete
  @route("/{runnerId}")
  remove(@path orgId: uuid, @path runnerId: uuid): void | ApiError;

  @get @route("/groups") listGroups(@path orgId: uuid, ...ListQuery): Page<RunnerGroup> | ApiError;
}

@route("/v1/orgs/{orgId}/caches")
@tag("ci")
interface Caches {
  @get @route("") list(@path orgId: uuid, @query scope?: CacheScope, @query key?: string, ...ListQuery): Page<CacheEntry> | ApiError;

  @doc("Evict one entry. Eviction is explicit; quota eviction is a separate scheduled path.")
  @delete
  @route("/{cacheId}")
  evict(@path orgId: uuid, @path cacheId: uuid): void | ApiError;
}
