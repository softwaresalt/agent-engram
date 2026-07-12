//! Integration tests for `calls_edge` provenance rehydration (082.012-T).
//!
//! Verifies that provenance survives the full round-trip
//! DB → export (JSONL) → rehydrate → DB, so a daemon restart does not silently
//! downgrade a `calls_resolved_singleton` edge to `direct`.
//!
//! Scenarios (3):
//!   1. a JSONL calls edge tagged `calls_resolved_singleton` rehydrates with
//!      that exact provenance (not downgraded)
//!   2. a JSONL calls edge with no `resolution` key rehydrates as `direct`
//!      (backward compatibility with pre-field exports)
//!   3. full round-trip — build edges, `serialize_edges_jsonl`, rehydrate, then
//!      `count_calls_edges_by_resolution` matches the pre-export split

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::{Path, PathBuf};

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::code_edge::{CodeEdge, CodeEdgeType};
use engram::services::dehydration::serialize_edges_jsonl;
use engram::services::hydration::hydrate_code_graph;

fn test_db_params(path: &Path) -> (PathBuf, String) {
    use sha2::{Digest, Sha256};
    let canon = path
        .canonicalize()
        .unwrap_or_else(|_| path.to_path_buf())
        .to_string_lossy()
        .to_lowercase();
    let branch = format!("{:x}", Sha256::digest(canon.as_bytes()));
    (std::env::temp_dir().join("engram-test"), branch)
}

/// Write `edges.jsonl` into the branch-aware code-graph directory that
/// `hydrate_code_graph` reads, and return an open queries handle.
async fn seed_edges_jsonl(data_dir: &Path, branch: &str, edges_jsonl: &str) -> CodeGraphQueries {
    let dir = data_dir.join("code-graph").join(branch);
    fs::create_dir_all(&dir).expect("create code-graph dir");
    fs::write(dir.join("edges.jsonl"), edges_jsonl).expect("write edges.jsonl");
    let db = connect_db(data_dir, branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

// Scenario 1: a singleton-tagged JSONL edge keeps its exact provenance.
#[test]
async fn singleton_provenance_survives_rehydrate() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    let line = r#"{"type":"calls","from":"function:a","to":"function:b","resolution":"calls_resolved_singleton"}"#;
    let q = seed_edges_jsonl(&data_dir, &branch, &format!("{line}\n")).await;

    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate_code_graph");

    let counts = q
        .count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution");
    assert_eq!(
        counts.get("calls_resolved_singleton"),
        Some(&1),
        "singleton provenance must survive rehydrate unshortened, {counts:?}"
    );
    assert_eq!(
        counts.get("direct"),
        None,
        "no direct edge expected, {counts:?}"
    );
}

// Scenario 2: a JSONL edge with no `resolution` key rehydrates as `direct`.
#[test]
async fn missing_resolution_rehydrates_as_direct() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    let line = r#"{"type":"calls","from":"function:c","to":"function:d"}"#;
    let q = seed_edges_jsonl(&data_dir, &branch, &format!("{line}\n")).await;

    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate_code_graph");

    let counts = q
        .count_calls_edges_by_resolution()
        .await
        .expect("count_calls_edges_by_resolution");
    assert_eq!(
        counts.get("direct"),
        Some(&1),
        "pre-field JSONL must default to direct, {counts:?}"
    );
}

// Scenario 3: full round-trip through serialize_edges_jsonl preserves the split.
#[test]
async fn full_round_trip_preserves_provenance_split() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);

    let edges = vec![
        CodeEdge {
            edge_type: CodeEdgeType::Calls,
            from: "function:a".to_string(),
            to: "function:b".to_string(),
            import_path: None,
            linked_by: None,
            resolution: Some("direct".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        CodeEdge {
            edge_type: CodeEdgeType::Calls,
            from: "function:e".to_string(),
            to: "function:f".to_string(),
            import_path: None,
            linked_by: None,
            resolution: Some("direct".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        CodeEdge {
            edge_type: CodeEdgeType::Calls,
            from: "function:x".to_string(),
            to: "function:y".to_string(),
            import_path: None,
            linked_by: None,
            resolution: Some("calls_resolved_singleton".to_string()),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];
    let jsonl = serialize_edges_jsonl(&edges);
    let q = seed_edges_jsonl(&data_dir, &branch, &jsonl).await;

    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate_code_graph");

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
}
