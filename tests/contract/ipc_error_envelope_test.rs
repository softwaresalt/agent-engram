//! Contract coverage for plan unit F39 (IPC error envelope).
//!
//! The IPC JSON-RPC error object must preserve the F38 domain envelope's
//! stable Engram code, symbolic name, and structured details across the wire.

#![forbid(unsafe_code)]

use engram::daemon::{error_transport, protocol::IpcResponse};
use engram::errors::{ActivationError, EngramError, ReadServerRefusalError};
use serde_json::json;

fn assert_error_roundtrip(case: &str, error: EngramError) {
    let expected = error.to_response();
    let expected_code = expected.error.code;
    let expected_name = expected.error.name;
    let expected_message = expected.error.message;
    let expected_details = expected.error.details;

    let response = IpcResponse::error(json!(case), error_transport::to_ipc_error(error));
    let line = response.to_line().expect("serialize IPC response");
    let decoded: IpcResponse = serde_json::from_str(&line).expect("deserialize IPC response");

    assert_eq!(
        decoded, response,
        "{case}: IPC response must round-trip exactly"
    );

    let wire_error = decoded.error.expect("error payload");
    assert_eq!(
        wire_error.code, -32_603,
        "{case}: domain errors must travel behind JSON-RPC internal error"
    );
    assert_eq!(
        wire_error.message, expected_message,
        "{case}: message must match the domain envelope"
    );

    let data = wire_error.data.expect("structured IPC error data");
    assert_eq!(
        data.get("engram_code"),
        Some(&json!(expected_code)),
        "{case}: stable Engram code must survive the wire round trip"
    );
    assert_eq!(
        data.get("engram_name"),
        Some(&json!(expected_name)),
        "{case}: symbolic Engram name must survive the wire round trip"
    );

    match expected_details {
        Some(details) => assert_eq!(
            data.get("engram_details"),
            Some(&details),
            "{case}: structured Engram details must remain structured"
        ),
        None => assert!(
            data.get("engram_details").is_none(),
            "{case}: details key must be omitted when the domain envelope has no details"
        ),
    }
}

#[test]
fn refusal_reserved_class_roundtrip_fidelity() {
    let cases = [
        (
            "write_control_refused",
            EngramError::from(ReadServerRefusalError::WriteControlRefused {
                operation: "write_memory".to_owned(),
            }),
        ),
        (
            "direct_sync_refused",
            EngramError::from(ReadServerRefusalError::DirectSyncRefused {
                operation: "sync_workspace".to_owned(),
            }),
        ),
        (
            "workspace_retarget_refused",
            EngramError::from(ReadServerRefusalError::WorkspaceRetargetRefused {
                requested_workspace: "C:\\workspace\\other".to_owned(),
            }),
        ),
    ];

    for (case, error) in cases {
        assert_error_roundtrip(case, error);
    }
}

#[test]
fn activation_reserved_class_roundtrip_fidelity() {
    let cases = [
        (
            "generation_not_yet_activated",
            EngramError::from(ActivationError::GenerationNotYetActivated {
                generation_id: "gen-1".to_owned(),
            }),
        ),
        (
            "activation_deadline_exceeded",
            EngramError::from(ActivationError::ActivationDeadlineExceeded { deadline_ms: 5000 }),
        ),
        (
            "transient_activation_failure",
            EngramError::from(ActivationError::TransientActivationFailure {
                reason: "database unavailable".to_owned(),
            }),
        ),
        (
            "manifest_schema_mismatch",
            EngramError::from(ActivationError::ManifestSchemaMismatch {
                expected: "v2".to_owned(),
                found: "v1".to_owned(),
            }),
        ),
        (
            "manifest_malformed",
            EngramError::from(ActivationError::ManifestMalformed {
                reason: "missing revision".to_owned(),
            }),
        ),
        (
            "manifest_field_out_of_bounds",
            EngramError::from(ActivationError::ManifestFieldOutOfBounds {
                field: "inventory.max_entries".to_owned(),
                reason: "must be positive".to_owned(),
            }),
        ),
        (
            "identity_mismatch",
            EngramError::from(ActivationError::IdentityMismatch {
                field: "branch".to_owned(),
                expected: "main".to_owned(),
                found: "feature/demo".to_owned(),
            }),
        ),
        (
            "digest_mismatch",
            EngramError::from(ActivationError::DigestMismatch {
                path: "index/segment-1.bin".to_owned(),
                expected: "abc123".to_owned(),
                found: "def456".to_owned(),
            }),
        ),
    ];

    for (case, error) in cases {
        assert_error_roundtrip(case, error);
    }
}
