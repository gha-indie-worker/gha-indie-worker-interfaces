#![forbid(unsafe_code)]

//! Version 1 of the GHA Indie Worker contract, as Rust types.
//!
//! One module per contract slice. Each module mirrors the two authorities in
//! `contracts/` — `contracts/typespec/<slice>.tsp` and
//! `contracts/json-schema/<slice>.schema.json` — which are independent peers:
//! **neither authority is generated from the other, and neither is generated
//! from this Rust.**
//!
//! What each artifact is for:
//!
//! | artifact | authority? | who writes it |
//! |---|---|---|
//! | `contracts/typespec/*.tsp` | yes | humans |
//! | `contracts/json-schema/*.schema.json` | yes | humans |
//! | `generated/<slice>/**` | no | `npx ores-contracts generate`, committed by the lane |
//! | `src/v1/*.rs` (this tree) | no | humans — the ergonomic wire surface the crate exports |
//!
//! `tests/fixtures_roundtrip.rs` is the seam: every fixture in
//! `contracts/fixtures/` must round-trip through the types here, so this module
//! cannot drift from the JSON Schema authority without CI noticing.
//!
//! Scalar mapping: `uuid`, `utcDateTime` and `plainDate` are `String`, because
//! this crate is data-only and must not push a `uuid`/`time` dependency onto
//! every consumer. `json` is `serde_json::Value`. `int32`/`int64`/`float64` are
//! `i32`/`i64`/`f64`.

pub mod chat;
pub mod embeddings;
pub mod errors;
pub mod identity;
pub mod onboarding;
pub mod runs;
pub mod schemas;
pub mod sync;
pub mod transport;
pub mod webhooks;
pub mod workers;

/// The contract major version these modules implement.
pub const CONTRACT_VERSION: &str = "v1";

/// Names of every slice, matching `contracts.config.json`.
pub const SLICES: [&str; 10] = [
    "identity",
    "onboarding",
    "runs",
    "workers",
    "webhooks",
    "chat",
    "embeddings",
    "sync",
    "transport",
    "errors",
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slice_list_matches_the_embedded_schemas() {
        let embedded: Vec<&str> = schemas::ALL.iter().map(|s| s.name).collect();
        assert_eq!(embedded, SLICES.to_vec());
    }
}
