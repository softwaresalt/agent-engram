//! Integration tests for the `calls_edge` resolution down-migration
//! (082.010-T rollback logic).
//!
//! Exercises the reusable rollback orchestrator through the public
//! `CodeGraphQueries` surface:
//!   * `retract_all_calls_resolved_singleton_edges` removes every
//!     `calls_resolved_singleton` edge while preserving `direct` edges, and
//!     leaves the `resolution` column in place (so provenance counts stay
//!     queryable)
//!   * `rollback_calls_resolution` performs the retract-then-drop rollback in
//!     strict order (singletons AND canonical edges retracted before the column
//!     drop) and is idempotent on re-invocation
//!
//! Scenario 2 (after retraction, dropping the `resolution` column reverts
//! `calls_edge` to `{from, to => created_at}` and the legacy writer
//! round-trips) is a crate-internal unit test in
//! `src/db/cozo_backend/schema.rs`, because verifying the reverted schema needs
//! a raw legacy-schema writer that is not exposed on the public surface.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::path::Path;

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;

fn test_db_params(path: &Path) -> (std::path::PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

async fn fresh_queries() -> (tempfile::TempDir, CodeGraphQueries) {
    let tmp = tempfile::tempdir().expect("tempdir");
    let (data_dir, branch) = test_db_params(tmp.path());
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    (tmp, CodeGraphQueries::new(db))
}

// Scenario 1: the retract step removes every `calls_resolved_singleton` edge,
// preserves `direct` edges, and keeps the `resolution` column present so
// provenance counts remain queryable BEFORE the column is dropped.
#[test]
async fn retract_removes_singletons_preserving_direct() {
    let (_tmp, q) = fresh_queries().await;
    q.create_calls_edge("fn:a", "fn:b")
        .await
        .expect("direct edge");
    q.create_calls_edge_with_resolution("fn:c", "fn:d", "calls_resolved_singleton")
        .await
        .expect("singleton edge 1");
    q.create_calls_edge_with_resolution("fn:e", "fn:f", "calls_resolved_singleton")
        .await
        .expect("singleton edge 2");

    let retracted = q
        .retract_all_calls_resolved_singleton_edges()
        .await
        .expect("retract singletons");
    assert_eq!(retracted, 2, "both singleton edges must be retracted");

    // Column is still present here, so provenance counts remain queryable.
    let counts = q
        .count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution");
    assert_eq!(
        counts.get("calls_resolved_singleton"),
        None,
        "no singleton edges must remain after retraction, {counts:?}"
    );
    assert_eq!(
        counts.get("direct"),
        Some(&1),
        "the direct edge must be preserved, {counts:?}"
    );
}

// Scenario 3: the full retract-then-drop rollback is idempotent — a second
// invocation retracts nothing and finds no column to drop, returning a zeroed
// `CallsResolutionRollback` without error. No provenance count is queried after
// the column is dropped.
#[test]
async fn rollback_calls_resolution_is_idempotent() {
    let (_tmp, q) = fresh_queries().await;
    q.create_calls_edge("fn:a", "fn:b")
        .await
        .expect("direct edge");
    q.create_calls_edge_with_resolution("fn:c", "fn:d", "calls_resolved_singleton")
        .await
        .expect("singleton edge");

    let first = q.rollback_calls_resolution().await.expect("first rollback");
    assert_eq!(
        first.total(),
        1,
        "first rollback retracts the singleton edge, {first:?}"
    );
    assert_eq!(
        first.singleton_edges, 1,
        "one singleton retracted, {first:?}"
    );
    assert_eq!(
        first.canonical_edges, 0,
        "no canonical edges present, {first:?}"
    );

    // Column is now dropped; a second rollback is a no-op with no error.
    let second = q
        .rollback_calls_resolution()
        .await
        .expect("second rollback must not error");
    assert_eq!(
        second.total(),
        0,
        "second rollback retracts nothing, {second:?}"
    );

    // The direct edge survives the rollback (schema-agnostic count).
    let total = q.count_calls_edges().await.expect("count_calls_edges");
    assert_eq!(total, 1, "the direct edge must survive rollback");
}

// Scenario 4 (088-S Unit B): the full rollback retracts BOTH singleton and
// canonical resolution-derived edges before dropping the column. Canonical
// edges must not survive the column drop as indistinguishable legacy edges —
// only `direct` edges may survive.
#[test]
async fn rollback_retracts_singleton_and_canonical_preserving_direct() {
    let (_tmp, q) = fresh_queries().await;
    q.create_calls_edge("fn:a", "fn:b")
        .await
        .expect("direct edge");
    q.create_calls_edge_with_resolution("fn:c", "fn:d", "calls_resolved_singleton")
        .await
        .expect("singleton edge");
    q.create_calls_edge_with_resolution("fn:e", "fn:f", "calls_resolved_canonical")
        .await
        .expect("canonical edge 1");
    q.create_calls_edge_with_resolution("fn:g", "fn:h", "calls_resolved_canonical")
        .await
        .expect("canonical edge 2");

    let rollback = q.rollback_calls_resolution().await.expect("rollback");
    assert_eq!(
        rollback.singleton_edges, 1,
        "one singleton retracted, {rollback:?}"
    );
    assert_eq!(
        rollback.canonical_edges, 2,
        "both canonical edges retracted, {rollback:?}"
    );
    assert_eq!(
        rollback.total(),
        3,
        "all three resolution-derived edges retracted, {rollback:?}"
    );

    // Only the direct edge survives; canonical edges did NOT leak past the
    // column drop as legacy edges.
    let total = q.count_calls_edges().await.expect("count_calls_edges");
    assert_eq!(
        total, 1,
        "only the direct edge survives; canonical edges must not leak, total={total}"
    );
}
