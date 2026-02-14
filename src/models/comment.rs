//! Comment entity for task discussion threads.
//!
//! Each comment is associated with a task and contains content,
//! an author identifier, and a creation timestamp.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

/// A comment attached to a task for discussion and context.
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub struct Comment {
    /// SurrealDB record ID (e.g., `comment:abc123`).
    pub id: String,
    /// Reference to the owning task.
    pub task_id: String,
    /// Comment body text.
    pub content: String,
    /// Identifier of the comment author.
    pub author: String,
    /// When the comment was created.
    pub created_at: DateTime<Utc>,
}
