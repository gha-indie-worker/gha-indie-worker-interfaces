#![forbid(unsafe_code)]

//! Accessor for the organization's WASM release profile schema.
//!
//! Follows the same shape as [`crate::schema`]: data only, no validator, no new
//! dependencies. The schema text is embedded with [`include_str!`] so a consumer
//! gets the exact bytes committed in this repository rather than fetching a URL.
//!
//! # Layering
//!
//! The OWLS `release-v1` contract is the BASE AUTHORITY. This profile only
//! narrows it. Validate in this order, and stop at the first failure:
//!
//! 1. `release-v1` ([`RELEASE_V1_SCHEMA_ID`]) — owned by `ores-wasm-loaders`.
//! 2. [`WASM_RELEASE_PROFILE_SCHEMA`] — owned by this organization.
//!
//! A document that fails step 1 is invalid regardless of step 2. This crate
//! deliberately embeds only step 2: `release-v1` belongs to another
//! organization, and vendoring a copy here would create a second, silently
//! diverging authority.

/// Bumped whenever the profile schema's meaning changes.
pub const WASM_RELEASE_PROFILE_REVISION: &str = "gha-indie-worker-wasm-release-profile-0001";

/// Canonical `$id` of the profile schema.
pub const WASM_RELEASE_PROFILE_SCHEMA_ID: &str =
    "https://github.com/gha-indie-worker/gha-indie-worker-interfaces/schema/v1/wasm-release-profile.json";

/// Canonical `$id` of the base contract this profile narrows.
///
/// Owned by `ores-wasm-loaders`, not by this organization. Recorded here so a
/// consumer knows which schema to evaluate first; the text is intentionally not
/// embedded.
pub const RELEASE_V1_SCHEMA_ID: &str =
    "https://ores-wasm-loaders.github.io/schemas/release-v1.json";

/// The profile schema source, exactly as committed.
pub const WASM_RELEASE_PROFILE_SCHEMA: &str =
    include_str!("../schema/v1/wasm-release-profile.json");

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_parses_as_json() {
        let value: serde_json::Value = serde_json::from_str(WASM_RELEASE_PROFILE_SCHEMA)
            .expect("embedded profile schema must be valid JSON");
        assert!(value.is_object(), "profile schema must be a JSON object");
    }

    #[test]
    fn schema_id_matches_the_constant() {
        let value: serde_json::Value =
            serde_json::from_str(WASM_RELEASE_PROFILE_SCHEMA).expect("valid JSON");
        assert_eq!(
            value["$id"].as_str(),
            Some(WASM_RELEASE_PROFILE_SCHEMA_ID),
            "the embedded schema's $id must match WASM_RELEASE_PROFILE_SCHEMA_ID"
        );
        assert_eq!(
            value["$schema"].as_str(),
            Some("https://json-schema.org/draft/2020-12/schema"),
            "the profile must declare draft 2020-12, like schema/v1/workerlease.json"
        );
    }

    #[test]
    fn profile_names_the_base_contract_it_narrows() {
        let value: serde_json::Value =
            serde_json::from_str(WASM_RELEASE_PROFILE_SCHEMA).expect("valid JSON");
        let description = value["description"].as_str().unwrap_or_default();
        assert!(
            description.contains(RELEASE_V1_SCHEMA_ID),
            "the profile's description must name the base contract it narrows"
        );
    }
}
