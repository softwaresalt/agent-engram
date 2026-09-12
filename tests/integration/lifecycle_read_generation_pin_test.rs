//! Integration coverage for plan unit F27: lifecycle reads pin one dispatch
//! snapshot and read-server binds only acknowledge the admitted workspace.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::db::workspace::canonicalize_workspace;
use engram::errors::{ActivationError, EngramError, ReadServerRefusalError};
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools::lifecycle;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";

struct LifecycleFixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl LifecycleFixture {
    async fn new(mode: DaemonMode) -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        create_git_workspace(workspace_a.path());
        create_git_workspace(workspace_b.path());

        let snap_a = snapshot("workspace-lifecycle-a", workspace_a.path());
        let snap_b = snapshot("workspace-lifecycle-b", workspace_b.path());
        let state = Arc::new(AppState::with_mode(mode, 4, StaleStrategy::Warn, 20, 60));
        state
            .set_workspace_and_config(snap_a.clone(), Some(config(false)))
            .await
            .expect("seed lifecycle workspace A");

        Self {
            _workspace_a: workspace_a,
            _workspace_b: workspace_b,
            state,
            snap_a,
            snap_b,
        }
    }

    /// Build a fixture whose `AppState` has never had a generation admitted
    /// (no `set_workspace_and_config` call), modeling the read-server startup
    /// window before the trusted activation gate has published a binding.
    fn new_unadmitted(mode: DaemonMode) -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        create_git_workspace(workspace_a.path());
        create_git_workspace(workspace_b.path());

        let snap_a = snapshot("workspace-lifecycle-a", workspace_a.path());
        let snap_b = snapshot("workspace-lifecycle-b", workspace_b.path());
        let state = Arc::new(AppState::with_mode(mode, 4, StaleStrategy::Warn, 20, 60));

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
            .set_workspace_and_config(self.snap_b.clone(), Some(config(true)))
            .await
            .expect("publish lifecycle workspace B");
    }
}

fn create_git_workspace(path: &Path) {
    fs::create_dir_all(path.join(".git")).expect("create git dir");
    fs::write(path.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write git HEAD");
    fs::write(path.join("lib.rs"), "pub fn lifecycle() {}\n").expect("write source file");
}

fn snapshot(workspace_id: &str, path: &Path) -> WorkspaceSnapshot {
    let canonical_path = canonicalize_workspace(path.to_string_lossy().as_ref())
        .expect("canonicalize lifecycle workspace");
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

fn config(retrieval_eval_enabled: bool) -> WorkspaceConfig {
    let mut config = WorkspaceConfig::default();
    config.retrieval_eval.enabled = retrieval_eval_enabled;
    config
}

async fn invoke_workspace_status_after_publication(
    fixture: &LifecycleFixture,
) -> lifecycle::WorkspaceStatus {
    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    lifecycle::install_generation_pin_test_hook("get_workspace_status", reached_tx, resume_rx);

    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { lifecycle::get_workspace_status(state.as_ref()).await });
    reached_rx
        .await
        .expect("workspace status should pin before publication");
    fixture.publish_b().await;
    resume_tx
        .send(())
        .expect("status handler should be waiting");
    task.await
        .expect("workspace status join")
        .expect("workspace status success")
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn workspace_status_stays_on_the_old_snapshot_after_publication() {
    let fixture = LifecycleFixture::new(DaemonMode::Managed).await;
    let status = invoke_workspace_status_after_publication(&fixture).await;
    assert_eq!(status.path, fixture.snap_a.path);
    assert_eq!(status.branch, BRANCH);
    assert!(!status.retrieval_eval_enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_server_same_workspace_bind_is_a_side_effect_free_no_op() {
    let fixture = LifecycleFixture::new(DaemonMode::ReadServer).await;
    let metrics_dir = PathBuf::from(&fixture.snap_a.path)
        .join(".engram")
        .join("metrics")
        .join(BRANCH);
    assert!(
        !metrics_dir.exists(),
        "test precondition: no metrics dir before bind"
    );

    let binding = lifecycle::set_workspace(Arc::clone(&fixture.state), fixture.snap_a.path.clone())
        .await
        .expect("same-workspace read-server bind should succeed");

    assert_eq!(binding.workspace_id, fixture.snap_a.workspace_id);
    assert_eq!(binding.path, fixture.snap_a.path);
    assert!(binding.hydrated);
    assert!(
        !binding.pending_scan,
        "identity-equal read-server bind must not queue hydration or sync"
    );
    assert!(
        !metrics_dir.exists(),
        "no-op bind must not initialize metrics state"
    );

    let active = fixture
        .state
        .snapshot_dispatch_context()
        .await
        .expect("active dispatch context");
    assert_eq!(active.workspace.path, fixture.snap_a.path);
    assert!(!active.config.retrieval_eval.enabled);
}

#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_server_retarget_bind_is_refused_without_side_effects() {
    let fixture = LifecycleFixture::new(DaemonMode::ReadServer).await;
    let requested = fixture.snap_b.path.clone();
    let requested_metrics_dir = PathBuf::from(&requested)
        .join(".engram")
        .join("metrics")
        .join(BRANCH);
    assert!(
        !requested_metrics_dir.exists(),
        "test precondition: no metrics dir before refused bind"
    );

    let error = lifecycle::set_workspace(Arc::clone(&fixture.state), requested.clone())
        .await
        .expect_err("retarget bind should be refused in read-server mode");

    match error {
        EngramError::ReadServerRefusal(ReadServerRefusalError::WorkspaceRetargetRefused {
            requested_workspace,
        }) => assert_eq!(requested_workspace, requested),
        other => panic!("unexpected error: {other}"),
    }

    let active = fixture
        .state
        .snapshot_dispatch_context()
        .await
        .expect("active dispatch context after refusal");
    assert_eq!(active.workspace.path, fixture.snap_a.path);
    assert!(!active.config.retrieval_eval.enabled);
    assert!(
        !requested_metrics_dir.exists(),
        "refused retarget must not initialize metrics or mutate the requested workspace"
    );
}

/// Before the trusted startup activation gate has published any generation
/// (`snapshot_dispatch_context()` returns `None`), a read-server must refuse
/// `set_workspace` rather than falling through to the full write-capable bind
/// path (hydration, config/registry processing, publication). Falling through
/// would let a caller trigger workspace setup during the startup race window,
/// which is exactly the write-control surface `ReadServer` mode exists to
/// refuse.
#[tokio::test(flavor = "multi_thread", worker_threads = 2)]
async fn read_server_bind_before_admission_is_refused_without_side_effects() {
    let fixture = LifecycleFixture::new_unadmitted(DaemonMode::ReadServer);
    assert!(
        fixture.state.snapshot_dispatch_context().await.is_none(),
        "test precondition: no generation admitted yet"
    );
    let requested = fixture.snap_a.path.clone();
    let requested_metrics_dir = PathBuf::from(&requested)
        .join(".engram")
        .join("metrics")
        .join(BRANCH);
    assert!(
        !requested_metrics_dir.exists(),
        "test precondition: no metrics dir before refused bind"
    );

    let error = lifecycle::set_workspace(Arc::clone(&fixture.state), requested)
        .await
        .expect_err("bind before admission should be refused in read-server mode");

    match error {
        EngramError::Activation(ActivationError::GenerationNotYetActivated { .. }) => {}
        other => panic!("unexpected error: {other}"),
    }

    assert!(
        fixture.state.snapshot_dispatch_context().await.is_none(),
        "refused pre-admission bind must not publish a workspace binding"
    );
    assert!(
        !requested_metrics_dir.exists(),
        "refused pre-admission bind must not initialize metrics or mutate the requested workspace"
    );
}
