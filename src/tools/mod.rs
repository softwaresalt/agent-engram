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

#[tracing::instrument(
    name = "tool_dispatch",
    skip(state, params),
    fields(tool = %method)
)]
pub async fn dispatch(
    state: SharedState,
    method: &str,
    params: Option<Value>,
) -> Result<Value, EngramError> {
    let start = std::time::Instant::now();

    let result = match method {
        "set_workspace" => {
            let parsed: WorkspaceParams =
                serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
                    EngramError::System(SystemError::InvalidParams {
                        reason: e.to_string(),
                    })
                })?;
            let inner = lifecycle::set_workspace(state.as_ref(), parsed.path).await?;
            serde_json::to_value(inner).map_err(|e| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("failed to serialize response: {e}"),
                })
            })
        }
        "get_daemon_status" => {
            let inner = lifecycle::get_daemon_status(state.as_ref()).await?;
            serde_json::to_value(inner).map_err(|e| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("failed to serialize response: {e}"),
                })
            })
        }
        "get_workspace_status" => {
            let inner = lifecycle::get_workspace_status(state.as_ref()).await?;
            serde_json::to_value(inner).map_err(|e| {
                EngramError::System(SystemError::DatabaseError {
                    reason: format!("failed to serialize response: {e}"),
                })
            })
        }
        "create_task" => write::create_task(state.clone(), params).await,
        "update_task" => write::update_task(state.clone(), params).await,
        "add_blocker" => write::add_blocker(state.clone(), params).await,
        "register_decision" => write::register_decision(state.clone(), params).await,
        "flush_state" => write::flush_state(state.clone(), params).await,
        "get_task_graph" => read::get_task_graph(state.clone(), params).await,
        "check_status" => read::check_status(state.clone(), params).await,
        "query_memory" => read::query_memory(state.clone(), params).await,
        "get_ready_work" => read::get_ready_work(state.clone(), params).await,
        "add_label" => write::add_label(state.clone(), params).await,
        "remove_label" => write::remove_label(state.clone(), params).await,
        "add_dependency" => write::add_dependency(state.clone(), params).await,
        "get_compaction_candidates" => read::get_compaction_candidates(state.clone(), params).await,
        "apply_compaction" => write::apply_compaction(state.clone(), params).await,
        "claim_task" => write::claim_task(state.clone(), params).await,
        "release_task" => write::release_task(state.clone(), params).await,
        "defer_task" => write::defer_task(state.clone(), params).await,
        "undefer_task" => write::undefer_task(state.clone(), params).await,
        "pin_task" => write::pin_task(state.clone(), params).await,
        "unpin_task" => write::unpin_task(state.clone(), params).await,
        "get_workspace_statistics" => read::get_workspace_statistics(state.clone(), params).await,
        "batch_update_tasks" => write::batch_update_tasks(state.clone(), params).await,
        "add_comment" => write::add_comment(state.clone(), params).await,
        "index_workspace" => write::index_workspace(state.clone(), params).await,
        "sync_workspace" => write::sync_workspace(state.clone(), params).await,
        "link_task_to_code" => write::link_task_to_code(state.clone(), params).await,
        "unlink_task_from_code" => write::unlink_task_from_code(state.clone(), params).await,
        "map_code" => read::map_code(state.clone(), params).await,
        "list_symbols" => read::list_symbols(state.clone(), params).await,
        "get_active_context" => read::get_active_context(state.clone(), params).await,
        "unified_search" => read::unified_search(state.clone(), params).await,
        "impact_analysis" => read::impact_analysis(state.clone(), params).await,
        "get_health_report" => read::get_health_report(state.clone(), params).await,
        "get_event_history" => read::get_event_history(state.clone(), params).await,
        "rollback_to_event" => write::rollback_to_event(state.clone(), params).await,
        "query_graph" => read::query_graph(state.clone(), params).await,
        "create_collection" => write::create_collection(state.clone(), params).await,
        "add_to_collection" => write::add_to_collection(state.clone(), params).await,
        "remove_from_collection" => write::remove_from_collection(state.clone(), params).await,
        "get_collection_context" => read::get_collection_context(state.clone(), params).await,
        _ => Err(not_implemented(method)),
    };

    // Record latency for all calls (lifecycle calls are cheap; the count stays
    // accurate and the VecDeque caps at 1 000 samples automatically).
    if !matches!(method, "_health" | "_shutdown") {
        state
            .record_tool_latency(u64::try_from(start.elapsed().as_micros()).unwrap_or(u64::MAX))
            .await;
    }

    result
}
