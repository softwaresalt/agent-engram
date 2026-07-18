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

/// 092.004-T: the four graph tool handlers (`index_workspace`, `sync_workspace`,
/// `map_code`, `impact_analysis`) all acquire their `(workspace, config)` pair
/// through `tools::snapshot_graph_handler_context`. This stresses that shared
/// seam directly under paired A/A <-> B/B rebinding; every observed pair must be
/// self-consistent, mirroring the daemon seam guard from 092.003-T. Pinning the
/// seam's atomicity keeps the single choke point the handlers share honest.
#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn graph_handler_seam_never_observes_torn_pair() {
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
        let snapshot = tools::snapshot_graph_handler_context(&state)
            .await
            .expect("workspace stays bound throughout the stress loop");
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
        "snapshot_graph_handler_context returned {torn_count} torn (workspace, config) pair(s) \
         across {iterations} samples (observed A={observed_a}, B={observed_b}); \
         examples: {torn_examples:?}"
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
