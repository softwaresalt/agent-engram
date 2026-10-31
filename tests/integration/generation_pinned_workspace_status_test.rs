//! PA5-T1 / 142.080-T, Route (c): real generation-backed workspace status.
//! Each scenario propagates the stub's `InvalidParams` marker before publication
//! or activation. No daemon is started; PRE-1/PRE-2 build the real index.

#![forbid(unsafe_code)]

use std::collections::{BTreeSet, HashMap};
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use anyhow::{Context, Result};
use engram::config::StaleStrategy;
use engram::daemon::request_entry::GENERATION_PINNED_READS;
use engram::daemon::startup_activation::ReadServerStartupGate;
use engram::models::config::{CodeGraphConfig, DaemonMode};
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::generations::candidate_build::{
    build_candidate_generation, open_generation_store, publish_sealed_candidate,
    read_server_layout, seal_candidate_inventory,
};
use engram::services::generations::{GenerationActivator, GenerationId};
use engram::tools::{dispatch, dispatch_with_read_context, lifecycle};
use serde_json::{Value, json};
use tempfile::TempDir;

const METHOD: &str = "get_workspace_status";
const GENERATION_ID: &str = "cand-pinned-status-1";

struct Fixture {
    _dir: TempDir,
    workspace: PathBuf,
    snapshot: WorkspaceSnapshot,
    state: Arc<AppState>,
}

fn state(mode: DaemonMode) -> Arc<AppState> {
    Arc::new(AppState::with_mode(mode, 4, StaleStrategy::Warn, 20, 60))
}

impl Fixture {
    async fn new() -> Result<Self> {
        let (dir, workspace, snapshot) = tokio::task::spawn_blocking(|| -> Result<_> {
            let dir = tempfile::Builder::new()
                .prefix("pinned-status-")
                .tempdir_in("tmp")?;
            let workspace = dir.path().join("work space");
            fs::create_dir_all(workspace.join(".git"))?;
            fs::write(workspace.join(".git/HEAD"), "ref: refs/heads/main\n")?;
            fs::create_dir_all(workspace.join("src"))?;
            fs::write(
                workspace.join("src/lib.rs"),
                "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    2\n}\n",
            )?;
            let workspace = fs::canonicalize(workspace)?;
            let snapshot = WorkspaceSnapshot {
                workspace_id: "workspace-pinned-status".to_owned(),
                workspace_uuid: "00000000-0000-0000-0000-000000000080".to_owned(),
                branch: "main".to_owned(),
                data_dir: fs::canonicalize(dir.path())?.join("managed/.engram"),
                path: workspace.to_string_lossy().into_owned(),
                last_flush: None,
                stale_files: true,
                connection_count: 0,
                file_mtimes: HashMap::new(),
            };
            Ok((dir, workspace, snapshot))
        })
        .await??;
        let state = state(DaemonMode::Managed);
        state
            .set_workspace_and_config(snapshot.clone(), None)
            .await?;
        Ok(Self {
            _dir: dir,
            workspace,
            snapshot,
            state,
        })
    }

    /// First handler call in every scenario; `?` exposes the exact Rust marker.
    async fn before_publication(&self) -> Result<Value> {
        let status = lifecycle::get_workspace_status_with_context(&self.state, None).await?;
        assert!(
            GENERATION_PINNED_READS.contains(&METHOD),
            "Worker: 142.080-T"
        );
        Ok(serde_json::to_value(status)?)
    }

    async fn publish_and_activate(&self) -> Result<Arc<ReadRequestContext>> {
        let layout = read_server_layout(self.workspace.to_str().context("UTF-8 workspace")?)?;
        let store = open_generation_store(&layout, true)?;
        let id = GenerationId::new(GENERATION_ID)?;
        let built =
            build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default()).await?;
        let sealed = seal_candidate_inventory(built)?;
        publish_sealed_candidate(&layout, &store, sealed, "pinned-status-harness")?;
        let activator = Arc::new(GenerationActivator::new(
            store,
            &layout.runtime_root,
            layout.expected_identity(),
            Duration::from_secs(60),
        ));
        let gate = ReadServerStartupGate::new(activator);
        gate.socket_bound().await;
        Ok(gate.run_initial_activation().await?)
    }

    async fn pinned(&self, ctx: &Arc<ReadRequestContext>) -> Result<Value> {
        Ok(dispatch_with_read_context(Arc::clone(&self.state), METHOD, None, Some(ctx)).await?)
    }
}

/// Recursively list directories and files, including sidecars; never hide I/O errors.
async fn listing(root: &Path) -> Result<BTreeSet<PathBuf>> {
    let root = root.to_path_buf();
    tokio::task::spawn_blocking(move || {
        fn walk(base: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) -> Result<()> {
            for entry in fs::read_dir(dir)? {
                let entry = entry?;
                let path = entry.path();
                out.insert(path.strip_prefix(base)?.to_path_buf());
                if entry.file_type()?.is_dir() {
                    walk(base, &path, out)?;
                }
            }
            Ok(())
        }
        let mut files = BTreeSet::new();
        walk(&root, &root, &mut files)?;
        Ok(files)
    })
    .await?
}

/// Scenario 1 (OD-5 Q3, R-1): only graph counts come from the generation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_counts_preserve_managed_workspace_binding() -> Result<()> {
    let fixture = Fixture::new().await?;
    let baseline = fixture.before_publication().await?;
    let ctx = fixture.publish_and_activate().await?;
    let pinned = fixture.pinned(&ctx).await?;
    assert_eq!(pinned.pointer("/code_graph/code_files"), Some(&json!(1)));
    assert_eq!(pinned.pointer("/code_graph/functions"), Some(&json!(2)));
    assert_eq!(
        pinned.pointer("/provenance/generation_id"),
        Some(&json!(GENERATION_ID))
    );
    assert_eq!(pinned.get("path"), Some(&json!(fixture.snapshot.path)));
    assert_eq!(pinned.get("branch"), Some(&json!(fixture.snapshot.branch)));
    let db_path = fixture.snapshot.data_dir.join("cozo/main/engram.db");
    assert_eq!(
        pinned.get("db_path"),
        Some(&json!(db_path.display().to_string()))
    );
    assert_eq!(pinned.get("stale_files"), Some(&json!(true)));
    for key in [
        "path",
        "branch",
        "db_path",
        "stale_files",
        "scan_status",
        "generation",
    ] {
        assert_eq!(pinned.get(key), baseline.get(key), "managed binding: {key}");
    }
    Ok(())
}

/// Scenario 2: the same workspace has real generation data, not relabeled zeros.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn generation_data_differs_from_unindexed_managed_data() -> Result<()> {
    let fixture = Fixture::new().await?;
    let baseline = fixture.before_publication().await?;
    assert_eq!(baseline.pointer("/code_graph/code_files"), Some(&json!(0)));
    let ctx = fixture.publish_and_activate().await?;
    let pinned = fixture.pinned(&ctx).await?;
    let managed = dispatch(Arc::clone(&fixture.state), METHOD, None).await?;
    assert_eq!(managed.pointer("/code_graph/code_files"), Some(&json!(0)));
    assert!(managed.get("provenance").is_none());
    assert_eq!(pinned.pointer("/code_graph/code_files"), Some(&json!(1)));
    assert_ne!(
        pinned.pointer("/code_graph/code_files"),
        managed.pointer("/code_graph/code_files")
    );
    Ok(())
}

/// Scenario 3 (4628001C, R7.7): no bootstrap or sidecars on either read-server path.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_and_no_context_read_server_reads_create_no_databases() -> Result<()> {
    let fixture = Fixture::new().await?;
    let _baseline = fixture.before_publication().await?;

    // A genuinely ReadServer state, bound to this workspace, with no active generation.
    let read_server = state(DaemonMode::ReadServer);
    let mut snapshot = fixture.snapshot.clone();
    snapshot.data_dir = fixture.workspace.join(".engram");
    read_server.set_workspace_and_config(snapshot, None).await?;
    let before = listing(&fixture.workspace).await?;
    let status = dispatch(Arc::clone(&read_server), METHOD, None).await?;
    assert_eq!(
        status.get("code_graph"),
        Some(&json!({"code_files": 0, "functions": 0, "classes": 0, "interfaces": 0, "edges": 0}))
    );
    assert!(status.get("provenance").is_none());
    assert!(!fixture.workspace.join(".engram/cozo").exists());
    assert_eq!(before, listing(&fixture.workspace).await?);

    let ctx = fixture.publish_and_activate().await?;
    let before = listing(ctx.data_dir()).await?;
    let managed_before = listing(&fixture.snapshot.data_dir).await?;
    let pinned = fixture.pinned(&ctx).await?;
    assert_eq!(pinned.pointer("/code_graph/code_files"), Some(&json!(1)));
    assert_eq!(
        before,
        listing(ctx.data_dir()).await?,
        "no generation sidecars"
    );
    assert_eq!(managed_before, listing(&fixture.snapshot.data_dir).await?);
    assert!(
        GENERATION_PINNED_READS.contains(&METHOD),
        "Worker: 142.080-T"
    );
    Ok(())
}
