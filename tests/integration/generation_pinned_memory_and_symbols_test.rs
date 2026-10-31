//! PA5-T2 / 142.081-T, Route (c): generation-backed memory and symbol reads.
//! Each roster test propagates its own task stub's Rust `InvalidParams` marker
//! before PRE-1 publication or PRE-2 activation. No daemon is started.

#![forbid(unsafe_code)]

use std::collections::{BTreeMap, BTreeSet, HashMap};
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

const GENERATION_ID: &str = "cand-pinned-memory-symbols-1";

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
                .prefix("pinned-memory-symbols-")
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
                workspace_id: "workspace-pinned-memory-symbols".to_owned(),
                workspace_uuid: "00000000-0000-0000-0000-000000000081".to_owned(),
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
            // Add searchable content to the real indexed candidate BEFORE sealing.
            // query_memory searches content rather than alpha/beta code symbols.
            let database = built.database_path();
            let db = cozo::DbInstance::new(
                "sqlite",
                database.to_str().context("UTF-8 candidate database")?,
                Default::default(),
            )
            .map_err(anyhow::Error::msg)?;
            db.run_script(
                r#"
                ?[file_path, content_type, content_hash, content, source_path, ingested_at] <-
                  [["backlog/alpha.md", "backlog", "fixture-alpha",
                    "alpha generation memory", "backlog", "2026-10-31T00:00:00Z"]]
                :put backlog_content_record {
                    file_path => content_type, content_hash, content, source_path, ingested_at
                }
                "#,
                BTreeMap::new(),
                cozo::ScriptMutability::Mutable,
            )
            .map_err(anyhow::Error::msg)?;
            drop(db);
            let sealed = seal_candidate_inventory(built)?;
            publish_sealed_candidate(&layout, &store, sealed, "pinned-memory-symbols-harness")?;
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
        Ok(dispatch_with_read_context(
            Arc::clone(&self.state),
            method,
            Some(params(method)),
            Some(ctx),
        )
        .await?)
    }
}

fn params(method: &str) -> Value {
    if method == "query_memory" {
        json!({"query": "alpha", "limit": 10})
    } else {
        json!({})
    }
}

fn assert_provenance(value: &Value) {
    assert_eq!(
        value.pointer("/provenance/generation_id"),
        Some(&json!(GENERATION_ID))
    );
}

fn assert_symbols(value: &Value) -> Result<()> {
    assert_eq!(value.get("total_count"), Some(&json!(2)));
    let names = value
        .get("symbols")
        .and_then(Value::as_array)
        .context("symbols array")?
        .iter()
        .map(|symbol| symbol.get("name").and_then(Value::as_str))
        .collect::<BTreeSet<_>>();
    assert_eq!(names, BTreeSet::from([Some("alpha"), Some("beta")]));
    Ok(())
}

fn assert_memory(value: &Value) -> Result<()> {
    let results = value
        .get("results")
        .and_then(Value::as_array)
        .context("memory results array")?;
    assert_eq!(results.len(), 1);
    let hit = results.first().context("generation memory hit")?;
    assert_eq!(hit.get("file_path"), Some(&json!("backlog/alpha.md")));
    assert_eq!(hit.get("content"), Some(&json!("alpha generation memory")));
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

/// Scenario 1: real alpha/beta generation data and provenance for both methods.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_memory_and_symbols_serve_opened_generation() -> Result<()> {
    let fixture = Fixture::new().await?;
    read::query_memory_with_context(
        Arc::clone(&fixture.state),
        Some(params("query_memory")),
        None,
    )
    .await?;
    let ctx = fixture.publish_and_activate().await?;
    let symbols = fixture.pinned("list_symbols", &ctx).await?;
    assert_symbols(&symbols)?;
    assert_provenance(&symbols);
    let memory = fixture.pinned("query_memory", &ctx).await?;
    assert_provenance(&memory);
    assert_memory(&memory)?;
    Ok(())
}

/// Scenario 2: pinned symbols cannot be relabeled empty managed data.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn generation_data_differs_from_unindexed_managed_data() -> Result<()> {
    let fixture = Fixture::new().await?;
    read::list_symbols_with_context(Arc::clone(&fixture.state), Some(json!({})), None).await?;
    let ctx = fixture.publish_and_activate().await?;
    let pinned = fixture.pinned("list_symbols", &ctx).await?;
    assert_symbols(&pinned)?;
    assert_provenance(&pinned);
    let managed = dispatch(Arc::clone(&fixture.state), "list_symbols", Some(json!({}))).await?;
    assert_eq!(managed.get("total_count"), Some(&json!(0)));
    assert!(managed.get("provenance").is_none());
    assert_ne!(pinned.get("total_count"), managed.get("total_count"));
    let memory = dispatch(
        Arc::clone(&fixture.state),
        "query_memory",
        Some(params("query_memory")),
    )
    .await?;
    assert!(memory.get("provenance").is_none());
    assert_eq!(memory.get("results"), Some(&json!([])));
    let pinned_memory = fixture.pinned("query_memory", &ctx).await?;
    assert_memory(&pinned_memory)?;
    assert_provenance(&pinned_memory);
    assert_ne!(pinned_memory.get("results"), memory.get("results"));
    Ok(())
}

/// Scenario 3: both methods are admitted and neither creates generation sidecars.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pinned_reads_create_no_stray_databases() -> Result<()> {
    let fixture = Fixture::new().await?;
    read::query_memory_with_context(
        Arc::clone(&fixture.state),
        Some(params("query_memory")),
        None,
    )
    .await?;
    for method in ["query_memory", "list_symbols"] {
        assert!(
            GENERATION_PINNED_READS.contains(&method),
            "Worker: 142.081-T: missing pinned admission for {method}"
        );
    }
    let ctx = fixture.publish_and_activate().await?;
    let before = listing(ctx.data_dir()).await?;
    for method in ["query_memory", "list_symbols"] {
        let value = fixture.pinned(method, &ctx).await?;
        assert_provenance(&value);
        if method == "list_symbols" {
            assert_symbols(&value)?;
        } else {
            assert_memory(&value)?;
        }
        assert_eq!(before, listing(ctx.data_dir()).await?, "{method}: sidecars");
    }
    Ok(())
}
