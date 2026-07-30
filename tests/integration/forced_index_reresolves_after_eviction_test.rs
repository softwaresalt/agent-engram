//! R3 (105.003-T / 7A317008): forced-index reconciliation ORDERING harness.
//!
//! The forced-index route runs the cross-file singleton post-pass
//! (`reresolve_calls_edges_with_canonical_context`) and, separately, evicts
//! every `indexed − discovered` file (103.002-T). Pre-R3 the eviction sits in
//! the generation-marker certify block AFTER the post-pass, so when an
//! excluded-but-still-indexed file duplicates a callee name that has exactly
//! ONE live definition elsewhere, the post-pass resolves against the STALE
//! pre-eviction set, sees ambiguity, and withholds the recoverable cross-file
//! singleton. Eviction then removes the duplicate but never re-runs resolution,
//! so the direct edge stays missing.
//!
//! R3 moves the `force || !any_hash_skipped`-gated eviction AHEAD of the
//! post-pass so it resolves against the POST-eviction set. This is a recall
//! RECOVERY (adds an edge that was previously withheld); it is fail-closed and
//! never emits a false edge, and the dangling-edge sweep + generation marker
//! still certify AFTER the post-pass (103-F order preserved).
//!
//! RED pre-R3: `forced_index_recovers_singleton_after_duplicate_callee_excluded`
//! finds no `call_it -> shared_target` singleton (post-pass saw pre-eviction
//! ambiguity). GREEN post-R3: the singleton resolves against the post-eviction
//! set.

#![allow(clippy::doc_markdown)]

use std::collections::HashMap;
use std::fs;
use std::path::Path;

use sha2::{Digest, Sha256};
use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::config::CodeGraphConfig;
use engram::services::code_graph;

// ── Corpus ───────────────────────────────────────────────────────────────────
//
//   src/caller.rs:  call_it() -> shared_target()      (cross-file call)
//   src/keep.rs:    shared_target()                   (the ONE live def to keep)
//   src/dropme.rs:  shared_target()                   (duplicate; excluded later)
//
// Initial index: `shared_target` is defined in BOTH keep.rs and dropme.rs, so
// the cross-file call is AMBIGUOUS and the post-pass withholds the singleton.
// After a forced re-index that excludes dropme.rs (still on disk), only keep.rs
// defines `shared_target`, so the singleton `call_it -> shared_target` becomes
// resolvable — but only if eviction runs BEFORE the post-pass.

const CALLER_RS: &str = "\
pub fn call_it() {
    shared_target();
}
";

const KEEP_RS: &str = "pub fn shared_target() -> u8 { 1 }\n";

const DROPME_RS: &str = "pub fn shared_target() -> u8 { 2 }\n";

const CARGO_TOML: &str = "[package]\nname = \"demo\"\nversion = \"0.1.0\"\nedition = \"2024\"\n";

// ── Harness helpers ──────────────────────────────────────────────────────────

fn write_one(ws: &Path, rel: &str, content: &str) {
    let full = ws.join(rel);
    if let Some(parent) = full.parent() {
        fs::create_dir_all(parent).expect("create dirs");
    }
    fs::write(full, content).expect("write file");
}

fn write_fixture(ws: &Path) {
    write_one(ws, "Cargo.toml", CARGO_TOML);
    write_one(ws, "src/caller.rs", CALLER_RS);
    write_one(ws, "src/keep.rs", KEEP_RS);
    write_one(ws, "src/dropme.rs", DROPME_RS);
}

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// A `CodeGraphConfig` that excludes `src/dropme.rs` from discovery (the file
/// stays on disk, so it is `indexed − discovered`, never a sync-deletion).
fn config_excluding_dropme() -> CodeGraphConfig {
    CodeGraphConfig {
        exclude_patterns: vec!["src/dropme.rs".to_owned()],
        ..CodeGraphConfig::default()
    }
}

/// `id -> name` over every indexed function.
async fn id_to_name(q: &CodeGraphQueries) -> HashMap<String, String> {
    q.all_functions()
        .await
        .expect("all_functions")
        .into_iter()
        .map(|f| (f.id, f.name))
        .collect()
}

/// Every `calls_resolved_singleton` edge mapped to `(from_name, to_name)`. A
/// missing endpoint id surfaces as `<dangling:...>` so a false/dangling edge is
/// never silently counted as a valid `(call_it, shared_target)` pair.
async fn singleton_edge_names(q: &CodeGraphQueries) -> Vec<(String, String)> {
    let names = id_to_name(q).await;
    q.list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list singleton edges")
        .into_iter()
        .map(|(from, to)| {
            let f = names
                .get(&from)
                .cloned()
                .unwrap_or_else(|| format!("<dangling:{from}>"));
            let t = names
                .get(&to)
                .cloned()
                .unwrap_or_else(|| format!("<dangling:{to}>"));
            (f, t)
        })
        .collect()
}

// ── AC1 (RED FIRST) + AC2 (GREEN): recall RECOVERY after eviction ────────────

/// A previously-indexed file that duplicated a callee name is excluded on a
/// forced re-index; the now-unambiguous cross-file singleton must resolve.
/// RED pre-R3 (post-pass sees pre-eviction ambiguity); GREEN once eviction runs
/// before the post-pass.
#[test]
async fn forced_index_recovers_singleton_after_duplicate_callee_excluded() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let (data_dir, branch) = test_db_params(ws);

    // Initial index: both keep.rs and dropme.rs define `shared_target`, so the
    // cross-file call is ambiguous and no singleton is emitted.
    code_graph::index_workspace(ws, &data_dir, &branch, &CodeGraphConfig::default(), false)
        .await
        .expect("initial index should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let before = singleton_edge_names(&q).await;
    assert!(
        !before.contains(&("call_it".to_owned(), "shared_target".to_owned())),
        "precondition: the duplicate-callee ambiguity withholds the singleton; got {before:?}"
    );

    // Forced re-index with dropme.rs excluded (still on disk, no longer
    // discovered). R3: eviction runs BEFORE the post-pass, so the post-pass sees
    // exactly one live `shared_target` and resolves the singleton.
    code_graph::index_workspace(ws, &data_dir, &branch, &config_excluding_dropme(), true)
        .await
        .expect("forced re-index should succeed");

    let db2 = connect_db(&data_dir, &branch).await.expect("db reconnect");
    let q2 = CodeGraphQueries::new(db2);
    let after = singleton_edge_names(&q2).await;

    assert!(
        after.contains(&("call_it".to_owned(), "shared_target".to_owned())),
        "AC1/AC2: forced-index reconciliation must recover the cross-file singleton once the duplicate callee is evicted BEFORE the post-pass; got {after:?}"
    );
    // AC3 fail-closed: the recovered edge is a real edge to a LIVE definition —
    // no `<dangling:...>` endpoint, and the excluded file's def is gone.
    assert!(
        !after
            .iter()
            .any(|(f, t)| f.starts_with("<dangling:") || t.starts_with("<dangling:")),
        "AC3: the recovered singleton must never be a dangling/false edge; got {after:?}"
    );
    assert!(
        q2.get_code_file_by_path("src/dropme.rs")
            .await
            .expect("query dropme.rs")
            .is_none(),
        "the excluded duplicate file must be evicted (its code_file record removed)"
    );
    // The generation marker still certifies AFTER the post-pass + dangling sweep.
    assert_eq!(
        q2.code_graph_extraction_generation()
            .expect("read generation marker"),
        Some("1".to_owned()),
        "AC3: the generation marker advances after a clean forced-index reconciliation"
    );
}

// ── AC3/AC4 (preserve): still-ambiguous stays fail-closed ────────────────────

/// When BOTH duplicate definitions remain discovered (nothing excluded), the
/// cross-file call stays AMBIGUOUS and NO singleton is emitted — the reordering
/// never manufactures a false edge to an arbitrary definition.
#[test]
async fn forced_index_keeps_ambiguous_call_unresolved_when_nothing_excluded() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    write_fixture(ws);
    let config = CodeGraphConfig::default();
    let (data_dir, branch) = test_db_params(ws);

    code_graph::index_workspace(ws, &data_dir, &branch, &config, false)
        .await
        .expect("initial index should succeed");
    // Forced re-index, nothing excluded — both `shared_target` defs still live.
    code_graph::index_workspace(ws, &data_dir, &branch, &config, true)
        .await
        .expect("forced re-index should succeed");

    let db = connect_db(&data_dir, &branch).await.expect("db connect");
    let q = CodeGraphQueries::new(db);
    let after = singleton_edge_names(&q).await;
    assert!(
        !after.contains(&("call_it".to_owned(), "shared_target".to_owned())),
        "AC3/AC4: an unexcluded duplicate keeps the call ambiguous — no false singleton; got {after:?}"
    );
}
