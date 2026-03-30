//! MCP tool implementations dispatched via JSON-RPC.
//!
//! The `dispatch` function routes tool names to handler functions in
//! the `lifecycle`, `read`, and `write` submodules.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::{EngramError, SystemError};
use crate::models::metrics::UsageEvent;
use crate::server::state::SharedState;
use crate::services::metrics;

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

fn should_record_metrics(method: &str) -> bool {
    matches!(
        method,
        "query_memory"
            | "get_workspace_statistics"
            | "map_code"
            | "list_symbols"
            | "unified_search"
            | "impact_analysis"
            | "get_health_report"
            | "query_graph"
            | "get_branch_metrics"
            | "get_token_savings_report"
    ) || cfg!(feature = "git-graph") && method == "query_changes"
}

fn value_array_len(value: Option<&Value>) -> u32 {
    value
        .and_then(Value::as_array)
        .and_then(|array| u32::try_from(array.len()).ok())
        .unwrap_or(0)
}

fn value_u32(value: Option<&Value>) -> u32 {
    value
        .and_then(Value::as_u64)
        .and_then(|count| u32::try_from(count).ok())
        .unwrap_or(0)
}

fn extract_counts(method: &str, value: &Value) -> (u32, u32) {
    match method {
        "map_code" => {
            let neighbors = value_array_len(value.get("neighbors"));
            let root_count = u32::from(!value.get("root").unwrap_or(&Value::Null).is_null());
            let total = neighbors.saturating_add(root_count);
            (total, total)
        }
        "list_symbols" => {
            let total = value_u32(value.get("total_count"));
            (total, total)
        }
        "unified_search" | "query_memory" => {
            let total = value_array_len(value.get("results"));
            (total, total)
        }
        "impact_analysis" => {
            let total = value_array_len(value.get("code_neighborhood"));
            (total, total)
        }
        "query_graph" => {
            let total = value_u32(value.get("row_count"));
            (total, total)
        }
        #[cfg(feature = "git-graph")]
        "query_changes" => {
            let total = value_u32(value.get("total"));
            (total, total)
        }
        "get_branch_metrics" => {
            let total = u32::from(value.get("comparison").is_some()) + 1;
            (0, total)
        }
        "get_workspace_statistics" | "get_health_report" | "get_token_savings_report" => (0, 1),
        _ => (0, 0),
    }
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
        "get_branch_metrics" => read::get_branch_metrics(state.clone(), params).await,
        "get_token_savings_report" => read::get_token_savings_report(state.clone(), params).await,
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

    if should_record_metrics(method) {
        if let Ok(value) = &result {
            if let Some(snapshot) = state.snapshot_workspace().await {
                let response_bytes = u64::try_from(value.to_string().len()).unwrap_or(u64::MAX);
                let (symbols_returned, results_returned) = extract_counts(method, value);
                metrics::record(UsageEvent {
                    tool_name: method.to_owned(),
                    timestamp: chrono::Utc::now().to_rfc3339(),
                    response_bytes,
                    estimated_tokens: response_bytes / 4,
                    symbols_returned,
                    results_returned,
                    branch: snapshot.branch,
                    connection_id: None,
                    agent_role: None,
                    outcome: "success".to_string(),
                });
            }
        }
    }

    result
}
