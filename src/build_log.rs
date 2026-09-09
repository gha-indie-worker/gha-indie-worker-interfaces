#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

pub const LOG_SIDECAR_PROTOCOL: &str = "gha-indie-worker.log-sidecar.v1";
pub const LOG_SIDECAR_FRAME_MAGIC: [u8; 4] = *b"GHLG";
pub const LOG_SIDECAR_FRAME_VERSION: u8 = 1;
pub const LOG_SIDECAR_STDOUT_STREAM_ID: u8 = 1;
pub const LOG_SIDECAR_STDERR_STREAM_ID: u8 = 2;
pub const LOG_SIDECAR_FRAME_HEADER_BYTES: usize = 10;
pub const LOG_SIDECAR_MAX_PAYLOAD_BYTES: usize = 64 * 1024;
pub const LOG_SIDECAR_MAX_SHUTDOWN_MILLIS: u64 = 8_000;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum LogSidecarFrameStream {
    Stdout,
    Stderr,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSidecarFrameDescriptor {
    pub protocol: String,
    pub magic: String,
    pub version: u8,
    pub stream_id: u8,
    pub stream: LogSidecarFrameStream,
    pub payload_length: u32,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSidecarWireVector {
    pub name: String,
    pub frame: LogSidecarFrameDescriptor,
    pub payload_hex: String,
    pub wire_hex: String,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSidecarStartEvent {
    CommandStarted,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum LogSidecarTerminalEvent {
    CommandFinished,
    CommandTimedOut,
    CommandWaitFailed,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSidecarCommandStarted {
    pub schema_version: String,
    pub event: LogSidecarStartEvent,
    pub at_ms: u64,
    pub job_id: Option<String>,
    pub program: String,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct LogSidecarCommandTerminal {
    pub schema_version: String,
    pub event: LogSidecarTerminalEvent,
    pub at_ms: u64,
    pub job_id: Option<String>,
    pub program: String,
    pub success: bool,
    pub exit_code: Option<i32>,
    pub dropped_frames: u64,
    pub dropped_bytes: u64,
}
