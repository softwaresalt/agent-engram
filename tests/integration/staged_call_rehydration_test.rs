//! Integration tests for durable `staged_call` JSONL rehydration (089.002-T).
//!
//! Rehydration is the read half of the 089-F durability fix: on daemon restart
//! the `staged_call` rows must be restored from `staged_calls.jsonl` so the
//! deferred cross-file post-pass can still resolve calls staged before the
//! restart. These tests pin the rehydration behavior and its compatibility
//! guarantees:
//!
//! Scenarios:
//!   1. `hydrate_code_graph` loads staged rows from `staged_calls.jsonl`,
//!      preserving all four columns; a second hydrate is idempotent.
//!   2. Legacy tolerance — a snapshot WITHOUT `staged_calls.jsonl` rehydrates
//!      with zero staged rows and NO error (backward compatible).
//!   3. Forward tolerance — a `staged_calls.jsonl` line carrying extra unknown
//!      marker fields (the `is_method` / `is_qualified` / `provenance` fields
//!      deferred to 088-S Unit B / 091.011-T) still loads; a line missing
//!      `created_at` also loads.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::{Path, PathBuf};

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
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

/// Seed a `staged_calls.jsonl` into the branch-aware code-graph directory that
/// `hydrate_code_graph` reads, and return an open queries handle.
async fn seed_staged_calls_jsonl(
    data_dir: &Path,
    branch: &str,
    staged_jsonl: &str,
) -> CodeGraphQueries {
    let dir = data_dir.join("code-graph").join(branch);
    fs::create_dir_all(&dir).expect("create code-graph dir");
    fs::write(dir.join("staged_calls.jsonl"), staged_jsonl).expect("write staged_calls.jsonl");
    let db = connect_db(data_dir, branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

// Scenario 1: staged rows load from JSONL and re-hydration is idempotent.
#[test]
async fn rehydrate_loads_staged_calls_and_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    let jsonl = concat!(
        r#"{"caller_id":"function:caller1","callee_name":"helper","source_file":"src/a.rs","created_at":"2026-01-01T00:00:00Z"}"#,
        "\n",
        r#"{"caller_id":"function:caller2","callee_name":"widget","source_file":"src/b.rs","created_at":"2026-01-02T00:00:00Z"}"#,
        "\n",
    );
    let q = seed_staged_calls_jsonl(&data_dir, &branch, jsonl).await;

    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate 1");

    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert_eq!(
        staged.len(),
        2,
        "both staged rows must load, got {staged:?}"
    );
    assert!(staged.iter().any(|s| s.callee_name == "helper"
        && s.caller_id == "function:caller1"
        && s.source_file == "src/a.rs"));
    assert!(staged.iter().any(|s| s.callee_name == "widget"));

    // Second hydrate is a no-op overwrite (keyed put), count unchanged.
    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate 2");
    let staged2 = q.list_staged_calls().await.expect("list_staged_calls 2");
    assert_eq!(
        staged2.len(),
        2,
        "re-hydration must be idempotent, got {staged2:?}"
    );
}

// Scenario 2: a legacy snapshot without staged_calls.jsonl loads zero rows, no error.
#[test]
async fn rehydrate_tolerates_missing_staged_calls_file() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    // Create the code-graph dir but write NO staged_calls.jsonl (legacy layout).
    let dir = data_dir.join("code-graph").join(&branch);
    fs::create_dir_all(&dir).expect("create code-graph dir");
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let q = CodeGraphQueries::new(db);

    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate must not error on missing staged_calls.jsonl");

    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert!(
        staged.is_empty(),
        "legacy snapshot must yield zero staged rows, got {staged:?}"
    );
}

// Scenario 3: extra unknown fields and a missing created_at are tolerated.
#[test]
async fn rehydrate_tolerates_extra_and_missing_fields() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    // Line 1: carries FUTURE marker fields (deferred to 088-S) — must be ignored.
    // Line 2: omits created_at entirely — must still load.
    let jsonl = concat!(
        r#"{"caller_id":"function:caller1","callee_name":"helper","source_file":"src/a.rs","created_at":"2026-01-01T00:00:00Z","is_method":true,"is_qualified":false,"provenance":"raw"}"#,
        "\n",
        r#"{"caller_id":"function:caller2","callee_name":"widget","source_file":"src/b.rs"}"#,
        "\n",
    );
    let q = seed_staged_calls_jsonl(&data_dir, &branch, jsonl).await;

    hydrate_code_graph(ws, &data_dir, &branch, &q)
        .await
        .expect("hydrate must tolerate extra/missing fields");

    let staged = q.list_staged_calls().await.expect("list_staged_calls");
    assert_eq!(
        staged.len(),
        2,
        "both forward-/backward-compatible rows must load, got {staged:?}"
    );
    assert!(staged.iter().any(|s| s.callee_name == "helper"));
    assert!(staged.iter().any(|s| s.callee_name == "widget"));
}
