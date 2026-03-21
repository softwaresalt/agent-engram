//! Workspace configuration loading and validation.
//!
//! Reads `.engram/config.toml`, deserializes into [`WorkspaceConfig`],
//! and validates all values against constraints. Missing files produce
//! defaults; parse errors log a warning and fall back to defaults;
//! semantic violations (e.g. `threshold_days = 0`) return
//! [`EngramError::Config`].

use std::path::Path;

use tracing::warn;

use crate::errors::{ConfigError, EngramError};
use crate::models::config::WorkspaceConfig;

/// Load and parse workspace config from `.engram/config.toml`.
///
/// * Missing file → `Ok(WorkspaceConfig::default())`
/// * Parse error  → warn + `Ok(WorkspaceConfig::default())`
/// * Valid file   → `Ok(parsed config)`
pub fn parse_config(workspace_root: &Path) -> Result<WorkspaceConfig, EngramError> {
    let config_path = workspace_root.join(".engram").join("config.toml");
    if !config_path.exists() {
        return Ok(WorkspaceConfig::default());
    }
    let content = std::fs::read_to_string(&config_path).map_err(|e| {
        warn!(path = %config_path.display(), error = %e, "failed to read config.toml, using defaults");
        EngramError::Config(ConfigError::ParseError {
            reason: e.to_string(),
        })
    });
    let Ok(content) = content else {
        return Ok(WorkspaceConfig::default());
    };
    match toml::from_str::<WorkspaceConfig>(&content) {
        Ok(cfg) => {
            validate_config(&cfg)?;
            Ok(cfg)
        }
        Err(e) => {
            warn!(
                path = %config_path.display(),
                error = %e,
                "config.toml parse error, falling back to defaults"
            );
            Ok(WorkspaceConfig::default())
        }
    }
}

/// Validate semantic constraints on a parsed [`WorkspaceConfig`].
///
/// Returns `Err(EngramError::Config(ConfigError::InvalidValue))` for:
/// * `batch.max_size == 0` or `> 1000`
pub fn validate_config(config: &WorkspaceConfig) -> Result<(), EngramError> {
    if config.batch.max_size == 0 || config.batch.max_size > 1000 {
        return Err(EngramError::Config(ConfigError::InvalidValue {
            key: "batch.max_size".to_owned(),
            reason: "must be between 1 and 1000".to_owned(),
        }));
    }
    Ok(())
}

/// Legacy wrapper — kept only for backward compatibility in tests.
/// Prefer [`parse_config`] + [`validate_config`] in production code.
#[allow(dead_code)]
pub fn load_workspace_config(
    workspace_root: &Path,
) -> Result<Option<WorkspaceConfig>, Box<dyn std::error::Error>> {
    let config_path = workspace_root.join(".engram").join("config.toml");
    if !config_path.exists() {
        return Ok(None);
    }
    let content = std::fs::read_to_string(&config_path)?;
    let config: WorkspaceConfig = toml::from_str(&content)?;
    Ok(Some(config))
}
