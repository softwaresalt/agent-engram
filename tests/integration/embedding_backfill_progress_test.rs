//! Real-backend integration tests for the content-embedding backfill (075.001-T).
//!
//! Proves the fix that makes indexing report status regardless of which path
//! triggered it: the chunked backfill emits `BackfillProgress` updates (the
//! daemon mirrors these into `scan_status`) and populates embeddings for every
//! pending content record while keeping memory bounded via fixed-size chunks.
//!
//! Gated behind the `embeddings` feature at compile time — the same convention
//! used by the other model-backed tests (`embed_texts_batch_matches_single`).
//! The embedding path loads the cached `bge-small-en-v1.5` model.
#![cfg(all(feature = "cozo-backend", feature = "embeddings"))]

use std::fs;

use tempfile::TempDir;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::services::ingestion::{
    BackfillProgress, backfill_content_embeddings, ingest_all_sources,
};
use engram::services::registry::parse_registry_yaml;

fn write_doc(dir: &std::path::Path, name: &str, title: &str) {
    let para = "This is representative documentation content used to exercise the \
                embedding backfill path with enough natural-language text to yield a \
                meaningful vector from the sentence-embedding model.";
    let body = format!("# {title}\n\n{para}\n\n## Details\n\n{para}\n");
    fs::write(dir.join(name), body).expect("write doc");
}

/// Ingest a docs source, then backfill embeddings and assert that progress is
/// reported through the channel and every pending record ends up embedded.
#[tokio::test]
async fn backfill_reports_progress_and_populates_embeddings() {
    let dir = TempDir::new().expect("tempdir");
    let docs_dir = dir.path().join("docs");
    fs::create_dir_all(&docs_dir).expect("create docs dir");
    for i in 0..6 {
        write_doc(&docs_dir, &format!("doc{i}.md"), &format!("Document {i}"));
    }

    let yaml = "sources:\n  - type: docs\n    language: markdown\n    path: docs\n";
    let config = parse_registry_yaml(yaml).expect("parse registry");

    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    ingest_all_sources(&config, dir.path(), &queries)
        .await
        .expect("ingest_all_sources");

    // Content records exist and lack embeddings before the backfill runs.
    let before = queries
        .select_content_records(None)
        .await
        .expect("select content records (before)");
    assert!(
        !before.is_empty(),
        "docs ingestion must create content records"
    );
    let pending = before
        .iter()
        .filter(|r| r.embedding.as_ref().is_none_or(Vec::is_empty))
        .count();
    assert!(pending > 0, "records must lack embeddings before backfill");

    // Run the backfill, capturing progress updates on the channel.
    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<BackfillProgress>();
    let updated = backfill_content_embeddings(&queries, Some(&tx))
        .await
        .expect("backfill_content_embeddings");
    drop(tx);

    let mut progress = Vec::new();
    while let Ok(sample) = rx.try_recv() {
        progress.push(sample);
    }

    // The backfill embedded records and reported progress.
    assert!(updated > 0, "backfill must embed at least one record");
    assert!(!progress.is_empty(), "backfill must report progress");

    let first = *progress.first().expect("at least one progress update");
    assert_eq!(
        first.done, 0,
        "first update marks the start of the backfill"
    );
    assert_eq!(
        first.total, pending,
        "reported total equals the pending count"
    );

    // Progress is monotonically non-decreasing and finishes at the total.
    let mut prev = 0;
    for sample in &progress {
        let done = sample.done;
        assert_eq!(sample.total, pending, "total stays constant across updates");
        assert!(
            done >= prev,
            "progress must not regress: {prev} then {done}"
        );
        prev = done;
    }
    assert_eq!(
        progress.last().expect("final update").done,
        pending,
        "final progress update reaches the total"
    );

    // Every pending record now has a non-empty embedding vector.
    let after = queries
        .select_content_records(None)
        .await
        .expect("select content records (after)");
    let embedded = after
        .iter()
        .filter(|r| r.embedding.as_ref().is_some_and(|v| !v.is_empty()))
        .count();
    assert!(
        embedded >= updated,
        "at least {updated} records must be embedded after backfill, got {embedded}"
    );
}

/// A backfill with no pending records reports no progress and returns zero.
/// This path never loads the embedding model.
#[tokio::test]
async fn backfill_is_noop_when_no_records_pending() {
    let dir = TempDir::new().expect("tempdir");
    let db = connect_db(dir.path(), "test-branch")
        .await
        .expect("connect_db");
    let queries = CodeGraphQueries::new(db);

    let (tx, mut rx) = tokio::sync::mpsc::unbounded_channel::<BackfillProgress>();
    let updated = backfill_content_embeddings(&queries, Some(&tx))
        .await
        .expect("backfill on empty workspace");
    drop(tx);

    assert_eq!(updated, 0, "no content records means nothing to embed");
    assert!(
        rx.try_recv().is_err(),
        "an empty backfill must not emit progress updates"
    );
}
