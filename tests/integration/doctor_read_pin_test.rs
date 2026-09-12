//! Integration coverage for plan unit F30: doctor reads pin one snapshot and
//! read-server smoke checks stay non-destructive.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools::doctor;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";

struct DoctorFixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_b: WorkspaceSnapshot,
}

impl DoctorFixture {
    async fn new() -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        create_git_workspace(workspace_a.path());
        create_git_workspace(workspace_b.path());

        let snap_a = snapshot("workspace-doctor-a", workspace_a.path(), false);
        let snap_b = snapshot("workspace-doctor-b", workspace_b.path(), true);
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state
            .set_workspace_and_config(snap_a, Some(config(false)))
            .await
            .expect("seed doctor workspace A");

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
            .expect("publish doctor workspace B");
    }
}

fn create_git_workspace(path: &Path) {
    fs::create_dir_all(path.join(".git")).expect("create git dir");
    fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write git HEAD");
    fs::write(path.join("lib.rs"), "pub fn doctor_fixture() {}\n").expect("write source file");
}

fn snapshot(workspace_id: &str, path: &Path, stale_files: bool) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: workspace_id.to_owned(),
        workspace_uuid: format!("uuid-{workspace_id}"),
        branch: BRANCH.to_owned(),
        data_dir: path.join(".engram-data"),
        path: path.to_string_lossy().into_owned(),
        last_flush: None,
        stale_files,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

fn config(retrieval_eval_enabled: bool) -> WorkspaceConfig {
    let mut config = WorkspaceConfig::default();
    config.retrieval_eval.enabled = retrieval_eval_enabled;
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
fn doctor_handler_body_no_longer_resnapshots_workspace_directly() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("doctor.rs");
    let source = fs::read_to_string(source_path).expect("read src/tools/doctor.rs");
    let body = named_function_body(&source, "get_health_report_for_daemon");
    assert!(
        !body.contains("snapshot_workspace("),
        "doctor health handler must not re-read workspace state directly"
    );
    assert!(
        !body.contains("workspace_config().await"),
        "doctor health handler must not re-read config directly"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn doctor_health_report_stays_on_the_old_snapshot_after_publication() {
    let fixture = DoctorFixture::new().await;
    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    doctor::install_generation_pin_test_hook("get_health_report_for_daemon", reached_tx, resume_rx);

    let state = Arc::clone(&fixture.state);
    let task =
        tokio::spawn(async move { doctor::get_health_report_for_daemon(state.as_ref()).await });
    reached_rx
        .await
        .expect("doctor health handler should pin before publication");
    fixture.publish_b().await;
    resume_tx
        .send(())
        .expect("doctor health handler should be waiting");

    let report = task
        .await
        .expect("doctor health handler join")
        .expect("doctor health handler success");
    let workspace_identity = report
        .checks
        .iter()
        .find(|check| check.name == "workspace_identity")
        .expect("workspace_identity check");
    let registry_validity = report
        .checks
        .iter()
        .find(|check| check.name == "registry_validity")
        .expect("registry_validity check");
    assert!(
        workspace_identity
            .message
            .as_deref()
            .unwrap_or_default()
            .contains("workspace-doctor-a"),
        "in-flight doctor health report should stay on workspace A"
    );
    assert_eq!(registry_validity.status.as_str(), "green");
}

#[test]
fn read_server_smoke_workflow_is_non_destructive() {
    let methods = doctor::smoke_request_methods_for_mode(DaemonMode::ReadServer);
    assert_eq!(methods, vec!["get_daemon_status", "get_workspace_status"]);
    assert!(
        !methods
            .iter()
            .any(|method| *method == "set_workspace" || *method == "_shutdown"),
        "read-server smoke workflow must not bind, repair, sync, or shut down the daemon"
    );
}
