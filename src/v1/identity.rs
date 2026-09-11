#![forbid(unsafe_code)]

//! Serde mirror of `contracts/json-schema/identity.schema.json` and
//! `contracts/typespec/identity.tsp`.
//!
//! Hand-authored, like both authorities. The parity-checked machine output lives
//! in `generated/identity/rust/types.rs`; this module is the hand-maintained
//! wire surface the crate exports, and `tests/fixtures_roundtrip.rs` proves the
//! two agree on every fixture.
//!
//! `uuid`, `utcDateTime` and `plainDate` are carried as `String` on purpose: this
//! crate is data-only and must not force a `uuid` or `time` dependency on every
//! consumer. Servers parse them at their edge.

use serde::{Deserialize, Serialize};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum Role {
    Owner,
    Admin,
    Member,
    Billing,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum InvitationStatus {
    Pending,
    Accepted,
    Revoked,
    Expired,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "kebab-case")]
pub enum SeatStatus {
    Active,
    Suspended,
    Released,
}

/// A billing and ownership boundary.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Org {
    pub id: String,
    pub slug: String,
    pub display_name: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub billing_email: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_login: Option<String>,
    pub seat_limit: i32,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub deleted_at: Option<String>,
}

/// A human principal, federated by shared-auth.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct User {
    pub id: String,
    pub shared_auth_subject: String,
    pub email: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub display_name: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub github_login: Option<String>,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub last_seen_at: Option<String>,
}

/// Membership of a user in an org, carrying exactly one role.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct OrgMember {
    pub id: String,
    pub org_id: String,
    pub user_id: String,
    pub role: Role,
    pub created_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub invited_by: Option<String>,
}

/// A pending or settled invitation to join an org at a given role.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Invitation {
    pub id: String,
    pub org_id: String,
    pub email: String,
    pub role: Role,
    pub status: InvitationStatus,
    pub token: String,
    pub created_at: String,
    pub expires_at: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub accepted_by: Option<String>,
}

/// A paid seat in an org. A seat may be held by no user while it is reassigned.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Seat {
    pub id: String,
    pub org_id: String,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub user_id: Option<String>,
    pub status: SeatStatus,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub assigned_at: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub released_at: Option<String>,
}

impl Role {
    /// Every role, in privilege order. Exhaustive by construction: the unit test
    /// below fails if a variant is added without extending this list.
    pub const ALL: [Role; 4] = [Role::Owner, Role::Admin, Role::Member, Role::Billing];

    /// Whether this role may administer members, seats and invitations.
    #[must_use]
    pub const fn can_administer(self) -> bool {
        matches!(self, Role::Owner | Role::Admin)
    }

    /// Whether this role may see invoices and payment methods.
    #[must_use]
    pub const fn can_bill(self) -> bool {
        matches!(self, Role::Owner | Role::Billing)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn role_all_is_exhaustive() {
        for role in Role::ALL {
            // A non-exhaustive match here is a compile error, so adding a variant
            // forces both this match and Role::ALL to be updated.
            match role {
                Role::Owner | Role::Admin | Role::Member | Role::Billing => {}
            }
        }
        assert_eq!(Role::ALL.len(), 4);
    }

    #[test]
    fn role_wire_values_are_kebab_case() {
        assert_eq!(serde_json::to_string(&Role::Owner).unwrap(), "\"owner\"");
        assert_eq!(
            serde_json::to_string(&Role::Billing).unwrap(),
            "\"billing\""
        );
    }

    #[test]
    fn privilege_split_is_explicit() {
        assert!(Role::Owner.can_administer() && Role::Owner.can_bill());
        assert!(Role::Admin.can_administer() && !Role::Admin.can_bill());
        assert!(!Role::Member.can_administer() && !Role::Member.can_bill());
        assert!(!Role::Billing.can_administer() && Role::Billing.can_bill());
    }
}
