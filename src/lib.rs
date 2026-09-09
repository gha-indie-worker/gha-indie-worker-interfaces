#![forbid(unsafe_code)]

//! Data-only contracts. Implementations live in clients, servers, and lib-core.

pub mod build_log;
pub mod error;
pub mod protocol;
pub mod schema;

pub use build_log::{
    BuildLogEvent, BuildLogMetadata, BuildLogStream, BUILD_LOG_METADATA_SCHEMA_VERSION,
    DEFAULT_DATA_FD, DEFAULT_METADATA_FD, MAX_RECEIVER_SHUTDOWN_SECONDS,
};
pub use error::InterfaceError;
pub use protocol::{Health, WorkerLease, PROTOCOL_VERSION};
pub use schema::{SCHEMA_ID, SCHEMA_REVISION};
