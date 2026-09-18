//! Integration coverage for plan unit F31: search service reads through one
//! caller-pinned request context and stays on that generation even if a newer
//! publication lands mid-request.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use chrono::Utc;
use engram::config::StaleStrategy;
use engram::db::connect_db;
use engram::db::queries::CodeGraphQueries;
use engram::models::ContentRecord;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::search;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";
const WORKSPACE_ID: &str = "workspace-search-pin";
const WORKSPACE_UUID: &str = "00000000-0000-0000-0000-000000000040";

struct Fixture {
    _workspace: TempDir,
    _data_a: TempDir,
    _data_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl Fixture {
    async fn new() -> Self {
        let workspace = tempfile::tempdir().expect("workspace tempdir");
        let data_a = tempfile::tempdir().expect("data-dir A tempdir");
        let data_b = tempfile::tempdir().expect("data-dir B tempdir");
        let data_dir_a = data_a.path().join(".engram");
        let data_dir_b = data_b.path().join(".engram");

        seed_content_record(
            &data_dir_a,
            content_record(
                "old-alpha",
                "Alpha doc",
                "alpha marker keeps the old generation searchable",
            ),
        )
        .await;
        seed_content_record(
            &data_dir_b,
            content_record(
                "new-beta",
                "Beta doc",
                "beta marker only appears after publication",
            ),
        )
        .await;

        let workspace_path = workspace.path().to_string_lossy().into_owned();
        let snap_a = snapshot(&workspace_path, &data_dir_a);
        let snap_b = snapshot(&workspace_path, &data_dir_b);
        let state = Arc::new(AppState::with_mode(
            DaemonMode::Managed,
            4,
            StaleStrategy::Warn,
            20,
            60,
        ));
        state
            .set_workspace_and_config(snap_a.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("seed old generation");

        Self {
            _workspace: workspace,
            _data_a: data_a,
            _data_b: data_b,
            state,
            snap_a,
            snap_b,
        }
    }

    async fn republish_new_generation(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("publish new generation");
    }

    async fn reset_to_old_generation(&self) {
        self.state
            .set_workspace_and_config(self.snap_a.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("rebind old generation");
    }
}

fn snapshot(workspace_path: &str, data_dir: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: WORKSPACE_ID.to_owned(),
        workspace_uuid: WORKSPACE_UUID.to_owned(),
        branch: BRANCH.to_owned(),
        data_dir: data_dir.to_path_buf(),
        path: workspace_path.to_owned(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
}

fn content_record(id: &str, title: &str, content: &str) -> ContentRecord {
    ContentRecord {
        id: id.to_owned(),
        content_type: "docs".to_owned(),
        file_path: format!("docs/{id}.md"),
        content_hash: format!("hash-{id}"),
        content: content.to_owned(),
        embedding: None,
        source_path: "docs".to_owned(),
        file_size_bytes: u64::try_from(content.len()).unwrap_or(u64::MAX),
        ingested_at: Utc::now(),
        record_kind: "markdown_chunk".to_owned(),
        chunk_id: Some(id.to_owned()),
        chunk_index: Some(1),
        heading_path: vec!["Guide".to_owned(), title.to_owned()],
        line_start: Some(1),
        line_end: Some(3),
        fallback_reason: None,
        lint_summary: None,
        suggestions: Vec::new(),
    }
}

async fn seed_content_record(data_dir: &Path, record: ContentRecord) {
    let db = connect_db(data_dir, BRANCH)
        .await
        .expect("connect search pin fixture db");
    let queries = CodeGraphQueries::new(db);
    queries
        .upsert_content_record(&record)
        .await
        .expect("seed content record");
}

async fn open_queries(context: &ReadRequestContext) -> CodeGraphQueries {
    let db = connect_db(context.data_dir(), context.branch())
        .await
        .expect("connect pinned search context db");
    CodeGraphQueries::new(db)
}

fn named_function_body(source: &str, function_name: &str) -> String {
    let needle = format!("pub async fn {function_name}");
    let start = source
        .find(&needle)
        .unwrap_or_else(|| panic!("function '{function_name}' not found"));
    let brace_start = source[start..].find('{').map_or_else(
        || panic!("function '{function_name}' has no body"),
        |offset| start + offset,
    );

    let mut depth = 0usize;
    let mut end = None;
    for (offset, ch) in source[brace_start..].char_indices() {
        match ch {
            '{' => depth += 1,
            '}' => {
                depth -= 1;
                if depth == 0 {
                    end = Some(brace_start + offset + ch.len_utf8());
                    break;
                }
            }
            _ => {}
        }
    }

    source[brace_start..end.expect("balanced braces")].to_owned()
}

#[test]
fn search_service_reads_through_the_caller_pinned_context() {
    let service_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("search.rs");
    let service_source =
        fs::read_to_string(service_source_path).expect("read src/services/search.rs");
    let service_body = named_function_body(&service_source, "query_memory_results");
    assert!(
        !service_body.contains("connect_db("),
        "search service must not open a database directly"
    );
    assert!(
        !service_body.contains("data_dir()"),
        "search service must not derive a database path from the pinned context"
    );
    assert!(
        !service_body.contains("branch()"),
        "search service must not derive a branch-specific database path itself"
    );

    let read_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("read.rs");
    let read_source = fs::read_to_string(read_source_path).expect("read src/tools/read.rs");
    let read_body = named_function_body(&read_source, "query_memory");
    assert!(
        read_body.contains("pinned_read_request_context("),
        "query_memory must pin one ReadRequestContext before delegating to the search service"
    );
    assert!(
        read_body.contains("query_memory_results("),
        "query_memory must delegate search candidate loading to src/services/search.rs"
    );
    assert!(
        !read_body.contains("select_content_records("),
        "query_memory must no longer load content records directly in the handler body"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn search_service_stays_on_the_old_generation_after_publication() {
    let fixture = Fixture::new().await;
    fixture.reset_to_old_generation().await;

    let context = ReadRequestContext::from_managed_state(fixture.state.as_ref())
        .await
        .expect("capture pinned managed search context");
    let queries = Arc::new(open_queries(context.as_ref()).await);

    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    search::install_generation_pin_test_hook("query_memory_results", reached_tx, resume_rx);

    let context = Arc::clone(&context);
    let queries = Arc::clone(&queries);
    let task = tokio::spawn(async move {
        search::query_memory_results(
            context.as_ref(),
            queries.as_ref(),
            "alpha marker",
            10,
            Some("docs"),
        )
        .await
    });

    reached_rx
        .await
        .expect("search service must reach the generation pin barrier");
    fixture.republish_new_generation().await;
    resume_tx
        .send(())
        .expect("search service must still be waiting");

    let results = task
        .await
        .expect("search service task must join")
        .expect("search service must succeed");

    assert_eq!(results.len(), 1);
    assert_eq!(results[0].id, "content_record:old-alpha");
    assert_eq!(results[0].title.as_deref(), Some("Alpha doc"));
    assert_eq!(results[0].file_path.as_deref(), Some("docs/old-alpha.md"));
    assert!(
        results[0].content.contains("old generation"),
        "the in-flight search must still read the old generation"
    );
}
