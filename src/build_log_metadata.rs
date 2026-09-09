#![forbid(unsafe_code)]

use serde::{Deserialize, Serialize};

use crate::error::InterfaceError;

pub const BUILD_LOG_METADATA_SCHEMA_VERSION: &str = "gha-indie-worker.build-log-metadata/v1";
pub const DEFAULT_METADATA_FD: i32 = 3;
pub const DEFAULT_DATA_FD: i32 = 4;
pub const MAX_RECEIVER_SHUTDOWN_SECONDS: u64 = 8;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum BuildLogEvent {
    Chunk,
    Dropped,
    ReceiverClosed,
    StreamClosed,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BuildLogStream {
    Stdout,
    Stderr,
    Worker,
}

#[derive(Clone, Debug, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct BuildLogMetadata {
    pub schema_version: String,
    pub event: BuildLogEvent,
    pub job_id: String,
    pub stream: BuildLogStream,
    pub sequence: u32,
    pub byte_length: u32,
    pub timestamp: String,
    pub repository: Option<String>,
    pub github_organization: Option<String>,
    pub workflow: Option<String>,
    pub run_id: Option<String>,
    pub job_name: Option<String>,
    pub step_name: Option<String>,
    pub attempt: Option<u32>,
    pub trace_id: Option<String>,
    pub span_id: Option<String>,
    pub dropped_chunks: Option<u32>,
    pub dropped_bytes: Option<u32>,
}

impl BuildLogMetadata {
    pub fn validate(self) -> Result<Self, InterfaceError> {
        if self.schema_version != BUILD_LOG_METADATA_SCHEMA_VERSION || self.job_id.trim().is_empty()
        {
            return Err(InterfaceError::SchemaMismatch);
        }
        match self.event {
            BuildLogEvent::Chunk if self.byte_length == 0 => Err(InterfaceError::SchemaMismatch),
            BuildLogEvent::Dropped
                if self.dropped_chunks.unwrap_or(0) == 0
                    && self.dropped_bytes.unwrap_or(0) == 0 =>
            {
                Err(InterfaceError::SchemaMismatch)
            }
            _ => Ok(self),
        }
    }

    pub fn parse_json_line(line: &str) -> Result<Self, InterfaceError> {
        let value =
            serde_json::from_str::<Self>(line).map_err(|_| InterfaceError::SchemaMismatch)?;
        value.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn chunk() -> BuildLogMetadata {
        BuildLogMetadata {
            schema_version: BUILD_LOG_METADATA_SCHEMA_VERSION.to_string(),
            event: BuildLogEvent::Chunk,
            job_id: "build-123".to_string(),
            stream: BuildLogStream::Stdout,
            sequence: 7,
            byte_length: 12,
            timestamp: "2026-09-09T19:45:00Z".to_string(),
            repository: Some("gha-indie-worker/gha-indie-worker.rs".to_string()),
            github_organization: Some("gha-indie-worker".to_string()),
            workflow: None,
            run_id: None,
            job_name: None,
            step_name: None,
            attempt: Some(1),
            trace_id: None,
            span_id: None,
            dropped_chunks: None,
            dropped_bytes: None,
        }
    }

    #[test]
    fn round_trip_uses_wire_names() {
        let line = serde_json::to_string(&chunk()).expect("serialize");
        assert!(line.contains("\"schemaVersion\""));
        assert!(line.contains("\"stdout\""));
        assert_eq!(BuildLogMetadata::parse_json_line(&line), Ok(chunk()));
    }

    #[test]
    fn rejects_empty_chunk_and_unaccounted_drop() {
        let mut empty = chunk();
        empty.byte_length = 0;
        assert_eq!(empty.validate(), Err(InterfaceError::SchemaMismatch));

        let mut dropped = chunk();
        dropped.event = BuildLogEvent::Dropped;
        dropped.byte_length = 0;
        dropped.dropped_chunks = Some(0);
        dropped.dropped_bytes = Some(0);
        assert_eq!(dropped.validate(), Err(InterfaceError::SchemaMismatch));
    }

    #[test]
    fn rejects_unknown_cross_runtime_fields() {
        let mut value = serde_json::to_value(chunk()).expect("serialize");
        value.as_object_mut().expect("metadata object").insert(
            "accessToken".to_string(),
            serde_json::json!("must-not-cross"),
        );
        assert_eq!(
            BuildLogMetadata::parse_json_line(&value.to_string()),
            Err(InterfaceError::SchemaMismatch)
        );
    }
}
