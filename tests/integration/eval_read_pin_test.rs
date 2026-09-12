//! Integration coverage for plan unit F28: retrieval-eval handlers pin one
//! dispatch snapshot and avoid direct reopen logic in the handler bodies.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::db::workspace::canonicalize_workspace;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::models::retrieval_eval::RetrievalEvalReport;
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::services::retrieval_eval::eval_dir;
use engram::tools::eval;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";

struct EvalFixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_b: WorkspaceSnapshot,
}

impl EvalFixture {
    async fn new() -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        create_git_workspace(workspace_a.path());
        create_git_workspace(workspace_b.path());
        seed_report(
            workspace_a.path(),
            RetrievalEvalReport::empty(false, BRANCH),
        );
        seed_report(workspace_b.path(), RetrievalEvalReport::empty(true, BRANCH));

        let snap_a = snapshot("workspace-eval-a", workspace_a.path());
        let snap_b = snapshot("workspace-eval-b", workspace_b.path());
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
            .expect("seed eval workspace A");

        Self {
            _workspace_a: workspace_a,
            _workspace_b: workspace_b,
            state,
            snap_b,
        }
    }

    async fn publish_b(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(config(true)))
            .await
            .expect("publish eval workspace B");
    }
}

fn create_git_workspace(path: &Path) {
    fs::create_dir_all(path.join(".git")).expect("create git dir");
    fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write git HEAD");
    fs::write(path.join("lib.rs"), "pub fn eval_fixture() {}\n").expect("write source file");
}

fn seed_report(workspace: &Path, report: RetrievalEvalReport) {
    let engram_dir = workspace.join(".engram");
    let report_dir = eval_dir(&engram_dir, BRANCH);
    fs::create_dir_all(&report_dir).expect("create retrieval eval dir");
    let report_json = serde_json::to_string_pretty(&report).expect("serialize report");
    fs::write(report_dir.join("1.json"), report_json).expect("write retrieval eval report");
}

fn snapshot(workspace_id: &str, path: &Path) -> WorkspaceSnapshot {
    let canonical_path = canonicalize_workspace(path.to_string_lossy().as_ref())
        .expect("canonicalize eval workspace");
    WorkspaceSnapshot {
        workspace_id: workspace_id.to_owned(),
        workspace_uuid: format!("uuid-{workspace_id}"),
        branch: BRANCH.to_owned(),
        data_dir: canonical_path.join(".engram-data"),
        path: canonical_path.to_string_lossy().into_owned(),
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

#[test]
fn eval_handler_bodies_no_longer_open_or_resnapshot_directly() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("eval.rs");
    let source = fs::read_to_string(source_path).expect("read src/tools/eval.rs");

    for function_name in ["run_retrieval_eval", "get_retrieval_eval_report"] {
        let body = named_function_body(&source, function_name);
        assert!(
            !body.contains("connect_db("),
            "{function_name} must not open the database directly"
        );
        assert!(
            !body.contains("snapshot_dispatch_context("),
            "{function_name} must not re-read dispatch state directly"
        );
        assert!(
            !body.contains("snapshot_workspace("),
            "{function_name} must not re-read workspace state directly"
        );
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn retrieval_eval_report_stays_on_the_old_snapshot_after_publication() {
    let fixture = EvalFixture::new().await;
    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    eval::install_generation_pin_test_hook("get_retrieval_eval_report", reached_tx, resume_rx);

    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { eval::get_retrieval_eval_report(state, None).await });
    reached_rx
        .await
        .expect("retrieval eval handler should pin before publication");
    fixture.publish_b().await;
    resume_tx
        .send(())
        .expect("retrieval eval handler should be waiting");

    let response = task
        .await
        .expect("retrieval eval handler join")
        .expect("retrieval eval report success");
    assert_eq!(response["enabled"], false);
    assert_eq!(response["branch"], BRANCH);
}
