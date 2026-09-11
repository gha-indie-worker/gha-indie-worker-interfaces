import "./ores.tsp";
using Ores;

namespace GhaIndieWorker.V1.Onboarding;

enum OrgOnboardingState {
  Created: "created",
  OrgProfile: "org-profile",
  BillingLinked: "billing-linked",
  GithubAppInstalled: "github-app-installed",
  FirstWorkflowPlanned: "first-workflow-planned",
  Active: "active",
  Suspended: "suspended",
}

enum UserOnboardingState {
  Created: "created",
  EmailVerified: "email-verified",
  ProfileComplete: "profile-complete",
  OrgJoined: "org-joined",
  Active: "active",
  Dormant: "dormant",
}

enum OnboardingSubjectKind {
  Org: "org",
  User: "user",
}

enum OnboardingOutcome {
  Advanced: "advanced",
  AlreadyThere: "already-there",
  Rejected: "rejected",
}

@doc("One accepted org-onboarding state transition. The table is the audit log of the org state machine.")
@Ores.table("org_onboarding_transitions")
@Ores.unique("orgId,sequence")
@Ores.index("orgId,occurredAt")
model OrgOnboardingTransition {
  @key id: uuid;
  orgId: uuid;
  sequence: int64;
  fromState: OrgOnboardingState;
  toState: OrgOnboardingState;

  @minLength(1)
  @maxLength(64) event: string;

  occurredAt: utcDateTime;
  actorUserId?: uuid;
}

@doc("One accepted user-onboarding state transition.")
@Ores.table("user_onboarding_transitions")
@Ores.unique("userId,sequence")
@Ores.index("userId,occurredAt")
model UserOnboardingTransition {
  @key id: uuid;
  userId: uuid;
  sequence: int64;
  fromState: UserOnboardingState;
  toState: UserOnboardingState;

  @minLength(1)
  @maxLength(64) event: string;

  occurredAt: utcDateTime;
}

@doc("A request to advance an onboarding state machine by one event. Replayable through idempotencyKey.")
@Ores.table("onboarding_advance_requests")
@Ores.unique("idempotencyKey")
@Ores.index("subjectKind,subjectId")
model AdvanceRequest {
  @key id: uuid;
  subjectKind: OnboardingSubjectKind;
  subjectId: uuid;

  @minLength(1)
  @maxLength(64) event: string;

  @minLength(8)
  @maxLength(128) idempotencyKey: string;

  requestedAt: utcDateTime;
  requestedBy?: uuid;
}

@doc("The settled outcome of an AdvanceRequest. Exactly one of orgState or userState is present.")
@Ores.table("onboarding_advance_responses")
@Ores.unique("requestId")
@Ores.index("outcome,respondedAt")
model AdvanceResponse {
  @key id: uuid;
  @Ores.references("AdvanceRequest.id") requestId: uuid;
  outcome: OnboardingOutcome;
  orgState?: OrgOnboardingState;
  userState?: UserOnboardingState;

  @maxLength(200) reason?: string;

  respondedAt: utcDateTime;
}
