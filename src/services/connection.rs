#![allow(dead_code)]

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
