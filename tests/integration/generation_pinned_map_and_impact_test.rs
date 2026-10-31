//! PA5-T3 / 142.082-T, Route (c): generation-backed map and impact reads.
//! Each scenario propagates the task's Rust `InvalidParams` marker before
//! PRE-1 publication or PRE-2 activation, then retains real GREEN assertions.
//! Task-specific marker disposition overrides the generic Python convention.

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
use engram::errors::{CodeGraphError, EngramError};
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

const GENERATION_ID: &str = "cand-pinned-map-impact-1";

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
                .prefix("pinned-map-impact-")
                .tempdir_in("tmp")?;
            let workspace = dir.path().join("work space");
            fs::create_dir_all(workspace.join(".git"))?;
            fs::write(workspace.join(".git/HEAD"), "ref: refs/heads/main\n")?;
            fs::create_dir_all(workspace.join("src"))?;
            fs::write(
                workspace.join("src/lib.rs"),
                "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    alpha()\n}\n",
            )?;
            let workspace = fs::canonicalize(workspace)?;
            let snapshot = WorkspaceSnapshot {
                workspace_id: "workspace-pinned-map-impact".to_owned(),
                workspace_uuid: "00000000-0000-0000-0000-000000000082".to_owned(),
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
            publish_sealed_candidate(&layout, &store, sealed, "pinned-map-impact-harness")?;
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

    async fn pinned(&self, method: &str, ctx: &Arc<ReadRequestContext>) -> Result<Value> {
        Ok(
            dispatch_with_read_context(Arc::clone(&self.state), method, Some(params()), Some(ctx))
                .await?,
        )
    }
}

fn params() -> Value {
    json!({"symbol_name": "alpha", "depth": 2, "max_nodes": 10})
}

/// Reach the impact stub before publication; GREEN expects a missing managed symbol.
async fn preflight_impact(state: &Arc<AppState>) -> Result<()> {
    match read::impact_analysis_with_context(Arc::clone(state), Some(params()), None).await {
        Err(EngramError::CodeGraph(CodeGraphError::SymbolNotFound { name })) => {
            assert_eq!(name, "alpha");
        }
        Err(error) => return Err(error.into()),
        Ok(_) => anyhow::bail!("unindexed preflight impact unexpectedly succeeded"),
    }
    Ok(())
}

fn assert_provenance(value: &Value) {
    assert_eq!(
        value.pointer("/provenance/generation_id"),
        Some(&json!(GENERATION_ID))
    );
}

fn assert_map(value: &Value) -> Result<()> {
    assert_eq!(value.pointer("/root/name"), Some(&json!("alpha")));
    assert_eq!(value.get("fallback_used"), Some(&json!(false)));
    let root_id = value.pointer("/root/id").context("alpha root ID")?;
    let neighbors = value
        .get("neighbors")
        .and_then(Value::as_array)
        .context("map neighbors")?;
    let beta = neighbors
        .iter()
        .find(|node| node.get("name") == Some(&json!("beta")))
        .context("beta caller in alpha neighborhood")?;
    assert!(
        beta.get("body")
            .and_then(Value::as_str)
            .context("beta body")?
            .contains("alpha()")
    );
    let beta_id = beta.get("id").context("beta ID")?;
    let edges = value
        .get("edges")
        .and_then(Value::as_array)
        .context("map edges")?;
    assert!(
        edges.iter().any(|edge| {
            edge.get("type") == Some(&json!("calls"))
                && edge.get("from") == Some(beta_id)
                && edge.get("to") == Some(root_id)
        }),
        "generation must contain beta -> alpha call edge"
    );
    Ok(())
}

fn assert_impact(value: &Value) -> Result<()> {
    assert_eq!(value.pointer("/symbol/name"), Some(&json!("alpha")));
    assert_eq!(value.get("root_kind"), Some(&json!("code_symbol")));
    let neighbors = value
        .get("code_neighborhood")
        .and_then(Value::as_array)
        .context("impact code neighborhood")?;
    let beta = neighbors
        .iter()
        .find(|node| node.get("name") == Some(&json!("beta")))
        .context("beta caller in impact neighborhood")?;
    assert!(
        beta.get("body")
            .and_then(Value::as_str)
            .context("beta body")?
            .contains("alpha()")
    );
    Ok(())
}

/// Include directories and sidecars, propagating every I/O error.
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

/// Scenario 1: both pinned methods return real graph data and provenance.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t3_pinned_map_and_impact_serve_opened_generation() -> Result<()> {
    let fixture = Fixture::new().await?;
    read::map_code_with_context(Arc::clone(&fixture.state), Some(params()), None).await?;
    let ctx = fixture.publish_and_activate().await?;
    let map = fixture.pinned("map_code", &ctx).await?;
    assert_map(&map)?;
    assert_provenance(&map);
    let impact = fixture.pinned("impact_analysis", &ctx).await?;
    assert_impact(&impact)?;
    assert_provenance(&impact);
    Ok(())
}

/// Scenario 2: unindexed managed data cannot substitute for the generation.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t3_generation_graph_differs_from_unindexed_managed_data() -> Result<()> {
    let fixture = Fixture::new().await?;
    preflight_impact(&fixture.state).await?;
    let ctx = fixture.publish_and_activate().await?;
    let map = fixture.pinned("map_code", &ctx).await?;
    assert_map(&map)?;
    assert_provenance(&map);
    let impact = fixture.pinned("impact_analysis", &ctx).await?;
    assert_impact(&impact)?;
    assert_provenance(&impact);
    let managed = dispatch(Arc::clone(&fixture.state), "map_code", Some(params())).await?;
    assert_eq!(managed.get("root"), Some(&Value::Null));
    assert!(managed.get("provenance").is_none());
    assert_ne!(map.get("root"), managed.get("root"));
    let error = dispatch(
        Arc::clone(&fixture.state),
        "impact_analysis",
        Some(params()),
    )
    .await
    .err()
    .context("unindexed managed impact must return SymbolNotFound")?;
    assert!(matches!(
        &error,
        EngramError::CodeGraph(CodeGraphError::SymbolNotFound { name }) if name == "alpha"
    ));
    let response = serde_json::to_value(error.to_response())?;
    assert_eq!(response.get("code"), Some(&json!(7004)));
    assert!(response.get("provenance").is_none());
    Ok(())
}

/// Scenario 3: admission roster and unchanged generation directory inventory.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t3_pinned_graph_reads_create_no_stray_databases() -> Result<()> {
    let fixture = Fixture::new().await?;
    read::map_code_with_context(Arc::clone(&fixture.state), Some(params()), None).await?;
    preflight_impact(&fixture.state).await?;
    let ctx = fixture.publish_and_activate().await?;
    for method in ["map_code", "impact_analysis"] {
        assert!(
            GENERATION_PINNED_READS.contains(&method),
            "missing pinned admission: {method}"
        );
    }
    let before = listing(ctx.data_dir()).await?;
    let map = fixture.pinned("map_code", &ctx).await?;
    assert_map(&map)?;
    assert_provenance(&map);
    assert_eq!(before, listing(ctx.data_dir()).await?, "map_code sidecars");
    let impact = fixture.pinned("impact_analysis", &ctx).await?;
    assert_impact(&impact)?;
    assert_provenance(&impact);
    assert_eq!(
        before,
        listing(ctx.data_dir()).await?,
        "impact_analysis sidecars"
    );
    Ok(())
}
