#![forbid(unsafe_code)]

//! Serde mirror of the `errors` slice: RFC 9457 Problem Details.

use serde::{Deserialize, Serialize};
use serde_json::Value;

/// The registry prefix every `type` URI in this fleet lives under.
pub const PROBLEM_TYPE_PREFIX: &str = "https://indiebuild.dev/problems/";
/// The media type servers must set when returning a [`Problem`].
pub const PROBLEM_CONTENT_TYPE: &str = "application/problem+json";

/// An RFC 9457 problem document.
#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Problem {
    pub id: String,
    #[serde(rename = "type")]
    pub r#type: String,
    pub title: String,
    pub status: i32,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub detail: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub instance: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub code: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub trace_id: Option<String>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub extensions: Option<Value>,
    pub occurred_at: String,
}

impl Problem {
    /// `type` is inside the published registry and `status` is a real HTTP status.
    #[must_use]
    pub fn is_well_formed(&self) -> bool {
        self.r#type.starts_with(PROBLEM_TYPE_PREFIX)
            && (100..=599).contains(&self.status)
            && !self.title.is_empty()
    }

    /// The problem slug, i.e. the `type` URI with the registry prefix removed.
    #[must_use]
    pub fn slug(&self) -> Option<&str> {
        self.r#type.strip_prefix(PROBLEM_TYPE_PREFIX)
    }
}

/// One field-level violation attached to a problem.
#[derive(Clone, Debug, Eq, PartialEq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ProblemViolation {
    pub id: String,
    pub problem_id: String,
    pub pointer: String,
    pub rule: String,
    pub message: String,
}

#[cfg(test)]
mod tests {
    use super::*;

    fn problem() -> Problem {
        Problem {
            id: "p1".into(),
            r#type: format!("{PROBLEM_TYPE_PREFIX}plan-unsupported"),
            title: "Workflow is outside the independent subset".into(),
            status: 422,
            detail: None,
            instance: None,
            code: Some("plan-unsupported".into()),
            trace_id: None,
            extensions: None,
            occurred_at: "2026-03-14T09:26:53Z".into(),
        }
    }

    #[test]
    fn type_field_serializes_as_type() {
        let json = serde_json::to_string(&problem()).unwrap();
        assert!(json.contains("\"type\":\"https://indiebuild.dev/problems/plan-unsupported\""));
        assert!(!json.contains("\"r#type\""));
    }

    #[test]
    fn registry_prefix_and_status_range_are_enforced() {
        assert!(problem().is_well_formed());
        assert_eq!(problem().slug(), Some("plan-unsupported"));
        assert!(!Problem {
            status: 42,
            ..problem()
        }
        .is_well_formed());
        assert!(!Problem {
            r#type: "urn:oops".into(),
            ..problem()
        }
        .is_well_formed());
        assert_eq!(
            Problem {
                r#type: "urn:oops".into(),
                ..problem()
            }
            .slug(),
            None
        );
    }
}
