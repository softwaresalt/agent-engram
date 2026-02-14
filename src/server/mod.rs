//! HTTP/SSE server layer built on axum 0.7.
//!
//! Provides the SSE endpoint (`/sse`), MCP JSON-RPC handler (`/mcp`),
//! health check (`/health`), and shared application state.

pub mod mcp;
pub mod router;
pub mod sse;
pub mod state;
