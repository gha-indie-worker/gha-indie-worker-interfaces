#![forbid(unsafe_code)]

//! Data-only contracts. Implementations live in clients, servers, and lib-core.

pub mod error;
pub mod protocol;
pub mod schema;

// owls:profile:start
// OWLS WASM release profile. Additive; see schema/v1/README.md.
// release-v1 (owned by ores-wasm-loaders) remains the base authority; this
// module only exposes this organization's narrowing profile.
pub mod wasm_release_profile;

pub use wasm_release_profile::{
    RELEASE_V1_SCHEMA_ID, WASM_RELEASE_PROFILE_REVISION, WASM_RELEASE_PROFILE_SCHEMA,
    WASM_RELEASE_PROFILE_SCHEMA_ID,
};
// owls:profile:end

pub use error::InterfaceError;
pub use protocol::{Health, WorkerLease, PROTOCOL_VERSION};
pub use schema::{SCHEMA_ID, SCHEMA_REVISION};

