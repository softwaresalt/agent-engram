use axum::{Json, extract::State, response::IntoResponse};
use serde::Deserialize;
use serde_json::{Value, json};

use crate::{errors::EngramError, server::state::SharedState, tools};

#[derive(Deserialize)]
struct RpcRequest {
    #[allow(dead_code)]
    jsonrpc: String,
    method: String,
    #[serde(default)]
    params: Option<Value>,
    #[serde(default)]
    id: Option<Value>,
}

/// MCP JSON-RPC handler dispatching to tool registry.
pub async fn mcp_handler(
    State(state): State<SharedState>,
    Json(payload): Json<Value>,
) -> impl IntoResponse {
    let id = payload.get("id").cloned().unwrap_or(Value::Null);
    let req: RpcRequest = match serde_json::from_value(payload) {
        Ok(r) => r,
        Err(e) => {
            let err = EngramError::System(crate::errors::SystemError::InvalidParams {
                reason: format!("Invalid request: {e}"),
            })
            .to_response();
            return Json(json!({ "jsonrpc": "2.0", "error": err.error, "id": id }));
        }
    };

    match tools::dispatch(state, &req.method, req.params).await {
        Ok(result) => {
            Json(json!({ "jsonrpc": "2.0", "result": result, "id": req.id.unwrap_or(id) }))
        }
        Err(e) => {
            let err = e.to_response();
            Json(json!({ "jsonrpc": "2.0", "error": err.error, "id": req.id.unwrap_or(id) }))
        }
    }
}
