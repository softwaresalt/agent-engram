//! Integration coverage for plan unit PRE-4b (142.065-T): the pinned
//! statistics read serves the opened generation's data, not a provenance
//! label on managed data.
//!
//! Each test first calls `get_workspace_statistics_with_context(state, None,
//! None)` on a managed `AppState` before any publish or activation, which is
//! the managed baseline (scenario 2) and the call that carries this task's
//! `Worker: 142.065-T` placeholder marker while the harness is RED.

#![forbid(unsafe_code)]

use std::collections::BTreeSet;
use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;
use std::time::Duration;

use engram::config::StaleStrategy;
use engram::daemon::startup_activation::ReadServerStartupGate;
use engram::models::config::{CodeGraphConfig, DaemonMode};
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::generations::GenerationActivator;
use engram::services::generations::GenerationId;
use engram::services::generations::candidate_build::{
    build_candidate_generation, open_generation_store, publish_sealed_candidate,
    read_server_layout, seal_candidate_inventory,
};
use engram::tools::{dispatch, dispatch_with_read_context, read};
use serde_json::{Value, json};
use tempfile::TempDir;

const BRANCH: &str = "main";
const GENERATION_ID: &str = "cand-pinned-stats-1";
const PRODUCER: &str = "pinned-statistics-harness";

struct Fixture {
    _workspace_dir: TempDir,
    _managed_dir: TempDir,
    workspace: PathBuf,
    state: Arc<AppState>,
}

impl Fixture {
    /// Managed `AppState` bound to the fixture workspace with an unindexed
    /// managed data directory.
    async fn new() -> Self {
        let workspace_dir = tempfile::tempdir().expect("workspace tempdir");
        let managed_dir = tempfile::tempdir().expect("managed data tempdir");
        let workspace = workspace_dir.path().join("work space");
        fs::create_dir_all(workspace.join(".git")).expect("create .git");
        fs::write(
            workspace.join(".git").join("HEAD"),
            "ref: refs/heads/main\n",
        )
        .expect("write HEAD");
        fs::create_dir_all(workspace.join("src")).expect("create src");
        fs::write(
            workspace.join("src").join("lib.rs"),
            "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    2\n}\n",
        )
        .expect("write lib.rs");

        let snapshot = WorkspaceSnapshot {
            workspace_id: "workspace-pinned-stats".to_owned(),
            workspace_uuid: "00000000-0000-0000-0000-000000000065".to_owned(),
            branch: BRANCH.to_owned(),
            data_dir: managed_dir.path().join(".engram"),
            path: workspace.to_string_lossy().into_owned(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        };
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state
            .set_workspace_and_config(snapshot, None)
            .await
            .expect("bind managed snapshot");

        Self {
            _workspace_dir: workspace_dir,
            _managed_dir: managed_dir,
            workspace,
            state,
        }
    }

    /// Managed baseline through the new entry point with no context.
    async fn managed_baseline(&self) -> Value {
        read::get_workspace_statistics_with_context(Arc::clone(&self.state), None, None)
            .await
            .unwrap_or_else(|error| panic!("managed baseline must succeed: {error}"))
    }

    /// Build, seal, publish (PRE-1/PRE-2) and activate the fixture generation.
    async fn publish_and_activate(&self) -> Arc<ReadRequestContext> {
        let workspace = self.workspace.to_str().expect("utf8 path");
        let layout = read_server_layout(workspace).expect("layout");
        let store = open_generation_store(&layout, true).expect("open store");
        let id = GenerationId::new(GENERATION_ID).expect("id");
        let built = build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default())
            .await
            .expect("build");
        let sealed = seal_candidate_inventory(built).expect("seal");
        publish_sealed_candidate(&layout, &store, sealed, PRODUCER).expect("publish");

        let activator = Arc::new(GenerationActivator::new(
            store,
            &layout.runtime_root,
            layout.expected_identity(),
            Duration::from_secs(60),
        ));
        let gate = ReadServerStartupGate::new(activator);
        gate.socket_bound().await;
        gate.run_initial_activation()
            .await
            .expect("published generation must activate")
    }
}

fn listing(root: &Path) -> BTreeSet<PathBuf> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeSet<PathBuf>) {
        let Ok(entries) = fs::read_dir(dir) else {
            return;
        };
        for entry in entries {
            let path = entry.expect("entry").path();
            out.insert(path.strip_prefix(base).expect("relative").to_path_buf());
            if path.is_dir() {
                walk(base, &path, out);
            }
        }
    }
    let mut out = BTreeSet::new();
    walk(root, root, &mut out);
    out
}

async fn pinned_statistics(fixture: &Fixture, ctx: &Arc<ReadRequestContext>) -> Value {
    dispatch_with_read_context(
        Arc::clone(&fixture.state),
        "get_workspace_statistics",
        None,
        Some(ctx),
    )
    .await
    .unwrap_or_else(|error| panic!("pinned statistics must succeed: {error}"))
}

/// Scenario 1.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_read_serves_the_opened_generation() {
    let fixture = Fixture::new().await;
    let _baseline = fixture.managed_baseline().await;

    let ctx = fixture.publish_and_activate().await;
    let stats = pinned_statistics(&fixture, &ctx).await;

    assert_eq!(stats["code_files"], 1, "{stats}");
    assert_eq!(stats["functions"], 2, "{stats}");
    assert_eq!(
        stats.pointer("/provenance/generation_id"),
        Some(&json!(GENERATION_ID)),
        "{stats}"
    );
}

/// Scenario 2.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_data_differs_from_managed_data_for_the_same_workspace() {
    let fixture = Fixture::new().await;
    let baseline = fixture.managed_baseline().await;
    assert_eq!(baseline["code_files"], 0, "{baseline}");

    let ctx = fixture.publish_and_activate().await;
    let pinned = pinned_statistics(&fixture, &ctx).await;

    let managed = dispatch(Arc::clone(&fixture.state), "get_workspace_statistics", None)
        .await
        .expect("managed dispatch");
    assert_eq!(managed["code_files"], 0, "{managed}");
    assert!(managed.get("provenance").is_none(), "{managed}");
    assert_ne!(pinned["code_files"], managed["code_files"]);
}

/// Scenario 3.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_read_creates_no_stray_databases() {
    let fixture = Fixture::new().await;
    let _baseline = fixture.managed_baseline().await;

    let ctx = fixture.publish_and_activate().await;
    let query = || {
        dispatch(
            Arc::clone(&fixture.state),
            "query_memory",
            Some(json!({ "query": "alpha" })),
        )
    };
    let memory_before = format!("{:?}", query().await);

    let before = listing(ctx.data_dir());
    let _stats = pinned_statistics(&fixture, &ctx).await;
    let after = listing(ctx.data_dir());
    assert_eq!(before, after, "pinned read must not bootstrap a database");

    assert_eq!(memory_before, format!("{:?}", query().await));
}
