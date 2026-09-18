//! Integration coverage for plan unit F34: the metrics read services consume one
//! caller-pinned request context and stay on that generation even if a newer
//! publication lands mid-request.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::models::metrics::UsageEvent;
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::metrics;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";
const WORKSPACE_ID: &str = "workspace-metrics-pin";
const WORKSPACE_UUID: &str = "00000000-0000-0000-0000-000000000043";
const OLD_CORRELATION_ID: &str = "old-run";
const NEW_CORRELATION_ID: &str = "new-run";
const OLD_TOOL: &str = "map_code";
const NEW_TOOL: &str = "unified_search";

struct Fixture {
    _workspace: TempDir,
    _data_a: TempDir,
    _data_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl Fixture {
    async fn new() -> Self {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data_a = tempfile::tempdir().expect("data-dir A tempdir");
        let data_b = tempfile::tempdir().expect("data-dir B tempdir");
        let data_dir_a = data_a.path().join(".engram");
        let data_dir_b = data_b.path().join(".engram");
        let workspace_path = workspace.path().to_string_lossy().into_owned();

        seed_usage_events(
            &data_dir_a,
            &[usage_event(OLD_TOOL, OLD_CORRELATION_ID, &workspace_path)],
        );
        seed_usage_events(
            &data_dir_b,
            &[usage_event(NEW_TOOL, NEW_CORRELATION_ID, &workspace_path)],
        );

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

fn usage_event(tool_name: &str, correlation_id: &str, workspace_path: &str) -> UsageEvent {
    UsageEvent {
        tool_name: tool_name.to_owned(),
        timestamp: "2026-09-18T08:00:00Z".to_owned(),
        branch: BRANCH.to_owned(),
        workspace: workspace_path.to_owned(),
        correlation_id: Some(correlation_id.to_owned()),
        response_bytes: 128,
        estimated_output_tokens: 32,
        estimated_tokens: 32,
        result_count: 1,
        results_returned: 1,
        ..UsageEvent::default()
    }
}

fn seed_usage_events(data_dir: &Path, events: &[UsageEvent]) {
    let metrics_dir = data_dir.join("metrics").join(BRANCH);
    fs::create_dir_all(&metrics_dir).expect("create metrics dir");
    let body = events
        .iter()
        .map(|event| serde_json::to_string(event).expect("serialize usage event"))
        .collect::<Vec<_>>()
        .join("\n");
    fs::write(metrics_dir.join("usage.jsonl"), format!("{body}\n")).expect("write usage.jsonl");
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
fn metrics_service_reads_through_the_caller_pinned_context() {
    let service_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("metrics.rs");
    let service_source =
        fs::read_to_string(service_source_path).expect("read src/services/metrics.rs");
    let summary_body = named_function_body(&service_source, "load_summary");
    assert!(
        !summary_body.contains("connect_db("),
        "metrics summary service must not open a database directly"
    );
    assert!(
        summary_body.contains("context.data_dir()"),
        "metrics summary service must read the pinned data directory from its caller"
    );

    let query_stats_body = named_function_body(&service_source, "load_query_statistics");
    assert!(
        !query_stats_body.contains("connect_db("),
        "query-stat service must not open a database directly"
    );
    assert!(
        query_stats_body.contains("context.data_dir()"),
        "query-stat service must read the pinned data directory from its caller"
    );
    assert!(
        !query_stats_body.contains("snapshot_dispatch_context("),
        "query-stat service must not consult live app state"
    );

    let read_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("read.rs");
    let read_source = fs::read_to_string(read_source_path).expect("read src/tools/read.rs");

    let branch_metrics_body = named_function_body(&read_source, "get_branch_metrics");
    assert!(
        branch_metrics_body.contains("metrics::load_summary("),
        "get_branch_metrics must delegate summary loading to src/services/metrics.rs"
    );
    assert!(
        !branch_metrics_body.contains("metrics::compute_summary("),
        "get_branch_metrics must not compute summaries directly in the handler body"
    );

    let token_report_body = named_function_body(&read_source, "get_token_savings_report");
    assert!(
        token_report_body.contains("metrics::load_query_statistics("),
        "get_token_savings_report must delegate query-stat loading to src/services/metrics.rs"
    );
    assert!(
        !token_report_body.contains("metrics::load_events("),
        "get_token_savings_report must not load raw usage events directly in the handler body"
    );

    let evaluation_body = named_function_body(&read_source, "get_evaluation_report");
    assert!(
        evaluation_body.contains("metrics::load_usage_events("),
        "get_evaluation_report must delegate usage-event loading to src/services/metrics.rs"
    );
    assert!(
        !evaluation_body.contains("metrics::load_events("),
        "get_evaluation_report must not bypass the pinned metrics service seam"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn metrics_service_stays_on_the_old_generation_after_publication() {
    let fixture = Fixture::new().await;
    fixture.reset_to_old_generation().await;

    let context = ReadRequestContext::from_managed_state(fixture.state.as_ref())
        .await
        .expect("capture pinned managed metrics context");

    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    metrics::install_generation_pin_test_hook("load_query_statistics", reached_tx, resume_rx);

    let context = Arc::clone(&context);
    let task =
        tokio::spawn(async move { metrics::load_query_statistics(context.as_ref(), None).await });

    reached_rx
        .await
        .expect("metrics service must reach the generation pin barrier");
    fixture.republish_new_generation().await;
    resume_tx
        .send(())
        .expect("metrics service must still be waiting");

    let (summary, by_correlation_id) = task
        .await
        .expect("metrics service task must join")
        .expect("metrics service must succeed");

    assert_eq!(summary.total_tool_calls, 1);
    assert_eq!(summary.distinct_correlation_ids, 1);
    assert!(summary.by_tool.contains_key(OLD_TOOL));
    assert!(!summary.by_tool.contains_key(NEW_TOOL));
    assert!(by_correlation_id.contains_key(OLD_CORRELATION_ID));
    assert!(!by_correlation_id.contains_key(NEW_CORRELATION_ID));
}
