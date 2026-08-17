use std::path::Path;
use std::sync::Arc;

use tokio::sync::{Mutex, oneshot};

use super::*;

const SAFE_OLD: &str = "resource \"safe\" \"old\" {\n  value = var.old\n}\n";
const SAFE_NEW: &str = "resource \"safe\" \"new\" {\n  value = var.new\n}\n";
const EXTERNAL_SENTINEL: &str =
    "resource \"external\" \"secret\" {\n  secret = \"OUTSIDE_SENTINEL\"\n}\n";

struct BarrierControl {
    hook: SourceReadTestHook,
    reached: Option<oneshot::Receiver<()>>,
    resume: Option<oneshot::Sender<()>>,
}

impl BarrierControl {
    fn new(target: &str) -> Self {
        let (reached_tx, reached) = oneshot::channel();
        let (resume, resume_rx) = oneshot::channel();
        Self {
            hook: SourceReadTestHook {
                target: target.to_owned(),
                reached: Arc::new(Mutex::new(Some(reached_tx))),
                resume: Arc::new(Mutex::new(Some(resume_rx))),
            },
            reached: Some(reached),
            resume: Some(resume),
        }
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

async fn graph_state(
    data_dir: &Path,
    branch: &str,
) -> anyhow::Result<(Vec<CodeFile>, Vec<String>)> {
    let db = connect_db(data_dir, branch).await?;
    let queries = CodeGraphQueries::new(db);
    let files = queries.list_code_files().await?;
    let classes = queries
        .all_classes()
        .await?
        .into_iter()
        .map(|class| class.name)
        .collect();
    Ok((files, classes))
}

fn assert_external_absent(files: &[CodeFile], classes: &[String], marker: &str) {
    assert!(
        files.iter().all(|file| {
            !file.content_hash.contains("OUTSIDE_SENTINEL") && !file.path.contains("outside")
        }),
        "RED:{marker}: external path/body metadata persisted: {files:?}"
    );
    assert!(
        classes
            .iter()
            .all(|name| name != "hcl.block.resource.external.secret"),
        "RED:{marker}: external HCL symbol persisted: {classes:?}"
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
    let hook = barrier.hook.clone();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        index_workspace(workspace.path(), &data_dir, &branch, &config, true),
    );
    let controller = async {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace.path().join("victim.tf"))?;
        require_link(
            symlink_file(
                &outside.path().join("secret.tfvars"),
                &workspace.path().join("victim.tf"),
            ),
            "file-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (indexed, controlled) = tokio::join!(operation, controller);
    controlled?;
    indexed?;
    let (files, classes) = graph_state(&data_dir, &branch).await?;
    assert_external_absent(&files, &classes, "SOURCE_FINAL_LINK_ESCAPE");
    assert!(
        classes
            .iter()
            .any(|name| name == "hcl.block.service.control")
    );
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
    let hook = barrier.hook.clone();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(workspace.path(), &data_dir, &branch, &config),
    );
    let controller = async {
        barrier.wait_until_reached().await?;
        std::fs::rename(
            workspace.path().join("nested"),
            workspace.path().join("nested-original"),
        )?;
        require_link(
            symlink_dir(outside.path(), &workspace.path().join("nested")),
            "directory-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = tokio::join!(operation, controller);
    controlled?;
    synced?;
    let (files, classes) = graph_state(&data_dir, &branch).await?;
    assert_external_absent(&files, &classes, "SOURCE_ANCESTOR_LINK_ESCAPE");
    assert!(
        classes
            .iter()
            .any(|name| name == "hcl.block.service.control")
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
    let hook = barrier.hook.clone();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        sync_workspace(workspace.path(), &data_dir, &branch, &config),
    );
    let controller = async {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace.path().join("victim.tf"))?;
        require_link(
            symlink_file(
                &outside.path().join("secret.tf"),
                &workspace.path().join("victim.tf"),
            ),
            "file-link",
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (synced, controlled) = tokio::join!(operation, controller);
    controlled?;
    synced?;
    let after = graph_state(&data_dir, &branch).await?;
    assert_external_absent(&after.0, &after.1, "SOURCE_SYNC_LKG_ESCAPE");
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
    let hook = barrier.hook.clone();

    let operation = SOURCE_READ_TEST_HOOK.scope(
        hook,
        index_workspace(workspace.path(), &data_dir, &branch, &config, true),
    );
    let controller = async {
        barrier.wait_until_reached().await?;
        std::fs::remove_file(workspace.path().join("victim.tf"))?;
        std::fs::rename(
            workspace.path().join("replacement.tmp"),
            workspace.path().join("victim.tf"),
        )?;
        barrier.release();
        Ok::<(), anyhow::Error>(())
    };

    let (indexed, controlled) = tokio::join!(operation, controller);
    controlled?;
    indexed?;
    let (_, classes) = graph_state(&data_dir, &branch).await?;
    assert!(
        classes
            .iter()
            .any(|name| name == "hcl.block.resource.safe.new")
    );
    Ok(())
}
