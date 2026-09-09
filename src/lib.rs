#![forbid(unsafe_code)]

//! Data-only contracts. Implementations live in clients, servers, and lib-core.
//!
//! The `v1` slice family mirrors the two independent authorities in
//! `contracts/typespec/<slice>.tsp` and `contracts/json-schema/<slice>.schema.json`.
//! The build-log families are independently authored and independently admitted
//! by their own TJSV gates. Generated witnesses/IR/receipts are evidence only;
//! none of these generated artifacts is an authored authority.
//!
//! The legacy `protocol` / `schema` surface (`schema/v1/workerlease.json`) is
//! retained for existing callers.

pub mod build_log;
pub mod build_log_metadata;
pub mod error;
pub mod protocol;
pub mod schema;
pub mod v1;

pub use build_log::{
    LogSidecarCommandStarted, LogSidecarCommandTerminal, LogSidecarFrameDescriptor,
    LogSidecarFrameStream, LogSidecarStartEvent, LogSidecarTerminalEvent, LogSidecarWireVector,
    LOG_SIDECAR_FRAME_HEADER_BYTES, LOG_SIDECAR_FRAME_MAGIC, LOG_SIDECAR_FRAME_VERSION,
    LOG_SIDECAR_MAX_PAYLOAD_BYTES, LOG_SIDECAR_MAX_SHUTDOWN_MILLIS, LOG_SIDECAR_PROTOCOL,
    LOG_SIDECAR_STDERR_STREAM_ID, LOG_SIDECAR_STDOUT_STREAM_ID,
};
pub use build_log_metadata::{
    BuildLogEvent, BuildLogMetadata, BuildLogStream, BUILD_LOG_METADATA_SCHEMA_VERSION,
    DEFAULT_DATA_FD, DEFAULT_METADATA_FD, MAX_RECEIVER_SHUTDOWN_SECONDS,
};
pub use error::InterfaceError;
pub use protocol::{Health, WorkerLease, PROTOCOL_VERSION};
pub use schema::{SCHEMA_ID, SCHEMA_REVISION};
pub use v1::{CONTRACT_VERSION, SLICES};
