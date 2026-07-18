//! Concurrency regression for `get_workspace_status` workspace+config atomicity
//! (086.004-T).
//!
//! Before the fix the handler reads the workspace snapshot at entry but the
//! `retrieval_eval_enabled` flag from a SEPARATE `workspace_config()` read taken
//! after the `connect_db` round-trip. A concurrent `set_workspace` (which updates
//! the workspace binding and its config in two separate awaits) can complete
//! entirely inside that wide window, so the status response pairs one workspace's
//! `path`/`branch` with a DIFFERENT config's `retrieval_eval_enabled`.
//!
//! This test drives a writer that flips between two self-consistent states —
//! (workspace A, retrieval-eval ENABLED) and (workspace B, retrieval-eval
//! DISABLED) — while a reader repeatedly calls `get_workspace_status`. Every
//! response must be one of the two consistent pairs; a cross pair
//! (A + disabled, or B + enabled) is a torn read.
//!
//! RED (pre-fix): the reader's wide window straddles whole writer flips, so torn
//! pairs appear. GREEN (post-fix): the handler captures workspace + config in a
//! single `snapshot_dispatch_context()` at entry, so the pair is always
//! self-consistent.

#![allow(clippy::doc_markdown)]

use std::collections::HashMap;
use std::future::Future;
use std::path::Path;
use std::pin::Pin;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;

use engram::errors::{EngramError, WorkspaceError};
use engram::models::config::WorkspaceConfig;
use engram::models::retrieval_eval::RetrievalEvalConfig;
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::services::code_graph;
use engram::tools;

#[allow(dead_code)]
type PublishFuture<'a> = Pin<Box<dyn Future<Output = Result<(), WorkspaceError>> + Send + 'a>>;

#[allow(dead_code)]
type ReaderFuture<'a> =
    Pin<Box<dyn Future<Output = Option<(WorkspaceSnapshot, WorkspaceConfig)>> + Send + 'a>>;

// The red phase uses this extension method to model the old two-await writer.
// Once `AppState` has an inherent method with this name, method resolution
// selects the production atomic writer instead.
#[allow(dead_code)]
trait NonAtomicPublishForRedHarness {
    fn set_workspace_and_config(
        &self,
        snapshot: WorkspaceSnapshot,
        config: Option<WorkspaceConfig>,
    ) -> PublishFuture<'_>;
}

impl NonAtomicPublishForRedHarness for AppState {
    fn set_workspace_and_config(
        &self,
        snapshot: WorkspaceSnapshot,
        config: Option<WorkspaceConfig>,
    ) -> PublishFuture<'_> {
        Box::pin(async move {
            self.set_workspace(snapshot).await?;
            tokio::time::sleep(Duration::from_micros(50)).await;
            self.set_workspace_config(config).await;
            Ok(())
        })
    }
}

// The red phase uses this extension method to model the old two-await reader.
// Once `AppState` has an inherent method with this name, method resolution
// selects the production atomic reader instead.
#[allow(dead_code)]
trait NonAtomicReaderForRedHarness {
    fn snapshot_workspace_and_config(&self) -> ReaderFuture<'_>;
}

#[allow(dead_code)]
impl NonAtomicReaderForRedHarness for AppState {
    fn snapshot_workspace_and_config(&self) -> ReaderFuture<'_> {
        Box::pin(async move {
            let snapshot = self.snapshot_workspace().await?;
            tokio::time::sleep(Duration::from_micros(50)).await;
            let config = self.workspace_config().await?;
            Some((snapshot, config))
        })
    }
}

/// Build a workspace snapshot with a distinct `path` but a shared `data_dir`
/// (so the reader's `connect_db` targets one database and stays fast).
fn make_snapshot(id: &str, path: &str, data_dir: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: id.to_owned(),
        workspace_uuid: format!("uuid-{id}"),
        branch: "main".to_owned(),
        data_dir: data_dir.to_path_buf(),
        path: path.to_owned(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

/// A `WorkspaceConfig` whose retrieval-eval subsystem opt-in matches `enabled`.
fn config_with_eval(enabled: bool) -> WorkspaceConfig {
    WorkspaceConfig {
        retrieval_eval: RetrievalEvalConfig {
            enabled,
            ..RetrievalEvalConfig::default()
        },
        ..WorkspaceConfig::default()
    }
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn snapshot_workspace_and_config_never_tears_reader_side_pair() {
    let shared = TempDir::new().expect("shared data-dir tempdir");
    let data_dir = shared.path().join(".engram");
    let dir_a = TempDir::new().expect("workspace A tempdir");
    let dir_b = TempDir::new().expect("workspace B tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();
    let path_b = dir_b.path().to_string_lossy().into_owned();

    let state = Arc::new(AppState::new(10));
    let snap_a = make_snapshot("ws-a", &path_a, &data_dir);
    let snap_b = make_snapshot("ws-b", &path_b, &data_dir);
    let cfg_a = config_with_eval(true);
    let cfg_b = config_with_eval(false);

    state
        .set_workspace_and_config(snap_a.clone(), Some(cfg_a.clone()))
        .await
        .expect("seed bind A");

    let stop = Arc::new(AtomicBool::new(false));
    let writer_state = state.clone();
    let writer_stop = stop.clone();
    let writer = tokio::spawn(async move {
        while !writer_stop.load(Ordering::Relaxed) {
            writer_state
                .set_workspace_and_config(snap_b.clone(), Some(cfg_b.clone()))
                .await
                .expect("bind B with config B");
            tokio::task::yield_now().await;
            writer_state
                .set_workspace_and_config(snap_a.clone(), Some(cfg_a.clone()))
                .await
                .expect("bind A with config A");
            tokio::task::yield_now().await;
        }
    });

    let mut torn_count = 0u32;
    let mut torn_examples: Vec<(String, bool)> = Vec::new();
    let mut observed_a = 0u32;
    let mut observed_b = 0u32;
    let mut iterations = 0u32;
    while (iterations < 1_000 || observed_a == 0 || observed_b == 0) && iterations < 20_000 {
        iterations += 1;
        let Some((snapshot, config)) = state.snapshot_workspace_and_config().await else {
            continue;
        };
        let path = snapshot.path;
        let enabled = config.retrieval_eval.enabled;
        if path == path_a {
            observed_a += 1;
        } else if path == path_b {
            observed_b += 1;
        }
        let torn_pair = (path == path_a && !enabled) || (path == path_b && enabled);
        if torn_pair {
            torn_count += 1;
            if torn_examples.len() < 8 {
                torn_examples.push((path, enabled));
            }
        }
        tokio::task::yield_now().await;
    }

    stop.store(true, Ordering::Relaxed);
    writer.await.expect("writer task must join");

    assert!(
        observed_a > 0 && observed_b > 0,
        "test is vacuous unless BOTH checked states are observed \
         (A={observed_a}, B={observed_b}, iterations={iterations})"
    );
    assert!(
        torn_count == 0,
        "snapshot_workspace_and_config returned {torn_count} torn (workspace, config) pair(s) \
         across {iterations} samples (observed A={observed_a}, B={observed_b}); \
         examples: {torn_examples:?}"
    );
}

#[tokio::test]
async fn snapshot_workspace_and_config_returns_none_when_config_absent() {
    let shared = TempDir::new().expect("shared data-dir tempdir");
    let data_dir = shared.path().join(".engram");
    let dir_a = TempDir::new().expect("workspace A tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();

    let state = AppState::new(10);
    state
        .set_workspace(make_snapshot("ws-a", &path_a, &data_dir))
        .await
        .expect("bind A without config");

    assert!(
        state.snapshot_workspace_and_config().await.is_none(),
        "reader-side background paths must skip when config is absent"
    );
}

// ── 092.004-T: the tool handlers rely on the COMPLEMENT of the test above ─────
//
// index_workspace, sync_workspace, map_code, and impact_analysis migrated their
// paired (workspace, config) reads to a single `snapshot_dispatch_context()`.
// That primitive is the behavior-preserving choice precisely because — unlike
// `snapshot_workspace_and_config()` (which None-gates on absent config, proven
// above) — it default-substitutes the config when a workspace is bound but no
// config has been loaded. If that default-substitution ever regressed (or a
// handler were "simplified" to `snapshot_workspace_and_config`), those handlers
// would wrongly return `WorkspaceError::NotSet` on a config-less-but-bound
// workspace instead of proceeding with `WorkspaceConfig::default()`.
#[tokio::test]
async fn snapshot_dispatch_context_default_substitutes_absent_config() {
    let shared = TempDir::new().expect("shared data-dir tempdir");
    let data_dir = shared.path().join(".engram");
    let dir_a = TempDir::new().expect("workspace A tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();

    let state = AppState::new(10);
    state
        .set_workspace(make_snapshot("ws-a", &path_a, &data_dir))
        .await
        .expect("bind A without config");

    let ctx = state
        .snapshot_dispatch_context()
        .await
        .expect("dispatch context present when workspace bound and config absent");
    assert_eq!(
        ctx.workspace.path, path_a,
        "snapshot must carry the bound workspace"
    );
    assert_eq!(
        ctx.config,
        WorkspaceConfig::default(),
        "absent config must default-substitute, not None-gate — this is what \
         the migrated tool handlers depend on (092.004-T)"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn get_workspace_status_never_tears_workspace_and_config() {
    let shared = TempDir::new().expect("shared data-dir tempdir");
    let data_dir = shared.path().join(".engram");
    let dir_a = TempDir::new().expect("workspace A tempdir");
    let dir_b = TempDir::new().expect("workspace B tempdir");
    let dir_n = TempDir::new().expect("neutral workspace tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();
    let path_b = dir_b.path().to_string_lossy().into_owned();
    let path_n = dir_n.path().to_string_lossy().into_owned();

    let state = Arc::new(AppState::new(10));

    // Consistent pairs: A ⇔ enabled=true, B ⇔ enabled=false. `N` is a neutral
    // binding the reader does not assert on; the writer flips the config ONLY
    // while `N` is bound, so config never changes while a *checked* workspace
    // (A or B) is active. An atomic reader therefore can never observe a checked
    // cross pair — only a reader that tears across its OWN two reads (the pre-fix
    // bug: workspace at entry, config after `connect_db`) can pair checked A/B
    // with the other state's config.
    let snap_a = make_snapshot("ws-a", &path_a, &data_dir);
    let snap_b = make_snapshot("ws-b", &path_b, &data_dir);
    let snap_n = make_snapshot("ws-n", &path_n, &data_dir);
    let cfg_a = config_with_eval(true);
    let cfg_b = config_with_eval(false);

    // Seed an initial consistent state so the very first read is well-defined.
    state.set_workspace(snap_a.clone()).await.expect("bind A");
    state.set_workspace_config(Some(cfg_a.clone())).await;

    let stop = Arc::new(AtomicBool::new(false));

    // Writer: flip between the two consistent states, ALWAYS changing the config
    // while the neutral workspace is bound, and holding each checked state briefly
    // so the consistent state dominates the reader's samples.
    let writer_state = state.clone();
    let writer_stop = stop.clone();
    let writer = tokio::spawn(async move {
        while !writer_stop.load(Ordering::Relaxed) {
            // A -> B, routed through neutral.
            writer_state
                .set_workspace(snap_n.clone())
                .await
                .expect("bind N");
            writer_state.set_workspace_config(Some(cfg_b.clone())).await;
            writer_state
                .set_workspace(snap_b.clone())
                .await
                .expect("bind B");
            tokio::time::sleep(Duration::from_millis(2)).await;
            // B -> A, routed through neutral.
            writer_state
                .set_workspace(snap_n.clone())
                .await
                .expect("bind N");
            writer_state.set_workspace_config(Some(cfg_a.clone())).await;
            writer_state
                .set_workspace(snap_a.clone())
                .await
                .expect("bind A");
            tokio::time::sleep(Duration::from_millis(2)).await;
        }
    });

    // Reader: sample the status and record any torn (path, enabled) pair. The
    // neutral binding is intentionally not asserted on (it carries whichever
    // in-flight config value). Track how often each CHECKED state is observed so
    // the test cannot pass VACUOUSLY if scheduling hides the A/B invariant
    // (Copilot PR#249): sample at least 300 times AND keep going (bounded) until
    // BOTH A and B have actually been observed.
    let mut torn: Vec<(String, bool)> = Vec::new();
    let mut observed_a = 0u32;
    let mut observed_b = 0u32;
    let mut iterations = 0u32;
    while (iterations < 300 || observed_a == 0 || observed_b == 0) && iterations < 5_000 {
        iterations += 1;
        let status = tools::dispatch(state.clone(), "get_workspace_status", Some(json!({})))
            .await
            .expect("get_workspace_status must succeed");
        let path = status["path"].as_str().unwrap_or_default().to_owned();
        let enabled = status["retrieval_eval_enabled"].as_bool().unwrap_or(false);
        if path == path_a {
            observed_a += 1;
        } else if path == path_b {
            observed_b += 1;
        }
        let torn_pair = (path == path_a && !enabled) || (path == path_b && enabled);
        if torn_pair {
            torn.push((path, enabled));
        }
    }

    stop.store(true, Ordering::Relaxed);
    writer.await.expect("writer task must join");

    assert!(
        observed_a > 0 && observed_b > 0,
        "test is vacuous unless BOTH checked states are observed \
         (A={observed_a}, B={observed_b}, iterations={iterations})"
    );
    assert!(
        torn.is_empty(),
        "get_workspace_status returned {} torn (workspace, config) pair(s) across {iterations} \
         samples (observed A={observed_a}, B={observed_b}): {torn:?}",
        torn.len()
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn snapshot_dispatch_context_never_observes_writer_side_torn_publish() {
    let shared = TempDir::new().expect("shared data-dir tempdir");
    let data_dir = shared.path().join(".engram");
    let dir_a = TempDir::new().expect("workspace A tempdir");
    let dir_b = TempDir::new().expect("workspace B tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();
    let path_b = dir_b.path().to_string_lossy().into_owned();

    let state = Arc::new(AppState::new(10));
    let snap_a = make_snapshot("ws-a", &path_a, &data_dir);
    let snap_b = make_snapshot("ws-b", &path_b, &data_dir);
    let cfg_a = config_with_eval(true);
    let cfg_b = config_with_eval(false);

    state
        .set_workspace_and_config(snap_a.clone(), Some(cfg_a.clone()))
        .await
        .expect("seed bind A");

    let stop = Arc::new(AtomicBool::new(false));
    let writer_state = state.clone();
    let writer_stop = stop.clone();
    let writer = tokio::spawn(async move {
        while !writer_stop.load(Ordering::Relaxed) {
            writer_state
                .set_workspace_and_config(snap_b.clone(), Some(cfg_b.clone()))
                .await
                .expect("bind B with config B");
            tokio::task::yield_now().await;
            writer_state
                .set_workspace_and_config(snap_a.clone(), Some(cfg_a.clone()))
                .await
                .expect("bind A with config A");
            tokio::task::yield_now().await;
        }
    });

    let mut torn_count = 0u32;
    let mut torn_examples: Vec<(String, bool)> = Vec::new();
    let mut observed_a = 0u32;
    let mut observed_b = 0u32;
    let mut iterations = 0u32;
    while (iterations < 1_000 || observed_a == 0 || observed_b == 0) && iterations < 20_000 {
        iterations += 1;
        let Some(snapshot) = state.snapshot_dispatch_context().await else {
            continue;
        };
        let path = snapshot.workspace.path;
        let enabled = snapshot.config.retrieval_eval.enabled;
        if path == path_a {
            observed_a += 1;
        } else if path == path_b {
            observed_b += 1;
        }
        let torn_pair = (path == path_a && !enabled) || (path == path_b && enabled);
        if torn_pair {
            torn_count += 1;
            if torn_examples.len() < 8 {
                torn_examples.push((path, enabled));
            }
        }
        tokio::task::yield_now().await;
    }

    stop.store(true, Ordering::Relaxed);
    writer.await.expect("writer task must join");

    assert!(
        observed_a > 0 && observed_b > 0,
        "test is vacuous unless BOTH checked states are observed \
         (A={observed_a}, B={observed_b}, iterations={iterations})"
    );
    assert!(
        torn_count == 0,
        "snapshot_dispatch_context returned {torn_count} torn (workspace, config) pair(s) across \
         {iterations} samples (observed A={observed_a}, B={observed_b}); \
         examples: {torn_examples:?}"
    );
}

/// 092.004-T: a genuine handler-level routing guard that fails if `map_code`
/// bypasses the shared atomicity seam.
///
/// Runs the real `map_code` handler under paired A/B rebinding. Workspace A
/// (its own data-dir, indexed with the marker symbol) is bound with
/// `max_traversal_depth = DEPTH_A`; workspace B (its own data-dir, WITHOUT the
/// marker) with `DEPTH_B`. Each response reveals BOTH the workspace it read
/// (`root` present iff data-dir A) and the config it read (echoed
/// `effective_depth`). A handler that reverted to separate workspace/config
/// reads could observe a torn pair — `root` present with `effective_depth ==
/// DEPTH_B`, or `root` absent with `== DEPTH_A` — which this test rejects. This
/// is the enforceable routing coverage the seam-only guard cannot provide: it
/// exercises the real handler, so it stays honest even if a handler later
/// stopped routing through the seam.
#[allow(clippy::too_many_lines)]
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn map_code_handler_never_observes_torn_pair() {
    const DEPTH_A: usize = 2;
    const DEPTH_B: usize = 9;
    const MARKER: &str = "alpha_marker";

    let dir_a = TempDir::new().expect("workspace A tempdir");
    let dir_b = TempDir::new().expect("workspace B tempdir");
    let data_a = TempDir::new().expect("data-dir A tempdir");
    let data_b = TempDir::new().expect("data-dir B tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();
    let path_b = dir_b.path().to_string_lossy().into_owned();
    let data_dir_a = data_a.path().join(".engram");
    let data_dir_b = data_b.path().join(".engram");

    // Only workspace A's source defines the marker symbol.
    std::fs::write(
        dir_a.path().join("marker.rs"),
        format!("pub fn {MARKER}() {{}}\n"),
    )
    .expect("write A source");
    std::fs::write(dir_b.path().join("other.rs"), "pub fn other_fn() {}\n")
        .expect("write B source");

    // Index each workspace into its own data-dir on branch "main" (matching
    // `make_snapshot`), so `map_code`'s `connect_db` targets the right graph.
    let index_cfg = WorkspaceConfig::default().code_graph;
    code_graph::index_workspace(dir_a.path(), &data_dir_a, "main", &index_cfg, false)
        .await
        .expect("index workspace A");
    code_graph::index_workspace(dir_b.path(), &data_dir_b, "main", &index_cfg, false)
        .await
        .expect("index workspace B");

    let state = Arc::new(AppState::new(10));
    let snap_a = make_snapshot("ws-a", &path_a, &data_dir_a);
    let snap_b = make_snapshot("ws-b", &path_b, &data_dir_b);
    let cfg_a = WorkspaceConfig {
        code_graph: engram::models::config::CodeGraphConfig {
            max_traversal_depth: DEPTH_A,
            ..WorkspaceConfig::default().code_graph
        },
        ..WorkspaceConfig::default()
    };
    let cfg_b = WorkspaceConfig {
        code_graph: engram::models::config::CodeGraphConfig {
            max_traversal_depth: DEPTH_B,
            ..WorkspaceConfig::default().code_graph
        },
        ..WorkspaceConfig::default()
    };

    state
        .set_workspace_and_config(snap_a.clone(), Some(cfg_a.clone()))
        .await
        .expect("seed bind A");

    let stop = Arc::new(AtomicBool::new(false));
    let writer_state = state.clone();
    let writer_stop = stop.clone();
    let writer = tokio::spawn(async move {
        while !writer_stop.load(Ordering::Relaxed) {
            writer_state
                .set_workspace_and_config(snap_b.clone(), Some(cfg_b.clone()))
                .await
                .expect("bind B with config B");
            tokio::task::yield_now().await;
            writer_state
                .set_workspace_and_config(snap_a.clone(), Some(cfg_a.clone()))
                .await
                .expect("bind A with config A");
            tokio::task::yield_now().await;
        }
    });

    let mut torn = 0u32;
    let mut torn_examples: Vec<(bool, usize)> = Vec::new();
    let mut observed_a = 0u32;
    let mut observed_b = 0u32;
    let mut iterations = 0u32;
    // The real handler opens a DB per call, so keep the iteration bound modest
    // while still requiring both bindings to be observed for non-vacuity.
    while (iterations < 100 || observed_a == 0 || observed_b == 0) && iterations < 2_000 {
        iterations += 1;
        let resp = tools::read::map_code(
            state.clone(),
            Some(json!({ "symbol_name": MARKER, "depth": 999 })),
        )
        .await
        .expect("map_code must not error for a bound, indexed workspace");
        let root_present = !resp["root"].is_null();
        let effective_depth = usize::try_from(
            resp["effective_depth"]
                .as_u64()
                .expect("effective_depth is a number"),
        )
        .expect("effective_depth fits usize");
        if root_present {
            observed_a += 1;
        } else {
            observed_b += 1;
        }
        let torn_pair = (root_present && effective_depth == DEPTH_B)
            || (!root_present && effective_depth == DEPTH_A);
        if torn_pair {
            torn += 1;
            if torn_examples.len() < 8 {
                torn_examples.push((root_present, effective_depth));
            }
        }
        tokio::task::yield_now().await;
    }

    stop.store(true, Ordering::Relaxed);
    writer.await.expect("writer task must join");

    assert!(
        observed_a > 0 && observed_b > 0,
        "test is vacuous unless BOTH bindings are exercised \
         (A={observed_a}, B={observed_b}, iterations={iterations})"
    );
    assert!(
        torn == 0,
        "map_code observed {torn} torn (workspace, config) pair(s) across {iterations} \
         samples (root-present-with-DEPTH_B or root-absent-with-DEPTH_A); \
         examples (root_present, effective_depth): {torn_examples:?}"
    );
}

/// 092.004-T: the shared seam maps "no workspace bound" to `NotSet`, preserving
/// each graph handler's pre-migration early-error contract.
#[tokio::test]
async fn graph_handler_seam_errors_not_set_when_unbound() {
    let state = AppState::new(10);
    let err = tools::snapshot_graph_handler_context(&state)
        .await
        .expect_err("unbound workspace must error");
    assert!(
        matches!(err, EngramError::Workspace(WorkspaceError::NotSet)),
        "expected NotSet, got {err:?}"
    );
}

#[tokio::test]
async fn set_workspace_and_config_limit_reached_leaves_state_unchanged() {
    let shared = TempDir::new().expect("shared data-dir tempdir");
    let data_dir = shared.path().join(".engram");
    let dir_a = TempDir::new().expect("workspace A tempdir");
    let dir_b = TempDir::new().expect("workspace B tempdir");
    let path_a = dir_a.path().to_string_lossy().into_owned();
    let path_b = dir_b.path().to_string_lossy().into_owned();

    let state = AppState::new(1);
    let snap_a = make_snapshot("ws-a", &path_a, &data_dir);
    let snap_b = make_snapshot("ws-b", &path_b, &data_dir);

    state
        .set_workspace_and_config(snap_a, Some(config_with_eval(true)))
        .await
        .expect("seed bind A");

    let result = state
        .set_workspace_and_config(snap_b, Some(config_with_eval(false)))
        .await;

    assert!(
        matches!(result, Err(WorkspaceError::LimitReached { limit: 1 })),
        "expected LimitReached, got {result:?}"
    );
    let snapshot = state
        .snapshot_dispatch_context()
        .await
        .expect("workspace remains bound");
    assert_eq!(snapshot.workspace.path, path_a);
    assert!(snapshot.config.retrieval_eval.enabled);
}
