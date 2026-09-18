//! Integration coverage for plan unit F36: the git-graph indexing seam accepts one
//! caller-pinned request context and keeps using that pinned workspace/database pair even
//! if a newer publication lands mid-request.

#![forbid(unsafe_code)]

use std::fs;
use std::path::PathBuf;

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
fn git_graph_service_reads_through_the_caller_pinned_context() {
    let service_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("services")
        .join("git_graph.rs");
    let service_source =
        fs::read_to_string(service_source_path).expect("read src/services/git_graph.rs");
    assert!(
        service_source.contains("context: &ReadRequestContext"),
        "git-graph service must accept a caller-pinned ReadRequestContext"
    );
    let service_body = named_function_body(&service_source, "index_git_history_from_context");
    assert!(
        !service_body.contains("connect_db("),
        "git-graph service must not open a database directly"
    );
    assert!(
        !service_body.contains("data_dir()"),
        "git-graph service must not derive a database path from the pinned context"
    );

    let tool_source_path = PathBuf::from(env!("CARGO_MANIFEST_DIR"))
        .join("src")
        .join("tools")
        .join("write.rs");
    let tool_source = fs::read_to_string(tool_source_path).expect("read src/tools/write.rs");
    let tool_body = named_function_body(&tool_source, "index_git_history");
    assert!(
        tool_source.contains("ReadRequestContext::from_workspace_snapshot("),
        "index_git_history must pin one ReadRequestContext from a single workspace snapshot"
    );
    assert!(
        tool_body.contains("index_git_history_from_context("),
        "index_git_history must delegate git walking through the pinned git-graph service seam"
    );
    assert!(
        !tool_body.contains("workspace_path(&state)"),
        "index_git_history must not capture the workspace root from a separate snapshot"
    );
    assert!(
        !tool_source.contains("async fn workspace_path("),
        "src/tools/write.rs must not keep the old split snapshot helper"
    );
}

#[cfg(feature = "git-graph")]
mod runtime {
    use std::collections::HashMap;
    use std::fs;
    use std::path::Path;
    use std::sync::Arc;

    use engram::config::StaleStrategy;
    use engram::db::connect_db;
    use engram::db::queries::CodeGraphQueries;
    use engram::models::config::{DaemonMode, WorkspaceConfig};
    use engram::server::state::{AppState, WorkspaceSnapshot};
    use engram::services::git_graph;
    use engram::tools::write;
    use git2::{Repository, Signature};
    use serde_json::json;
    use tempfile::TempDir;
    use tokio::sync::oneshot;

    const BRANCH: &str = "main";
    const WORKSPACE_ID: &str = "workspace-git-graph-pin";
    const WORKSPACE_UUID: &str = "00000000-0000-0000-0000-000000000045";
    const TRACKED_FILE: &str = "tracked.rs";
    const OLD_BASE: &str = "old base";
    const OLD_LATEST: &str = "old followup";
    const NEW_BASE: &str = "new base";
    const NEW_LATEST: &str = "new followup";

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
            seed_git_workspace(workspace_a.path(), OLD_BASE, OLD_LATEST);
            seed_git_workspace(workspace_b.path(), NEW_BASE, NEW_LATEST);

            let snap_a = snapshot(workspace_a.path());
            let snap_b = snapshot(workspace_b.path());
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
    }

    fn snapshot(workspace: &Path) -> WorkspaceSnapshot {
        WorkspaceSnapshot {
            workspace_id: WORKSPACE_ID.to_owned(),
            workspace_uuid: WORKSPACE_UUID.to_owned(),
            branch: BRANCH.to_owned(),
            data_dir: workspace.join(".engram"),
            path: workspace.to_string_lossy().into_owned(),
            last_flush: None,
            stale_files: false,
            connection_count: 0,
            file_mtimes: HashMap::new(),
        }
    }

    fn seed_git_workspace(workspace: &Path, base_message: &str, latest_message: &str) {
        fs::create_dir_all(workspace.join(".engram")).expect("create .engram dir");
        let repo = init_repo_with_commit(
            workspace,
            TRACKED_FILE,
            "pub fn version() -> i32 { 1 }\n",
            base_message,
        );
        add_commit(
            &repo,
            TRACKED_FILE,
            "pub fn version() -> i32 { 2 }\n",
            latest_message,
        );
    }

    fn init_repo_with_commit(
        dir: &Path,
        file_name: &str,
        content: &str,
        message: &str,
    ) -> Repository {
        let repo = Repository::init(dir).expect("init repo");
        let sig = Signature::now("Test User", "test@example.com").expect("signature");

        let file_path = dir.join(file_name);
        fs::write(&file_path, content).expect("write tracked file");

        let mut index = repo.index().expect("repo index");
        index
            .add_path(Path::new(file_name))
            .expect("stage tracked file");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[])
            .expect("initial commit");

        drop(tree);
        repo
    }

    fn add_commit(repo: &Repository, file_name: &str, content: &str, message: &str) {
        let sig = Signature::now("Test User", "test@example.com").expect("signature");
        let head = repo.head().expect("repo head");
        let parent_commit = repo
            .find_commit(head.target().expect("head target"))
            .expect("parent commit");

        let workspace = repo.workdir().expect("repo workdir");
        fs::write(workspace.join(file_name), content).expect("update tracked file");

        let mut index = repo.index().expect("repo index");
        index
            .add_path(Path::new(file_name))
            .expect("stage tracked file");
        index.write().expect("write index");
        let tree_id = index.write_tree().expect("write tree");
        let tree = repo.find_tree(tree_id).expect("find tree");
        repo.commit(Some("HEAD"), &sig, &sig, message, &tree, &[&parent_commit])
            .expect("followup commit");
    }

    async fn commit_messages(data_dir: &Path) -> Vec<String> {
        let db = connect_db(data_dir, BRANCH)
            .await
            .expect("connect git-graph pin fixture db");
        let queries = CodeGraphQueries::new(db);
        queries
            .select_commits_by_file_path(TRACKED_FILE, 10)
            .await
            .expect("load indexed commits")
            .into_iter()
            .map(|commit| commit.message)
            .collect()
    }

    #[tokio::test(flavor = "multi_thread", worker_threads = 4)]
    async fn git_graph_service_stays_on_the_old_generation_after_publication() {
        let fixture = Fixture::new().await;

        let (reached_tx, reached_rx) = oneshot::channel();
        let (resume_tx, resume_rx) = oneshot::channel();
        git_graph::install_generation_pin_test_hook(
            "index_git_history_from_context",
            reached_tx,
            resume_rx,
        );

        let state = Arc::clone(&fixture.state);
        let task = tokio::spawn(async move {
            write::index_git_history(state, Some(json!({"depth": 100, "force": true}))).await
        });

        reached_rx
            .await
            .expect("git-graph service must reach the generation pin barrier");
        fixture.republish_new_generation().await;
        resume_tx
            .send(())
            .expect("git-graph service must still be waiting");

        let summary = task
            .await
            .expect("git-graph task must join")
            .expect("git-graph indexing must succeed");

        assert_eq!(summary["commits_indexed"].as_u64(), Some(2));
        assert_eq!(summary["new_commits"].as_u64(), Some(2));

        let old_messages = commit_messages(&fixture.snap_a.data_dir).await;
        let new_messages = commit_messages(&fixture.snap_b.data_dir).await;

        assert_eq!(old_messages.len(), 2);
        assert!(
            old_messages.iter().any(|message| message == OLD_BASE),
            "the old generation database must index the old base commit"
        );
        assert!(
            old_messages.iter().any(|message| message == OLD_LATEST),
            "the old generation database must index the old followup commit"
        );
        assert!(
            old_messages
                .iter()
                .all(|message| !message.starts_with("new ")),
            "the in-flight request must not index commits from the newly published workspace"
        );
        assert!(
            new_messages.is_empty(),
            "the newly published generation must remain untouched by the in-flight request"
        );
    }
}
