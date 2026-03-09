//! Contract tests for the IPC JSON-RPC 2.0 protocol types (T010).
//!
//! Scenarios covered:
//! - S014: valid JSON-RPC request serializes and deserializes without loss
//! - S015: numeric id is echoed in response
//! - S016: string id is echoed in response
//! - S017: missing `jsonrpc` field → `validate()` returns Err with code -32600
//! - S018: wrong `jsonrpc` version → `validate()` returns Err with code -32600
//! - S019: missing `method` field → `validate()` returns Err with code -32600
//! - S020: missing `id` field → `validate()` returns Err with id=null, code -32600
//! - S025: non-JSON / malformed content → `from_line()` returns Err with code -32700
//! - Additional: `from_line()` with non-object JSON, `to_line()` newline terminator,
//!   health response shape, shutdown response shape.

use engram::daemon::protocol::{IpcError, IpcRequest, IpcResponse};
use serde_json::{Value, json};

// ── S014: valid round-trip ────────────────────────────────────────────────────

#[test]
fn s014_valid_request_roundtrip() {
    let original = IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(json!(42)),
        method: "get_task_graph".to_owned(),
        params: Some(json!({ "workspace": "/tmp/ws" })),
    };

    let serialized = serde_json::to_string(&original).expect("serialize");
    let decoded: IpcRequest = serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(original, decoded);
}

#[test]
fn s014_valid_response_roundtrip() {
    let original = IpcResponse::success(json!(1), json!({ "tasks": [] }));
    let serialized = serde_json::to_string(&original).expect("serialize");
    let decoded: IpcResponse = serde_json::from_str(&serialized).expect("deserialize");
    assert_eq!(original, decoded);
}

// ── S015: numeric id echoed ───────────────────────────────────────────────────

#[test]
fn s015_numeric_id_echoed_in_response() {
    let id = json!(7);
    let response = IpcResponse::success(id.clone(), json!(null));
    assert_eq!(response.id, id);
    assert_eq!(response.jsonrpc, "2.0");
    assert!(response.result.is_some());
    assert!(response.error.is_none());
}

// ── S016: string id echoed ────────────────────────────────────────────────────

#[test]
fn s016_string_id_echoed_in_response() {
    let id = json!("req-abc-123");
    let response = IpcResponse::error(
        id.clone(),
        IpcError {
            code: -32_601,
            message: "method not found".to_owned(),
            data: None,
        },
    );
    assert_eq!(response.id, id);
    assert!(response.error.is_some());
    assert!(response.result.is_none());
}

// ── S017: missing jsonrpc field ───────────────────────────────────────────────

#[test]
fn s017_missing_jsonrpc_validate_returns_invalid_request() {
    // `#[serde(default)]` on `jsonrpc` means absent → empty string
    let req: IpcRequest =
        serde_json::from_str(r#"{"id": 1, "method": "get_task_graph", "params": null}"#)
            .expect("parse");

    let err = req.validate().expect_err("should fail validation");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_600);
    assert_eq!(err.id, json!(1));
}

// ── S018: wrong jsonrpc version ───────────────────────────────────────────────

#[test]
fn s018_wrong_jsonrpc_version_validate_returns_invalid_request() {
    let req: IpcRequest =
        serde_json::from_str(r#"{"jsonrpc": "1.0", "id": 2, "method": "ping", "params": null}"#)
            .expect("parse");

    let err = req.validate().expect_err("should fail validation");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_600);
    assert!(wire_err.message.contains("2.0"));
}

// ── S019: missing method field ────────────────────────────────────────────────

#[test]
fn s019_missing_method_validate_returns_invalid_request() {
    // `#[serde(default)]` on `method` → empty string
    let req: IpcRequest =
        serde_json::from_str(r#"{"jsonrpc": "2.0", "id": 3, "params": null}"#).expect("parse");

    let err = req.validate().expect_err("should fail validation");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_600);
    assert!(wire_err.message.contains("method"));
}

// ── S020: missing id field ────────────────────────────────────────────────────

#[test]
fn s020_missing_id_validate_returns_invalid_request_with_null_id() {
    // `#[serde(default)]` on `id` → None
    let req: IpcRequest =
        serde_json::from_str(r#"{"jsonrpc": "2.0", "method": "get_task_graph", "params": null}"#)
            .expect("parse");

    let err = req.validate().expect_err("should fail validation");
    // When id is absent, the error response carries id=null per JSON-RPC spec
    assert_eq!(err.id, Value::Null);
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_600);
}

// ── S025: non-JSON / malformed input ─────────────────────────────────────────

#[test]
fn s025_malformed_json_from_line_returns_parse_error() {
    let line = "not valid json at all {{{";
    let err = IpcRequest::from_line(line).expect_err("should fail parsing");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_700);
    assert_eq!(err.id, Value::Null);
}

#[test]
fn s025_binary_like_content_from_line_returns_parse_error() {
    // Simulate arriving bytes that aren't valid JSON (replacement chars, etc.)
    let line = "\u{FFFD}\u{FFFD}binary\u{0000}garbage";
    let err = IpcRequest::from_line(line).expect_err("should fail parsing");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_700);
}

// ── from_line: non-object JSON ────────────────────────────────────────────────

#[test]
fn from_line_json_array_returns_invalid_request() {
    let line = r#"["jsonrpc","2.0"]"#;
    let err = IpcRequest::from_line(line).expect_err("should fail for array");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_600);
    assert_eq!(err.id, Value::Null);
}

#[test]
fn from_line_json_string_literal_returns_invalid_request() {
    let line = r#""just a string""#;
    let err = IpcRequest::from_line(line).expect_err("should fail for string literal");
    let wire_err = err.error.expect("should have error payload");
    assert_eq!(wire_err.code, -32_600);
}

// ── from_line: valid JSON object ──────────────────────────────────────────────

#[test]
fn from_line_valid_object_parses_correctly() {
    let line = r#"{"jsonrpc":"2.0","id":99,"method":"check_status","params":{"task_id":"t1"}}"#;
    let req = IpcRequest::from_line(line).expect("should parse");
    assert_eq!(req.jsonrpc, "2.0");
    assert_eq!(req.id, Some(json!(99)));
    assert_eq!(req.method, "check_status");
    assert!(req.params.is_some());
}

#[test]
fn from_line_trims_newline_before_parsing() {
    let line = "{\"jsonrpc\":\"2.0\",\"id\":1,\"method\":\"_health\",\"params\":null}\n";
    let req = IpcRequest::from_line(line).expect("should parse with trailing newline");
    assert_eq!(req.method, "_health");
}

// ── to_line: newline terminator ───────────────────────────────────────────────

#[test]
fn to_line_adds_trailing_newline() {
    let resp = IpcResponse::success(json!(1), json!({ "ok": true }));
    let line = resp.to_line().expect("serialize");
    assert!(
        line.ends_with('\n'),
        "to_line() must append a newline for IPC framing"
    );
    // Strip the newline and round-trip
    let decoded: IpcResponse = serde_json::from_str(line.trim_end()).expect("deserialize trimmed");
    assert_eq!(resp, decoded);
}

// ── Health response shape ─────────────────────────────────────────────────────

#[test]
fn health_response_has_required_fields() {
    let payload = json!({
        "status": "ready",
        "uptime_seconds": 42_u64,
        "workspace": "/tmp/test",
        "active_connections": 0_u64,
    });
    let response = IpcResponse::success(json!(1), payload.clone());

    let reserialized = serde_json::to_value(&response).expect("serialize");
    let result = &reserialized["result"];
    assert_eq!(result["status"], "ready");
    assert!(result["uptime_seconds"].is_number());
    assert!(result["active_connections"].is_number());
}

// ── Shutdown response shape ───────────────────────────────────────────────────

#[test]
fn shutdown_response_has_required_fields() {
    let payload = json!({ "status": "shutting_down", "flush_started": true });
    let response = IpcResponse::success(json!("req-1"), payload);
    let reserialized = serde_json::to_value(&response).expect("serialize");
    let result = &reserialized["result"];
    assert_eq!(result["status"], "shutting_down");
    assert_eq!(result["flush_started"], true);
}

// ── skip_serializing_if on optional fields ────────────────────────────────────

#[test]
fn error_response_omits_result_field() {
    let response = IpcResponse::parse_error("bad input".to_owned());
    let json_str = serde_json::to_string(&response).expect("serialize");
    assert!(!json_str.contains("\"result\""), "result must be omitted");
    assert!(json_str.contains("\"error\""));
}

#[test]
fn success_response_omits_error_field() {
    let response = IpcResponse::success(json!(1), json!({}));
    let json_str = serde_json::to_string(&response).expect("serialize");
    assert!(!json_str.contains("\"error\""), "error must be omitted");
    assert!(json_str.contains("\"result\""));
}

#[test]
fn ipc_error_omits_data_when_none() {
    let err = IpcError {
        code: -32_603,
        message: "internal".to_owned(),
        data: None,
    };
    let json_str = serde_json::to_string(&err).expect("serialize");
    assert!(
        !json_str.contains("\"data\""),
        "data must be omitted when None"
    );
}

// ── Specific constructor codes ────────────────────────────────────────────────

#[test]
fn method_not_found_has_code_minus_32601() {
    let resp = IpcResponse::method_not_found(json!(5), "unknown_tool");
    let err = resp.error.expect("has error");
    assert_eq!(err.code, -32_601);
    assert!(err.message.contains("unknown_tool"));
}

#[test]
fn internal_error_has_code_minus_32603() {
    let resp = IpcResponse::internal_error(json!(6), "db unavailable".to_owned());
    let err = resp.error.expect("has error");
    assert_eq!(err.code, -32_603);
}

#[test]
fn parse_error_has_null_id_and_code_minus_32700() {
    let resp = IpcResponse::parse_error("could not parse".to_owned());
    assert_eq!(resp.id, Value::Null);
    let err = resp.error.expect("has error");
    assert_eq!(err.code, -32_700);
}
