//! RED contract harness for HCL-family MCP and IPC graph responses (121.001-T).

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use std::sync::Arc;

use engram::daemon::protocol::IpcResponse;
use engram::models::config::{CodeGraphConfig, WorkspaceConfig};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::services::code_graph::{self, IndexResult};
use engram::tools;
use serde_json::{Value, json};
use tempfile::TempDir;

struct IndexedFixture {
    _workspace: TempDir,
    state: Arc<AppState>,
    index: IndexResult,
}

fn write_fixture_file(workspace: &std::path::Path, path: &str, source: &str) {
    let full_path = workspace.join(path);
    if let Some(parent) = full_path.parent() {
        std::fs::create_dir_all(parent).expect("create HCL fixture parent");
    }
    std::fs::write(full_path, source).expect("write HCL fixture");
}

async fn index_mixed_hcl_fixture() -> IndexedFixture {
    let workspace = tempfile::tempdir().expect("create isolated workspace");
    write_fixture_file(
        workspace.path(),
        "infra/main.tf",
        r#"
terraform {
  required_version = ">= 1.6"
}

resource "aws_instance" "web" {
  region = var.region
}
"#,
    );
    write_fixture_file(
        workspace.path(),
        "infra/values.tfvars",
        "region = \"us-west-2\"\nreplicas = 2\n",
    );
    write_fixture_file(
        workspace.path(),
        "infra/service.hcl",
        r#"
service "api" {
  endpoint = module.vpc.id
}
"#,
    );

    let code_graph = CodeGraphConfig {
        supported_languages: vec!["hcl".to_owned()],
        ..CodeGraphConfig::default()
    };
    let config = WorkspaceConfig {
        code_graph: code_graph.clone(),
        ..WorkspaceConfig::default()
    };
    let data_dir = workspace.path().join(".engram");
    let branch = "hcl-contract-red";
    let index = code_graph::index_workspace(workspace.path(), &data_dir, branch, &code_graph, true)
        .await
        .expect("mixed HCL indexing must return its bounded result");

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    state
        .set_workspace_and_config(
            WorkspaceSnapshot {
                workspace_id: "hcl-contract-red".to_owned(),
                workspace_uuid: "hcl-contract-red".to_owned(),
                branch: branch.to_owned(),
                data_dir,
                path: workspace.path().display().to_string(),
                last_flush: None,
                stale_files: false,
                connection_count: 1,
                file_mtimes: std::collections::HashMap::new(),
            },
            Some(config),
        )
        .await
        .expect("bind isolated contract workspace");

    IndexedFixture {
        _workspace: workspace,
        state,
        index,
    }
}

#[tokio::test]
async fn list_symbols_reports_namespaced_declarations_for_all_hcl_aliases() {
    let fixture = index_mixed_hcl_fixture().await;
    assert_eq!(
        fixture.index.files_parsed, 3,
        "RED:HCL_CONTRACT_INDEX_MISSING expected .tf/.tfvars/.hcl to parse; errors={:?}",
        fixture.index.errors
    );

    let response = tools::dispatch(fixture.state, "list_symbols", Some(json!({})))
        .await
        .expect("list_symbols must serve indexed HCL declarations");
    let names: Vec<&str> = response["symbols"]
        .as_array()
        .expect("symbols must be an array")
        .iter()
        .filter_map(|symbol| symbol["name"].as_str())
        .collect();

    for expected in [
        "hcl.block.terraform",
        "hcl.block.resource.aws_instance.web",
        "hcl.attribute.region",
        "hcl.attribute.replicas",
        "hcl.block.service.api",
    ] {
        assert!(
            names.contains(&expected),
            "HCL list_symbols response is missing {expected}; got {names:?}"
        );
    }
}

#[tokio::test]
async fn map_code_returns_hcl_root_without_fabricated_reference_targets() {
    let fixture = index_mixed_hcl_fixture().await;
    assert!(
        fixture.index.errors.is_empty(),
        "RED:HCL_CONTRACT_MAP_MISSING valid HCL produced index errors: {:?}",
        fixture.index.errors
    );

    let expected_root = "hcl.block.resource.aws_instance.web";
    let response = tools::dispatch(
        fixture.state,
        "map_code",
        Some(json!({ "symbol_name": expected_root })),
    )
    .await
    .expect("map_code must serve the indexed HCL structural symbol");

    assert_eq!(response["root"]["name"], expected_root);
    assert_eq!(response["fallback_used"], false);
    let serialized = serde_json::to_string(&response).expect("serialize map_code response");
    assert!(
        !serialized.contains("\"name\":\"var.region\""),
        "map_code must not fabricate a resolved symbol for the var.region hint: {serialized}"
    );
}

#[tokio::test]
async fn index_result_and_ipc_envelope_keep_valid_hcl_errors_empty_and_bounded() {
    let fixture = index_mixed_hcl_fixture().await;
    assert!(
        fixture.index.errors.is_empty(),
        "RED:HCL_CONTRACT_ERRORS_PRESENT valid HCL must have no per-file errors: {:?}",
        fixture.index.errors
    );
    assert!(
        fixture.index.errors.len() <= 3,
        "HCL error collection must be bounded by the three-file input"
    );

    let payload = serde_json::to_value(&fixture.index).expect("serialize index result");
    let envelope = IpcResponse::success(json!(121_001), payload);
    let line = envelope.to_line().expect("serialize IPC response");
    let decoded: Value = serde_json::from_str(line.trim_end()).expect("decode IPC response");

    assert_eq!(decoded["result"]["files_parsed"], 3);
    assert_eq!(decoded["result"]["errors"], json!([]));
    assert!(decoded["error"].is_null());
}
