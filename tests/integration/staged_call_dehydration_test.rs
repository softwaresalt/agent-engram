//! Integration tests for durable `staged_call` JSONL export (089.001-T).
//!
//! The daemon stages unresolved cross-file calls into the Cozo `staged_call`
//! relation. Before 089-F those rows were LOST on dehydration, so a call staged
//! before a daemon restart could never resolve after restart. These tests pin
//! the export half of the fix:
//!
//! Scenarios:
//!   1. `serialize_staged_calls_jsonl` is deterministic (stable ordering) and
//!      idempotent (re-serializing identical input yields byte-identical output).
//!   2. `dehydrate_code_graph` writes `staged_calls.jsonl` when staged rows exist,
//!      preserving all four columns, and re-dehydration is byte-identical.
//!   3. `dehydrate_code_graph` removes a stale `staged_calls.jsonl` when the
//!      staging relation is empty.
//!
//! Scope note (084-S): only the EXISTING four columns
//! (`caller_id, callee_name, source_file, created_at`) are persisted. The marker
//! fields `is_method` / `is_qualified` / `provenance` are intentionally deferred
//! to 088-S Unit B (091.011-T); the JSONL format is forward-compatible for them.

#![allow(clippy::needless_raw_string_hashes)]
#![allow(clippy::doc_markdown)]

use std::fs;
use std::path::{Path, PathBuf};

use tokio::test;

use engram::db::connect_db;
use engram::db::queries::{CodeGraphQueries, StagedCallProvenanceRecord, StagedCallRecord};
use engram::services::dehydration::{dehydrate_code_graph, serialize_staged_calls_jsonl};

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

async fn queries_for(data_dir: &Path, branch: &str) -> CodeGraphQueries {
    let db = connect_db(data_dir, branch).await.expect("connect_db");
    CodeGraphQueries::new(db)
}

// Scenario 1: pure serialization is deterministic and idempotent.
#[test]
async fn serialize_staged_calls_jsonl_is_deterministic_and_idempotent() {
    // Deliberately unsorted input to prove the serializer imposes a stable order.
    let records = vec![
        StagedCallRecord {
            caller_id: "function:zzz".to_string(),
            callee_name: "helper".to_string(),
            source_file: "src/z.rs".to_string(),
            created_at: "2026-01-02T00:00:00Z".to_string(),
        },
        StagedCallRecord {
            caller_id: "function:aaa".to_string(),
            callee_name: "helper".to_string(),
            source_file: "src/a.rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
        StagedCallRecord {
            caller_id: "function:aaa".to_string(),
            callee_name: "other".to_string(),
            source_file: "src/a.rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
        },
    ];

    let first = serialize_staged_calls_jsonl(&records);
    let second = serialize_staged_calls_jsonl(&records);
    assert_eq!(first, second, "serialization must be idempotent");

    // Sorting is by (caller_id, callee_name, source_file): aaa/helper, aaa/other,
    // zzz/helper — regardless of input order.
    let lines: Vec<&str> = first.lines().collect();
    assert_eq!(lines.len(), 3, "one line per staged row, got: {first}");
    assert!(
        lines[0].contains("function:aaa") && lines[0].contains("\"helper\""),
        "first line must be the lexicographically smallest key, got {}",
        lines[0]
    );
    assert!(
        lines[2].contains("function:zzz"),
        "last line must be the largest key, got {}",
        lines[2]
    );
    assert!(first.ends_with('\n'), "output must be newline-terminated");
    // All four columns must be present in the payload.
    assert!(first.contains("\"created_at\":\"2026-01-01T00:00:00Z\""));
    assert!(first.contains("\"source_file\":\"src/a.rs\""));

    // Reordering the input must not change the output (determinism).
    let mut shuffled = records.clone();
    shuffled.reverse();
    assert_eq!(
        serialize_staged_calls_jsonl(&shuffled),
        first,
        "output must be independent of input ordering"
    );
}

#[test]
async fn serialize_staged_calls_jsonl_is_deterministic_across_qualifier_ties() {
    // Regression (091.012-T / dehydration tie-breaker): `raw_qualifier` is part
    // of the staged_call key, so two rows can share (caller, callee, source,
    // created_at) yet differ only by qualifier. The serializer must produce a
    // byte-identical, order-independent result for such ties.
    let records = vec![
        StagedCallProvenanceRecord {
            caller_id: "function:caller".to_string(),
            callee_name: "build".to_string(),
            source_file: "src/caller.rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            raw_qualifier: "crate::b".to_string(),
            qualifier_kind: "module".to_string(),
            enclosing_canonical_type: String::new(),
        },
        StagedCallProvenanceRecord {
            caller_id: "function:caller".to_string(),
            callee_name: "build".to_string(),
            source_file: "src/caller.rs".to_string(),
            created_at: "2026-01-01T00:00:00Z".to_string(),
            raw_qualifier: "crate::a".to_string(),
            qualifier_kind: "module".to_string(),
            enclosing_canonical_type: String::new(),
        },
    ];

    let first = serialize_staged_calls_jsonl(&records);
    assert_eq!(first.lines().count(), 2, "both tied rows must serialize");

    let mut reversed = records.clone();
    reversed.reverse();
    assert_eq!(
        serialize_staged_calls_jsonl(&reversed),
        first,
        "qualifier ties must serialize deterministically regardless of input order"
    );

    // The JSON tie-breaker sorts "crate::a" before "crate::b".
    let lines: Vec<&str> = first.lines().collect();
    assert!(
        lines[0].contains("\"raw_qualifier\":\"crate::a\""),
        "first line must be the qualifier that sorts first, got {}",
        lines[0]
    );
}

// Scenario 2: dehydration writes staged_calls.jsonl and is idempotent.
#[test]
async fn dehydrate_writes_staged_calls_jsonl_and_is_idempotent() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    let q = queries_for(&data_dir, &branch).await;

    // Stage two unresolved cross-file calls directly.
    q.put_staged_call("function:caller1", "helper", "src/a.rs")
        .await
        .expect("stage call 1");
    q.put_staged_call("function:caller2", "widget", "src/b.rs")
        .await
        .expect("stage call 2");

    dehydrate_code_graph(&q, &data_dir, &branch)
        .await
        .expect("dehydrate 1");

    let staged_path = data_dir
        .join("code-graph")
        .join(&branch)
        .join("staged_calls.jsonl");
    assert!(
        staged_path.exists(),
        "staged_calls.jsonl must be written when staged rows exist"
    );
    let content1 = fs::read_to_string(&staged_path).expect("read staged_calls.jsonl");
    assert_eq!(content1.lines().count(), 2, "two staged rows: {content1}");
    assert!(content1.contains("function:caller1") && content1.contains("\"helper\""));
    assert!(content1.contains("function:caller2") && content1.contains("\"widget\""));

    // Re-dehydration with unchanged staging must be byte-identical (idempotent).
    dehydrate_code_graph(&q, &data_dir, &branch)
        .await
        .expect("dehydrate 2");
    let content2 = fs::read_to_string(&staged_path).expect("re-read staged_calls.jsonl");
    assert_eq!(content1, content2, "re-export must be byte-identical");
}

// Scenario 3: empty staging removes a stale staged_calls.jsonl.
#[test]
async fn dehydrate_removes_stale_staged_calls_jsonl_when_empty() {
    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    let q = queries_for(&data_dir, &branch).await;

    q.put_staged_call("function:caller1", "helper", "src/a.rs")
        .await
        .expect("stage call");
    dehydrate_code_graph(&q, &data_dir, &branch)
        .await
        .expect("dehydrate with staged rows");
    let staged_path = data_dir
        .join("code-graph")
        .join(&branch)
        .join("staged_calls.jsonl");
    assert!(staged_path.exists(), "file written with staged rows");

    // Clear staging then re-dehydrate — the stale file must be removed.
    q.clear_staged_calls_for_file("src/a.rs")
        .await
        .expect("clear staging");
    dehydrate_code_graph(&q, &data_dir, &branch)
        .await
        .expect("dehydrate empty");
    assert!(
        !staged_path.exists(),
        "empty staging must remove the stale staged_calls.jsonl"
    );
}

#[test]
async fn serialize_and_dehydrate_preserve_staged_call_provenance() {
    let records = vec![StagedCallProvenanceRecord {
        caller_id: "function:caller".to_string(),
        callee_name: "helper".to_string(),
        source_file: "src/a.rs".to_string(),
        created_at: "2026-01-01T00:00:00Z".to_string(),
        raw_qualifier: "crate::a::b".to_string(),
        qualifier_kind: "module".to_string(),
        enclosing_canonical_type: "demo::a::Widget".to_string(),
    }];

    let jsonl = serialize_staged_calls_jsonl(&records);
    assert!(jsonl.contains("\"raw_qualifier\":\"crate::a::b\""));
    assert!(jsonl.contains("\"qualifier_kind\":\"module\""));
    assert!(jsonl.contains("\"enclosing_canonical_type\":\"demo::a::Widget\""));

    let tmp = tempfile::tempdir().expect("tempdir");
    let ws = tmp.path();
    let (data_dir, branch) = test_db_params(ws);
    let q = queries_for(&data_dir, &branch).await;
    q.put_staged_call_with_provenance(
        "function:caller",
        "helper",
        "src/a.rs",
        "crate::a::b",
        "module",
        "demo::a::Widget",
    )
    .await
    .expect("stage provenance");

    dehydrate_code_graph(&q, &data_dir, &branch)
        .await
        .expect("dehydrate");

    let staged_path = data_dir
        .join("code-graph")
        .join(&branch)
        .join("staged_calls.jsonl");
    let content = fs::read_to_string(&staged_path).expect("read staged_calls.jsonl");
    assert!(content.contains("\"raw_qualifier\":\"crate::a::b\""));
    assert!(content.contains("\"qualifier_kind\":\"module\""));
    assert!(content.contains("\"enclosing_canonical_type\":\"demo::a::Widget\""));
}
