//! MCP tool implementations dispatched via JSON-RPC.
//!
//! The `dispatch` function routes tool names to handler functions in
//! the `lifecycle`, `read`, and `write` submodules.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::{EngramError, SystemError};
use crate::server::state::SharedState;

pub mod lifecycle;
pub mod read;
pub mod write;

#[derive(Debug, Deserialize)]
struct WorkspaceParams {
    #[serde(default)]
    path: String,
}

fn not_implemented(method: &str) -> EngramError {
    EngramError::System(SystemError::InvalidParams {
        reason: format!("{method} not implemented"),
    })
}

pub async fn dispatch(
    state: SharedState,
    method: &str,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    match method {
        "set_workspace" => {
            let parsed: WorkspaceParams =
                serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
                    EngramError::System(SystemError::InvalidParams {
                        reason: e.to_string(),
                    })
                })?;
            let result = lifecycle::set_workspace(state.as_ref(), parsed.path).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_daemon_status" => {
            let result = lifecycle::get_daemon_status(state.as_ref()).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "get_workspace_status" => {
            let result = lifecycle::get_workspace_status(state.as_ref()).await?;
            Ok(serde_json::to_value(result).unwrap())
        }
        "create_task" => write::create_task(state, params).await,
        "update_task" => write::update_task(state, params).await,
        "add_blocker" => write::add_blocker(state, params).await,
        "register_decision" => write::register_decision(state, params).await,
        "flush_state" => write::flush_state(state, params).await,
        "get_task_graph" => read::get_task_graph(state, params).await,
        "check_status" => read::check_status(state, params).await,
        "query_memory" => read::query_memory(state, params).await,
        "get_ready_work" => read::get_ready_work(state, params).await,
        "add_label" => write::add_label(state, params).await,
        "remove_label" => write::remove_label(state, params).await,
        "add_dependency" => write::add_dependency(state, params).await,
        "get_compaction_candidates" => read::get_compaction_candidates(state, params).await,
        "apply_compaction" => write::apply_compaction(state, params).await,
        "claim_task" => write::claim_task(state, params).await,
        "release_task" => write::release_task(state, params).await,
        "defer_task" => write::defer_task(state, params).await,
        "undefer_task" => write::undefer_task(state, params).await,
        "pin_task" => write::pin_task(state, params).await,
        "unpin_task" => write::unpin_task(state, params).await,
        "get_workspace_statistics" => read::get_workspace_statistics(state, params).await,
        "batch_update_tasks" => write::batch_update_tasks(state, params).await,
        "add_comment" => write::add_comment(state, params).await,
        _ => Err(not_implemented(method)),
    }
}
