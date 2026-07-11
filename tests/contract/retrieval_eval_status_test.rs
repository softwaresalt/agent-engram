//! Contract tests for retrieval-eval status exposure + persistence (081.006-T).
//!
//! Scenarios:
//! 1. `get_workspace_status.retrieval_eval_enabled` is `true` when configured.
//! 2. `retrieval_eval_enabled` is `false` by default.
//! 3. `run_retrieval_eval` writes a JSON run under `.engram/eval/{branch}/`.
//! 4. `get_retrieval_eval_report` returns the latest persisted run.

use std::fs;
use std::sync::Arc;
use std::time::Duration;

use serde_json::json;
use tokio::test;

use engram::models::config::WorkspaceConfig;
use engram::models::retrieval_eval::RetrievalEvalConfig;
use engram::server::state::AppState;
use engram::tools;

/// Dispatch a tool call, retrying a bounded number of times on the known
/// transient `CozoDB` reopen lock (U015-FLK1 residual, tracked stash `100EACD8`).
///
/// Rapid sequential reopens of the same branch database — as this test does
/// with two back-to-back `run_retrieval_eval` dispatches followed by a report
/// read — can surface a `database is locked` (`SQLITE_BUSY`) transient on
/// platforms where the OS releases the prior connection's file lock lazily
/// (notably Windows). That transient is a pre-existing infrastructure concern,
/// unrelated to retrieval-eval logic, so the test retries rather than treating
/// it as a failure. A durable fix in the shared DB open path is deferred to a
/// separately-tested reliability change (outside this feature's freeze-scope).
async fn dispatch_retry(
    state: Arc<AppState>,
    tool: &str,
    args: Option<serde_json::Value>,
) -> serde_json::Value {
    let mut last_err = String::new();
    for _ in 0u32..8 {
        match tools::dispatch(state.clone(), tool, args.clone()).await {
            Ok(value) => return value,
            Err(err) => {
                let lowered = err.to_string().to_ascii_lowercase();
                if lowered.contains("database is locked") || lowered.contains("sqlite_busy") {
                    last_err = err.to_string();
                    tokio::time::sleep(Duration::from_millis(75)).await;
                    continue;
                }
                panic!("dispatch {tool} failed: {err}");
            }
        }
    }
    panic!("dispatch {tool} failed after retries on transient DB lock: {last_err}");
}

/// Bind a temp workspace with the given config and return the state + tempdir.
async fn setup_workspace(config: WorkspaceConfig) -> (Arc<AppState>, tempfile::TempDir) {
    let workspace = tempfile::tempdir().expect("tempdir");
    let git_dir = workspace.path().join(".git");
    fs::create_dir_all(&git_dir).expect("create .git");
    fs::write(git_dir.join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");

    let state = Arc::new(AppState::new(10));
    let path = workspace.path().to_string_lossy().to_string();

    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace must succeed");

    state.set_workspace_config(Some(config)).await;
    (state, workspace)
}

/// A config with the retrieval-eval subsystem enabled and a distinctive `k`.
fn enabled_config(k: usize) -> WorkspaceConfig {
    WorkspaceConfig {
        retrieval_eval: RetrievalEvalConfig {
            enabled: true,
            k,
            ..RetrievalEvalConfig::default()
        },
        ..WorkspaceConfig::default()
    }
}

// ── Scenario 1: status reflects enabled=true when configured ─────────────────

#[test]
async fn status_reports_enabled_true_when_configured() {
    let (state, _ws) = setup_workspace(enabled_config(10)).await;

    let status = tools::dispatch(state.clone(), "get_workspace_status", Some(json!({})))
        .await
        .expect("get_workspace_status must succeed");

    assert_eq!(
        status["retrieval_eval_enabled"],
        json!(true),
        "status must expose retrieval_eval_enabled=true when configured"
    );
}

// ── Scenario 2: status reflects enabled=false by default ─────────────────────

#[test]
async fn status_reports_enabled_false_by_default() {
    let (state, _ws) = setup_workspace(WorkspaceConfig::default()).await;

    let status = tools::dispatch(state.clone(), "get_workspace_status", Some(json!({})))
        .await
        .expect("get_workspace_status must succeed");

    assert_eq!(
        status["retrieval_eval_enabled"],
        json!(false),
        "default config must report retrieval_eval_enabled=false"
    );
}

// ── Scenario 3: a run writes JSON under .engram/eval/{branch}/ ────────────────

#[test]
async fn run_persists_report_under_eval_branch_dir() {
    let (state, ws) = setup_workspace(enabled_config(10)).await;

    tools::dispatch(state.clone(), "run_retrieval_eval", Some(json!({})))
        .await
        .expect("run_retrieval_eval must succeed");

    let eval_dir = ws.path().join(".engram").join("eval").join("main");
    let runs: Vec<_> = fs::read_dir(&eval_dir)
        .expect("eval/main dir must exist")
        .filter_map(Result::ok)
        .filter(|e| e.path().extension().and_then(|x| x.to_str()) == Some("json"))
        .collect();

    assert!(
        !runs.is_empty(),
        "a run must persist at least one JSON file under {}",
        eval_dir.display()
    );

    // The persisted file must be a well-formed report.
    let contents = fs::read_to_string(runs[0].path()).expect("read run file");
    let parsed: serde_json::Value = serde_json::from_str(&contents).expect("run file is JSON");
    assert_eq!(parsed["enabled"], json!(true), "persisted run is enabled");
    assert_eq!(parsed["branch"], json!("main"), "persisted run branch");
}

// ── Scenario 4: report tool reads the latest persisted run ───────────────────

#[test]
async fn report_reads_latest_persisted_run() {
    // First run with k=5, then a later run with k=9. The report tool must
    // return the most recent run (k=9), not an empty default (k=0).
    let (state, _ws) = setup_workspace(enabled_config(5)).await;

    dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    // Advance config + wall clock so the second run is unambiguously newer.
    tokio::time::sleep(Duration::from_millis(20)).await;
    state.set_workspace_config(Some(enabled_config(9))).await;

    dispatch_retry(state.clone(), "run_retrieval_eval", Some(json!({}))).await;

    let report = dispatch_retry(state.clone(), "get_retrieval_eval_report", Some(json!({}))).await;

    assert_eq!(
        report["k"],
        json!(9),
        "report must reflect the latest persisted run (k=9)"
    );
    assert_eq!(report["enabled"], json!(true));
}
