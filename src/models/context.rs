//! Context entity for ephemeral knowledge captured during execution.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// Ephemeral knowledge node linked to tasks via `relates_to` edges.
///
/// Context records are append-only; they are never updated or deleted
/// during normal operation.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct Context {
    pub id: String,
    pub content: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub embedding: Option<Vec<f32>>,
    pub source_client: String,
    pub created_at: DateTime<Utc>,
}
