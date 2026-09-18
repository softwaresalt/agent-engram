//! Integration coverage for plan unit F35: the DAX lint service accepts one
//! caller-pinned request context and stays on that generation even if a newer
//! publication lands mid-request.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, ReadRequestContext, WorkspaceSnapshot};
use engram::services::dax_lint;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";
const DIVIDE_RULE: &str = "dax.divide_operator";

struct Fixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_a: WorkspaceSnapshot,
    snap_b: WorkspaceSnapshot,
}

impl Fixture {
    async fn new() -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        seed_powerbi_workspace(workspace_a.path(), true);
        seed_powerbi_workspace(workspace_b.path(), false);

        let snap_a = snapshot("workspace-dax-lint-a", workspace_a.path());
        let snap_b = snapshot("workspace-dax-lint-b", workspace_b.path());
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
            _workspace_a: workspace_a,
            _workspace_b: workspace_b,
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

fn seed_powerbi_workspace(workspace: &Path, nonconformant: bool) {
    fs::create_dir_all(workspace.join(".git")).expect("create git dir");
    fs::write(
        workspace.join(".git").join("HEAD"),
        "ref: refs/heads/main\n",
    )
    .expect("write git HEAD");

    let engram_dir = workspace.join(".engram");
    fs::create_dir_all(&engram_dir).expect("create .engram dir");
    fs::write(
        engram_dir.join("registry.yaml"),
        "sources:\n  - type: powerbi\n    path: models\n",
    )
    .expect("write registry.yaml");

    let tables = workspace
        .join("models")
        .join("Sales.SemanticModel")
        .join("definition")
        .join("tables");
    fs::create_dir_all(&tables).expect("create tmdl dirs");
    let sales_tmdl = if nonconformant {
        "table Sales\n  column Amount\n    dataType: double\n  measure Ratio = Sales[Amount] / 2\n"
    } else {
        "table Sales\n  column Amount\n    dataType: double\n  measure Total = SUM(Sales[Amount])\n"
    };
    fs::write(tables.join("Sales.tmdl"), sales_tmdl).expect("write Sales.tmdl");
}

fn snapshot(workspace_id: &str, path: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: workspace_id.to_owned(),
        workspace_uuid: format!("uuid-{workspace_id}"),
        branch: BRANCH.to_owned(),
        data_dir: path.join(".engram"),
        path: path.to_string_lossy().into_owned(),
        last_flush: None,
        stale_files: false,
        connection_count: 0,
        file_mtimes: HashMap::new(),
    }
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
fn dax_lint_service_reads_through_the_caller_pinned_context() {
    let service_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("dax_lint.rs");
    let service_source =
        fs::read_to_string(service_source_path).expect("read src/services/dax_lint.rs");
    assert!(
        service_source.contains("context: &ReadRequestContext"),
        "dax lint service must accept a caller-pinned ReadRequestContext"
    );
    let service_body = named_function_body(&service_source, "load_lint_report");
    assert!(
        !service_body.contains("connect_db("),
        "dax lint service must not open a database directly"
    );
    assert!(
        !service_body.contains("context.data_dir()"),
        "dax lint service must not derive a database path from the pinned context"
    );
    assert!(
        service_body.contains("lint_indexed_models("),
        "dax lint service must delegate the pure file walk through lint_indexed_models"
    );

    let tool_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("lint.rs");
    let tool_source = fs::read_to_string(tool_source_path).expect("read src/tools/lint.rs");
    let tool_body = named_function_body(&tool_source, "lint_dax");
    assert!(
        tool_source.contains("ReadRequestContext::from_workspace_snapshot("),
        "lint_dax must convert its pinned dispatch snapshot into a ReadRequestContext"
    );
    assert!(
        tool_body.contains("load_lint_report("),
        "lint_dax must delegate service work to src/services/dax_lint.rs"
    );
    assert!(
        !tool_body.contains("load_registry("),
        "lint_dax must not load registry.yaml directly in the handler body"
    );
    assert!(
        !tool_body.contains("validate_sources("),
        "lint_dax must not validate registry sources directly in the handler body"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn dax_lint_service_stays_on_the_old_generation_after_publication() {
    let fixture = Fixture::new().await;
    fixture.reset_to_old_generation().await;

    let context = ReadRequestContext::from_managed_state(fixture.state.as_ref())
        .await
        .expect("capture pinned managed lint context");
    let workspace_root = PathBuf::from(&fixture.snap_a.path);

    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    dax_lint::install_generation_pin_test_hook("load_lint_report", reached_tx, resume_rx);

    let context = Arc::clone(&context);
    let task = tokio::spawn(async move {
        dax_lint::load_lint_report(context.as_ref(), &workspace_root, None).await
    });

    reached_rx
        .await
        .expect("dax lint service must reach the generation pin barrier");
    fixture.republish_new_generation().await;
    resume_tx
        .send(())
        .expect("dax lint service must still be waiting");

    let report = task
        .await
        .expect("dax lint service task must join")
        .expect("dax lint service must succeed");

    assert!(
        !report.conformant,
        "the old generation must keep its non-conformant lint findings after publication"
    );
    assert!(
        report
            .findings
            .iter()
            .any(|finding| finding.rule == DIVIDE_RULE),
        "the old generation must still report the bare division finding"
    );
}
