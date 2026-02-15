//! Workspace configuration loading and validation.
//!
//! Reads `.tmem/config.toml`, deserializes into [`WorkspaceConfig`],
//! and validates all values against constraints.

use std::path::Path;

use crate::models::config::WorkspaceConfig;

/// Load workspace config from `.tmem/config.toml` at the given workspace root.
/// Returns `None` if the file does not exist. Returns an error if the file
/// exists but cannot be parsed.
pub fn load_workspace_config(
    workspace_root: &Path,
) -> Result<Option<WorkspaceConfig>, Box<dyn std::error::Error>> {
    let config_path = workspace_root.join(".tmem").join("config.toml");
    if !config_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config: WorkspaceConfig = toml::from_str(&content)?;
    Ok(Some(config))
}
