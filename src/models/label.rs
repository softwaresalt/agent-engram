//! Label entity for task categorization.
//!
//! A label is a string tag associated with a task, stored in a
//! separate table for efficient AND-filtering across tasks.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A string tag associated with a task for categorization and filtering.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Label {
    /// SurrealDB record ID (e.g., `label:abc123`).
    pub id: String,
    /// Reference to the owning task.
    pub task_id: String,
    /// Label string (e.g., `"frontend"`, `"bug"`).
    pub name: String,
    /// When the label was attached.
    pub created_at: DateTime<Utc>,
}
