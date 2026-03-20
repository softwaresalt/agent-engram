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
        "flush_state" => write::flush_state(state.clone(), params).await,
        "query_memory" => read::query_memory(state.clone(), params).await,
        "get_workspace_statistics" => read::get_workspace_statistics(state.clone(), params).await,
        "index_workspace" => write::index_workspace(state.clone(), params).await,
        "sync_workspace" => write::sync_workspace(state.clone(), params).await,
        "map_code" => read::map_code(state.clone(), params).await,
        "list_symbols" => read::list_symbols(state.clone(), params).await,
        "unified_search" => read::unified_search(state.clone(), params).await,
        "impact_analysis" => read::impact_analysis(state.clone(), params).await,
        "get_health_report" => read::get_health_report(state.clone(), params).await,
        "query_graph" => read::query_graph(state.clone(), params).await,
        #[cfg(feature = "git-graph")]
        "query_changes" => read::query_changes(state.clone(), params).await,
        #[cfg(feature = "git-graph")]
        "index_git_history" => write::index_git_history(state.clone(), params).await,
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
