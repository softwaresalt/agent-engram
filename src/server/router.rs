#![allow(dead_code)]

use axum::{
    Router,
    routing::{get, post},
};

use crate::server::{mcp::mcp_handler, sse::sse_handler, state::SharedState};

/// Build axum router with SSE and MCP endpoints.
pub fn build_router(state: SharedState) -> Router {
    Router::new()
        .route("/sse", get(sse_handler))
        .route("/mcp", post(mcp_handler))
        .with_state(state)
}
