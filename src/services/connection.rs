#![allow(dead_code)]

use chrono::{DateTime, Utc};
use uuid::Uuid;

use crate::db::queries::Queries;
use crate::db::workspace::canonicalize_workspace;
use crate::errors::{TMemError, WorkspaceError};
use crate::models::context::Context;
use crate::models::task::TaskStatus;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ConnectionState {
    Disconnected,
    Connected,
    Active,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ConnectionLifecycle {
    state: ConnectionState,
}

impl ConnectionLifecycle {
    pub fn new() -> Self {
        Self {
            state: ConnectionState::Disconnected,
        }
    }

    pub fn on_connect(&mut self) {
        self.state = ConnectionState::Connected;
    }

    pub fn on_bind_workspace(&mut self) {
        self.state = ConnectionState::Active;
    }

    pub fn on_disconnect(&mut self) {
        self.state = ConnectionState::Disconnected;
    }

    pub fn state(&self) -> ConnectionState {
        self.state
    }
}

/// Validate workspace path before binding a connection.
pub fn validate_workspace_path(path: &str) -> Result<(), WorkspaceError> {
    let _canonical = canonicalize_workspace(path)?;
    Ok(())
}

/// Automatically create a context note recording a task status transition.
///
/// Called on every `update_task` invocation per FR-015: the system MUST append
/// context notes on task updates (never overwrite existing context). The note
/// captures previous and new status, optional user-supplied notes, and the
/// timestamp of the transition.
pub async fn create_status_change_note(
    queries: &Queries,
    task_id: &str,
    previous: TaskStatus,
    new: TaskStatus,
    user_notes: Option<&str>,
    timestamp: DateTime<Utc>,
) -> Result<String, TMemError> {
    let mut content = format!(
        "Status changed from {} to {}",
        format_status(previous),
        format_status(new),
    );
    if let Some(notes) = user_notes {
        content.push_str(&format!("\n\n{notes}"));
    }

    let ctx_id = format!("context:{}", Uuid::new_v4());
    let ctx = Context {
        id: ctx_id.clone(),
        content,
        embedding: None,
        source_client: "daemon".into(),
        created_at: timestamp,
    };
    queries.insert_context(&ctx).await?;
    queries.link_task_context(task_id, &ctx_id).await?;
    Ok(ctx_id)
}

fn format_status(status: TaskStatus) -> &'static str {
    match status {
        TaskStatus::Todo => "todo",
        TaskStatus::InProgress => "in_progress",
        TaskStatus::Done => "done",
        TaskStatus::Blocked => "blocked",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_path_is_rejected() {
        let result = validate_workspace_path("/path/that/does/not/exist");
        assert!(result.is_err());
    }

    #[test]
    fn lifecycle_transitions() {
        let mut lifecycle = ConnectionLifecycle::new();
        assert_eq!(lifecycle.state(), ConnectionState::Disconnected);
        lifecycle.on_connect();
        assert_eq!(lifecycle.state(), ConnectionState::Connected);
        lifecycle.on_bind_workspace();
        assert_eq!(lifecycle.state(), ConnectionState::Active);
        lifecycle.on_disconnect();
        assert_eq!(lifecycle.state(), ConnectionState::Disconnected);
    }
}
