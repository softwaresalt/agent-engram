//! Integration tests for `calls_edge` provenance storage (082.003-T).
//!
//! Verifies that the `resolution` provenance attribute is written, defaulted,
//! counted, and enumerated correctly through the public `CodeGraphQueries`
//! surface:
//!   * the stable two-argument `create_calls_edge` writer records `direct`
//!   * the additive `create_calls_edge_with_resolution` writer records the
//!     exact provenance string (never shortened) — e.g.
//!     `calls_resolved_singleton`
//!   * `count_calls_edges_by_resolution` groups edges by provenance
//!   * `list_calls_edges_by_resolution` enumerates the `(from, to)` pairs for a
//!     given provenance value
//!
//! Scenario 1 (legacy relation → migration defaults to `direct`) is a
//! crate-internal unit test in `src/db/cozo_backend/schema.rs`, because it
//! needs a raw legacy-schema CozoDB instance.

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

// Scenario 2: the stable two-argument writer records `direct` provenance.
#[test]
async fn create_calls_edge_records_direct_provenance() {
    let (_tmp, q) = fresh_queries().await;
    q.create_calls_edge("fn:a", "fn:b")
        .await
        .expect("create_calls_edge");

    let counts = q
        .count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution");
    assert_eq!(
        counts.get("direct"),
        Some(&1),
        "direct edge count, {counts:?}"
    );

    let direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("list_calls_edges_by_resolution");
    assert_eq!(
        direct,
        vec![("fn:a".to_owned(), "fn:b".to_owned())],
        "direct edge must be enumerable"
    );
}

// Scenario 3: the additive writer records the exact provenance string,
// unshortened — the canonical cross-file value is `calls_resolved_singleton`.
#[test]
async fn create_calls_edge_with_resolution_preserves_exact_value() {
    let (_tmp, q) = fresh_queries().await;
    q.create_calls_edge_with_resolution("fn:c", "fn:d", "calls_resolved_singleton")
        .await
        .expect("create_calls_edge_with_resolution");

    let counts = q
        .count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution");
    assert_eq!(
        counts.get("calls_resolved_singleton"),
        Some(&1),
        "singleton edge must be stored under the exact provenance value, {counts:?}"
    );
    assert_eq!(
        counts.get("direct"),
        None,
        "no direct edge was written, {counts:?}"
    );

    let singletons = q
        .list_calls_edges_by_resolution("calls_resolved_singleton")
        .await
        .expect("list_calls_edges_by_resolution");
    assert_eq!(
        singletons,
        vec![("fn:c".to_owned(), "fn:d".to_owned())],
        "singleton edge must be enumerable under its exact provenance value"
    );
}

// Scenario 4: counts group correctly across mixed provenance, and enumeration
// filters to the requested value only.
#[test]
async fn counts_and_enumeration_group_by_resolution() {
    let (_tmp, q) = fresh_queries().await;
    q.create_calls_edge("fn:a", "fn:b").await.expect("edge 1");
    q.create_calls_edge("fn:a", "fn:c").await.expect("edge 2");
    q.create_calls_edge_with_resolution("fn:x", "fn:y", "calls_resolved_singleton")
        .await
        .expect("edge 3");

    let counts = q
        .count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution");
    assert_eq!(
        counts.get("direct"),
        Some(&2),
        "two direct edges, {counts:?}"
    );
    assert_eq!(
        counts.get("calls_resolved_singleton"),
        Some(&1),
        "one singleton edge, {counts:?}"
    );

    let mut direct = q
        .list_calls_edges_by_resolution("direct")
        .await
        .expect("list direct");
    direct.sort();
    assert_eq!(
        direct,
        vec![
            ("fn:a".to_owned(), "fn:b".to_owned()),
            ("fn:a".to_owned(), "fn:c".to_owned()),
        ],
        "enumeration must return only direct edges"
    );

    let unknown = q
        .list_calls_edges_by_resolution("no_such_resolution")
        .await
        .expect("list unknown");
    assert!(unknown.is_empty(), "unknown provenance yields no edges");
}
