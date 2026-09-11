#![forbid(unsafe_code)]

//! The JSON Schema authority, embedded at compile time.
//!
//! The `include_str!` calls live here — in the crate that owns the files — so the
//! bytes a consumer validates against are exactly the bytes in this repository,
//! and `cargo` rebuilds every dependent when a schema changes.
//! `gha-indie-worker-lib-core` re-exports these under its `embedded-schemas`
//! feature instead of reaching across the workspace with a relative path.
//!
//! These are the **JSON Schema** authority only. TypeSpec is the peer authority
//! and is never derived from these files, nor they from it; `npx ores-contracts
//! check` is what proves the two agree.

/// One contract slice: its name and its JSON Schema authority document.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Slice {
    /// Slice name, matching `contracts/json-schema/<name>.schema.json`.
    pub name: &'static str,
    /// The `x-ores-namespace` both authorities declare.
    pub namespace: &'static str,
    /// The verbatim JSON Schema document.
    pub schema: &'static str,
}

pub const IDENTITY: &str = include_str!("../../contracts/json-schema/identity.schema.json");
pub const ONBOARDING: &str = include_str!("../../contracts/json-schema/onboarding.schema.json");
pub const RUNS: &str = include_str!("../../contracts/json-schema/runs.schema.json");
pub const WORKERS: &str = include_str!("../../contracts/json-schema/workers.schema.json");
pub const WEBHOOKS: &str = include_str!("../../contracts/json-schema/webhooks.schema.json");
pub const CHAT: &str = include_str!("../../contracts/json-schema/chat.schema.json");
pub const EMBEDDINGS: &str = include_str!("../../contracts/json-schema/embeddings.schema.json");
pub const SYNC: &str = include_str!("../../contracts/json-schema/sync.schema.json");
pub const TRANSPORT: &str = include_str!("../../contracts/json-schema/transport.schema.json");
pub const ERRORS: &str = include_str!("../../contracts/json-schema/errors.schema.json");

/// Every slice, in the order `contracts.config.json` lists them.
pub const ALL: &[Slice] = &[
    Slice {
        name: "identity",
        namespace: "GhaIndieWorker.V1.Identity",
        schema: IDENTITY,
    },
    Slice {
        name: "onboarding",
        namespace: "GhaIndieWorker.V1.Onboarding",
        schema: ONBOARDING,
    },
    Slice {
        name: "runs",
        namespace: "GhaIndieWorker.V1.Runs",
        schema: RUNS,
    },
    Slice {
        name: "workers",
        namespace: "GhaIndieWorker.V1.Workers",
        schema: WORKERS,
    },
    Slice {
        name: "webhooks",
        namespace: "GhaIndieWorker.V1.Webhooks",
        schema: WEBHOOKS,
    },
    Slice {
        name: "chat",
        namespace: "GhaIndieWorker.V1.Chat",
        schema: CHAT,
    },
    Slice {
        name: "embeddings",
        namespace: "GhaIndieWorker.V1.Embeddings",
        schema: EMBEDDINGS,
    },
    Slice {
        name: "sync",
        namespace: "GhaIndieWorker.V1.Sync",
        schema: SYNC,
    },
    Slice {
        name: "transport",
        namespace: "GhaIndieWorker.V1.Transport",
        schema: TRANSPORT,
    },
    Slice {
        name: "errors",
        namespace: "GhaIndieWorker.V1.Errors",
        schema: ERRORS,
    },
];

/// Look up a slice's JSON Schema authority by name.
#[must_use]
pub fn schema_for(slice: &str) -> Option<&'static str> {
    ALL.iter().find(|s| s.name == slice).map(|s| s.schema)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_slice_is_valid_json_with_the_declared_namespace() {
        for slice in ALL {
            let doc: serde_json::Value = serde_json::from_str(slice.schema)
                .unwrap_or_else(|e| panic!("{} is not valid JSON: {e}", slice.name));
            assert_eq!(
                doc["x-ores-namespace"].as_str(),
                Some(slice.namespace),
                "{} namespace",
                slice.name
            );
            let defs = doc["$defs"].as_object().expect("$defs");
            assert!(!defs.is_empty(), "{} has no $defs", slice.name);
            assert_eq!(
                doc["$schema"].as_str(),
                Some("https://json-schema.org/draft/2020-12/schema"),
                "{} must declare draft 2020-12",
                slice.name
            );
        }
    }

    #[test]
    fn every_model_is_sealed_and_keyed() {
        for slice in ALL {
            let doc: serde_json::Value = serde_json::from_str(slice.schema).unwrap();
            for (name, def) in doc["$defs"].as_object().unwrap() {
                if def.get("enum").is_some() {
                    continue;
                }
                assert_eq!(
                    def["additionalProperties"],
                    serde_json::Value::Bool(false),
                    "{}.{name} must be sealed",
                    slice.name
                );
                assert!(
                    def["x-ores-primary-key"].is_array(),
                    "{}.{name} needs x-ores-primary-key",
                    slice.name
                );
                assert!(
                    def["x-ores-table"].is_string(),
                    "{}.{name} needs x-ores-table",
                    slice.name
                );
            }
        }
    }

    #[test]
    fn lookup_by_name() {
        assert_eq!(schema_for("runs"), Some(RUNS));
        assert_eq!(schema_for("nope"), None);
        assert_eq!(ALL.len(), 10);
    }
}
