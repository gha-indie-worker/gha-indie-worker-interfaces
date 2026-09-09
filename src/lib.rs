#![forbid(unsafe_code)]

//! Data-only contracts. Implementations live in clients, servers, and lib-core.

pub mod build_log;
pub mod error;
pub mod protocol;
pub mod schema;

pub use build_log::{
    LogSidecarCommandStarted, LogSidecarCommandTerminal, LogSidecarFrameDescriptor,
    LogSidecarFrameStream, LogSidecarStartEvent, LogSidecarTerminalEvent, LogSidecarWireVector,
    LOG_SIDECAR_FRAME_HEADER_BYTES, LOG_SIDECAR_FRAME_MAGIC, LOG_SIDECAR_FRAME_VERSION,
    LOG_SIDECAR_MAX_PAYLOAD_BYTES, LOG_SIDECAR_MAX_SHUTDOWN_MILLIS, LOG_SIDECAR_PROTOCOL,
    LOG_SIDECAR_STDERR_STREAM_ID, LOG_SIDECAR_STDOUT_STREAM_ID,
};
pub use error::InterfaceError;
pub use protocol::{Health, WorkerLease, PROTOCOL_VERSION};
pub use schema::{SCHEMA_ID, SCHEMA_REVISION};
