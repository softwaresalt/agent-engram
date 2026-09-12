//! Integration coverage for plan unit F25: core read handlers pin one binding
//! at handler entry and keep serving it even if a newer generation is published
//! before the handler finishes.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::services::code_graph;
use engram::tools::read;
use serde_json::{Value, json};
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";
const WORKSPACE_ID: &str = "workspace-core-pin";
const WORKSPACE_UUID: &str = "00000000-0000-0000-0000-000000000034";
const OLD_SYMBOL: &str = "alpha_marker";
const OLD_HELPER: &str = "alpha_helper";
const NEW_SYMBOL: &str = "beta_marker";

struct Fixture {
    _workspace: TempDir,
    _data_a: TempDir,
    _data_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
    alpha_id: String,
    helper_id: String,
}

impl Fixture {
    async fn new() -> Self {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data_a = tempfile::tempdir().expect("data-dir A tempdir");
        let data_b = tempfile::tempdir().expect("data-dir B tempdir");
        let data_dir_a = data_a.path().join(".engram");
        let data_dir_b = data_b.path().join(".engram");

        write_old_workspace(workspace.path());
        code_graph::index_workspace(
            workspace.path(),
            &data_dir_a,
            BRANCH,
            &WorkspaceConfig::default().code_graph,
            false,
        )
        .await
        .expect("index old generation");

        let (alpha_id, helper_id) = symbol_ids(&data_dir_a).await;

        write_new_workspace(workspace.path());
        code_graph::index_workspace(
            workspace.path(),
            &data_dir_b,
            BRANCH,
            &WorkspaceConfig::default().code_graph,
            false,
        )
        .await
        .expect("index new generation");

        let workspace_path = workspace.path().to_string_lossy().into_owned();
        let snap_a = snapshot(&workspace_path, &data_dir_a);
        let snap_b = snapshot(&workspace_path, &data_dir_b);
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state
            .set_workspace_and_config(snap_a.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("seed old generation");

        Self {
            _workspace: workspace,
            _data_a: data_a,
            _data_b: data_b,
            state,
            snap_a,
            snap_b,
            alpha_id,
            helper_id,
        }
    }

    async fn republish_new_generation(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("publish new generation");
    }

    async fn reset_to_old_generation(&self) {
        self.state
            .set_workspace_and_config(self.snap_a.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("rebind old generation");
    }
}

fn snapshot(workspace_path: &str, data_dir: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: WORKSPACE_ID.to_owned(),
        workspace_uuid: WORKSPACE_UUID.to_owned(),
        branch: BRANCH.to_owned(),
        data_dir: data_dir.to_path_buf(),
        path: workspace_path.to_owned(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

fn write_old_workspace(workspace: &Path) {
    fs::write(
        workspace.join("sample.rs"),
        format!(
            "pub fn {OLD_SYMBOL}() {{\n    {OLD_HELPER}();\n}}\n\npub fn {OLD_HELPER}() {{}}\n"
        ),
    )
    .expect("write old workspace");
}

fn write_new_workspace(workspace: &Path) {
    fs::write(
        workspace.join("sample.rs"),
        format!("pub fn {NEW_SYMBOL}() {{}}\n"),
    )
    .expect("write new workspace");
}

async fn symbol_ids(data_dir: &Path) -> (String, String) {
    let db = connect_db(data_dir, BRANCH).await.expect("connect old db");
    let queries = CodeGraphQueries::new(db);
    let alpha = queries
        .find_symbols_by_name(OLD_SYMBOL)
        .await
        .expect("find old root symbol")
        .into_iter()
        .next()
        .expect("old root symbol present");
    let helper = queries
        .find_symbols_by_name(OLD_HELPER)
        .await
        .expect("find old helper symbol")
        .into_iter()
        .next()
        .expect("old helper symbol present");
    (alpha.id, helper.id)
}

async fn invoke_with_mid_request_publication<F, Fut>(
    fixture: &Fixture,
    method: &str,
    invoke: F,
) -> Value
where
    F: FnOnce(Arc<AppState>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Value, engram::errors::EngramError>> + Send + 'static,
{
    fixture.reset_to_old_generation().await;
    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    read::install_generation_pin_test_hook(method, reached_tx, resume_rx);

    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { invoke(state).await });

    reached_rx
        .await
        .expect("handler must reach the generation pin barrier");
    fixture.republish_new_generation().await;
    resume_tx
        .send(())
        .expect("generation pin barrier must still be waiting");

    task.await
        .expect("handler task must join")
        .expect("handler must succeed")
}

fn named_function_body(source: &str, function_name: &str) -> String {
    let needle = format!("pub async fn {function_name}");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("function '{function_name}' not found"));
    let brace_start = source[start..].find('{').map_or_else(
        || panic!("function '{function_name}' has no body"),
        |offset| start + offset,
    );

    let mut depth = 0usize;
    let mut end = None;
    for (offset, ch) in source[brace_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(brace_start + offset + ch.len_utf8());
                    break;
                }
            }
            _ => {}
        }
    }

    source[brace_start..end.expect("balanced braces")].to_owned()
}

#[test]
fn migrated_core_handler_bodies_no_longer_open_or_resnapshot_directly() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("read.rs");
    let source = fs::read_to_string(source_path).expect("read src/tools/read.rs");

    for function_name in [
        "get_workspace_statistics",
        "query_memory",
        "map_code",
        "list_symbols",
        "unified_search",
        "impact_analysis",
        "query_graph",
    ] {
        let body = named_function_body(&source, function_name);
        assert!(
            !body.contains("connect_db("),
            "{function_name} must not open a database directly"
        );
        assert!(
            !body.contains("snapshot_workspace("),
            "{function_name} must not re-read workspace state directly"
        );
        assert!(
            !body.contains("workspace_db("),
            "{function_name} must not derive data_dir/branch from a fresh snapshot"
        );
        assert!(
            !body.contains("workspace_snapshot_path_and_branch("),
            "{function_name} must not re-derive workspace path/branch from AppState"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_flight_core_reads_stay_on_the_old_generation_after_publication() {
    let fixture = Fixture::new().await;

    let list_symbols =
        invoke_with_mid_request_publication(&fixture, "list_symbols", |state| async move {
            read::list_symbols(state, Some(json!({ "name_prefix": "alpha_", "limit": 10 }))).await
        })
        .await;
    let symbol_names: Vec<&str> = list_symbols["symbols"]
        .as_array()
        .expect("symbols array")
        .iter()
        .filter_map(|entry| entry["name"].as_str())
        .collect();
    assert!(symbol_names.contains(&OLD_SYMBOL));
    assert!(!symbol_names.contains(&NEW_SYMBOL));

    let map_code = invoke_with_mid_request_publication(&fixture, "map_code", |state| async move {
        read::map_code(
            state,
            Some(json!({ "symbol_name": OLD_SYMBOL, "depth": 1 })),
        )
        .await
    })
    .await;
    assert_eq!(map_code["root"]["name"], OLD_SYMBOL);
    assert!(!map_code["root"].is_null(), "old root must stay readable");

    let impact =
        invoke_with_mid_request_publication(&fixture, "impact_analysis", |state| async move {
            read::impact_analysis(
                state,
                Some(json!({ "symbol_name": OLD_SYMBOL, "depth": 1 })),
            )
            .await
        })
        .await;
    assert_eq!(impact["symbol"]["name"], OLD_SYMBOL);

    let query_graph = invoke_with_mid_request_publication(&fixture, "query_graph", {
        let alpha_id = fixture.alpha_id.clone();
        let helper_id = fixture.helper_id.clone();
        move |state| {
            let alpha_id = alpha_id.clone();
            let helper_id = helper_id.clone();
            async move {
                read::query_graph(
                    state,
                    Some(json!({
                        "operation": "find_path",
                        "from": alpha_id,
                        "to": helper_id,
                        "max_depth": 2,
                        "edge_types": ["calls"],
                    })),
                )
                .await
            }
        }
    })
    .await;
    assert_eq!(query_graph["found"], true);
    assert_eq!(query_graph["hop_count"], 1);

    let statistics = invoke_with_mid_request_publication(
        &fixture,
        "get_workspace_statistics",
        |state| async move { read::get_workspace_statistics(state, None).await },
    )
    .await;
    assert_eq!(statistics["functions"], 2);
    assert_eq!(statistics["code_files"], 1);
}
