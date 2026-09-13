//! `lint_dax` MCP tool — Tier-1 + Tier-2 DAX lint over the bound workspace's
//! indexed Power BI models.
//!
//! Tier-2 broken-reference detection reparses the indexed model expressions
//! against a model-scope-aggregated schema built at lint time (keyed by
//! `canonical_tmdl_model_path`), so a stale reference in an unchanged sibling
//! `.tmdl` is caught even when an incremental index pass skipped that file. The
//! tool is read-only and daemon-backed (the resolved schema is required, per
//! decision D1).

use std::path::PathBuf;
use std::sync::{Mutex, OnceLock};

use serde::Deserialize;
use serde_json::Value;
use tokio::sync::oneshot;

use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::models::registry::ContentSourceStatus;
use crate::server::state::SharedState;
use crate::services::dax_lint::{LintError, lint_indexed_models};

/// Parameters for the `lint_dax` tool.
#[derive(Debug, Default, Deserialize)]
struct LintDaxParams {
    /// Optional TMDL model path, canonicalised to one model scope via
    /// `canonical_tmdl_model_path`. Omitted lints every indexed model in the
    /// bound workspace.
    #[serde(default)]
    model_path: Option<String>,
}

struct GenerationPinTestHook {
    method: String,
    reached: Option<oneshot::Sender<()>>,
    resume: Option<oneshot::Receiver<()>>,
}

fn generation_pin_test_hook() -> &'static Mutex<Option<GenerationPinTestHook>> {
    static HOOK: OnceLock<Mutex<Option<GenerationPinTestHook>>> = OnceLock::new();
    HOOK.get_or_init(|| Mutex::new(None))
}

/// Install a one-shot barrier reached immediately after the lint handler pins its dispatch context.
#[doc(hidden)]
pub fn install_generation_pin_test_hook(
    method: &str,
    reached: oneshot::Sender<()>,
    resume: oneshot::Receiver<()>,
) {
    let hook = GenerationPinTestHook {
        method: method.to_owned(),
        reached: Some(reached),
        resume: Some(resume),
    };
    *generation_pin_test_hook()
        .lock()
        .unwrap_or_else(std::sync::PoisonError::into_inner) = Some(hook);
}

async fn maybe_pause_generation_pin_test_hook(method: &str) {
    let pending_resume = {
        let mut slot = generation_pin_test_hook()
            .lock()
            .unwrap_or_else(std::sync::PoisonError::into_inner);
        let Some(mut hook) = slot.take() else {
            return;
        };
        if hook.method != method {
            *slot = Some(hook);
            return;
        }
        if let Some(reached) = hook.reached.take() {
            let _ = reached.send(());
        }
        hook.resume.take()
    };

    if let Some(resume) = pending_resume {
        let _ = resume.await;
    }
}

async fn pinned_workspace_root(state: &SharedState, method: &str) -> Result<PathBuf, EngramError> {
    let context = state
        .snapshot_dispatch_context()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    maybe_pause_generation_pin_test_hook(method).await;
    Ok(PathBuf::from(context.workspace.path))
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
/// - `SystemError::DatabaseError` when an indexed model file cannot be read or
///   decoded as UTF-8 (the registry could not be validated, or a serialization
///   failure occurred).
pub async fn lint_dax(state: SharedState, params: Option<Value>) -> Result<Value, EngramError> {
    let workspace_root = pinned_workspace_root(&state, "lint_dax").await?;

    let parsed: LintDaxParams = match params {
        Some(value) if !value.is_null() => serde_json::from_value(value).map_err(|e| {
            EngramError::System(SystemError::InvalidParams {
                reason: format!("invalid params: {e}"),
            })
        })?,
        _ => LintDaxParams::default(),
    };

    let model_path = parsed
        .model_path
        .map(|path| path.trim().to_owned())
        .filter(|path| !path.is_empty());

    let outcome = tokio::task::spawn_blocking(move || -> Result<_, EngramError> {
        let registry_path = workspace_root.join(".engram").join("registry.yaml");
        let (source_paths, max_file_size) =
            match crate::services::registry::load_registry(&registry_path) {
                Ok(Some(mut config)) => {
                    // Populate per-source status (path-traversal / missing / duplicate
                    // detection). A hard validation failure (e.g. the workspace root
                    // cannot be canonicalised) is surfaced as a tool error rather than
                    // silently degrading to "no Power BI sources".
                    crate::services::registry::validate_sources(&mut config, &workspace_root)?;
                    let limit = config.max_file_size_bytes;
                    let paths = config
                        .sources
                        .into_iter()
                        .filter(|source| {
                            source.content_type == "powerbi"
                                && source.status == ContentSourceStatus::Active
                        })
                        .map(|source| source.path)
                        .collect::<Vec<_>>();
                    (paths, limit)
                }
                // No registry file is a benign empty scope — nothing to lint. The
                // limit is moot with no sources; use the registry default.
                Ok(None) => (
                    Vec::new(),
                    crate::models::registry::RegistryConfig::default().max_file_size_bytes,
                ),
                // A registry that exists but cannot be read/parsed is an error, not a
                // silent pass to `{ conformant: true }`.
                Err(e) => return Err(e),
            };
        Ok(lint_indexed_models(
            &workspace_root,
            &source_paths,
            max_file_size,
            model_path.as_deref(),
        ))
    })
    .await
    .map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("lint_dax worker failed: {e}"),
        })
    })??;

    let report = outcome.map_err(|err| match err {
        LintError::ModelPathNotIndexed(path) => {
            EngramError::Workspace(WorkspaceError::NotFound { path })
        }
        // An unreadable/undecodable active model file is a hard tool error, not a
        // silent partial pass; surface the offending path in the reason.
        LintError::FileUnreadable { path, reason } => {
            EngramError::System(SystemError::DatabaseError {
                reason: format!("failed to read indexed Power BI model file '{path}': {reason}"),
            })
        }
    })?;

    serde_json::to_value(&report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize lint_dax response: {e}"),
        })
    })
}
