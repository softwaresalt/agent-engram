//! Integration coverage for plan unit F32: the registry status service reads
//! through one caller-pinned request context and stays on that generation even
//! if a newer publication lands mid-request.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::registry;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";
const WORKSPACE_ID: &str = "workspace-registry-pin";
const WORKSPACE_UUID: &str = "00000000-0000-0000-0000-000000000041";
const OLD_SOURCE: &str = "docs-old";
const NEW_SOURCE: &str = "docs-new";

struct Fixture {
    _root_a: TempDir,
    _root_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl Fixture {
    async fn new() -> Self {
        let root_a = tempfile::tempdir().expect("old generation root tempdir");
        let root_b = tempfile::tempdir().expect("new generation root tempdir");
        let data_dir_a = root_a.path().join(".engram");
        let data_dir_b = root_b.path().join(".engram");

        fs::create_dir_all(root_a.path().join(OLD_SOURCE)).expect("create old source dir");
        fs::create_dir_all(root_b.path().join(NEW_SOURCE)).expect("create new source dir");
        fs::create_dir_all(&data_dir_a).expect("create old .engram dir");
        fs::create_dir_all(&data_dir_b).expect("create new .engram dir");
        fs::write(data_dir_a.join("registry.yaml"), registry_yaml(OLD_SOURCE))
            .expect("write old registry.yaml");
        fs::write(data_dir_b.join("registry.yaml"), registry_yaml(NEW_SOURCE))
            .expect("write new registry.yaml");

        let root_a_path = root_a.path().to_string_lossy().into_owned();
        let root_b_path = root_b.path().to_string_lossy().into_owned();
        let snap_a = snapshot(&root_a_path, &data_dir_a);
        let snap_b = snapshot(&root_b_path, &data_dir_b);
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
            _root_a: root_a,
            _root_b: root_b,
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

fn registry_yaml(source_path: &str) -> String {
    format!("sources:\n  - type: docs\n    path: {source_path}\n    language: markdown\n")
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
fn registry_service_reads_through_the_caller_pinned_context() {
    let service_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("registry.rs");
    let service_source =
        fs::read_to_string(service_source_path).expect("read src/services/registry.rs");
    let service_body = named_function_body(&service_source, "load_registry_status");
    assert!(
        !service_body.contains("connect_db("),
        "registry service must not open a database directly"
    );
    assert!(
        service_body.contains("context.data_dir()"),
        "registry service must resolve registry.yaml from the caller-pinned context"
    );

    let read_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("read.rs");
    let read_source = fs::read_to_string(read_source_path).expect("read src/tools/read.rs");
    let read_body = named_function_body(&read_source, "get_workspace_statistics");
    assert!(
        read_body.contains("pinned_read_request_context("),
        "get_workspace_statistics must pin one ReadRequestContext before delegating to the registry service"
    );
    assert!(
        read_body.contains("queries_from_read_context("),
        "get_workspace_statistics must open its queries from the pinned ReadRequestContext"
    );
    assert!(
        read_body.contains("registry::load_registry_status("),
        "get_workspace_statistics must delegate registry loading to src/services/registry.rs"
    );
    assert!(
        !read_body.contains("pinned_queries("),
        "get_workspace_statistics must not pin a second dispatch snapshot for registry reads"
    );
    assert!(
        !read_source.contains("async fn load_registry_status("),
        "src/tools/read.rs must not keep its own registry-loading helper"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn registry_service_stays_on_the_old_generation_after_publication() {
    let fixture = Fixture::new().await;
    fixture.reset_to_old_generation().await;

    let context = ReadRequestContext::from_managed_state(fixture.state.as_ref())
        .await
        .expect("capture pinned managed registry context");

    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    registry::install_generation_pin_test_hook("load_registry_status", reached_tx, resume_rx);

    let context = Arc::clone(&context);
    let task = tokio::spawn(async move { registry::load_registry_status(context.as_ref()).await });

    reached_rx
        .await
        .expect("registry service must reach the generation pin barrier");
    fixture.republish_new_generation().await;
    resume_tx
        .send(())
        .expect("registry service must still be waiting");

    let status = task
        .await
        .expect("registry service task must join")
        .expect("registry service must succeed")
        .expect("registry status must be present");

    assert_eq!(status["total_sources"], 1);
    let sources = status["sources"].as_array().expect("sources array");
    assert_eq!(sources.len(), 1);
    assert_eq!(sources[0]["path"], OLD_SOURCE);
    assert_eq!(sources[0]["status"], "active");
    assert_ne!(sources[0]["path"], NEW_SOURCE);
}
