use std::future::Future;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

use tokio::sync::oneshot;

use super::*;

const SAFE_OLD: &str = "resource \"safe\" \"old\" {\n  value = var.old\n}\n";
const SAFE_NEW: &str = "resource \"safe\" \"new\" {\n  value = var.new\n}\n";
const EXTERNAL_SENTINEL: &str =
    "resource \"external\" \"secret\" {\n  secret = \"OUTSIDE_SENTINEL\"\n}\n";
const INTERNAL_SENTINEL: &str =
    "resource \"internal\" \"replacement\" {\n  secret = \"INTERNAL_SENTINEL\"\n}\n";
const TEST_WATCHDOG: Duration = Duration::from_secs(30);

struct BarrierControl {
    hook: Option<SourceReadTestHook>,
    reached: Option<oneshot::Receiver<()>>,
    resume: Option<oneshot::Sender<()>>,
}

impl BarrierControl {
    fn new(target: &str) -> Self {
        let (reached_tx, reached) = oneshot::channel();
        let (resume, resume_rx) = oneshot::channel();
        Self {
            hook: Some(SourceReadTestHook {
                target: target.to_owned(),
                reached: Arc::new(std::sync::Mutex::new(Some(reached_tx))),
                resume: Arc::new(std::sync::Mutex::new(Some(resume_rx))),
            }),
            reached: Some(reached),
            resume: Some(resume),
        }
    }

    fn take_hook(&mut self) -> anyhow::Result<SourceReadTestHook> {
        self.hook
            .take()
            .ok_or_else(|| anyhow::anyhow!("source-read barrier hook already consumed"))
    }

    async fn wait_until_reached(&mut self) -> anyhow::Result<()> {
        self.reached
            .take()
            .ok_or_else(|| anyhow::anyhow!("source-read barrier already consumed"))?
            .await?;
        Ok(())
    }

    fn release(&mut self) {
        if let Some(resume) = self.resume.take() {
            let _ = resume.send(());
        }
    }
}

async fn bounded_join<T, U>(
    operation: impl Future<Output = T>,
    controller: impl Future<Output = U>,
) -> anyhow::Result<(T, U)> {
    tokio::time::timeout(TEST_WATCHDOG, async { tokio::join!(operation, controller) })
        .await
        .map_err(|_| anyhow::anyhow!("RED:SOURCE_BARRIER_WATCHDOG: source-read test deadlocked"))
}

impl Drop for BarrierControl {
    fn drop(&mut self) {
        self.release();
    }
}

#[cfg(unix)]
fn symlink_file(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[cfg(windows)]
fn symlink_file(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_file(source, link)
}

#[cfg(unix)]
fn symlink_dir(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::unix::fs::symlink(source, link)
}

#[cfg(windows)]
fn symlink_dir(source: &Path, link: &Path) -> std::io::Result<()> {
    std::os::windows::fs::symlink_dir(source, link)
}

fn require_link(result: std::io::Result<()>, kind: &str) -> anyhow::Result<()> {
    match result {
        Ok(()) => Ok(()),
        #[cfg(windows)]
        Err(error) if error.raw_os_error() == Some(1314) => {
            anyhow::bail!(
                "Windows {kind} security coverage requires symlink privilege \
                 (ERROR_PRIVILEGE_NOT_HELD=1314); this is not a passing skip"
            )
        }
        Err(error) => Err(error.into()),
    }
}

fn write_source(root: &Path, relative: &str, source: &str) -> anyhow::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, source)?;
    Ok(())
}

fn write_bytes(root: &Path, relative: &str, source: &[u8]) -> anyhow::Result<()> {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(path, source)?;
    Ok(())
}

fn hcl_config() -> CodeGraphConfig {
    CodeGraphConfig {
        supported_languages: vec!["hcl".to_owned()],
        ..CodeGraphConfig::default()
    }
}

fn db_identity(workspace: &Path, suffix: &str) -> (std::path::PathBuf, String) {
    (
        workspace.join(format!(".engram-source-race-{suffix}")),
        format!("source-race-{suffix}-{}", Uuid::new_v4()),
    )
}

#[derive(Debug, PartialEq)]
struct GraphState {
    files: Vec<CodeFile>,
    functions: Vec<(String, String)>,
    classes: Vec<(String, String)>,
}

async fn graph_state(data_dir: &Path, branch: &str) -> anyhow::Result<GraphState> {
    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);
    let files = queries.list_code_files().await?;
    let mut functions: Vec<_> = queries
        .all_functions()
        .await?
        .into_iter()
        .map(|function| (function.name, function.body))
        .collect();
    functions.sort();
    let mut classes: Vec<_> = queries
        .all_classes()
        .await?
        .into_iter()
        .map(|class| (class.name, class.body))
        .collect();
    classes.sort();
    Ok(GraphState {
        files,
        functions,
        classes,
    })
}

#[derive(Debug, PartialEq)]
struct PublicationState {
    files: Vec<CodeFile>,
    functions: Vec<(String, String, String, String)>,
    classes: Vec<(String, String, String, String)>,
    canonical_workspace: Option<canonical::CanonicalWorkspace>,
    python_marker: Option<String>,
    generation_marker: Option<String>,
}

async fn publication_state(data_dir: &Path, branch: &str) -> anyhow::Result<PublicationState> {
    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);
    let files = queries.list_code_files().await?;
    let mut functions: Vec<_> = queries
        .all_functions()
        .await?
        .into_iter()
        .map(|function| {
            (
                function.id,
                function.name,
                function.file_path,
                function.body,
            )
        })
        .collect();
    functions.sort();
    let mut classes: Vec<_> = queries
        .all_classes()
        .await?
        .into_iter()
        .map(|class| (class.id, class.name, class.file_path, class.body))
        .collect();
    classes.sort();
    Ok(PublicationState {
        files,
        functions,
        classes,
        canonical_workspace: queries.load_index_canonical_workspace_snapshot().await?,
        python_marker: queries.python_extraction_version()?,
        generation_marker: queries.code_graph_extraction_generation()?,
    })
}

fn assert_external_absent(state: &GraphState, marker: &str) {
    assert!(
        state
            .files
            .iter()
            .all(|file| !file.path.contains("outside")),
        "RED:{marker}: external path metadata persisted: {:?}",
        state.files
    );
    assert!(
        state
            .functions
            .iter()
            .all(|(name, body)| !name.contains("external") && !body.contains("OUTSIDE_SENTINEL")),
        "RED:{marker}: external function body persisted: {:?}",
        state.functions
    );
    assert!(
        state.classes.iter().all(|(name, body)| {
            name != "hcl.block.resource.external.secret" && !body.contains("OUTSIDE_SENTINEL")
        }),
        "RED:{marker}: external HCL class body persisted: {:?}",
        state.classes
    );
}

fn assert_never_published(state: &GraphState, marker: &str, sentinel: &str) {
    assert!(
        state.files.is_empty(),
        "RED:{marker}: abort published code files: {:?}",
        state.files
    );
    assert!(
        state
            .functions
            .iter()
            .all(|(_, body)| !body.contains(sentinel)),
        "RED:{marker}: rejected function body was admitted: {:?}",
        state.functions
    );
    assert!(
        state
            .classes
            .iter()
            .all(|(_, body)| !body.contains(sentinel)),
        "RED:{marker}: rejected class body was admitted: {:?}",
        state.classes
    );
}

#[tokio::test]
async fn full_index_rejects_file_replaced_by_external_link_after_discovery() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), "victim.tf", SAFE_OLD)?;
    write_source(workspace.path(), "control.hcl", "service \"control\" {}\n")?;
    write_source(outside.path(), "secret.tfvars", EXTERNAL_SENTINEL)?;
    let (data_dir, branch) = db_identity(workspace.path(), "final-file");
    let config = hcl_config();
    let mut barrier = BarrierControl::new("victim.tf");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();
    let outside_path = outside.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        index_workspace(workspace.path(), &data_dir, &branch, &config, true),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace_path.join("victim.tf"))?;
        require_link(
            symlink_file(
                &outside_path.join("secret.tfvars"),
                &workspace_path.join("victim.tf"),
            ),
            "file-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (indexed, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    indexed?;
    let state = graph_state(&data_dir, &branch).await?;
    assert_external_absent(&state, "SOURCE_FINAL_LINK_ESCAPE");
    assert!(
        state
            .classes
            .iter()
            .any(|(name, _)| name == "hcl.block.service.control")
    );
    Ok(())
}

#[tokio::test]
async fn invalid_utf8_rust_nonforce_sync_retains_last_known_good_without_aborting()
-> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "Cargo.toml",
        "[package]\nname = \"invalid-content-sync\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    write_source(
        workspace.path(),
        "src/lib.rs",
        "pub fn retained_after_invalid_content() -> usize { 7 }\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "invalid-content-sync");
    let config = CodeGraphConfig::default();
    index_workspace(workspace.path(), &data_dir, &branch, &config, true).await?;
    let before = publication_state(&data_dir, &branch).await?;
    std::fs::write(workspace.path().join("src/lib.rs"), [0xff, 0xfe, 0xfd])?;

    let result = sync_workspace(workspace.path(), &data_dir, &branch, &config)
        .await
        .expect(
            "RED:RUST_PREPASS_CONTENT_REJECTION_ABORTED: invalid source content must retain LKG \
             without converting a content error into a capability-boundary abort",
        );
    assert!(
        result.errors.iter().any(|error| error.file == "src/lib.rs"),
        "invalid source content must remain observable: {result:?}"
    );
    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:RUST_PREPASS_CONTENT_REJECTION_MUTATED: invalid source content changed prior \
         publication"
    );
    Ok(())
}

#[tokio::test]
async fn sync_rejects_file_replaced_by_internal_link_after_discovery() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(workspace.path(), "victim.tf", SAFE_OLD)?;
    write_source(workspace.path(), "replacement.payload", INTERNAL_SENTINEL)?;
    let (data_dir, branch) = db_identity(workspace.path(), "internal-final");
    let config = hcl_config();
    sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;
    let before = publication_state(&data_dir, &branch).await?;
    let mut barrier = BarrierControl::new("victim.tf");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(workspace.path(), &data_dir, &branch, &config),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace_path.join("victim.tf"))?;
        require_link(
            symlink_file(
                Path::new("replacement.payload"),
                &workspace_path.join("victim.tf"),
            ),
            "internal-file-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    let result = synced?;
    assert!(
        result.errors.iter().any(|error| error.file == "victim.tf"),
        "RED:SOURCE_INTERNAL_FINAL_ACCEPTED: internal final link was not rejected: {result:?}"
    );
    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:SOURCE_INTERNAL_FINAL_LKG: internal final link changed graph/publication state"
    );
    Ok(())
}

#[tokio::test]
async fn sync_rejects_ancestor_replaced_by_internal_link_after_discovery() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(workspace.path(), "nested/victim.tf", SAFE_OLD)?;
    let (data_dir, branch) = db_identity(workspace.path(), "internal-ancestor");
    let config = hcl_config();
    sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;
    let before = publication_state(&data_dir, &branch).await?;
    let mut barrier = BarrierControl::new("nested/victim.tf");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(workspace.path(), &data_dir, &branch, &config),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::rename(
            workspace_path.join("nested"),
            workspace_path.join("nested-original"),
        )?;
        write_source(&workspace_path, "replacement/victim.tf", INTERNAL_SENTINEL)?;
        require_link(
            symlink_dir(Path::new("replacement"), &workspace_path.join("nested")),
            "internal-directory-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    let result = synced?;
    assert!(
        result
            .errors
            .iter()
            .any(|error| error.file == "nested/victim.tf"),
        "RED:SOURCE_INTERNAL_ANCESTOR_ACCEPTED: internal ancestor link was not rejected: {result:?}"
    );
    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:SOURCE_INTERNAL_ANCESTOR_LKG: internal ancestor link changed graph/publication state"
    );
    Ok(())
}

#[tokio::test]
async fn replaced_ignore_links_cannot_hide_and_delete_indexed_sources() -> anyhow::Result<()> {
    for ignore_name in [".gitignore", ".ignore"] {
        let workspace = tempfile::tempdir()?;
        write_source(workspace.path(), "victim.tf", SAFE_OLD)?;
        write_source(workspace.path(), ignore_name, "")?;
        write_source(workspace.path(), "ignore-rules.payload", "victim.tf\n")?;
        let (data_dir, branch) = db_identity(workspace.path(), ignore_name);
        let config = hcl_config();
        sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;
        let before = publication_state(&data_dir, &branch).await?;

        std::fs::remove_file(workspace.path().join(ignore_name))?;
        require_link(
            symlink_file(
                Path::new("ignore-rules.payload"),
                &workspace.path().join(ignore_name),
            ),
            "ignore-file-link",
        )?;
        let error = sync_workspace(workspace.path(), &data_dir, &branch, &config)
            .await
            .expect_err("rejected ignore layer must abort sync globally");

        assert_eq!(
            publication_state(&data_dir, &branch).await?,
            before,
            "RED:IGNORE_LINK_DELETED_LKG: {ignore_name} link hid and deleted victim.tf"
        );
        assert!(
            error.to_string().contains(ignore_name),
            "RED:IGNORE_LINK_AMBIENT_READ: {ignore_name} link rejection was not actionable: {error}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn rejected_ignore_layers_abort_fresh_index_without_admitting_hidden_body()
-> anyhow::Result<()> {
    const SENTINEL: &str = "IGNORE_POLICY_SENTINEL";

    for ignore_path in [".gitignore", "nested/.ignore"] {
        let workspace = tempfile::tempdir()?;
        let hidden_path = if ignore_path.starts_with("nested/") {
            "nested/hidden.tf"
        } else {
            "hidden.tf"
        };
        write_source(
            workspace.path(),
            hidden_path,
            &format!("resource \"ignored_policy\" \"hidden\" {{\n  value = \"{SENTINEL}\"\n}}\n"),
        )?;
        write_source(
            workspace.path(),
            "visible.tf",
            "resource \"visible\" \"control\" {}\n",
        )?;
        write_source(workspace.path(), "ignore-rules.payload", "hidden.tf\n")?;
        if let Some(parent) = workspace.path().join(ignore_path).parent() {
            std::fs::create_dir_all(parent)?;
        }
        let target = if ignore_path.starts_with("nested/") {
            Path::new("../ignore-rules.payload")
        } else {
            Path::new("ignore-rules.payload")
        };
        require_link(
            symlink_file(target, &workspace.path().join(ignore_path)),
            "fresh-ignore-file-link",
        )?;
        let (data_dir, branch) = db_identity(workspace.path(), ignore_path);

        let indexed =
            index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true).await;
        let state = graph_state(&data_dir, &branch).await?;
        assert_never_published(&state, "IGNORE_LAYER_PARTIAL_PUBLICATION", SENTINEL);
        let error = indexed.expect_err(
            "RED:IGNORE_LAYER_REJECTION_SWALLOWED: rejected ignore layer must abort fresh index",
        );
        assert!(
            error.to_string().contains(ignore_path),
            "ignore rejection must identify {ignore_path}: {error}"
        );
    }
    Ok(())
}

#[tokio::test]
async fn oversized_ignore_layer_aborts_fresh_index_without_admitting_hidden_body()
-> anyhow::Result<()> {
    const SENTINEL: &str = "OVERSIZED_IGNORE_SENTINEL";

    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "hidden.tf",
        &format!("resource \"oversized_ignore\" \"hidden\" {{\n  value = \"{SENTINEL}\"\n}}\n"),
    )?;
    write_source(
        workspace.path(),
        "visible.tf",
        "resource \"visible\" \"control\" {}\n",
    )?;
    write_source(
        workspace.path(),
        ".gitignore",
        &format!("hidden.tf\n#{}\n", "x".repeat(1024 * 1024)),
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "oversized-ignore");

    let indexed = index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true).await;
    let state = graph_state(&data_dir, &branch).await?;
    assert_never_published(&state, "OVERSIZED_IGNORE_PARTIAL_PUBLICATION", SENTINEL);
    let error = indexed.expect_err(
        "RED:OVERSIZED_IGNORE_SWALLOWED: oversized ignore layer must abort fresh index",
    );
    assert!(
        error.to_string().contains(".gitignore")
            && error.to_string().contains("exceeds capability read limit"),
        "oversized ignore rejection must be actionable: {error}"
    );
    Ok(())
}

#[tokio::test]
async fn git_info_exclude_remains_authoritative_through_capability_reads() -> anyhow::Result<()> {
    const SENTINEL: &str = "GIT_INFO_EXCLUDE_SENTINEL";

    let workspace = tempfile::tempdir()?;
    write_source(workspace.path(), ".git/info/exclude", "*.tf\n")?;
    write_source(workspace.path(), ".gitignore", "!visible.tf\n")?;
    write_source(
        workspace.path(),
        "hidden.tf",
        &format!("resource \"info_excluded\" \"hidden\" {{ value = \"{SENTINEL}\" }}\n"),
    )?;
    write_source(
        workspace.path(),
        "visible.tf",
        "resource \"visible\" \"control\" {}\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "git-info-exclude");

    let result = index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true).await?;
    assert!(
        result.errors.is_empty(),
        "ordinary .git/info/exclude must be readable: {result:?}"
    );
    let state = graph_state(&data_dir, &branch).await?;
    assert_eq!(
        state
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["visible.tf"]
    );
    assert!(
        state
            .classes
            .iter()
            .any(|(name, _)| name == "hcl.block.resource.visible.control"),
        "visible control was not indexed: {:?}",
        state.classes
    );
    assert!(
        state
            .classes
            .iter()
            .all(|(name, body)| !name.contains("info_excluded") && !body.contains(SENTINEL)),
        ".git/info/exclude hidden body was indexed: {:?}",
        state.classes
    );
    Ok(())
}

#[tokio::test]
async fn linked_worktree_common_git_info_exclude_remains_authoritative() -> anyhow::Result<()> {
    const SENTINEL: &str = "WORKTREE_POINTER_SENTINEL";

    let workspace = tempfile::tempdir()?;
    let linked_git_directory = tempfile::tempdir()?;
    let common_git_directory = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        ".git",
        &format!("gitdir: {}\n", linked_git_directory.path().display()),
    )?;
    write_source(
        linked_git_directory.path(),
        "commondir",
        &format!("{}\n", common_git_directory.path().display()),
    )?;
    write_source(common_git_directory.path(), "info/exclude", "hidden.tf\n")?;
    write_source(
        workspace.path(),
        "hidden.tf",
        &format!(
            "resource \"worktree_pointer\" \"must_not_publish\" {{ value = \"{SENTINEL}\" }}\n"
        ),
    )?;
    write_source(
        workspace.path(),
        "visible.tf",
        "resource \"visible\" \"control\" {}\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "git-file-pointer");

    let indexed =
        index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true).await?;
    assert!(
        indexed.errors.is_empty(),
        "linked-worktree metadata should load safely: {indexed:?}"
    );
    let state = graph_state(&data_dir, &branch).await?;
    assert!(
        state.files.iter().all(|file| file.path != "hidden.tf"),
        "RED:LINKED_WORKTREE_EXCLUDE_IGNORED: hidden file was indexed: {:?}",
        state.files
    );
    assert!(
        state
            .classes
            .iter()
            .all(|(_, body)| !body.contains(SENTINEL)),
        "linked-worktree excluded sentinel body was persisted: {:?}",
        state.classes
    );
    assert!(
        state.files.iter().any(|file| file.path == "visible.tf"),
        "visible control must remain indexable"
    );
    Ok(())
}

#[tokio::test]
async fn malformed_git_pointer_aborts_before_source_publication() -> anyhow::Result<()> {
    const SENTINEL: &str = "MALFORMED_GIT_POINTER_SENTINEL";

    let workspace = tempfile::tempdir()?;
    write_source(workspace.path(), ".git", "not-a-gitdir-pointer\n")?;
    write_source(
        workspace.path(),
        "victim.tf",
        &format!("resource \"malformed\" \"pointer\" {{ value = \"{SENTINEL}\" }}\n"),
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "malformed-git-pointer");

    let indexed = index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true).await;
    let state = graph_state(&data_dir, &branch).await?;
    assert_never_published(&state, "MALFORMED_GIT_POINTER_PUBLICATION", SENTINEL);
    let error = indexed.expect_err("malformed .git pointer must fail closed");
    assert!(
        error.to_string().contains(".git") && error.to_string().contains("gitdir"),
        "malformed pointer rejection must identify the gitdir field: {error}"
    );
    Ok(())
}

async fn assert_linked_metadata_replacement_rejected(replace_ancestor: bool) -> anyhow::Result<()> {
    const SENTINEL: &str = "LINKED_METADATA_REPLACEMENT_SENTINEL";

    let workspace = tempfile::tempdir()?;
    let linked_git_directory = tempfile::tempdir()?;
    let common_git_directory = tempfile::tempdir()?;
    let external_metadata = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        ".git",
        &format!("gitdir: {}\n", linked_git_directory.path().display()),
    )?;
    write_source(
        linked_git_directory.path(),
        "commondir",
        &format!("{}\n", common_git_directory.path().display()),
    )?;
    write_source(common_git_directory.path(), "info/exclude", "safe.tf\n")?;
    write_source(
        external_metadata.path(),
        "info/exclude",
        &format!("victim.tf\n# {SENTINEL}\n"),
    )?;
    write_source(
        workspace.path(),
        "victim.tf",
        &format!("resource \"metadata\" \"victim\" {{ value = \"{SENTINEL}\" }}\n"),
    )?;

    if replace_ancestor {
        std::fs::rename(
            common_git_directory.path().join("info"),
            common_git_directory.path().join("info-original"),
        )?;
        require_link(
            symlink_dir(
                &external_metadata.path().join("info"),
                &common_git_directory.path().join("info"),
            ),
            "linked-metadata-ancestor",
        )?;
    } else {
        std::fs::remove_file(common_git_directory.path().join("info/exclude"))?;
        require_link(
            symlink_file(
                &external_metadata.path().join("info/exclude"),
                &common_git_directory.path().join("info/exclude"),
            ),
            "linked-metadata-final",
        )?;
    }

    let suffix = if replace_ancestor {
        "linked-metadata-ancestor"
    } else {
        "linked-metadata-final"
    };
    let (data_dir, branch) = db_identity(workspace.path(), suffix);
    let indexed = index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true).await;
    let state = graph_state(&data_dir, &branch).await?;
    assert_never_published(&state, "LINKED_METADATA_REPLACEMENT_PUBLICATION", SENTINEL);
    let error = indexed.expect_err("linked metadata replacement must fail closed");
    assert!(
        error.to_string().contains("exclude") || error.to_string().contains(".git"),
        "metadata replacement rejection must be actionable: {error}"
    );
    Ok(())
}

#[tokio::test]
async fn linked_metadata_final_replacement_aborts_before_publication() -> anyhow::Result<()> {
    assert_linked_metadata_replacement_rejected(false).await
}

#[tokio::test]
async fn linked_metadata_ancestor_replacement_aborts_before_publication() -> anyhow::Result<()> {
    assert_linked_metadata_replacement_rejected(true).await
}

#[tokio::test]
async fn invalid_configured_exclude_pattern_aborts_before_publication() -> anyhow::Result<()> {
    const SENTINEL: &str = "INVALID_EXCLUDE_SENTINEL";

    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "hidden.tf",
        &format!("resource \"excluded\" \"hidden\" {{ value = \"{SENTINEL}\" }}\n"),
    )?;
    write_source(
        workspace.path(),
        "visible.tf",
        "resource \"visible\" \"control\" {}\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "invalid-configured-exclude");
    let mut config = hcl_config();
    config.exclude_patterns = vec!["hidden.tf".to_owned(), "[".to_owned()];

    let indexed = index_workspace(workspace.path(), &data_dir, &branch, &config, true).await;
    let state = graph_state(&data_dir, &branch).await?;
    assert_never_published(
        &state,
        "INVALID_EXCLUDE_PATTERN_PARTIAL_PUBLICATION",
        SENTINEL,
    );
    let error = indexed.expect_err(
        "RED:INVALID_EXCLUDE_PATTERN_ACCEPTED: invalid configured excludes must be fatal",
    );
    assert!(
        error.to_string().contains("exclude") && error.to_string().contains('['),
        "invalid configured exclude rejection must identify the pattern: {error}"
    );
    Ok(())
}

#[tokio::test]
async fn exclude_override_build_failure_aborts_before_publication() -> anyhow::Result<()> {
    const SENTINEL: &str = "OVERRIDE_BUILD_SENTINEL";

    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "hidden.tf",
        &format!("resource \"excluded\" \"hidden\" {{ value = \"{SENTINEL}\" }}\n"),
    )?;
    write_source(
        workspace.path(),
        "visible.tf",
        "resource \"visible\" \"control\" {}\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "override-build-failure");
    let mut config = hcl_config();
    config.exclude_patterns = vec!["hidden.tf".to_owned()];

    let indexed = FORCE_OVERRIDE_BUILD_FAILURE
        .scope(
            true,
            index_workspace(workspace.path(), &data_dir, &branch, &config, true),
        )
        .await;
    let state = graph_state(&data_dir, &branch).await?;
    assert_never_published(
        &state,
        "OVERRIDE_BUILD_FAILURE_PARTIAL_PUBLICATION",
        SENTINEL,
    );
    let error = indexed
        .expect_err("RED:OVERRIDE_BUILD_FAILURE_ACCEPTED: override build failure must be fatal");
    assert!(
        error.to_string().contains("exclude") && error.to_string().contains("build"),
        "override build rejection must identify the failed policy phase: {error}"
    );
    Ok(())
}

#[tokio::test]
async fn ignored_subtree_diagnostics_do_not_abort_or_suppress_certification() -> anyhow::Result<()>
{
    const SENTINEL: &str = "IGNORED_SUBTREE_SENTINEL";

    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), ".gitignore", "ignored/\n")?;
    write_source(workspace.path(), "ignore-rules.payload", "*.tf\n")?;
    write_source(
        workspace.path(),
        "ignored/hidden.tf",
        &format!("resource \"ignored\" \"hidden\" {{ value = \"{SENTINEL}\" }}\n"),
    )?;
    require_link(
        symlink_file(
            Path::new("../ignore-rules.payload"),
            &workspace.path().join("ignored/.gitignore"),
        ),
        "ignored-subtree-ignore-link",
    )?;
    write_bytes(workspace.path(), "ignored/.ignore", &[0xff, 0xfe, b'\n'])?;
    write_source(outside.path(), "outside.tf", EXTERNAL_SENTINEL)?;
    require_link(
        symlink_dir(outside.path(), &workspace.path().join("ignored/blocked")),
        "ignored-subtree-blocked-link",
    )?;
    write_source(workspace.path(), "active/.ignore", "hidden.tf\n")?;
    write_source(workspace.path(), "active/hidden.tf", INTERNAL_SENTINEL)?;
    write_source(workspace.path(), "active/visible.tf", SAFE_NEW)?;
    write_source(
        workspace.path(),
        "visible.tf",
        "resource \"visible\" \"control\" {}\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "ignored-subtree-diagnostics");

    let result = index_workspace(workspace.path(), &data_dir, &branch, &hcl_config(), true)
        .await
        .map_err(|error| {
            anyhow::anyhow!(
                "RED:IGNORED_SUBTREE_DIAGNOSTIC_ABORT: ignored diagnostics aborted publication: \
                 {error}"
            )
        })?;
    assert!(
        result.errors.is_empty(),
        "ignored-subtree diagnostics must be localized: {result:?}"
    );
    let state = publication_state(&data_dir, &branch).await?;
    assert_eq!(
        state
            .files
            .iter()
            .map(|file| file.path.as_str())
            .collect::<Vec<_>>(),
        ["active/visible.tf", "visible.tf"],
        "nested ignore semantics outside the excluded tree must remain authoritative"
    );
    assert!(
        state
            .classes
            .iter()
            .all(|(_, _, _, body)| !body.contains(SENTINEL) && !body.contains("OUTSIDE_SENTINEL")),
        "ignored or blocked subtree source was published: {:?}",
        state.classes
    );
    assert!(
        state.canonical_workspace.is_some()
            && state.python_marker.as_deref() == Some(PYTHON_CANONICAL_EXTRACTION_VERSION)
            && state.generation_marker.as_deref() == Some(CODE_GRAPH_EXTRACTION_GENERATION),
        "RED:IGNORED_SUBTREE_SUPPRESSED_CERTIFICATION: ignored diagnostics suppressed markers: \
         {state:?}"
    );
    Ok(())
}

#[tokio::test]
async fn regular_root_and_nested_ignore_rules_remain_authoritative() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(workspace.path(), ".gitignore", "root-hidden.tf\n")?;
    write_source(workspace.path(), "root-hidden.tf", INTERNAL_SENTINEL)?;
    write_source(workspace.path(), "keep.tf", SAFE_OLD)?;
    write_source(workspace.path(), "nested/.ignore", "hidden.tf\n")?;
    write_source(workspace.path(), "nested/hidden.tf", INTERNAL_SENTINEL)?;
    write_source(workspace.path(), "nested/keep.tf", SAFE_NEW)?;
    let (data_dir, branch) = db_identity(workspace.path(), "regular-ignore");
    let result = sync_workspace(workspace.path(), &data_dir, &branch, &hcl_config()).await?;

    assert!(result.errors.is_empty(), "regular ignores must be readable");
    let state = graph_state(&data_dir, &branch).await?;
    let paths: Vec<_> = state.files.iter().map(|file| file.path.as_str()).collect();
    assert_eq!(paths, ["keep.tf", "nested/keep.tf"]);
    Ok(())
}

#[tokio::test]
async fn rejected_manifest_aborts_forced_index_before_publication_mutation() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "Cargo.toml",
        "[package]\nname = \"safe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    write_source(workspace.path(), "src/lib.rs", "pub fn safe() {}\n")?;
    write_source(
        workspace.path(),
        "alternate-manifest.payload",
        "[package]\nname = \"replacement\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "manifest-reject");
    let config = CodeGraphConfig::default();
    index_workspace(workspace.path(), &data_dir, &branch, &config, true).await?;
    let before = publication_state(&data_dir, &branch).await?;

    std::fs::remove_file(workspace.path().join("Cargo.toml"))?;
    require_link(
        symlink_file(
            Path::new("alternate-manifest.payload"),
            &workspace.path().join("Cargo.toml"),
        ),
        "manifest-file-link",
    )?;
    let error = index_workspace(workspace.path(), &data_dir, &branch, &config, true)
        .await
        .expect_err("RED:MANIFEST_REJECTION_SWALLOWED: rejected manifest must abort");
    assert!(
        error.to_string().contains("Cargo.toml"),
        "manifest rejection must identify Cargo.toml: {error}"
    );
    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:MANIFEST_REJECTION_MUTATED: rejected manifest changed graph/snapshot/markers"
    );
    Ok(())
}

#[tokio::test]
async fn rejected_rust_prepass_aborts_forced_index_before_publication_mutation()
-> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "Cargo.toml",
        "[package]\nname = \"safe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    write_source(workspace.path(), "src/lib.rs", "pub fn safe() {}\n")?;
    write_source(
        workspace.path(),
        "replacement.payload",
        "#[path = \"redirect.rs\"]\npub mod unsafe_replacement;\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "prepass-reject");
    let config = CodeGraphConfig::default();
    index_workspace(workspace.path(), &data_dir, &branch, &config, true).await?;
    let before = publication_state(&data_dir, &branch).await?;
    let mut barrier = BarrierControl::new("src/lib.rs");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        index_workspace(workspace.path(), &data_dir, &branch, &config, true),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace_path.join("src/lib.rs"))?;
        require_link(
            symlink_file(
                Path::new("../replacement.payload"),
                &workspace_path.join("src/lib.rs"),
            ),
            "Rust-prepass-file-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };
    let (indexed, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    let error =
        indexed.expect_err("RED:RUST_PREPASS_REJECTION_SWALLOWED: rejected prepass must abort");
    assert!(
        error.to_string().contains("src/lib.rs"),
        "prepass rejection must identify src/lib.rs: {error}"
    );
    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:RUST_PREPASS_MUTATED: rejected prepass changed graph/snapshot/markers"
    );
    Ok(())
}

#[tokio::test]
async fn oversized_rust_prepass_aborts_before_publication_mutation() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "Cargo.toml",
        "[package]\nname = \"safe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    let rust_source = format!(
        "#[path = \"redirect.rs\"]\npub mod unsafe_mapping;\n\
         pub fn retained_publication() -> usize {{ 7 }}\n// {}\n",
        "prepass-padding".repeat(64)
    );
    write_source(workspace.path(), "src/lib.rs", &rust_source)?;
    let (data_dir, branch) = db_identity(workspace.path(), "prepass-oversized");
    let initial_config = CodeGraphConfig {
        max_file_size_bytes: 4096,
        ..CodeGraphConfig::default()
    };
    index_workspace(workspace.path(), &data_dir, &branch, &initial_config, true).await?;
    let before = publication_state(&data_dir, &branch).await?;
    let restrictive_config = CodeGraphConfig {
        max_file_size_bytes: 128,
        ..CodeGraphConfig::default()
    };

    let error = index_workspace(
        workspace.path(),
        &data_dir,
        &branch,
        &restrictive_config,
        true,
    )
    .await
    .expect_err("RED:RUST_PREPASS_OVERSIZE_SWALLOWED: oversized Rust prepass must abort");
    assert!(
        error.to_string().contains("src/lib.rs")
            && error.to_string().contains("Rust unsafe-module prepass")
            && error.to_string().contains("128"),
        "oversized Rust prepass error must identify file, phase, and limit: {error}"
    );
    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:RUST_PREPASS_OVERSIZE_MUTATED: oversized prepass changed prior publication"
    );
    Ok(())
}

#[tokio::test]
async fn rust_rewrite_after_prepass_aborts_before_publication_mutation() -> anyhow::Result<()> {
    const SAFE_SOURCE: &str = "pub fn retained_publication() -> usize { 7 }\n";
    const REMAPPED_SOURCE: &str = "#[path = \"redirect.rs\"]\npub mod remapped;\n\
         #[cfg(unix)]\npub mod conditional;\n\
         pub fn retained_publication() -> usize { 11 }\n";

    for (suffix, prepass_source, rewritten_source) in [
        ("adds-remap", SAFE_SOURCE, REMAPPED_SOURCE),
        ("removes-remap", REMAPPED_SOURCE, SAFE_SOURCE),
    ] {
        let workspace = tempfile::tempdir()?;
        write_source(
            workspace.path(),
            "Cargo.toml",
            "[package]\nname = \"safe\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
        )?;
        write_source(workspace.path(), "src/lib.rs", prepass_source)?;
        let (data_dir, branch) = db_identity(workspace.path(), suffix);
        let config = CodeGraphConfig::default();
        index_workspace(workspace.path(), &data_dir, &branch, &config, true).await?;
        let before = publication_state(&data_dir, &branch).await?;
        let mut barrier = BarrierControl::new(POST_PREPASS_TEST_BARRIER);
        let hook = barrier.take_hook()?;
        let workspace_path = workspace.path().to_path_buf();

        let operation = SOURCE_READ_TEST_HOOK.scope(
            hook,
            index_workspace(workspace.path(), &data_dir, &branch, &config, true),
        );
        let controller = async move {
            barrier.wait_until_reached().await?;
            std::fs::write(workspace_path.join("src/lib.rs"), rewritten_source)?;
            barrier.release();
            Ok::<(), anyhow::Error>(())
        };
        let (indexed, controlled) = bounded_join(operation, controller).await?;
        controlled?;
        let error = indexed
            .expect_err("RED:RUST_POST_PREPASS_REWRITE_ACCEPTED: regular Rust rewrite must abort");
        assert!(
            error.to_string().contains("src/lib.rs")
                && (error.to_string().contains("snapshot")
                    || error.to_string().contains("changed")),
            "Rust snapshot mismatch must identify the changed input: {error}"
        );
        assert_eq!(
            publication_state(&data_dir, &branch).await?,
            before,
            "RED:RUST_POST_PREPASS_REWRITE_MUTATED: {suffix} changed graph/snapshot/markers"
        );
    }
    Ok(())
}

#[tokio::test]
async fn blocked_prefix_does_not_suppress_unrelated_authoritative_deletion() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), "blocked/victim.tf", SAFE_OLD)?;
    write_source(
        workspace.path(),
        "authoritatively-deleted.tf",
        "resource \"deleted\" \"gone\" {}\n",
    )?;
    let (data_dir, branch) = db_identity(workspace.path(), "localized-block");
    let config = hcl_config();
    sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;

    std::fs::rename(
        workspace.path().join("blocked"),
        outside.path().join("blocked-target"),
    )?;
    require_link(
        symlink_dir(
            &outside.path().join("blocked-target"),
            &workspace.path().join("blocked"),
        ),
        "blocked-prefix-link",
    )?;
    std::fs::remove_file(workspace.path().join("authoritatively-deleted.tf"))?;
    let result = sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;
    let state = graph_state(&data_dir, &branch).await?;

    assert!(
        state
            .files
            .iter()
            .any(|file| file.path == "blocked/victim.tf"),
        "blocked prefix must retain its prior graph: {:?}",
        state.files
    );
    assert!(
        state
            .files
            .iter()
            .all(|file| file.path != "authoritatively-deleted.tf"),
        "RED:LOCAL_BLOCK_SUPPRESSED_GLOBAL_DELETE: unrelated authoritative deletion survived: \
         {:?}; result={result:?}",
        state.files
    );
    assert!(
        state
            .classes
            .iter()
            .any(|(name, _)| name == "hcl.block.resource.safe.old")
    );
    assert!(
        state
            .classes
            .iter()
            .all(|(name, _)| name != "hcl.block.resource.deleted.gone")
    );
    Ok(())
}

#[cfg(unix)]
async fn assert_discovery_root_replacement_preserves_lkg(
    replace_ancestor: bool,
) -> anyhow::Result<()> {
    let base = tempfile::tempdir()?;
    let database = tempfile::tempdir()?;
    let selected = base.path().join("selected");
    let workspace = selected.join("workspace");
    std::fs::create_dir_all(&workspace)?;
    write_source(&workspace, "retained.tf", SAFE_OLD)?;
    write_source(
        &workspace,
        "authoritatively-deleted.tf",
        "resource \"deleted\" \"gone\" {}\n",
    )?;
    let branch = format!(
        "source-race-discovery-root-{}-{}",
        replace_ancestor,
        Uuid::new_v4()
    );
    let config = hcl_config();
    sync_workspace(&workspace, database.path(), &branch, &config).await?;
    std::fs::remove_file(workspace.join("authoritatively-deleted.tf"))?;

    let mut barrier = BarrierControl::new(SOURCE_DISCOVERY_TEST_BARRIER);
    let hook = barrier.take_hook()?;
    let worker_workspace = workspace.clone();
    let controller_workspace = workspace.clone();
    let controller_selected = selected.clone();
    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(&worker_workspace, database.path(), &branch, &config),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        if replace_ancestor {
            std::fs::rename(&controller_selected, base.path().join("selected-original"))?;
            std::fs::create_dir_all(&controller_workspace)?;
        } else {
            std::fs::rename(
                &controller_workspace,
                controller_selected.join("workspace-original"),
            )?;
            std::fs::create_dir_all(&controller_workspace)?;
        }
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    synced?;
    let state = graph_state(database.path(), &branch).await?;
    assert!(
        state.files.iter().any(|file| file.path == "retained.tf"),
        "RED:DISCOVERY_ROOT_REPLACEMENT_DELETED_LKG: retained source was evicted after \
         replacement; ancestor={replace_ancestor}; files={:?}",
        state.files
    );
    assert!(
        state
            .files
            .iter()
            .all(|file| file.path != "authoritatively-deleted.tf"),
        "unrelated authoritative deletion must still reconcile; ancestor={replace_ancestor}"
    );
    Ok(())
}

#[cfg(unix)]
#[tokio::test]
async fn final_workspace_replacement_cannot_redirect_discovery_or_deletion() -> anyhow::Result<()> {
    assert_discovery_root_replacement_preserves_lkg(false).await
}

#[cfg(unix)]
#[tokio::test]
async fn workspace_ancestor_replacement_cannot_redirect_discovery_or_deletion() -> anyhow::Result<()>
{
    assert_discovery_root_replacement_preserves_lkg(true).await
}

#[cfg(windows)]
#[tokio::test]
async fn unicode_case_variant_blocked_prefix_retains_last_known_good_graph() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), "blöcked/victim.tf", SAFE_OLD)?;
    let (data_dir, branch) = db_identity(workspace.path(), "case-variant-block");
    let config = hcl_config();
    sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;
    let before = publication_state(&data_dir, &branch).await?;

    std::fs::rename(
        workspace.path().join("blöcked"),
        outside.path().join("blocked-target"),
    )?;
    require_link(
        symlink_dir(
            &outside.path().join("blocked-target"),
            &workspace.path().join("BLÖCKED"),
        ),
        "case-variant-blocked-prefix-link",
    )?;
    sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;

    assert_eq!(
        publication_state(&data_dir, &branch).await?,
        before,
        "RED:WINDOWS_UNICODE_CASE_BLOCKED_LKG: Unicode case-variant blocked prefix deleted prior \
         publication"
    );
    Ok(())
}

#[tokio::test]
async fn barrier_observes_operation_early_exit_without_deadlock() -> anyhow::Result<()> {
    let mut barrier = BarrierControl::new("never-reached.tf");
    let hook = barrier.take_hook()?;
    let operation = SOURCE_READ_TEST_HOOK.scope(hook, async {
        Err::<(), anyhow::Error>(anyhow::anyhow!("intentional operation early exit"))
    });
    let controller = async move {
        let closed = barrier.wait_until_reached().await;
        assert!(
            closed.is_err(),
            "operation exit must close the barrier reached channel"
        );
        Ok::<(), anyhow::Error>(())
    };

    let (operation_result, controller_result) = bounded_join(operation, controller).await?;
    assert!(operation_result.is_err());
    controller_result?;
    Ok(())
}

#[tokio::test]
async fn barrier_controller_error_resumes_waiting_operation() -> anyhow::Result<()> {
    let mut barrier = BarrierControl::new("victim.tf");
    let hook = barrier.take_hook()?;
    let operation = SOURCE_READ_TEST_HOOK.scope(hook, async {
        source_read_test_barrier("victim.tf").await;
        Ok::<(), anyhow::Error>(())
    });
    let controller = async move {
        barrier.wait_until_reached().await?;
        Err::<(), anyhow::Error>(anyhow::anyhow!("intentional controller early exit"))
    };

    let (operation_result, controller_result) = bounded_join(operation, controller).await?;
    operation_result?;
    assert!(controller_result.is_err());
    Ok(())
}

#[tokio::test]
async fn cold_sync_rejects_ancestor_replaced_by_external_link_after_discovery() -> anyhow::Result<()>
{
    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), "nested/victim.tf", SAFE_OLD)?;
    write_source(workspace.path(), "control.hcl", "service \"control\" {}\n")?;
    write_source(outside.path(), "victim.tf", EXTERNAL_SENTINEL)?;
    let (data_dir, branch) = db_identity(workspace.path(), "ancestor");
    let config = hcl_config();
    let mut barrier = BarrierControl::new("nested/victim.tf");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();
    let outside_path = outside.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(workspace.path(), &data_dir, &branch, &config),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::rename(
            workspace_path.join("nested"),
            workspace_path.join("nested-original"),
        )?;
        require_link(
            symlink_dir(&outside_path, &workspace_path.join("nested")),
            "directory-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    synced?;
    let state = graph_state(&data_dir, &branch).await?;
    assert_external_absent(&state, "SOURCE_ANCESTOR_LINK_ESCAPE");
    assert!(
        state
            .classes
            .iter()
            .any(|(name, _)| name == "hcl.block.service.control")
    );
    Ok(())
}

#[tokio::test]
async fn explicit_sync_rejection_retains_last_known_good_graph() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), "victim.tf", SAFE_OLD)?;
    write_source(outside.path(), "secret.tf", EXTERNAL_SENTINEL)?;
    let (data_dir, branch) = db_identity(workspace.path(), "lkg");
    let config = hcl_config();
    sync_workspace(workspace.path(), &data_dir, &branch, &config).await?;
    write_source(workspace.path(), "victim.tf", SAFE_NEW)?;
    let before = graph_state(&data_dir, &branch).await?;
    let mut barrier = BarrierControl::new("victim.tf");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();
    let outside_path = outside.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(workspace.path(), &data_dir, &branch, &config),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace_path.join("victim.tf"))?;
        require_link(
            symlink_file(
                &outside_path.join("secret.tf"),
                &workspace_path.join("victim.tf"),
            ),
            "file-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    synced?;
    let after = graph_state(&data_dir, &branch).await?;
    assert_external_absent(&after, "SOURCE_SYNC_LKG_ESCAPE");
    assert_eq!(
        after, before,
        "RED:SOURCE_SYNC_LKG_REPLACED: rejected source replacement changed the graph"
    );
    Ok(())
}

#[tokio::test]
async fn regular_file_controls_remain_indexable() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(workspace.path(), "victim.tf", SAFE_OLD)?;
    write_source(workspace.path(), "replacement.tmp", SAFE_NEW)?;
    let (data_dir, branch) = db_identity(workspace.path(), "regular");
    let config = hcl_config();
    let mut barrier = BarrierControl::new("victim.tf");
    let hook = barrier.take_hook()?;
    let workspace_path = workspace.path().to_path_buf();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        index_workspace(workspace.path(), &data_dir, &branch, &config, true),
    );
    let controller = async move {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace_path.join("victim.tf"))?;
        std::fs::rename(
            workspace_path.join("replacement.tmp"),
            workspace_path.join("victim.tf"),
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (indexed, controlled) = bounded_join(operation, controller).await?;
    controlled?;
    indexed?;
    let state = graph_state(&data_dir, &branch).await?;
    assert!(
        state
            .classes
            .iter()
            .any(|(name, _)| name == "hcl.block.resource.safe.new")
    );
    Ok(())
}

#[tokio::test]
async fn capability_directory_enumeration_classifies_without_following_links() -> anyhow::Result<()>
{
    use crate::services::workspace_source::CapabilityEntryKind;

    let workspace = tempfile::tempdir()?;
    let outside = tempfile::tempdir()?;
    write_source(workspace.path(), "visible.tf", SAFE_OLD)?;
    write_source(
        workspace.path(),
        "nested/child.hcl",
        "service \"child\" {}\n",
    )?;
    write_source(outside.path(), "external.tf", EXTERNAL_SENTINEL)?;
    require_link(
        symlink_file(
            &outside.path().join("external.tf"),
            &workspace.path().join("linked.tf"),
        ),
        "enumeration-file-link",
    )?;

    let reader = WorkspaceSourceReader::open(workspace.path()).await?;
    let entries = reader
        .list_directory_blocking(None)
        .map_err(|error| anyhow::anyhow!(error.to_string()))?;

    assert!(
        entries
            .iter()
            .any(|entry| { entry.name == "visible.tf" && entry.kind == CapabilityEntryKind::File })
    );
    assert!(
        entries.iter().any(|entry| {
            entry.name == "nested" && entry.kind == CapabilityEntryKind::Directory
        })
    );
    assert!(
        entries
            .iter()
            .any(|entry| { entry.name == "linked.tf" && entry.kind == CapabilityEntryKind::Other })
    );
    Ok(())
}

#[tokio::test]
async fn oversized_rust_nonforce_sync_evicts_stale_file_without_aborting() -> anyhow::Result<()> {
    let workspace = tempfile::tempdir()?;
    write_source(
        workspace.path(),
        "Cargo.toml",
        "[package]\nname = \"oversized-sync\"\nversion = \"0.1.0\"\nedition = \"2024\"\n",
    )?;
    let source = format!(
        "pub fn stale_oversized_symbol() -> usize {{ 7 }}\n// {}\n",
        "oversized-sync-padding".repeat(64)
    );
    write_source(workspace.path(), "src/lib.rs", &source)?;
    let (data_dir, branch) = db_identity(workspace.path(), "oversized-nonforce-sync");
    let initial_config = CodeGraphConfig {
        max_file_size_bytes: 4096,
        ..CodeGraphConfig::default()
    };
    index_workspace(workspace.path(), &data_dir, &branch, &initial_config, true).await?;
    let restrictive_config = CodeGraphConfig {
        max_file_size_bytes: 128,
        ..CodeGraphConfig::default()
    };

    let result = sync_workspace(workspace.path(), &data_dir, &branch, &restrictive_config)
        .await
        .expect(
            "RED:RUST_PREPASS_OVERSIZE_NONFORCE_ABORTED: routine sync must evict an oversized Rust \
         file without aborting the workspace pass",
        );
    assert_eq!(result.oversized_files_skipped, 1);
    let state = graph_state(&data_dir, &branch).await?;
    assert!(
        state.files.iter().all(|file| file.path != "src/lib.rs"),
        "RED:RUST_PREPASS_OVERSIZE_STALE_FILE: oversized Rust code_file survived: {:?}",
        state.files
    );
    assert!(
        state
            .functions
            .iter()
            .all(|(name, _)| name != "stale_oversized_symbol"),
        "RED:RUST_PREPASS_OVERSIZE_STALE_SYMBOL: oversized Rust symbol survived: {:?}",
        state.functions
    );
    Ok(())
}
