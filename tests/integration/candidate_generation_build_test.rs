//! Integration coverage for plan unit PRE-1a (142.075-T): read-server layout,
//! identity spelling, and generation-ID minting.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, SystemTime};

use engram::db::workspace::{canonicalize_workspace, workspace_hash};
use engram::errors::WorkspaceError;
use engram::services::generations::ExpectedIdentity;
use engram::services::generations::candidate_build::{
    READ_SERVER_ACTIVATION_DEADLINE, mint_generation_id, read_server_layout,
};
use tempfile::TempDir;

fn spaced_dir() -> TempDir {
    tempfile::Builder::new()
        .prefix("candidate build ")
        .tempdir()
        .expect("tempdir")
}

fn git_fixture(head: Option<&str>) -> (TempDir, PathBuf) {
    let dir = spaced_dir();
    let root = dir.path().join("work space");
    fs::create_dir_all(root.join(".git")).expect("create .git");
    if let Some(head) = head {
        fs::write(root.join(".git").join("HEAD"), head).expect("write HEAD");
    }
    (dir, root)
}

fn snapshot(root: &Path) -> BTreeMap<PathBuf, Option<Vec<u8>>> {
    fn walk(base: &Path, dir: &Path, out: &mut BTreeMap<PathBuf, Option<Vec<u8>>>) {
        for entry in fs::read_dir(dir).expect("read_dir") {
            let path = entry.expect("entry").path();
            let rel = path.strip_prefix(base).expect("relative").to_path_buf();
            if path.is_dir() {
                out.insert(rel, None);
                walk(base, &path, out);
            } else {
                out.insert(rel, Some(fs::read(&path).expect("read")));
            }
        }
    }
    let mut out = BTreeMap::new();
    walk(root, root, &mut out);
    out
}

fn as_str(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

#[test]
fn layout_matches_canonical_workspace_and_identity() {
    let (_dir, root) = git_fixture(Some("ref: refs/heads/main\n"));
    let before = snapshot(&root);

    let layout = read_server_layout(as_str(&root)).expect("layout");
    let canonical = canonicalize_workspace(as_str(&root)).expect("canonical");

    assert_eq!(layout.workspace, canonical);
    assert_eq!(layout.branch, "main");
    assert_eq!(layout.workspace_id, workspace_hash(&canonical, "main"));
    assert_eq!(
        layout.generations_root,
        canonical.join(".engram").join("generations")
    );
    assert!(layout.runtime_root.is_absolute());
    assert_eq!(layout.runtime_root, layout.generations_root.join("runtime"));
    assert_eq!(layout.activation_deadline, READ_SERVER_ACTIVATION_DEADLINE);
    assert_eq!(READ_SERVER_ACTIVATION_DEADLINE, Duration::from_secs(60));
    assert_eq!(
        layout.expected_identity(),
        ExpectedIdentity::new("main", layout.workspace_id.clone())
    );

    assert!(
        !root.join(".engram").exists(),
        "layout must not create .engram"
    );
    assert_eq!(snapshot(&root), before, "fixture tree must be unchanged");
}

#[test]
fn layout_is_identical_for_std_canonical_spelling() {
    let (_dir, root) = git_fixture(Some("ref: refs/heads/main\n"));
    let plain = read_server_layout(as_str(&root)).expect("plain layout");
    let verbatim_input = fs::canonicalize(&root).expect("std canonicalize");
    let verbatim = read_server_layout(as_str(&verbatim_input)).expect("verbatim layout");

    assert_eq!(plain, verbatim);
    assert_eq!(plain.workspace_id, verbatim.workspace_id);

    #[cfg(windows)]
    {
        use std::path::Component;
        let Some(Component::Prefix(prefix)) = plain.workspace.components().next() else {
            panic!("expected a prefix component on Windows");
        };
        assert!(
            !prefix.kind().is_verbatim(),
            "workspace must not be verbatim"
        );
    }
}

#[test]
fn missing_head_gives_default_branch() {
    let (_dir, root) = git_fixture(None);
    let before = snapshot(&root);

    let layout = read_server_layout(as_str(&root)).expect("layout");

    assert_eq!(layout.branch, "default");
    assert_eq!(
        layout.workspace_id,
        workspace_hash(&layout.workspace, "default")
    );
    assert!(!root.join(".engram").exists());
    assert_eq!(snapshot(&root), before);
}

#[test]
fn non_git_directory_is_not_git_root() {
    let dir = spaced_dir();
    let root = dir.path().join("plain dir");
    fs::create_dir_all(&root).expect("create dir");
    let before = snapshot(&root);

    let err = read_server_layout(as_str(&root)).expect_err("must fail");

    assert!(matches!(err, WorkspaceError::NotGitRoot { .. }), "{err:?}");
    assert!(!root.join(".engram").exists());
    assert_eq!(snapshot(&root), before);
}

#[test]
fn missing_path_is_not_found() {
    let dir = spaced_dir();
    let missing = dir.path().join("no such dir");

    let err = read_server_layout(as_str(&missing)).expect_err("must fail");

    assert!(matches!(err, WorkspaceError::NotFound { .. }), "{err:?}");
    assert!(!missing.exists());
}

#[test]
fn read_server_layout_signature_is_str_to_workspace_error() {
    let _: fn(&str) -> Result<_, WorkspaceError> = read_server_layout;
}

#[test]
fn same_millisecond_and_pid_mint_identical_ids() {
    let now = SystemTime::UNIX_EPOCH + Duration::from_millis(2_001);
    let first = mint_generation_id(now).expect("first id");
    let second = mint_generation_id(now).expect("second id");
    assert_eq!(first, second);
}

#[test]
fn minted_ids_are_valid_distinct_and_not_runtime() {
    let first = mint_generation_id(SystemTime::UNIX_EPOCH + Duration::from_millis(1_500))
        .expect("first id");
    let second = mint_generation_id(SystemTime::UNIX_EPOCH + Duration::from_millis(1_501))
        .expect("second id");

    assert_ne!(first, second);
    for id in [&first, &second] {
        assert_ne!(id.as_str(), "runtime");
        assert!(id.as_str().starts_with("preflight-"), "{id}");
    }
    assert_eq!(
        first.as_str(),
        format!("preflight-1500-{}", std::process::id())
    );
}
