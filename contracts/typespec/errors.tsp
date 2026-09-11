import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Errors;

// RFC 9457 Problem Details for HTTP APIs. `type` is an absolute URI that
// identifies the problem class; the fleet serves its registry under
// https://indiebuild.dev/problems/ and never changes a published meaning.
// `code` and `traceId` are the two registered extension members. Servers emit
// this document with Content-Type application/problem+json.

@doc("An RFC 9457 problem document as emitted by every server in the fleet, and as persisted for support triage.")
@Ores.table("problem_events")
@Ores.index("status,occurredAt")
@Ores.index("code")
model Problem {
  @key id: uuid;

  @minLength(1)
  @maxLength(255) type: string;

  @minLength(1)
  @maxLength(200) title: string;

  @minValue(100)
  @maxValue(599) status: int32;

  @maxLength(2000) detail?: string;

  @maxLength(255) instance?: string;

  @maxLength(64)
  @pattern("^[a-z0-9][a-z0-9-]*[a-z0-9]$") code?: string;

  @maxLength(64) traceId?: string;

  extensions?: json;
  occurredAt: utcDateTime;
}

@doc("One field-level violation attached to a problem. pointer is an RFC 6901 JSON pointer into the rejected document.")
@Ores.table("problem_violations")
@Ores.unique("problemId,pointer,rule")
@Ores.index("rule")
model ProblemViolation {
  @key id: uuid;
  @Ores.references("Problem.id") problemId: uuid;

  @maxLength(255) pointer: string;

  @minLength(1)
  @maxLength(64) rule: string;

  @minLength(1)
  @maxLength(400) message: string;
}
