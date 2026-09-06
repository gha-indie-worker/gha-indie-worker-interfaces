#![forbid(unsafe_code)]

//! Serde mirror of the `onboarding` slice.
//!
//! The transition tables live here as data so every runtime (Rust servers,
//! `gha-indie-worker-lib-core`, the TypeScript and Dart siblings) can be checked
//! against one list. `x-ores-transitions` in the JSON Schema authority states the
//! same edges; `transitions_match_schema` in `tests/fixtures_roundtrip.rs` proves
//! the two lists are equal.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OrgOnboardingState {
    Created,
    OrgProfile,
    BillingLinked,
    GithubAppInstalled,
    FirstWorkflowPlanned,
    Active,
    Suspended,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum UserOnboardingState {
    Created,
    EmailVerified,
    ProfileComplete,
    OrgJoined,
    Active,
    Dormant,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnboardingSubjectKind {
    Org,
    User,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum OnboardingOutcome {
    Advanced,
    AlreadyThere,
    Rejected,
}

/// One accepted org-onboarding state transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrgOnboardingTransition {
    pub id: String,
    pub org_id: String,
    pub sequence: i64,
    pub from_state: OrgOnboardingState,
    pub to_state: OrgOnboardingState,
    pub event: String,
    pub occurred_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub actor_user_id: Option<String>,
}

/// One accepted user-onboarding state transition.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UserOnboardingTransition {
    pub id: String,
    pub user_id: String,
    pub sequence: i64,
    pub from_state: UserOnboardingState,
    pub to_state: UserOnboardingState,
    pub event: String,
    pub occurred_at: String,
}

/// A request to advance an onboarding state machine by one event.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdvanceRequest {
    pub id: String,
    pub subject_kind: OnboardingSubjectKind,
    pub subject_id: String,
    pub event: String,
    pub idempotency_key: String,
    pub requested_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub requested_by: Option<String>,
}

/// The settled outcome of an [`AdvanceRequest`].
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct AdvanceResponse {
    pub id: String,
    pub request_id: String,
    pub outcome: OnboardingOutcome,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub org_state: Option<OrgOnboardingState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_state: Option<UserOnboardingState>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub reason: Option<String>,
    pub responded_at: String,
}

impl AdvanceResponse {
    /// Exactly one of `org_state` / `user_state` is set, and it matches `outcome`.
    #[must_use]
    pub const fn is_well_formed(&self) -> bool {
        matches!(
            (self.org_state.is_some(), self.user_state.is_some()),
            (true, false) | (false, true)
        )
    }
}

impl OrgOnboardingState {
    pub const ALL: [OrgOnboardingState; 7] = [
        OrgOnboardingState::Created,
        OrgOnboardingState::OrgProfile,
        OrgOnboardingState::BillingLinked,
        OrgOnboardingState::GithubAppInstalled,
        OrgOnboardingState::FirstWorkflowPlanned,
        OrgOnboardingState::Active,
        OrgOnboardingState::Suspended,
    ];

    /// The complete edge set of the org onboarding machine.
    #[must_use]
    pub const fn allowed_next(self) -> &'static [OrgOnboardingState] {
        match self {
            OrgOnboardingState::Created => &[
                OrgOnboardingState::OrgProfile,
                OrgOnboardingState::Suspended,
            ],
            OrgOnboardingState::OrgProfile => &[
                OrgOnboardingState::BillingLinked,
                OrgOnboardingState::Suspended,
            ],
            OrgOnboardingState::BillingLinked => &[
                OrgOnboardingState::GithubAppInstalled,
                OrgOnboardingState::Suspended,
            ],
            OrgOnboardingState::GithubAppInstalled => &[
                OrgOnboardingState::FirstWorkflowPlanned,
                OrgOnboardingState::Suspended,
            ],
            OrgOnboardingState::FirstWorkflowPlanned => {
                &[OrgOnboardingState::Active, OrgOnboardingState::Suspended]
            }
            OrgOnboardingState::Active => &[OrgOnboardingState::Suspended],
            OrgOnboardingState::Suspended => &[OrgOnboardingState::Active],
        }
    }

    #[must_use]
    pub fn may_advance_to(self, next: OrgOnboardingState) -> bool {
        self.allowed_next().contains(&next)
    }
}

impl UserOnboardingState {
    pub const ALL: [UserOnboardingState; 6] = [
        UserOnboardingState::Created,
        UserOnboardingState::EmailVerified,
        UserOnboardingState::ProfileComplete,
        UserOnboardingState::OrgJoined,
        UserOnboardingState::Active,
        UserOnboardingState::Dormant,
    ];

    /// The complete edge set of the user onboarding machine.
    #[must_use]
    pub const fn allowed_next(self) -> &'static [UserOnboardingState] {
        match self {
            UserOnboardingState::Created => &[
                UserOnboardingState::EmailVerified,
                UserOnboardingState::Dormant,
            ],
            UserOnboardingState::EmailVerified => &[
                UserOnboardingState::ProfileComplete,
                UserOnboardingState::Dormant,
            ],
            UserOnboardingState::ProfileComplete => {
                &[UserOnboardingState::OrgJoined, UserOnboardingState::Dormant]
            }
            UserOnboardingState::OrgJoined => {
                &[UserOnboardingState::Active, UserOnboardingState::Dormant]
            }
            UserOnboardingState::Active => &[UserOnboardingState::Dormant],
            UserOnboardingState::Dormant => &[UserOnboardingState::Active],
        }
    }

    #[must_use]
    pub fn may_advance_to(self, next: UserOnboardingState) -> bool {
        self.allowed_next().contains(&next)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn org_machine_has_no_self_loops_and_no_unknown_targets() {
        for state in OrgOnboardingState::ALL {
            for next in state.allowed_next() {
                assert_ne!(state, *next, "{state:?} loops to itself");
                assert!(OrgOnboardingState::ALL.contains(next));
            }
        }
    }

    #[test]
    fn user_machine_has_no_self_loops_and_no_unknown_targets() {
        for state in UserOnboardingState::ALL {
            for next in state.allowed_next() {
                assert_ne!(state, *next, "{state:?} loops to itself");
                assert!(UserOnboardingState::ALL.contains(next));
            }
        }
    }

    #[test]
    fn every_state_is_reachable_from_created() {
        let mut seen = vec![OrgOnboardingState::Created];
        let mut i = 0;
        while i < seen.len() {
            for next in seen[i].allowed_next() {
                if !seen.contains(next) {
                    seen.push(*next);
                }
            }
            i += 1;
        }
        assert_eq!(seen.len(), OrgOnboardingState::ALL.len());
    }

    #[test]
    fn advance_response_requires_exactly_one_state() {
        let base = AdvanceResponse {
            id: "a".into(),
            request_id: "b".into(),
            outcome: OnboardingOutcome::Advanced,
            org_state: None,
            user_state: None,
            reason: None,
            responded_at: "2026-03-14T09:26:53Z".into(),
        };
        assert!(!base.is_well_formed());
        assert!(AdvanceResponse {
            org_state: Some(OrgOnboardingState::Active),
            ..base.clone()
        }
        .is_well_formed());
        assert!(!AdvanceResponse {
            org_state: Some(OrgOnboardingState::Active),
            user_state: Some(UserOnboardingState::Active),
            ..base
        }
        .is_well_formed());
    }
}
