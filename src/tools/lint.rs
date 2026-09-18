//! `lint_dax` MCP tool — Tier-1 + Tier-2 DAX lint over the bound workspace's
//! indexed Power BI models.
//!
//! Tier-2 broken-reference detection reparses the indexed model expressions
//! against a model-scope-aggregated schema built at lint time (keyed by
//! `canonical_tmdl_model_path`), so a stale reference in an unchanged sibling
//! `.tmdl` is caught even when an incremental index pass skipped that file. The
//! tool is read-only and daemon-backed (the resolved schema is required, per
//! decision D1).

use std::sync::{Arc, Mutex, OnceLock};

use serde::Deserialize;
use serde_json::Value;
use tokio::sync::oneshot;

use crate::errors::{EngramError, SystemError, WorkspaceError};
use crate::server::state::{ReadRequestContext, SharedState};
use crate::services::dax_lint::load_lint_report;

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

async fn pinned_lint_context(
    state: &SharedState,
    method: &str,
) -> Result<Arc<ReadRequestContext>, EngramError> {
    let context = state
        .snapshot_dispatch_context()
        .await
        .ok_or(EngramError::Workspace(WorkspaceError::NotSet))?;
    maybe_pause_generation_pin_test_hook(method).await;

    Ok(ReadRequestContext::from_workspace_snapshot(
        context.workspace,
    ))
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
    let read_context = pinned_lint_context(&state, "lint_dax").await?;

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

    let report = load_lint_report(
        read_context.as_ref(),
        read_context.data_dir(),
        model_path.as_deref(),
    )
    .await?;

    serde_json::to_value(&report).map_err(|e| {
        EngramError::System(SystemError::DatabaseError {
            reason: format!("failed to serialize lint_dax response: {e}"),
        })
    })
}
