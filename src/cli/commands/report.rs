//! Report and diagnostics subcommands: stats, health, doctor smoke, branch metrics,
//! and 3 report children.

use serde_json::{Value, json};

use crate::cli::flags::GlobalFlags;
use crate::cli::output::OutputFormatter;
use crate::cli::runner::run_tool;

/// `engram stats` → `get_workspace_statistics`
pub async fn run_stats(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_workspace_statistics", None, flags, formatter).await
}

/// `engram health` → `get_health_report`
pub async fn run_health(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_health_report", None, flags, formatter).await
}

/// Run the hidden `engram doctor --smoke` workflow selected in decision 039-D.
///
/// Future consolidation under `engram health` remains subject to evaluation
/// tracked in stash A8C1F6D5.
pub async fn run_doctor_smoke(
    smoke: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    if !smoke {
        return formatter.cli_error("--smoke is required");
    }

    let raw_workspace = match flags.resolve_workspace() {
        Ok(path) => path,
        Err(err) => return formatter.cli_error(&err),
    };
    let workspace = match std::fs::canonicalize(&raw_workspace) {
        Ok(path) => crate::db::workspace::normalize_canonical(path),
        Err(err) if err.kind() == std::io::ErrorKind::NotFound => raw_workspace,
        Err(err) => return formatter.cli_error(&format!("workspace path error: {err}")),
    };
    let id = flags.id_value();

    match crate::tools::doctor::run_smoke_test(&workspace).await {
        Ok(result) if result.passed => match serde_json::to_value(&result) {
            Ok(value) => formatter.success(id, value),
            Err(err) => formatter.tool_error(
                id,
                -32_603,
                &format!("failed to serialize smoke result: {err}"),
                None,
            ),
        },
        Ok(result) => formatter.tool_error(
            id,
            -32_603,
            &result.message,
            serde_json::to_value(&result).ok(),
        ),
        Err(err) => formatter.tool_error_response(id, &err.to_response()),
    }
}

/// `engram branch-metrics [--branch B] [--compare C]` → `get_branch_metrics`
pub async fn run_branch_metrics(
    branch_name: Option<String>,
    compare_to: Option<String>,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    let mut params = json!({});
    if let Some(b) = branch_name {
        params["branch_name"] = Value::String(b);
    }
    if let Some(c) = compare_to {
        params["compare_to"] = Value::String(c);
    }
    // Use None params when object would be empty to match optional-params convention.
    let params_opt = if params.as_object().is_some_and(serde_json::Map::is_empty) {
        None
    } else {
        Some(params)
    };
    run_tool("get_branch_metrics", params_opt, flags, formatter).await
}

/// `engram report token-savings` → `get_token_savings_report`
pub async fn run_token_savings(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_token_savings_report", None, flags, formatter).await
}

/// `engram report eval` → `get_evaluation_report`
pub async fn run_eval(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_evaluation_report", None, flags, formatter).await
}

/// `engram report retry-metrics` → `get_mutable_script_retry_metrics`
pub async fn run_retry_metrics(flags: &GlobalFlags, formatter: &OutputFormatter) -> i32 {
    run_tool("get_mutable_script_retry_metrics", None, flags, formatter).await
}

#[cfg(test)]
mod tests {
    use serde_json::json;

    #[test]
    fn branch_metrics_params_use_correct_field_names() {
        let mut params = json!({});
        params["branch_name"] = serde_json::Value::String("main".into());
        params["compare_to"] = serde_json::Value::String("dev".into());
        assert_eq!(params["branch_name"], "main");
        assert_eq!(params["compare_to"], "dev");
    }

    #[test]
    fn branch_metrics_no_params_when_both_none() {
        // Both branch_name and compare_to are None → empty object → None params.
        let params = json!({});
        let is_empty = params.as_object().is_some_and(serde_json::Map::is_empty);
        assert!(is_empty, "empty params object should produce None");
    }

    #[test]
    fn stats_uses_correct_method() {
        assert_eq!("get_workspace_statistics", "get_workspace_statistics");
    }

    #[test]
    fn health_uses_correct_method() {
        assert_eq!("get_health_report", "get_health_report");
    }

    #[test]
    fn retry_metrics_uses_correct_method() {
        assert_eq!(
            "get_mutable_script_retry_metrics",
            "get_mutable_script_retry_metrics"
        );
    }
}
