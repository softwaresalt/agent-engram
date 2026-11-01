//! PA5-T5 / 142.085-T, Route (c): three generation-backed git-change scenarios.
//! Feature roster tests propagate the exact Rust task marker before publication
//! or activation. Default-build counterparts are structural RED, outside that
//! roster. The task's Rust contract overrides generic per-test Python markers.

#![forbid(unsafe_code)]

use anyhow::Result;
use engram::daemon::request_entry::GENERATION_PINNED_READS;

fn source_contract(scenario: &str) {
    assert!(
        GENERATION_PINNED_READS.contains(&"query_changes"),
        "Worker: 142.085-T {scenario}: query_changes admission entry"
    );
    let source = include_str!("../../src/tools/read.rs").replace("\r\n", "\n");
    let wrapper = "#[cfg(feature = \"git-graph\")]\npub async fn query_changes(";
    let handler = "#[cfg(feature = \"git-graph\")]\npub async fn query_changes_with_context(";
    assert!(
        source.find(wrapper).zip(source.find(handler)).is_some_and(
            |(wrapper_position, handler_position)| wrapper_position < handler_position
        ),
        "Worker: 142.085-T {scenario}: feature-gated handler must follow wrapper"
    );
    let body = source
        .split_once(handler)
        .map_or("", |(_, tail)| tail.split("\n/// ").next().unwrap_or(""));
    assert!(
        body.contains("Option<&Arc<ReadRequestContext>>")
            && body.contains("queries_from_read_context(")
            && !body.contains("connect_db(")
            && !body.contains("from_managed_state("),
        "Worker: 142.085-T {scenario}: opened-generation source contract"
    );
}

#[cfg(feature = "git-graph")]
mod generation {
    use std::collections::{BTreeMap, BTreeSet, HashMap};
    use std::fs;
    use std::path::{Path, PathBuf};
    use std::sync::Arc;
    use std::time::Duration;

    use anyhow::{Context, Result};
    use engram::config::StaleStrategy;
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

    const GENERATION_ID: &str = "cand-pinned-query-changes-1";

    pub(super) struct Fixture {
        _dir: TempDir,
        workspace: PathBuf,
        state: Arc<AppState>,
    }

    impl Fixture {
        pub(super) async fn new() -> Result<Self> {
            let (dir, workspace, snapshot) = tokio::task::spawn_blocking(|| -> Result<_> {
                fs::create_dir_all("tmp")?;
                let dir = tempfile::Builder::new()
                    .prefix("pinned-query-changes-")
                    .tempdir_in("tmp")?;
                let workspace = dir.path().join("work space");
                fs::create_dir_all(workspace.join(".git"))?;
                fs::write(workspace.join(".git/HEAD"), "ref: refs/heads/main\n")?;
                fs::write(workspace.join("tracked.rs"), "pub fn alpha() {}\n")?;
                let workspace = fs::canonicalize(workspace)?;
                let snapshot = WorkspaceSnapshot {
                    workspace_id: "workspace-pinned-query-changes".to_owned(),
                    workspace_uuid: "00000000-0000-0000-0000-000000000085".to_owned(),
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

        /// This task's own stub is called before any build/publish/activation.
        pub(super) async fn preflight(&self, scenario: &str) -> Result<()> {
            let empty =
                read::query_changes_with_context(Arc::clone(&self.state), Some(params()), None)
                    .await
                    .with_context(|| format!("{scenario} pre-publication"))?;
            assert_eq!(empty.get("total"), Some(&json!(0)));
            assert!(empty.get("provenance").is_none());
            Ok(())
        }

        pub(super) async fn activate(&self) -> Result<Arc<ReadRequestContext>> {
            let workspace = self.workspace.clone();
            let (layout, store) = tokio::task::spawn_blocking(move || -> Result<_> {
                let layout = read_server_layout(workspace.to_str().context("UTF-8 workspace")?)?;
                let store = open_generation_store(&layout, true)?;
                Ok((layout, store))
            })
            .await??;
            let id = GenerationId::new(GENERATION_ID)?;
            let built =
                build_candidate_generation(&layout, &store, &id, &CodeGraphConfig::default())
                    .await?;
            let (layout, store) = tokio::task::spawn_blocking(move || -> Result<_> {
                // Seed an indexed commit in the real candidate database, never
                // in managed storage. Close it before inventory hashing.
                let path = built.database_path();
                let db = cozo::DbInstance::new(
                    "sqlite",
                    path.to_str().context("UTF-8 candidate database")?,
                    Default::default(),
                )
                .map_err(|error| anyhow::anyhow!("open candidate: {error}"))?;
                db.run_script(
                    r#"?[id, hash, short_hash, author_name, author_email, timestamp,
                         message, parent_hashes, changes] <- [[
                         "commit-pinned", "aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa",
                         "aaaaaaa", "Harness", "harness@example.invalid",
                         1793318400, "indexed generation commit", [], []]]
                       :put commit_node {id => hash, short_hash, author_name,
                         author_email, timestamp, message, parent_hashes, changes}"#,
                    BTreeMap::default(),
                    cozo::ScriptMutability::Mutable,
                )
                .map_err(|error| anyhow::anyhow!("seed candidate commit: {error}"))?;
                drop(db);
                let sealed = seal_candidate_inventory(built)?;
                publish_sealed_candidate(&layout, &store, sealed, "pinned-query-changes-harness")?;
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

        pub(super) async fn pinned(&self, ctx: &Arc<ReadRequestContext>) -> Result<Value> {
            let value = dispatch_with_read_context(
                Arc::clone(&self.state),
                "query_changes",
                Some(params()),
                Some(ctx),
            )
            .await?;
            assert_eq!(value.get("total"), Some(&json!(1)));
            assert_eq!(
                value.pointer("/provenance/generation_id"),
                Some(&json!(GENERATION_ID))
            );
            assert_eq!(
                value.pointer("/commits/0/message"),
                Some(&json!("indexed generation commit"))
            );
            Ok(value)
        }

        pub(super) async fn managed(&self) -> Result<Value> {
            Ok(dispatch(Arc::clone(&self.state), "query_changes", Some(params())).await?)
        }
    }

    fn params() -> Value {
        json!({"since": "2026-10-29T00:00:00Z", "until": "2026-10-31T00:00:00Z"})
    }

    pub(super) async fn listing(root: &Path) -> Result<BTreeSet<PathBuf>> {
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
}

/// Scenario 1: default source contract or real generation data/provenance.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t5_pinned_query_changes_serves_opened_generation() -> Result<()> {
    #[cfg(feature = "git-graph")]
    {
        let fixture = generation::Fixture::new().await?;
        fixture
            .preflight("pa5_t5_pinned_query_changes_serves_opened_generation")
            .await?;
        let ctx = fixture.activate().await?;
        fixture.pinned(&ctx).await?;
    }
    source_contract("pa5_t5_pinned_query_changes_serves_opened_generation");
    Ok(())
}

/// Scenario 2: indexed generation differs from the same unindexed managed workspace.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t5_generation_changes_differ_from_unindexed_managed_data() -> Result<()> {
    #[cfg(feature = "git-graph")]
    {
        let fixture = generation::Fixture::new().await?;
        fixture
            .preflight("pa5_t5_generation_changes_differ_from_unindexed_managed_data")
            .await?;
        let ctx = fixture.activate().await?;
        let pinned = fixture.pinned(&ctx).await?;
        let managed = fixture.managed().await?;
        assert_eq!(managed.get("total"), Some(&serde_json::json!(0)));
        assert!(managed.get("provenance").is_none());
        assert_ne!(pinned.get("total"), managed.get("total"));
    }
    source_contract("pa5_t5_generation_changes_differ_from_unindexed_managed_data");
    Ok(())
}

/// Scenario 3: pinned reads leave the entire generation data tree unchanged.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn pa5_t5_pinned_query_changes_create_no_stray_databases() -> Result<()> {
    #[cfg(feature = "git-graph")]
    {
        let fixture = generation::Fixture::new().await?;
        fixture
            .preflight("pa5_t5_pinned_query_changes_create_no_stray_databases")
            .await?;
        let ctx = fixture.activate().await?;
        let before = generation::listing(ctx.data_dir()).await?;
        fixture.pinned(&ctx).await?;
        assert_eq!(before, generation::listing(ctx.data_dir()).await?);
    }
    source_contract("pa5_t5_pinned_query_changes_create_no_stray_databases");
    Ok(())
}
