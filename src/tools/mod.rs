//! MCP tool implementations dispatched via JSON-RPC.
//!
//! The `dispatch` function routes tool names to handler functions in
//! the `lifecycle`, `read`, and `write` submodules.

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::{SystemError, TMemError};
use crate::server::state::SharedState;

pub mod lifecycle;
pub mod read;
pub mod write;

#[derive(Debug, Deserialize)]
struct WorkspaceParams {
    #[serde(default)]
    path: String,
}

fn not_implemented(method: &str) -> TMemError {
    TMemError::System(SystemError::InvalidParams {
        reason: format!("{method} not implemented"),
    })
}

pub async fn dispatch(
    state: SharedState,
    method: &str,
    params: Option<Value>,
) -> Result<Value, TMemError> {
    match method {
        "set_workspace" => {
            let parsed: WorkspaceParams =
                serde_json::from_value(params.unwrap_or_else(|| json!({}))).map_err(|e| {
                    TMemError::System(SystemError::InvalidParams {
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
        _ => Err(not_implemented(method)),
    }
}
