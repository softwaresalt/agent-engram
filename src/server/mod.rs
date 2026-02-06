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
