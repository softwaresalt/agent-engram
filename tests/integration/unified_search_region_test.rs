//! End-to-end integration test for `unified_search`'s `region` filter (080-F).
//!
//! Guards the call-site wiring of `should_include_content`: with `region: "code"`
//! the content fetch must be skipped so no content (Task-region) results are
//! returned, while `region: "all"` returns both code and content.
//!
//! Gated behind the `embeddings` feature: `unified_search` embeds the query via
//! the `bge-small-en-v1.5` model, and code symbols are embedded during indexing.
#![cfg(all(feature = "cozo-backend", feature = "embeddings"))]

use engram::config::StaleStrategy;
use engram::models::config::DaemonMode;
use std::sync::Arc;

use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::db::workspace::{canonicalize_workspace, resolve_data_dir, resolve_git_branch};
use engram::models::ContentRecord;
use engram::models::config::CodeGraphConfig;
use engram::server::state::AppState;
use engram::services::{code_graph, embedding};
use engram::tools;
use serde_json::{Value, json};

async fn upsert_doc(queries: &CodeGraphQueries, id: &str, content: &str) {
    let embedding = embedding::embed_text(content).expect("embed content record");
    let record = ContentRecord {
        id: id.to_owned(),
        content_type: "docs".to_owned(),
        file_path: format!("docs/{id}.md"),
        content_hash: format!("hash_{id}"),
        content: content.to_owned(),
        embedding: Some(embedding),
        source_path: "docs".to_owned(),
        file_size_bytes: content.len() as u64,
        ingested_at: chrono::Utc::now(),
        record_kind: "file".to_owned(),
        chunk_id: None,
        chunk_index: None,
        heading_path: Vec::new(),
        line_start: None,
        line_end: None,
        fallback_reason: None,
        lint_summary: None,
        suggestions: Vec::new(),
    };
    queries
        .upsert_content_record(&record)
        .await
        .expect("upsert_content_record");
}

fn has_region(result: &Value, region: &str) -> bool {
    result["results"]
        .as_array()
        .expect("results array")
        .iter()
        .any(|r| r["region"] == region)
}

/// `region: "code"` must exclude content; `region: "all"` includes both. This
/// exercises the `should_include_content` gate at its call site — a unit test on
/// the helper alone would still pass if the gate were removed or inverted.
#[tokio::test]
async fn unified_search_region_code_excludes_content() {
    let workspace = tempfile::tempdir().expect("workspace tempdir");
    let ws = workspace.path().canonicalize().expect("canonicalize");
    std::fs::create_dir(ws.join(".git")).expect("create .git");
    std::fs::write(ws.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    std::fs::create_dir(ws.join("src")).expect("create src");
    std::fs::write(
        ws.join("src").join("lib.rs"),
        "/// Build a widget for the widget catalog.\npub fn build_widget() -> u32 {\n    42\n}\n",
    )
    .expect("write source");

    let path = ws.to_string_lossy().to_string();
    let canonical = canonicalize_workspace(&path).expect("canonicalize workspace");
    let branch = resolve_git_branch(&canonical).unwrap_or_else(|_| "default".to_owned());
    let data_dir = resolve_data_dir(&canonical);

    // Index code (symbols get real embeddings) BEFORE binding so set_workspace's
    // background hydration takes the "DB already populated" fast path.
    let config = CodeGraphConfig::default();
    code_graph::index_workspace(&canonical, &data_dir, &branch, &config, false)
        .await
        .expect("index_workspace");

    // Insert a content (docs) record with an embedding into the same DB.
    let db = connect_db(&data_dir, &branch).await.expect("connect_db");
    let queries = CodeGraphQueries::new(db);
    upsert_doc(
        &queries,
        "widget_guide",
        "Widget documentation and usage guide.",
    )
    .await;
    drop(queries);

    let state = Arc::new(AppState::with_mode(
        DaemonMode::Managed,
        10,
        StaleStrategy::Warn,
        20,
        60,
    ));
    tools::dispatch(
        state.clone(),
        "set_workspace",
        Some(json!({ "path": path })),
    )
    .await
    .expect("set_workspace");

    // region: "all" → both code and content present.
    let all = tools::dispatch(
        state.clone(),
        "unified_search",
        Some(json!({ "query": "widget", "region": "all", "limit": 10 })),
    )
    .await
    .expect("unified_search region=all");
    assert!(
        has_region(&all, "code"),
        "region=all must include code results: {all}"
    );
    assert!(
        has_region(&all, "task"),
        "region=all must include content (task) results: {all}"
    );

    // region: "code" → content is excluded.
    let code = tools::dispatch(
        state.clone(),
        "unified_search",
        Some(json!({ "query": "widget", "region": "code", "limit": 10 })),
    )
    .await
    .expect("unified_search region=code");
    assert!(
        has_region(&code, "code"),
        "region=code must include code results: {code}"
    );
    assert!(
        !has_region(&code, "task"),
        "region=code must NOT include content (task) results: {code}"
    );
}
