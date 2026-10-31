//! PA5-T4b / 142.084-T: generation-backed unified search, Route (c).
//! All three roster tests propagate the exact Rust task marker before PRE-1
//! publication or PRE-2 activation; the task overrides generic Python markers.
//! The assertions after preflight exercise the real generation at GREEN.

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
use engram::tools::{dispatch, dispatch_with_read_context, read};
use serde_json::{Value, json};
use tempfile::TempDir;

const GENERATION_ID: &str = "cand-pinned-unified-search-1";

struct Fixture {
    _dir: TempDir,
    workspace: PathBuf,
    state: Arc<AppState>,
}

impl Fixture {
    async fn new() -> Result<Self> {
        let (dir, workspace, snapshot) = tokio::task::spawn_blocking(|| -> Result<_> {
            fs::create_dir_all("tmp")?;
            let dir = tempfile::Builder::new()
                .prefix("pinned-unified-search-")
                .tempdir_in("tmp")?;
            let workspace = dir.path().join("work space");
            fs::create_dir_all(workspace.join(".git"))?;
            fs::write(workspace.join(".git/HEAD"), "ref: refs/heads/main\n")?;
            fs::create_dir_all(workspace.join("src"))?;
            fs::write(
                workspace.join("src/lib.rs"),
                "/// Return the alpha value.\npub fn alpha() -> u32 {\n    1\n}\n",
            )?;
            let workspace = fs::canonicalize(workspace)?;
            let snapshot = WorkspaceSnapshot {
                workspace_id: "workspace-pinned-unified-search".to_owned(),
                workspace_uuid: "00000000-0000-0000-0000-000000000084".to_owned(),
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
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state.set_workspace_and_config(snapshot, None).await?;
        Ok(Self {
            _dir: dir,
            workspace,
            state,
        })
    }

    /// No generation is built, published, or activated before this stub call.
    async fn preflight(&self) -> Result<()> {
        let empty =
            read::unified_search_with_context(Arc::clone(&self.state), Some(search_params()), None)
                .await?;
        assert_eq!(empty.get("total_count"), Some(&json!(0)));
        assert!(empty.get("provenance").is_none());
        Ok(())
    }

    async fn publish_and_activate(&self) -> Result<Arc<ReadRequestContext>> {
        let workspace = self.workspace.clone();
        let (layout, store) = tokio::task::spawn_blocking(move || -> Result<_> {
            let layout = read_server_layout(workspace.to_str().context("UTF-8 workspace")?)?;
            let store = open_generation_store(&layout, true)?;
            Ok((layout, store))
        })
        .await??;
        let id = GenerationId::new(GENERATION_ID)?;
        let built =
            build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default()).await?;
        let (layout, store) = tokio::task::spawn_blocking(move || -> Result<_> {
            let sealed = seal_candidate_inventory(built)?;
            publish_sealed_candidate(&layout, &store, sealed, "pinned-unified-search-harness")?;
            Ok((layout, store))
        })
        .await??;
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
        Ok(dispatch_with_read_context(
            Arc::clone(&self.state),
            "unified_search",
            Some(search_params()),
            Some(ctx),
        )
        .await?)
    }
}

fn search_params() -> Value {
    json!({"query": "alpha", "region": "code"})
}

fn assert_generation_matches(value: &Value) -> Result<()> {
    let count = value
        .get("total_count")
        .and_then(Value::as_u64)
        .context("unified search total_count")?;
    assert!(count >= 1, "opened generation must contain alpha matches");
    assert_eq!(
        value.pointer("/provenance/generation_id"),
        Some(&json!(GENERATION_ID))
    );
    Ok(())
}

/// Recursive listing includes directories and database sidecars; no I/O is ignored.
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

/// Scenario 1: dispatch serves indexed symbols and the admitted generation ID.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t4b_pinned_unified_search_serves_opened_generation() -> Result<()> {
    let fixture = Fixture::new().await?;
    fixture
        .preflight()
        .await
        .context("pa5_t4b_pinned_unified_search_serves_opened_generation pre-publication")?;
    let ctx = fixture.publish_and_activate().await?;
    assert_generation_matches(&fixture.pinned(&ctx).await?)?;
    Ok(())
}

/// Scenario 2: the same workspace's unindexed managed DB differs from the generation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t4b_generation_search_differs_from_unindexed_managed_data() -> Result<()> {
    let fixture = Fixture::new().await?;
    fixture
        .preflight()
        .await
        .context("pa5_t4b_generation_search_differs_from_unindexed_managed_data pre-publication")?;
    let ctx = fixture.publish_and_activate().await?;
    let pinned = fixture.pinned(&ctx).await?;
    assert_generation_matches(&pinned)?;
    let managed = dispatch(
        Arc::clone(&fixture.state),
        "unified_search",
        Some(search_params()),
    )
    .await?;
    assert_eq!(managed.get("total_count"), Some(&json!(0)));
    assert!(managed.get("provenance").is_none());
    assert_ne!(pinned.get("total_count"), managed.get("total_count"));
    Ok(())
}

/// Scenario 3: pinned search creates no generation-local databases and is enrolled.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t4b_pinned_unified_search_creates_no_stray_databases() -> Result<()> {
    let fixture = Fixture::new().await?;
    fixture
        .preflight()
        .await
        .context("pa5_t4b_pinned_unified_search_creates_no_stray_databases pre-publication")?;
    let ctx = fixture.publish_and_activate().await?;
    let before = listing(ctx.data_dir()).await?;
    assert_generation_matches(&fixture.pinned(&ctx).await?)?;
    assert_eq!(before, listing(ctx.data_dir()).await?, "search sidecars");
    assert!(
        GENERATION_PINNED_READS.contains(&"unified_search"),
        "unified_search admission glue is required at GREEN"
    );
    Ok(())
}
