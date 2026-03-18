//! Content registry service for parsing and validating `.engram/registry.yaml`.
//!
//! Provides [`load_registry`] to parse the registry YAML file and
//! [`validate_sources`] to check each declared source path against
//! the workspace root for security and existence.

use std::collections::HashSet;
use std::path::Path;

use tracing::{info, warn};

use crate::errors::{EngramError, RegistryError};
use crate::models::registry::{ContentSourceStatus, RegistryConfig};

/// Maximum allowed value for `max_file_size_bytes` (100 MB).
const MAX_FILE_SIZE_LIMIT: u64 = 100 * 1024 * 1024;

/// Maximum allowed value for `batch_size`.
const MAX_BATCH_SIZE: usize = 500;

/// Parse a [`RegistryConfig`] from a YAML string.
///
/// Validates that `max_file_size_bytes` is within bounds (> 0, ≤ 100 MB)
/// and `batch_size` is within bounds (> 0, ≤ 500). Returns
/// [`RegistryError::ParseFailed`] on YAML syntax errors and
/// [`RegistryError::ValidationFailed`] on constraint violations.
pub fn parse_registry_yaml(yaml_str: &str) -> Result<RegistryConfig, EngramError> {
    let config: RegistryConfig =
        serde_yaml::from_str(yaml_str).map_err(|e| RegistryError::ParseFailed {
            reason: e.to_string(),
        })?;

    if config.max_file_size_bytes == 0 {
        return Err(RegistryError::ValidationFailed {
            reason: "max_file_size_bytes must be greater than 0".to_string(),
        }
        .into());
    }
    if config.max_file_size_bytes > MAX_FILE_SIZE_LIMIT {
        return Err(RegistryError::ValidationFailed {
            reason: format!(
                "max_file_size_bytes must be ≤ {} (100 MB), got {}",
                MAX_FILE_SIZE_LIMIT, config.max_file_size_bytes
            ),
        }
        .into());
    }
    if config.batch_size == 0 {
        return Err(RegistryError::ValidationFailed {
            reason: "batch_size must be greater than 0".to_string(),
        }
        .into());
    }
    if config.batch_size > MAX_BATCH_SIZE {
        return Err(RegistryError::ValidationFailed {
            reason: format!(
                "batch_size must be ≤ {MAX_BATCH_SIZE}, got {}",
                config.batch_size
            ),
        }
        .into());
    }

    Ok(config)
}

/// Load a [`RegistryConfig`] from a file at the given path.
///
/// Returns `Ok(None)` if the file does not exist (legacy fallback).
/// Returns an error if the file exists but cannot be read or parsed.
pub fn load_registry(registry_path: &Path) -> Result<Option<RegistryConfig>, EngramError> {
    if !registry_path.exists() {
        info!(
            "No registry file at {}; using legacy fallback",
            registry_path.display()
        );
        return Ok(None);
    }

    let yaml_str =
        std::fs::read_to_string(registry_path).map_err(|e| RegistryError::ParseFailed {
            reason: format!("Failed to read {}: {e}", registry_path.display()),
        })?;

    let config = parse_registry_yaml(&yaml_str)?;
    info!(
        sources = config.sources.len(),
        "Loaded registry with {} source(s)",
        config.sources.len()
    );
    Ok(Some(config))
}

/// Validate all source paths in a [`RegistryConfig`] against the workspace root.
///
/// For each source:
/// - Resolves the path relative to `workspace_root`
/// - Rejects paths that resolve outside the workspace (path traversal)
/// - Detects duplicate paths across entries
/// - Sets [`ContentSourceStatus`] on each source
///
/// Returns the number of active (valid) sources.
pub fn validate_sources(
    config: &mut RegistryConfig,
    workspace_root: &Path,
) -> Result<usize, EngramError> {
    let canonical_root =
        workspace_root
            .canonicalize()
            .map_err(|e| RegistryError::ValidationFailed {
                reason: format!("Cannot canonicalize workspace root: {e}"),
            })?;

    let mut seen_paths: HashSet<String> = HashSet::new();
    let mut active_count = 0usize;

    for source in &mut config.sources {
        let resolved = canonical_root.join(&source.path);

        // Check for path traversal — resolved path must be within workspace root.
        match resolved.canonicalize() {
            Ok(canonical) => {
                if !canonical.starts_with(&canonical_root) {
                    warn!(
                        path = %source.path,
                        "Registry source path resolves outside workspace root — rejected"
                    );
                    source.status = ContentSourceStatus::Error;
                    continue;
                }
            }
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => {
                if resolved
                    .components()
                    .any(|c| c == std::path::Component::ParentDir)
                {
                    warn!(
                        path = %source.path,
                        "Registry source path contains '..' traversal — rejected"
                    );
                    source.status = ContentSourceStatus::Error;
                    continue;
                }
                warn!(path = %source.path, "Registry source path does not exist");
                source.status = ContentSourceStatus::Missing;
                continue;
            }
            Err(e) => {
                warn!(
                    path = %source.path,
                    error = %e,
                    "Registry source path could not be canonicalized"
                );
                source.status = ContentSourceStatus::Error;
                continue;
            }
        }

        // Check for duplicate paths.
        let normalized = source.path.replace('\\', "/");
        if !seen_paths.insert(normalized.clone()) {
            warn!(
                path = %source.path,
                "Duplicate registry source path — rejected"
            );
            source.status = ContentSourceStatus::Error;
            continue;
        }

        // Path exists and is within workspace root.
        source.status = ContentSourceStatus::Active;
        active_count += 1;
    }

    info!(
        active = active_count,
        total = config.sources.len(),
        "Registry validation complete"
    );
    Ok(active_count)
}
