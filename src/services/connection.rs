use chrono::{DateTime, Utc};

use crate::db::workspace::canonicalize_workspace;
use crate::errors::WorkspaceError;

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

/// Information about a single active SSE connection (US5/T091).
#[derive(Debug, Clone)]
pub struct ConnectionInfo {
    /// Unique connection identifier (UUID v4).
    pub id: String,
    /// Workspace path this connection is bound to, if any.
    pub workspace_path: Option<String>,
    /// When this connection was established.
    pub connected_at: DateTime<Utc>,
}

/// Registry tracking all active SSE connections (US5/T091).
///
/// Supports multi-client concurrent access by maintaining a thread-safe
/// map of connection IDs to their metadata. Used for connection counting,
/// cleanup on disconnect, and workspace binding tracking.
#[derive(Debug, Clone)]
pub struct ConnectionRegistry {
    connections:
        std::sync::Arc<tokio::sync::RwLock<std::collections::HashMap<String, ConnectionInfo>>>,
}

impl ConnectionRegistry {
    /// Create an empty connection registry.
    pub fn new() -> Self {
        Self {
            connections: std::sync::Arc::new(tokio::sync::RwLock::new(
                std::collections::HashMap::new(),
            )),
        }
    }

    /// Register a new connection and return its info.
    pub async fn register(&self, id: String) -> ConnectionInfo {
        let info = ConnectionInfo {
            id: id.clone(),
            workspace_path: None,
            connected_at: Utc::now(),
        };
        self.connections.write().await.insert(id, info.clone());
        info
    }

    /// Remove a connection from the registry.
    pub async fn unregister(&self, id: &str) {
        self.connections.write().await.remove(id);
    }

    /// Return the number of active connections.
    pub async fn count(&self) -> usize {
        self.connections.read().await.len()
    }

    /// Bind a connection to a workspace path.
    pub async fn bind_workspace(&self, connection_id: &str, workspace_path: &str) {
        if let Some(info) = self.connections.write().await.get_mut(connection_id) {
            info.workspace_path = Some(workspace_path.to_string());
        }
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
