//! PA5-T4a / 142.083-T, Route (c): generation-backed `query_graph` reads.
//! Each roster scenario propagates the task's Rust `InvalidParams` marker before
//! PRE-1 publication or PRE-2 activation. Real assertions remain for GREEN.
//! The task's exact marker contract overrides the generic Python convention.

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

const GENERATION_ID: &str = "cand-pinned-query-graph-1";

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
                .prefix("pinned-query-graph-")
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
                workspace_id: "workspace-pinned-query-graph".to_owned(),
                workspace_uuid: "00000000-0000-0000-0000-000000000083".to_owned(),
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

    /// Direct task-stub call: no publication/activation can precede RED.
    async fn preflight(&self) -> Result<()> {
        let empty = read::query_graph_with_context(
            Arc::clone(&self.state),
            Some(neighborhood("unindexed-preflight")),
            None,
        )
        .await?;
        assert_eq!(empty.get("node_count"), Some(&json!(0)));
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
            publish_sealed_candidate(&layout, &store, sealed, "pinned-query-graph-harness")?;
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

    async fn pinned(&self, ctx: &Arc<ReadRequestContext>, params: Value) -> Result<Value> {
        Ok(dispatch_with_read_context(
            Arc::clone(&self.state),
            "query_graph",
            Some(params),
            Some(ctx),
        )
        .await?)
    }

    /// Pinned `map_code` exposes IDs; `list_symbols` exposes names but no IDs.
    async fn symbol_ids(&self, ctx: &Arc<ReadRequestContext>) -> Result<(String, String)> {
        let response = dispatch_with_read_context(
            Arc::clone(&self.state),
            "map_code",
            Some(json!({"symbol_name": "alpha", "depth": 2, "max_nodes": 10})),
            Some(ctx),
        )
        .await?;
        assert_provenance(&response);
        assert_eq!(response.pointer("/root/name"), Some(&json!("alpha")));
        let alpha = response
            .pointer("/root/id")
            .and_then(Value::as_str)
            .context("generation symbol ID for alpha")?;
        let symbols = response
            .get("neighbors")
            .and_then(Value::as_array)
            .context("generation map neighbors")?;
        let id = |name: &str| -> Result<String> {
            Ok(symbols
                .iter()
                .find(|symbol| symbol.get("name") == Some(&json!(name)))
                .and_then(|symbol| symbol.get("id"))
                .and_then(Value::as_str)
                .with_context(|| format!("generation symbol ID for {name}"))?
                .to_owned())
        };
        Ok((alpha.to_owned(), id("beta")?))
    }
}

fn neighborhood(root: &str) -> Value {
    json!({
        "operation": "neighborhood", "root": root, "direction": "outgoing",
        "max_depth": 2, "max_nodes": 10, "edge_types": ["calls"]
    })
}

fn find_path(beta: &str, alpha: &str) -> Value {
    json!({
        "operation": "find_path", "from": beta, "to": alpha,
        "max_depth": 2, "edge_types": ["calls"]
    })
}

fn assert_provenance(value: &Value) {
    assert_eq!(
        value.pointer("/provenance/generation_id"),
        Some(&json!(GENERATION_ID))
    );
}

fn assert_graph(value: &Value, alpha: &str, beta: &str) -> Result<()> {
    assert_eq!(value.get("operation"), Some(&json!("neighborhood")));
    assert_eq!(value.get("root"), Some(&json!(beta)));
    // Neighborhood nodes exclude the separately reported root.
    assert_eq!(value.get("node_count"), Some(&json!(1)));
    let nodes = value
        .get("nodes")
        .and_then(Value::as_array)
        .context("graph nodes")?;
    assert_eq!(nodes.len(), 1);
    assert!(nodes.iter().any(|node| {
        node.get("name") == Some(&json!("alpha")) && node.get("id") == Some(&json!(alpha))
    }));
    let edges = value
        .get("edges")
        .and_then(Value::as_array)
        .context("graph edges")?;
    assert!(edges.iter().any(|edge| {
        edge.get("edge_type") == Some(&json!("calls"))
            && edge.get("from") == Some(&json!(beta))
            && edge.get("to") == Some(&json!(alpha))
    }));
    assert_provenance(value);
    Ok(())
}

fn assert_path(value: &Value, alpha: &str, beta: &str) {
    assert_eq!(value.get("operation"), Some(&json!("find_path")));
    assert_eq!(value.get("found"), Some(&json!(true)));
    assert_eq!(value.get("path"), Some(&json!([beta, alpha])));
    assert_eq!(value.get("hop_count"), Some(&json!(1)));
    assert_provenance(value);
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

/// Scenario 1: neighborhood and path serve real generation nodes/provenance.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t4a_pinned_query_graph_serves_opened_generation() -> Result<()> {
    let fixture = Fixture::new().await?;
    fixture.preflight().await?;
    let ctx = fixture.publish_and_activate().await?;
    let (alpha, beta) = fixture.symbol_ids(&ctx).await?;
    assert_graph(
        &fixture.pinned(&ctx, neighborhood(&beta)).await?,
        &alpha,
        &beta,
    )?;
    assert_path(
        &fixture.pinned(&ctx, find_path(&beta, &alpha)).await?,
        &alpha,
        &beta,
    );
    Ok(())
}

/// Scenario 2: managed DB for the same workspace has no indexed graph or provenance.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t4a_generation_graph_differs_from_unindexed_managed_data() -> Result<()> {
    let fixture = Fixture::new().await?;
    fixture.preflight().await?;
    let ctx = fixture.publish_and_activate().await?;
    let (alpha, beta) = fixture.symbol_ids(&ctx).await?;
    let pinned = fixture.pinned(&ctx, neighborhood(&beta)).await?;
    assert_graph(&pinned, &alpha, &beta)?;
    let managed = dispatch(
        Arc::clone(&fixture.state),
        "query_graph",
        Some(neighborhood(&beta)),
    )
    .await?;
    assert_eq!(managed.get("node_count"), Some(&json!(0)));
    assert!(managed.get("provenance").is_none());
    assert_ne!(pinned.get("node_count"), managed.get("node_count"));
    assert_path(
        &fixture.pinned(&ctx, find_path(&beta, &alpha)).await?,
        &alpha,
        &beta,
    );
    let managed_path = dispatch(
        Arc::clone(&fixture.state),
        "query_graph",
        Some(find_path(&beta, &alpha)),
    )
    .await?;
    assert_eq!(managed_path.get("found"), Some(&json!(false)));
    assert!(managed_path.get("provenance").is_none());
    Ok(())
}

/// Scenario 3: neither operation creates generation-local databases or sidecars.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t4a_pinned_query_graph_creates_no_stray_databases() -> Result<()> {
    let fixture = Fixture::new().await?;
    fixture.preflight().await?;
    let ctx = fixture.publish_and_activate().await?;
    let (alpha, beta) = fixture.symbol_ids(&ctx).await?;
    let before = listing(ctx.data_dir()).await?;
    assert_graph(
        &fixture.pinned(&ctx, neighborhood(&beta)).await?,
        &alpha,
        &beta,
    )?;
    assert_eq!(
        before,
        listing(ctx.data_dir()).await?,
        "neighborhood sidecars"
    );
    assert_path(
        &fixture.pinned(&ctx, find_path(&beta, &alpha)).await?,
        &alpha,
        &beta,
    );
    assert_eq!(before, listing(ctx.data_dir()).await?, "find_path sidecars");
    Ok(())
}

/// Structural assertion reaches no stub: explicitly outside expected-RED roster.
#[test]
fn query_graph_is_enrolled_for_generation_pinning() {
    assert!(
        GENERATION_PINNED_READS.contains(&"query_graph"),
        "query_graph admission glue is required at GREEN"
    );
}
