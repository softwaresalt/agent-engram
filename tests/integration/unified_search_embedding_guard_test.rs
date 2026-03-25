//! Integration tests for `unified_search` embedding guard (dxo.4.2).
//!
//! Verifies that `unified_search` returns an informative, actionable error
//! when the embeddings subsystem is unavailable, rather than silently
//! returning empty results or a generic database error.

use std::time::Duration;

use engram::daemon::protocol::IpcRequest;
use engram::shim::ipc_client::send_request;
use serde_json::{Value, json};

#[path = "../helpers/mod.rs"]
mod helpers;

use helpers::DaemonHarness;

fn make_request(id: i64, method: &str, params: Option<Value>) -> IpcRequest {
    IpcRequest {
        jsonrpc: "2.0".to_owned(),
        id: Some(Value::Number(serde_json::Number::from(id))),
        method: method.to_owned(),
        params,
    }
}

// ── unified_search embedding guard ────────────────────────────────────

/// When the `embeddings` feature is disabled, `unified_search` MUST return
/// an error response, not a success response with empty results.
#[cfg(not(feature = "embeddings"))]
#[tokio::test]
async fn unified_search_returns_error_when_embeddings_disabled() {
    // GIVEN a running daemon with no embeddings support
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call unified_search with a valid query
    let request = make_request(
        1,
        "unified_search",
        Some(json!({ "query": "find functions that parse JSON" })),
    );
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed at transport level");

    // THEN the response must contain an error, not a result
    assert!(
        response.error.is_some(),
        "unified_search must return an error when embeddings are unavailable, \
         not an empty success response; got result: {:?}",
        response.result
    );
    assert!(
        response.result.is_none(),
        "result must be absent when an error is returned"
    );
}

/// The error returned when embeddings are unavailable must use a query-domain
/// error code (4xxx), not a generic system/database error (5xxx).
#[cfg(not(feature = "embeddings"))]
#[tokio::test]
async fn unified_search_error_uses_query_domain_code() {
    // GIVEN a running daemon with no embeddings support
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call unified_search
    let request = make_request(
        1,
        "unified_search",
        Some(json!({ "query": "semantic query about types" })),
    );
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    // THEN the error code must be the standard JSON-RPC internal error (-32603),
    // and the domain error code in data.engram_code must be in the 4xxx range
    // (query errors), specifically NOT 5001 (DatabaseError) which is confusing
    let error = response
        .error
        .expect("response must have an error for unavailable embeddings");

    assert_eq!(
        error.code, -32_603,
        "tool errors must use JSON-RPC internal error code -32603"
    );

    let engram_code = error
        .data
        .as_ref()
        .and_then(|d| d["engram_code"].as_u64())
        .expect("error.data.engram_code must be present");

    assert!(
        (4000..5000).contains(&engram_code),
        "embedding-unavailable error must use a 4xxx query-domain engram_code, not {engram_code}"
    );
}

/// The error message must include actionable guidance explaining how to
/// enable the embeddings feature.
#[cfg(not(feature = "embeddings"))]
#[tokio::test]
async fn unified_search_error_message_contains_actionable_guidance() {
    // GIVEN a running daemon with no embeddings support
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call unified_search
    let request = make_request(
        1,
        "unified_search",
        Some(json!({ "query": "how does hydration work" })),
    );
    let response = send_request(&endpoint, &request, Duration::from_secs(10))
        .await
        .expect("IPC call must succeed");

    let error = response
        .error
        .expect("response must have an error for unavailable embeddings");

    // THEN the error message must contain text that helps the user understand
    // what is wrong and how to fix it
    let msg = error.message.to_lowercase();
    assert!(
        msg.contains("embed") || msg.contains("model") || msg.contains("feature"),
        "error message must reference embeddings, model, or feature flag; got: {}",
        error.message
    );
}

/// When the `embeddings` feature IS enabled and the model is loaded,
/// `unified_search` must not return an embeddings-unavailable error.
/// This test validates the guard does not fire for the happy path.
#[cfg(feature = "embeddings")]
#[tokio::test]
async fn unified_search_does_not_block_when_embeddings_available() {
    // GIVEN a running daemon with embeddings enabled
    let harness = DaemonHarness::spawn(Duration::from_secs(15))
        .await
        .expect("daemon must spawn");
    let endpoint = harness.ipc_path().to_str().expect("UTF-8").to_owned();

    // WHEN we call unified_search with a valid query
    let request = make_request(
        1,
        "unified_search",
        Some(json!({ "query": "find async functions" })),
    );
    let response = send_request(&endpoint, &request, Duration::from_secs(30))
        .await
        .expect("IPC call must succeed");

    // THEN the error must NOT be an embeddings-unavailable error
    // (it may fail for other reasons such as model download, but not the guard)
    if let Some(err) = &response.error {
        let engram_code = err.data.as_ref().and_then(|d| d["engram_code"].as_u64());
        if let Some(code) = engram_code {
            assert!(
                !((4000..5000).contains(&code) && err.message.to_lowercase().contains("embed")),
                "unified_search must not return embeddings-unavailable when feature is enabled; \
                 got engram_code={code}: {}",
                err.message
            );
        }
    }
}
