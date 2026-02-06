#![allow(dead_code)]

use serde::Deserialize;
use serde_json::{Value, json};

use crate::errors::{SystemError, TMemError};
use crate::server::state::SharedState;

pub mod lifecycle;

#[derive(Debug, Deserialize)]
struct RpcParams {
	#[serde(default)]
	path: String,
}

pub async fn dispatch(state: SharedState, method: &str, params: Option<Value>) -> Result<Value, TMemError> {
	match method {
		"set_workspace" => {
			let parsed: RpcParams = serde_json::from_value(params.unwrap_or_else(|| json!({})))
				.map_err(|e| TMemError::System(SystemError::DatabaseError { reason: e.to_string() }))?;
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
		_ => Err(TMemError::System(SystemError::DatabaseError {
			reason: format!("Unknown method: {method}"),
		})),
	}
}
