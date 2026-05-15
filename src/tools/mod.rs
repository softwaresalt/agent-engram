//! MCP tool implementations dispatched via JSON-RPC.
//!
//! The `dispatch` function routes tool names to handler functions in
//! the `lifecycle`, `read`, and `write` submodules.

use std::collections::BTreeMap;
use std::sync::Arc;

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::{EngramError, SystemError};
use crate::models::metrics::UsageEvent;
use crate::server::state::SharedState;
use crate::services::{metrics, policy};

pub mod doctor;
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
        "get_daemon_status"
            | "get_workspace_status"
            | "query_memory"
            | "get_workspace_statistics"
            | "map_code"
            | "list_symbols"
            | "unified_search"
            | "impact_analysis"
            | "get_health_report"
            | "query_graph"
            | "get_branch_metrics"
            | "get_token_savings_report"
            | "get_evaluation_report"
            | "get_mutable_script_retry_metrics"
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

fn insert_shape_count(shape_counts: &mut BTreeMap<String, u32>, key: &str, count: u32) {
    shape_counts.insert(key.to_owned(), count);
}

fn object_len_u32(value: Option<&Value>) -> u32 {
    value
        .and_then(Value::as_object)
        .and_then(|object| u32::try_from(object.len()).ok())
        .unwrap_or(0)
}

fn extract_counts(method: &str, value: &Value) -> (u32, u32, u32, BTreeMap<String, u32>) {
    let mut shape_counts = BTreeMap::new();
    match method {
        "map_code" => {
            let neighbors = value_array_len(value.get("neighbors"));
            let root_count = u32::from(!value.get("root").unwrap_or(&Value::Null).is_null());
            let total = neighbors.saturating_add(root_count);
            insert_shape_count(&mut shape_counts, "neighbors", neighbors);
            insert_shape_count(&mut shape_counts, "root", root_count);
            insert_shape_count(&mut shape_counts, "nodes", total);
            (total, total, total, shape_counts)
        }
        "list_symbols" => {
            let total = value_u32(value.get("total_count"));
            insert_shape_count(&mut shape_counts, "symbols", total);
            (total, total, total, shape_counts)
        }
        "unified_search" | "query_memory" => {
            let total = value_array_len(value.get("results"));
            insert_shape_count(&mut shape_counts, "results", total);
            (total, total, total, shape_counts)
        }
        "impact_analysis" => {
            let total = value_array_len(value.get("code_neighborhood"));
            insert_shape_count(&mut shape_counts, "code_neighborhood", total);
            (total, total, total, shape_counts)
        }
        "query_graph" => {
            let total = value_u32(value.get("row_count"));
            insert_shape_count(&mut shape_counts, "rows", total);
            (total, total, total, shape_counts)
        }
        #[cfg(feature = "git-graph")]
        "query_changes" => {
            let total = value_u32(value.get("total"));
            insert_shape_count(&mut shape_counts, "changes", total);
            (total, total, total, shape_counts)
        }
        "get_branch_metrics" => {
            let tool_entries = object_len_u32(
                value
                    .get("summary")
                    .and_then(|summary| summary.get("by_tool")),
            );
            let top_symbols = value_array_len(
                value
                    .get("summary")
                    .and_then(|summary| summary.get("top_symbols")),
            );
            let comparison_present = u32::from(
                value
                    .get("comparison")
                    .is_some_and(|comparison| !comparison.is_null()),
            );
            insert_shape_count(&mut shape_counts, "tool_entries", tool_entries);
            insert_shape_count(&mut shape_counts, "top_symbols", top_symbols);
            insert_shape_count(&mut shape_counts, "comparison", comparison_present);
            (0, 1, 1, shape_counts)
        }
        "get_workspace_statistics" => {
            let sections = object_len_u32(Some(value));
            insert_shape_count(&mut shape_counts, "sections", sections);
            (0, 1, 1, shape_counts)
        }
        "get_health_report" => {
            let sections = object_len_u32(Some(value));
            insert_shape_count(&mut shape_counts, "sections", sections);
            (0, 1, 1, shape_counts)
        }
        "get_token_savings_report" => {
            let report_present = u32::from(value.get("report").is_some());
            insert_shape_count(&mut shape_counts, "report", report_present);
            (0, 1, 1, shape_counts)
        }
        "get_evaluation_report" => {
            let agents = value_array_len(value.get("agents"));
            let anomalies = value_array_len(value.get("anomalies"));
            let recommendations = value_array_len(value.get("recommendations"));
            insert_shape_count(&mut shape_counts, "agents", agents);
            insert_shape_count(&mut shape_counts, "anomalies", anomalies);
            insert_shape_count(&mut shape_counts, "recommendations", recommendations);
            (0, 1, 1, shape_counts)
        }
        "get_workspace_status" => {
            let code_graph = value.get("code_graph");
            let code_files = value_u32(code_graph.and_then(|graph| graph.get("code_files")));
            let functions = value_u32(code_graph.and_then(|graph| graph.get("functions")));
            let classes = value_u32(code_graph.and_then(|graph| graph.get("classes")));
            let interfaces = value_u32(code_graph.and_then(|graph| graph.get("interfaces")));
            let edges = value_u32(code_graph.and_then(|graph| graph.get("edges")));
            let scan_status_present =
                u32::from(value.get("scan_status").is_some_and(|scan| !scan.is_null()));
            insert_shape_count(&mut shape_counts, "code_files", code_files);
            insert_shape_count(&mut shape_counts, "functions", functions);
            insert_shape_count(&mut shape_counts, "classes", classes);
            insert_shape_count(&mut shape_counts, "interfaces", interfaces);
            insert_shape_count(&mut shape_counts, "edges", edges);
            insert_shape_count(&mut shape_counts, "scan_status", scan_status_present);
            (0, 1, 1, shape_counts)
        }
        "get_daemon_status" => {
            let checks =
                value_array_len(value.get("health").and_then(|health| health.get("checks")));
            insert_shape_count(&mut shape_counts, "checks", checks);
            (0, 1, 1, shape_counts)
        }
        "get_mutable_script_retry_metrics" => {
            insert_shape_count(&mut shape_counts, "retry_snapshot", 1);
            (0, 1, 1, shape_counts)
        }
        _ => (0, 0, 0, shape_counts),
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
    let request_bytes = params
        .as_ref()
        .map(|value| u64::try_from(value.to_string().len()).unwrap_or(u64::MAX))
        .unwrap_or(0);
    let estimated_input_tokens = request_bytes / 4;

    // Extract agent identity from JSON-RPC _meta before dispatch.
    let agent_role = policy::extract_agent_role(&params);

    // Take an atomic snapshot of workspace binding + config at dispatch entry.
    //
    // Both locks are acquired and released inside `snapshot_dispatch_context`,
    // producing a frozen `DispatchSnapshot`. This eliminates the TOCTOU window
    // where a concurrent `set_workspace_config` call could change policy after
    // the check but before the tool runs. See TASK-018.
    //
    // Before a workspace is bound (`snapshot_dispatch_context` returns `None`)
    // the policy engine is bypassed. The initial `set_workspace` call is always
    // ungated. A daemon-level policy independent of workspace config would be
    // needed to gate workspace binding itself.
    let dispatch_snapshot = state.snapshot_dispatch_context().await;

    // Enforce sandbox policy when a workspace is bound.
    if let Some(ref snap) = dispatch_snapshot {
        if let Err(policy_err) =
            policy::evaluate(&snap.config.policy, agent_role.as_deref(), method)
        {
            // Record denied calls in metrics so evaluations reflect policy activity.
            metrics::record(UsageEvent {
                tool_name: method.to_owned(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                request_bytes,
                estimated_input_tokens,
                response_bytes: 0,
                estimated_output_tokens: 0,
                estimated_tokens: 0,
                result_count: 0,
                response_shape_counts: BTreeMap::new(),
                symbols_returned: 0,
                results_returned: 0,
                branch: snap.workspace.branch.clone(),
                connection_id: None,
                agent_role: agent_role.clone(),
                outcome: "denied".to_string(),
                prompt_tokens_attributed: None,
                completion_tokens_attributed: None,
                cached_tokens_attributed: None,
            });
            return Err(EngramError::from(policy_err));
        }
    }

    let result = match method {
        "set_workspace" => {
            let parsed: WorkspaceParams =
                serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
                    EngramError::System(SystemError::InvalidParams {
                        reason: e.to_string(),
                    })
                })?;
            let inner = lifecycle::set_workspace(Arc::clone(&state), parsed.path).await?;
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
        "get_evaluation_report" => read::get_evaluation_report(state.clone(), params).await,
        "query_graph" => read::query_graph(state.clone(), params).await,
        "get_mutable_script_retry_metrics" => {
            read::get_mutable_script_retry_metrics(state.clone(), params).await
        }
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
        if let Some(snap) = dispatch_snapshot {
            // Compute response stats from the result (zero defaults for errors).
            let (
                response_bytes,
                estimated_output_tokens,
                symbols_returned,
                results_returned,
                result_count,
                response_shape_counts,
            ) = match &result {
                Ok(value) => {
                    let rb = u64::try_from(value.to_string().len()).unwrap_or(u64::MAX);
                    let (sym, res, result_count, response_shape_counts) =
                        extract_counts(method, value);
                    (rb, rb / 4, sym, res, result_count, response_shape_counts)
                }
                Err(_) => (0_u64, 0_u64, 0_u32, 0_u32, 0_u32, BTreeMap::new()),
            };
            let outcome = if result.is_ok() { "success" } else { "error" };
            metrics::record(UsageEvent {
                tool_name: method.to_owned(),
                timestamp: chrono::Utc::now().to_rfc3339(),
                request_bytes,
                estimated_input_tokens,
                response_bytes,
                estimated_output_tokens,
                estimated_tokens: estimated_output_tokens,
                result_count,
                response_shape_counts,
                symbols_returned,
                results_returned,
                branch: snap.workspace.branch,
                connection_id: None,
                agent_role: agent_role.clone(),
                outcome: outcome.to_string(),
                prompt_tokens_attributed: None,
                completion_tokens_attributed: None,
                cached_tokens_attributed: None,
            });
        }
    }

    result
}

#[cfg(test)]
mod tests {
    use super::extract_counts;
    use serde_json::json;

    #[test]
    fn query_memory_preserves_legacy_symbol_count_compatibility() {
        let value = json!({
            "results": [
                { "id": "a" },
                { "id": "b" },
            ]
        });

        let (symbols_returned, results_returned, result_count, shape_counts) =
            extract_counts("query_memory", &value);

        assert_eq!(symbols_returned, 2);
        assert_eq!(results_returned, 2);
        assert_eq!(result_count, 2);
        assert_eq!(shape_counts.get("results"), Some(&2));
    }

    #[test]
    fn branch_metrics_ignores_null_comparison_payloads() {
        let value = json!({
            "summary": {
                "by_tool": {
                    "map_code": { "call_count": 1 }
                },
                "top_symbols": [
                    { "name": "map_code", "count": 1 }
                ]
            },
            "comparison": null
        });

        let (_, _, _, shape_counts) = extract_counts("get_branch_metrics", &value);

        assert_eq!(shape_counts.get("comparison"), Some(&0));
        assert_eq!(shape_counts.get("tool_entries"), Some(&1));
        assert_eq!(shape_counts.get("top_symbols"), Some(&1));
    }
}
