#![forbid(unsafe_code)]

//! Data-only contracts. Implementations live in clients, servers, and lib-core.
//!
//! `v1` is the current contract, mirroring the two independent authorities in
//! `contracts/`: `contracts/typespec/<slice>.tsp` and
//! `contracts/json-schema/<slice>.schema.json`. Neither authority is generated
//! from the other; `npx ores-contracts check` parses both and requires the
//! emitted artifacts to agree byte for byte.
//!
//! The legacy `protocol` / `schema` surface (`schema/v1/workerlease.json`) is
//! retained unchanged for existing callers.

pub mod error;
pub mod protocol;
pub mod schema;
pub mod v1;

pub use error::InterfaceError;
pub use protocol::{Health, WorkerLease, PROTOCOL_VERSION};
pub use schema::{SCHEMA_ID, SCHEMA_REVISION};
pub use v1::{CONTRACT_VERSION, SLICES};
