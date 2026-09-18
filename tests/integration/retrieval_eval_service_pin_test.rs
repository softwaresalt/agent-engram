//! Integration coverage for plan unit F33: the retrieval-eval report service
//! reads through one caller-pinned request context and stays on that generation
//! even if a newer publication lands mid-request.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::models::retrieval_eval::RetrievalEvalReport;
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::retrieval_eval;
use engram::services::retrieval_eval::eval_dir;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";
const WORKSPACE_ID: &str = "workspace-retrieval-eval-pin";
const WORKSPACE_UUID: &str = "00000000-0000-0000-0000-000000000042";

struct Fixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl Fixture {
    async fn new() -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        seed_report(
            workspace_a.path(),
            &RetrievalEvalReport::empty(false, BRANCH),
        );
        seed_report(
            workspace_b.path(),
            &RetrievalEvalReport::empty(true, BRANCH),
        );

        let snap_a = snapshot(workspace_a.path());
        let snap_b = snapshot(workspace_b.path());
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state
            .set_workspace_and_config(snap_a.clone(), Some(config(false)))
            .await
            .expect("seed old generation");

        Self {
            _workspace_a: workspace_a,
            _workspace_b: workspace_b,
            state,
            snap_a,
            snap_b,
        }
    }

    async fn republish_new_generation(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(config(true)))
            .await
            .expect("publish new generation");
    }
}

fn snapshot(workspace: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: WORKSPACE_ID.to_owned(),
        workspace_uuid: WORKSPACE_UUID.to_owned(),
        branch: BRANCH.to_owned(),
        data_dir: workspace.join(".engram"),
        path: workspace.to_string_lossy().into_owned(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

fn config(enabled: bool) -> WorkspaceConfig {
    let mut config = WorkspaceConfig::default();
    config.retrieval_eval.enabled = enabled;
    config
}

fn seed_report(workspace: &Path, report: &RetrievalEvalReport) {
    let engram_dir = workspace.join(".engram");
    let report_dir = eval_dir(&engram_dir, BRANCH);
    fs::create_dir_all(&report_dir).expect("create retrieval eval dir");
    let report_json = serde_json::to_string_pretty(report).expect("serialize report");
    fs::write(report_dir.join("1.json"), report_json).expect("write retrieval eval report");
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
fn retrieval_eval_report_service_reads_through_the_caller_pinned_context() {
    let service_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("retrieval_eval.rs");
    let service_source =
        fs::read_to_string(service_source_path).expect("read src/services/retrieval_eval.rs");
    let service_body = named_function_body(&service_source, "load_latest_report");
    assert!(
        !service_body.contains("connect_db("),
        "retrieval-eval report service must not open a database directly"
    );
    assert!(
        service_body.contains("context.branch()"),
        "retrieval-eval report service must read the branch from the caller-pinned context"
    );
    assert!(
        !service_body.contains("snapshot_dispatch_context("),
        "retrieval-eval report service must not consult live app state"
    );

    let tool_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("eval.rs");
    let tool_source = fs::read_to_string(tool_source_path).expect("read src/tools/eval.rs");
    let tool_body = named_function_body(&tool_source, "get_retrieval_eval_report");
    assert!(
        tool_body.contains("retrieval_eval::load_latest_report("),
        "get_retrieval_eval_report must delegate report loading to src/services/retrieval_eval.rs"
    );
    assert!(
        !tool_body.contains("retrieval_eval::latest_report("),
        "get_retrieval_eval_report must not bypass the pinned service seam"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn retrieval_eval_report_service_stays_on_the_old_generation_after_publication() {
    let fixture = Fixture::new().await;
    let context = ReadRequestContext::from_managed_state(fixture.state.as_ref())
        .await
        .expect("capture pinned managed retrieval-eval context");
    let engram_dir = fixture.snap_a.data_dir.clone();

    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    retrieval_eval::install_generation_pin_test_hook("load_latest_report", reached_tx, resume_rx);

    let context = Arc::clone(&context);
    let task = tokio::spawn(async move {
        retrieval_eval::load_latest_report(context.as_ref(), &engram_dir, false).await
    });

    reached_rx
        .await
        .expect("retrieval-eval service must reach the generation pin barrier");
    fixture.republish_new_generation().await;
    resume_tx
        .send(())
        .expect("retrieval-eval service must still be waiting");

    let report = task
        .await
        .expect("retrieval-eval service task must join")
        .expect("retrieval-eval service must succeed");

    assert!(!report.enabled);
    assert_eq!(report.branch, BRANCH);
}
