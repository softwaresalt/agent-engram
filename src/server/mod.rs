//! HTTP/SSE server layer built on axum 0.7.
//!
//! Provides the SSE endpoint (`/sse`), MCP JSON-RPC handler (`/mcp`),
//! health check (`/health`), and shared application state.

#![allow(dead_code)]

pub mod mcp;
pub mod router;
pub mod sse;
pub mod state;

/// Placeholder for correlation ID middleware. Will be wired into axum router in Phase 3.
pub struct CorrelationIds;

impl CorrelationIds {
    pub fn new() -> Self {
        Self
    }
}
