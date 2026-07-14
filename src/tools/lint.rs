//! `lint_dax` MCP tool — Tier-1 + Tier-2 DAX lint over the bound workspace's
//! indexed Power BI models.
//!
//! Tier-2 broken-reference detection reparses the indexed model expressions
//! against a model-scope-aggregated schema built at lint time (keyed by
//! `canonical_tmdl_model_path`), so a stale reference in an unchanged sibling
//! `.tmdl` is caught even when an incremental index pass skipped that file. The
//! tool is read-only and daemon-backed (the resolved schema is required, per
//! decision D1).

use serde::Deserialize;
use serde_json::Value;

use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::registry::ContentSourceStatus;
use crate::server::state::SharedState;
use crate::services::dax_lint::{ModelPathNotIndexed, lint_indexed_models};

/// Parameters for the `lint_dax` tool.
#[derive(Debug, Default, Deserialize)]
struct LintDaxParams {
    /// Optional TMDL model path, canonicalised to one model scope via
    /// `canonical_tmdl_model_path`. Omitted lints every indexed model in the
    /// bound workspace.
    #[serde(default)]
    model_path: Option<String>,
}

/// Lint the DAX in the bound workspace's indexed Power BI model(s).
///
/// Returns `{ conformant, findings[] }` where each finding carries a `rule`,
/// `message`, optional `line`, and `severity`. When `model_path` is supplied it
/// is canonicalised to a single model scope and only that model is linted; a
/// path that matches no indexed model is a `WorkspaceNotFound` error result.
///
/// # Errors
/// - `WorkspaceError::NotSet` (1003) when no workspace is bound.
/// - `WorkspaceError::NotFound` when `model_path` names no indexed model.
pub async fn lint_dax(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let snapshot = state
        .snapshot_workspace()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;

    let parsed: LintDaxParams = match params {
        Some(value) if !value.is_null() => serde_json::from_value(value).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?,
        _ => LintDaxParams::default(),
    };

    let workspace_root = std::path::PathBuf::from(snapshot.path);
    let model_path = parsed
        .model_path
        .map(|path| path.trim().to_owned())
        .filter(|path| !path.is_empty());

    let outcome = tokio::task::spawn_blocking(move || -> Result<_, EngramError> {
        let registry_path = workspace_root.join(".engram").join("registry.yaml");
        let source_paths = match crate::services::registry::load_registry(&registry_path) {
            Ok(Some(mut config)) => {
                // Populate per-source status (path-traversal / missing / duplicate
                // detection). A hard validation failure (e.g. the workspace root
                // cannot be canonicalised) is surfaced as a tool error rather than
                // silently degrading to "no Power BI sources".
                crate::services::registry::validate_sources(&mut config, &workspace_root)?;
                config
                    .sources
                    .into_iter()
                    .filter(|source| {
                        source.content_type == "powerbi"
                            && source.status == ContentSourceStatus::Active
                    })
                    .map(|source| source.path)
                    .collect::<Vec<_>>()
            }
            // No registry file is a benign empty scope — nothing to lint.
            Ok(None) => Vec::new(),
            // A registry that exists but cannot be read/parsed is an error, not a
            // silent pass to `{ conformant: true }`.
            Err(e) => return Err(e),
        };
        Ok(lint_indexed_models(
            &workspace_root,
            &source_paths,
            model_path.as_deref(),
        ))
    })
    .await
    .map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("lint_dax worker failed: {e}"),
        })
    })??;

    let report = outcome.map_err(|ModelPathNotIndexed(path)| {
        EngramError::Workspace(WorkspaceError::NotFound { path })
    })?;

    serde_json::to_value(&report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize lint_dax response: {e}"),
        })
    })
}
