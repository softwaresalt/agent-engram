//! Contract coverage for plan unit F41 (MCP envelope).
//!
//! The shim transport must surface both successful daemon responses and F38
//! domain-error envelopes through MCP `CallToolResult` with structured payloads
//! in `structuredContent`. Human-readable text remains a secondary view.

#![forbid(unsafe_code)]

use engram::daemon::{error_transport, protocol::IpcResponse};
use engram::errors::{
    ActivationError, EngramError, ReadServerRefusalError,
    codes::{GENERATION_NOT_YET_ACTIVATED, READ_SERVER_WRITE_CONTROL_REFUSED},
};
use engram::shim::transport;
use serde_json::{Value, json};

fn first_text(result_json: &Value) -> &str {
    result_json["content"][0]["text"]
        .as_str()
        .expect("transport must retain a text view")
}

#[test]
fn success_responses_carry_structured_provenance_and_text_view() {
    let response = IpcResponse::success(
        json!(1),
        json!({
            "content": [
                {
                    "type": "text",
                    "text": "served from captured generation gen-42 for workspace-alpha"
                }
            ],
            "provenance": {
                "workspace_id": "workspace-alpha",
                "branch": "feature/f41",
                "generation_id": "gen-42"
            },
            "row_count": 3
        }),
    );

    let result = transport::translate_ipc_response(response);
    let result_json = serde_json::to_value(&result).expect("serialize MCP call result");

    assert_eq!(result_json["isError"], false);
    assert_eq!(
        result_json["structuredContent"]["provenance"]["workspace_id"],
        json!("workspace-alpha")
    );
    assert_eq!(
        result_json["structuredContent"]["provenance"]["branch"],
        json!("feature/f41")
    );
    assert_eq!(
        result_json["structuredContent"]["provenance"]["generation_id"],
        json!("gen-42")
    );

    let text = first_text(&result_json);
    assert!(
        !text.is_empty(),
        "success payload must keep a human-readable text view alongside structuredContent"
    );
    assert!(
        text.contains("captured generation"),
        "text view should remain a useful secondary rendering: {text}"
    );
}

#[test]
fn error_responses_carry_structured_engram_fields_and_distinguish_error_classes() {
    let refusal = transport::translate_ipc_response(error_transport::to_response(
        json!(1),
        EngramError::from(ReadServerRefusalError::WriteControlRefused {
            operation: "write_memory".to_owned(),
        }),
    ));
    let refusal_json = serde_json::to_value(&refusal).expect("serialize refusal MCP result");

    assert_eq!(refusal_json["isError"], true);
    assert_eq!(
        refusal_json["structuredContent"]["engram_code"],
        json!(READ_SERVER_WRITE_CONTROL_REFUSED)
    );
    assert_eq!(
        refusal_json["structuredContent"]["engram_name"],
        json!("ReadServerWriteControlRefused")
    );
    assert_eq!(
        refusal_json["structuredContent"]["engram_details"]["operation"],
        json!("write_memory")
    );
    assert!(
        !first_text(&refusal_json).is_empty(),
        "error payload must keep a human-readable text view alongside structuredContent"
    );

    let availability = transport::translate_ipc_response(error_transport::to_response(
        json!(2),
        EngramError::from(ActivationError::GenerationNotYetActivated {
            generation_id: "gen-77".to_owned(),
        }),
    ));
    let availability_json =
        serde_json::to_value(&availability).expect("serialize availability MCP result");

    assert_eq!(availability_json["isError"], true);
    assert_eq!(
        availability_json["structuredContent"]["engram_code"],
        json!(GENERATION_NOT_YET_ACTIVATED)
    );
    assert_eq!(
        availability_json["structuredContent"]["engram_name"],
        json!("GenerationNotYetActivated")
    );
    assert_eq!(
        availability_json["structuredContent"]["engram_details"]["generation_id"],
        json!("gen-77")
    );

    let refusal_code = refusal_json["structuredContent"]["engram_code"]
        .as_u64()
        .expect("refusal engram_code must be numeric");
    let availability_code = availability_json["structuredContent"]["engram_code"]
        .as_u64()
        .expect("availability engram_code must be numeric");

    assert!(
        (16_000..17_000).contains(&refusal_code),
        "refusal class must remain machine-distinguishable without parsing prose: {refusal_json}"
    );
    assert!(
        (17_000..18_000).contains(&availability_code),
        "availability class must remain machine-distinguishable without parsing prose: {availability_json}"
    );
}
