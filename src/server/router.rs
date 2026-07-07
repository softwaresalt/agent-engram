use axum::{
    Json, Router,
    extract::State,
    routing::{get, post},
};
use serde_json::json;

use crate::server::{mcp::mcp_handler, sse::sse_handler, state::SharedState};

async fn health_handler(State(state): State<SharedState>) -> Json<serde_json::Value> {
    Json(json!({
        "version": env!("CARGO_PKG_VERSION"),
        "uptime_seconds": state.uptime_seconds(),
        "active_workspaces": state.active_workspaces().await,
        "active_connections": state.active_connections(),
        "memory_bytes": crate::services::process_memory::current_process_memory_bytes().unwrap_or(0),
    }))
}

/// Build axum router with SSE and MCP endpoints.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/sse", get(sse_handler))
        .route("/mcp", post(mcp_handler))
        .route("/health", get(health_handler))
        .with_state(state)
}
