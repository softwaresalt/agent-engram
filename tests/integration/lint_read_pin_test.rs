//! Integration coverage for plan unit F29: lint reads pin one dispatch
//! snapshot and do not re-read workspace state in the handler body.

#![forbid(unsafe_code)]

use std::collections::HashMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::sync::Arc;

use engram::config::StaleStrategy;
use engram::models::config::{DaemonMode, WorkspaceConfig};
use engram::server::state::{AppState, WorkspaceSnapshot};
use engram::tools::lint;
use tempfile::TempDir;
use tokio::sync::oneshot;

const BRANCH: &str = "main";

struct LintFixture {
    _workspace_a: TempDir,
    _workspace_b: TempDir,
    state: Arc<AppState>,
    snap_b: WorkspaceSnapshot,
}

impl LintFixture {
    async fn new() -> Self {
        let workspace_a = tempfile::tempdir().expect("workspace A tempdir");
        let workspace_b = tempfile::tempdir().expect("workspace B tempdir");
        seed_powerbi_workspace(workspace_a.path(), false);
        seed_powerbi_workspace(workspace_b.path(), true);

        let snap_a = snapshot("workspace-lint-a", workspace_a.path());
        let snap_b = snapshot("workspace-lint-b", workspace_b.path());
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
            .expect("seed lint workspace A");

        Self {
            _workspace_a: workspace_a,
            _workspace_b: workspace_b,
            state,
            snap_b,
        }
    }

    async fn publish_b(&self) {
        self.state
            .set_workspace_and_config(self.snap_b.clone(), Some(WorkspaceConfig::default()))
            .await
            .expect("publish lint workspace B");
    }
}

fn seed_powerbi_workspace(workspace: &Path, clean: bool) {
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
    let sales_tmdl = if clean {
        "table Sales\n  column Amount\n    dataType: double\n  measure Total = SUM(Sales[Amount])\n"
    } else {
        "table Sales\n  column Amount\n    dataType: double\n  measure Ratio = Sales[Amount] / 2\n"
    };
    fs::write(tables.join("Sales.tmdl"), sales_tmdl).expect("write Sales.tmdl");
}

fn snapshot(workspace_id: &str, path: &Path) -> WorkspaceSnapshot {
    WorkspaceSnapshot {
        workspace_id: workspace_id.to_owned(),
        workspace_uuid: format!("uuid-{workspace_id}"),
        branch: BRANCH.to_owned(),
        data_dir: path.join(".engram-data"),
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
fn lint_handler_body_no_longer_resnapshots_workspace_directly() {
    let source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("lint.rs");
    let source = fs::read_to_string(source_path).expect("read src/tools/lint.rs");
    let body = named_function_body(&source, "lint_dax");
    assert!(
        !body.contains("snapshot_workspace("),
        "lint_dax must not re-read workspace state directly"
    );
    assert!(
        !body.contains("snapshot_dispatch_context("),
        "lint_dax must not read dispatch state directly in the handler body"
    );
}

#[tokio::test(flavor = "multi_thread", worker_threads = 4)]
async fn lint_dax_stays_on_the_old_workspace_after_publication() {
    let fixture = LintFixture::new().await;
    let (reached_tx, reached_rx) = oneshot::channel();
    let (resume_tx, resume_rx) = oneshot::channel();
    lint::install_generation_pin_test_hook("lint_dax", reached_tx, resume_rx);

    let state = Arc::clone(&fixture.state);
    let task = tokio::spawn(async move { lint::lint_dax(state, None).await });
    reached_rx
        .await
        .expect("lint handler should pin before publication");
    fixture.publish_b().await;
    resume_tx.send(()).expect("lint handler should be waiting");

    let response = task
        .await
        .expect("lint handler join")
        .expect("lint handler success");
    assert_eq!(response["conformant"], false);
    let findings = response["findings"].as_array().expect("findings array");
    assert!(
        !findings.is_empty(),
        "the old workspace should keep its non-conformant findings after publication"
    );
}
