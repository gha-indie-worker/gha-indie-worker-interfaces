use gha_indie_worker_interfaces::{
    LogSidecarCommandStarted, LogSidecarCommandTerminal, LogSidecarFrameStream,
    LogSidecarWireVector, LOG_SIDECAR_FRAME_HEADER_BYTES, LOG_SIDECAR_FRAME_MAGIC,
    LOG_SIDECAR_FRAME_VERSION, LOG_SIDECAR_MAX_PAYLOAD_BYTES, LOG_SIDECAR_PROTOCOL,
    LOG_SIDECAR_STDERR_STREAM_ID, LOG_SIDECAR_STDOUT_STREAM_ID,
};

fn decode_hex(value: &str) -> Vec<u8> {
    assert_eq!(value.len() % 2, 0, "hex fixture must have complete bytes");
    value
        .as_bytes()
        .chunks_exact(2)
        .map(|pair| {
            let text = std::str::from_utf8(pair).expect("fixture hex is ASCII");
            u8::from_str_radix(text, 16).expect("fixture contains lowercase hexadecimal")
        })
        .collect()
}

fn assert_wire_vector(raw: &str) {
    let vector: LogSidecarWireVector = serde_json::from_str(raw).expect("valid wire vector");
    let payload = decode_hex(&vector.payload_hex);
    let wire = decode_hex(&vector.wire_hex);

    assert_eq!(vector.frame.protocol, LOG_SIDECAR_PROTOCOL);
    assert_eq!(vector.frame.magic.as_bytes(), LOG_SIDECAR_FRAME_MAGIC);
    assert_eq!(vector.frame.version, LOG_SIDECAR_FRAME_VERSION);
    assert_eq!(vector.frame.payload_length as usize, payload.len());
    assert!(payload.len() <= LOG_SIDECAR_MAX_PAYLOAD_BYTES);

    let expected_stream_id = match vector.frame.stream {
        LogSidecarFrameStream::Stdout => LOG_SIDECAR_STDOUT_STREAM_ID,
        LogSidecarFrameStream::Stderr => LOG_SIDECAR_STDERR_STREAM_ID,
    };
    assert_eq!(vector.frame.stream_id, expected_stream_id);

    let mut expected = Vec::with_capacity(LOG_SIDECAR_FRAME_HEADER_BYTES + payload.len());
    expected.extend_from_slice(&LOG_SIDECAR_FRAME_MAGIC);
    expected.push(LOG_SIDECAR_FRAME_VERSION);
    expected.push(expected_stream_id);
    expected.extend_from_slice(&(payload.len() as u32).to_be_bytes());
    expected.extend_from_slice(&payload);
    assert_eq!(wire, expected);
}

#[test]
fn valid_wire_vectors_match_the_binary_contract_exactly() {
    assert_wire_vector(include_str!(
        "../contracts/build-log-stream/instances/LogSidecarWireVector/valid/stdout-hello.json"
    ));
    assert_wire_vector(include_str!(
        "../contracts/build-log-stream/instances/LogSidecarWireVector/valid/stderr-binary.json"
    ));
}

#[test]
fn fd3_metadata_projection_accepts_all_v1_terminal_shapes() {
    let started: LogSidecarCommandStarted = serde_json::from_str(include_str!(
        "../contracts/build-log-stream/instances/LogSidecarCommandStarted/valid/command-started.json"
    ))
    .expect("command-started fixture");
    assert_eq!(started.schema_version, LOG_SIDECAR_PROTOCOL);

    let finished: LogSidecarCommandTerminal = serde_json::from_str(include_str!(
        "../contracts/build-log-stream/instances/LogSidecarCommandTerminal/valid/command-finished.json"
    ))
    .expect("command-finished fixture");
    assert_eq!(finished.schema_version, LOG_SIDECAR_PROTOCOL);
    assert_eq!(finished.exit_code, Some(0));

    let timed_out: LogSidecarCommandTerminal = serde_json::from_str(include_str!(
        "../contracts/build-log-stream/instances/LogSidecarCommandTerminal/valid/command-timed-out.json"
    ))
    .expect("command-timed-out fixture");
    assert_eq!(timed_out.schema_version, LOG_SIDECAR_PROTOCOL);
    assert_eq!(timed_out.exit_code, None);
    assert!(timed_out.dropped_frames > 0);
}

#[test]
fn rust_constants_match_the_independent_draft_2020_12_authority() {
    let schema: serde_json::Value = serde_json::from_str(include_str!(
        "../contracts/build-log-stream/authored.schema.json"
    ))
    .expect("authored schema parses");
    let defs = schema["$defs"].as_object().expect("schema defs");

    assert_eq!(
        defs["LogSidecarProtocol"]["enum"][0].as_str(),
        Some(LOG_SIDECAR_PROTOCOL)
    );
    assert_eq!(
        defs["LogSidecarFrameDescriptor"]["properties"]["payloadLength"]["maximum"].as_u64(),
        Some(LOG_SIDECAR_MAX_PAYLOAD_BYTES as u64)
    );
    assert_eq!(
        defs["LogSidecarFrameDescriptor"]["properties"]["version"]["maximum"].as_u64(),
        Some(LOG_SIDECAR_FRAME_VERSION as u64)
    );
}
