//! Integration coverage for plan unit F26: report-family handlers pin one
//! dispatch snapshot and keep serving it when publication changes mid-request.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(feature = "git-graph")]
use std::process::Command;
use std::sync::Arc;

use chrono::{TimeZone, Utc};
use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::models::evaluation::EvaluationConfig;
use engram::models::metrics::UsageEvent;
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools::read;
use serde_json::Value;
#[cfg(feature = "git-graph")]
use serde_json::json;
use tempfile::TempDir;
use tokio::sync::oneshot;

#[cfg(feature = "git-graph")]
use engram::db::connect_db;
#[cfg(feature = "git-graph")]
use engram::db::queries::CodeGraphQueries;
#[cfg(feature = "git-graph")]
use engram::services::code_graph;
#[cfg(feature = "git-graph")]
use engram::services::git_graph;

const BRANCH: &str = "main";

struct MetricsFixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl MetricsFixture {
    async fn new() -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        seed_metrics_workspace(
            workspace_a.path(),
            "workspace-report-a",
            3,
            config_a(),
            "map_code",
        );
        seed_metrics_workspace(
            workspace_b.path(),
            "workspace-report-b",
            1,
            config_b(),
            "query_graph",
        );

        let snap_a = metrics_snapshot("workspace-report-a", workspace_a.path());
        let snap_b = metrics_snapshot("workspace-report-b", workspace_b.path());
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state
            .set_workspace_and_config(snap_a.clone(), Some(config_a()))
            .await
            .expect("seed report workspace A");

        Self {
            _workspace_a: workspace_a,
            _workspace_b: workspace_b,
            state,
            snap_a,
            snap_b,
        }
    }

    async fn publish_b(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(config_b()))
            .await
            .expect("publish report workspace B");
    }

    async fn reset_a(&self) {
        self.state
            .set_workspace_and_config(self.snap_a.clone(), Some(config_a()))
            .await
            .expect("reset report workspace A");
    }
}

#[cfg(feature = "git-graph")]
struct QueryChangesFixture {
    _workspace: TempDir,
    _data_a: TempDir,
    _data_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

#[cfg(feature = "git-graph")]
impl QueryChangesFixture {
    async fn new() -> Self {
        let workspace = tempfile::tempdir().expect("git workspace tempdir");
        let data_a = tempfile::tempdir().expect("git data-dir A tempdir");
        let data_b = tempfile::tempdir().expect("git data-dir B tempdir");
        let data_dir_a = data_a.path().join(".engram");
        let data_dir_b = data_b.path().join(".engram");

        init_git_repo(workspace.path());
        write_file(
            workspace.path().join("tracked.rs"),
            "pub fn alpha_commit() {}\n",
        );
        git_commit(workspace.path(), "old generation");
        index_generation(workspace.path(), &data_dir_a).await;

        write_file(
            workspace.path().join("tracked.rs"),
            "pub fn beta_commit() {}\n",
        );
        git_commit(workspace.path(), "new generation");
        index_generation(workspace.path(), &data_dir_b).await;

        let workspace_path = workspace.path().to_string_lossy().into_owned();
        let snap_a = WorkspaceSnapshot {
            workspace_id: "workspace-query-changes".to_owned(),
            workspace_uuid: "00000000-0000-0000-0000-000000000035".to_owned(),
            branch: BRANCH.to_owned(),
            data_dir: data_dir_a,
            path: workspace_path.clone(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        };
        let snap_b = WorkspaceSnapshot {
            data_dir: data_dir_b,
            ..snap_a.clone()
        };

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
            .expect("seed query_changes workspace A");

        Self {
            _workspace: workspace,
            _data_a: data_a,
            _data_b: data_b,
            state,
            snap_a,
            snap_b,
        }
    }

    async fn publish_b(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("publish query_changes workspace B");
    }

    async fn reset_a(&self) {
        self.state
            .set_workspace_and_config(self.snap_a.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("reset query_changes workspace A");
    }
}

fn metrics_snapshot(workspace_id: &str, workspace: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: workspace_id.to_owned(),
        workspace_uuid: format!("uuid-{workspace_id}"),
        branch: BRANCH.to_owned(),
        data_dir: workspace.join(".engram-data"),
        path: workspace.to_string_lossy().into_owned(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

fn config_a() -> WorkspaceConfig {
    WorkspaceConfig {
        evaluation: EvaluationConfig {
            max_token_ratio: 100.0,
            max_error_rate: 1.0,
            min_tool_diversity: 1,
            ..EvaluationConfig::default()
        },
        ..WorkspaceConfig::default()
    }
}

fn config_b() -> WorkspaceConfig {
    WorkspaceConfig {
        evaluation: EvaluationConfig {
            max_token_ratio: 0.1,
            max_error_rate: 0.0,
            min_tool_diversity: 5,
            ..EvaluationConfig::default()
        },
        ..WorkspaceConfig::default()
    }
}

fn seed_metrics_workspace(
    workspace: &Path,
    workspace_label: &str,
    call_count: usize,
    _config: WorkspaceConfig,
    tool_name: &str,
) {
    let metrics_dir = workspace.join(".engram").join("metrics").join(BRANCH);
    fs::create_dir_all(&metrics_dir).expect("create metrics dir");
    let events = (0..call_count)
        .map(|index| UsageEvent {
            tool_name: tool_name.to_owned(),
            timestamp: Utc
                .timestamp_opt(
                    1_725_000_000 + i64::try_from(index).expect("index fits i64"),
                    0,
                )
                .single()
                .expect("valid timestamp")
                .to_rfc3339(),
            request_bytes: 128,
            estimated_input_tokens: 32,
            response_bytes: 400,
            estimated_output_tokens: 100,
            estimated_tokens: 100,
            result_count: 1,
            response_shape_counts: Default::default(),
            symbols_returned: 1,
            results_returned: 1,
            branch: BRANCH.to_owned(),
            connection_id: None,
            agent_role: Some("assistant".to_owned()),
            outcome: "success".to_owned(),
            prompt_tokens_attributed: None,
            completion_tokens_attributed: None,
            cached_tokens_attributed: None,
            schema_version: engram::models::metrics::USAGE_SCHEMA_VERSION,
            correlation_id: Some(format!("{workspace_label}-{index}")),
            latency_ms: 10,
            workspace: workspace.to_string_lossy().into_owned(),
            params_summary: None,
        })
        .collect::<Vec<_>>();
    let mut lines = events
        .iter()
        .map(|event| serde_json::to_string(event).expect("serialize usage event"))
        .collect::<Vec<_>>()
        .join("\n");
    lines.push('\n');
    fs::write(metrics_dir.join("usage.jsonl"), lines).expect("write usage metrics");
}

fn named_function_body(source: &str, function_name: &str) -> String {
    let needle = format!("pub async fn {function_name}");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("function '{function_name}' not found"));
    let brace_start = source[start..]
        .find('{')
        .map(|offset| start + offset)
        .unwrap_or_else(|| panic!("function '{function_name}' has no body"));
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

async fn invoke_with_publication<F, Fut>(
    state: Arc<AppState>,
    method: &str,
    publish_new: impl std::future::Future<Output = ()>,
    invoke: F,
) -> Value
where
    F: FnOnce(Arc<AppState>) -> Fut + Send + 'static,
    Fut: std::future::Future<Output = Result<Value, engram::errors::EngramError>> + Send + 'static,
{
    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    read::install_generation_pin_test_hook(method, reached_tx, resume_rx);
    let task = tokio::spawn(async move { invoke(state).await });
    reached_rx.await.expect("report handler must reach barrier");
    publish_new.await;
    resume_tx.send(()).expect("barrier must still be waiting");
    task.await
        .expect("report handler task must join")
        .expect("report handler must succeed")
}

#[cfg(feature = "git-graph")]
fn init_git_repo(workspace: &Path) {
    run_git(workspace, ["init"]);
    run_git(workspace, ["config", "user.name", "Copilot"]);
    run_git(
        workspace,
        ["config", "user.email", "copilot@example.invalid"],
    );
}

#[cfg(feature = "git-graph")]
fn git_commit(workspace: &Path, message: &str) {
    run_git(workspace, ["add", "tracked.rs"]);
    run_git(workspace, ["commit", "-m", message]);
}

#[cfg(feature = "git-graph")]
fn run_git<const N: usize>(workspace: &Path, args: [&str; N]) {
    let status = Command::new("git")
        .arg("-C")
        .arg(workspace)
        .args(args)
        .status()
        .expect("spawn git");
    assert!(status.success(), "git command failed: {:?}", args);
}

#[cfg(feature = "git-graph")]
fn write_file(path: PathBuf, contents: &str) {
    fs::write(path, contents).expect("write git fixture file");
}

#[cfg(feature = "git-graph")]
async fn index_generation(workspace: &Path, data_dir: &Path) {
    code_graph::index_workspace(
        workspace,
        data_dir,
        BRANCH,
        &WorkspaceConfig::default().code_graph,
        false,
    )
    .await
    .expect("index code graph");
    let db = connect_db(data_dir, BRANCH)
        .await
        .expect("connect git-graph db");
    let queries = CodeGraphQueries::new(db);
    git_graph::index_git_history(&queries, workspace, 100, true)
        .await
        .expect("index git history");
}

#[test]
fn migrated_report_handler_bodies_no_longer_open_or_resnapshot_directly() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("read.rs");
    let source = fs::read_to_string(source_path).expect("read src/tools/read.rs");

    for function_name in [
        "get_health_report",
        "get_branch_metrics",
        "get_token_savings_report",
        "get_evaluation_report",
        "get_mutable_script_retry_metrics",
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
            !body.contains("workspace_snapshot_path_and_branch("),
            "{function_name} must not re-derive workspace path/branch from AppState"
        );
    }

    assert!(
        source.contains("#[cfg(feature = \"git-graph\")]\npub async fn query_changes"),
        "query_changes must stay feature-gated behind git-graph"
    );

    #[cfg(feature = "git-graph")]
    {
        let body = named_function_body(&source, "query_changes");
        assert!(
            !body.contains("connect_db("),
            "query_changes must not open a database directly"
        );
        assert!(
            !body.contains("snapshot_workspace("),
            "query_changes must not re-read workspace state directly"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn in_flight_report_reads_stay_on_the_old_workspace_after_publication() {
    let fixture = MetricsFixture::new().await;

    fixture.reset_a().await;
    let health = invoke_with_publication(
        Arc::clone(&fixture.state),
        "get_health_report",
        fixture.publish_b(),
        |state| async move { read::get_health_report(state, None).await },
    )
    .await;
    assert_eq!(health["workspace_id"], "workspace-report-a");
    assert_eq!(health["metrics_summary"]["summary"]["total_tool_calls"], 3);

    fixture.reset_a().await;
    let branch_metrics = invoke_with_publication(
        Arc::clone(&fixture.state),
        "get_branch_metrics",
        fixture.publish_b(),
        |state| async move { read::get_branch_metrics(state, None).await },
    )
    .await;
    assert_eq!(branch_metrics["summary"]["total_tool_calls"], 3);

    fixture.reset_a().await;
    let token_report = invoke_with_publication(
        Arc::clone(&fixture.state),
        "get_token_savings_report",
        fixture.publish_b(),
        |state| async move { read::get_token_savings_report(state, None).await },
    )
    .await;
    assert_eq!(token_report["metrics"]["total_tool_calls"], 3);

    fixture.reset_a().await;
    let evaluation = invoke_with_publication(
        Arc::clone(&fixture.state),
        "get_evaluation_report",
        fixture.publish_b(),
        |state| async move { read::get_evaluation_report(state, None).await },
    )
    .await;
    let anomalies = evaluation["anomalies"].as_array().expect("anomalies array");
    let recommendations = evaluation["recommendations"]
        .as_array()
        .expect("recommendations array");
    assert!(
        anomalies.is_empty(),
        "A config should not flag anomalies: {evaluation}"
    );
    assert!(
        recommendations.is_empty(),
        "A config should not emit recommendations: {evaluation}"
    );

    let retry_metrics = read::get_mutable_script_retry_metrics(Arc::clone(&fixture.state), None)
        .await
        .expect("retry metrics must succeed");
    assert!(retry_metrics["retry_count"].as_u64().is_some());
}

#[cfg(feature = "git-graph")]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn query_changes_keeps_the_old_generation_after_publication() {
    let fixture = QueryChangesFixture::new().await;
    fixture.reset_a().await;

    let response = invoke_with_publication(
        Arc::clone(&fixture.state),
        "query_changes",
        fixture.publish_b(),
        |state| async move {
            read::query_changes(
                state,
                Some(json!({ "file_path": "tracked.rs", "limit": 5 })),
            )
            .await
        },
    )
    .await;

    let commits = response["commits"].as_array().expect("commits array");
    assert_eq!(
        commits.len(),
        1,
        "old generation has exactly one commit indexed"
    );
    assert_eq!(response["total"], 1);
    assert!(
        commits[0]["message"]
            .as_str()
            .expect("commit message")
            .contains("old generation"),
        "old generation commit must remain visible after publication: {response}"
    );
}
