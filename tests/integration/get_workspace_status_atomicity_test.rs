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
use std::path::Path;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, Ordering};
use std::time::Duration;

use serde_json::json;
use tempfile::TempDir;

use engram::models::config::WorkspaceConfig;
use engram::models::retrieval_eval::RetrievalEvalConfig;
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools;

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
    // in-flight config value).
    let mut torn: Vec<(String, bool)> = Vec::new();
    for _ in 0..300u32 {
        let status = tools::dispatch(state.clone(), "get_workspace_status", Some(json!({})))
            .await
            .expect("get_workspace_status must succeed");
        let path = status["path"].as_str().unwrap_or_default().to_owned();
        let enabled = status["retrieval_eval_enabled"].as_bool().unwrap_or(false);
        let torn_pair = (path == path_a && !enabled) || (path == path_b && enabled);
        if torn_pair {
            torn.push((path, enabled));
        }
    }

    stop.store(true, Ordering::Relaxed);
    writer.await.expect("writer task must join");

    assert!(
        torn.is_empty(),
        "get_workspace_status returned {} torn (workspace, config) pair(s): {torn:?}",
        torn.len()
    );
}
