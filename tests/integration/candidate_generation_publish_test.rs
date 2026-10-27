//! Integration coverage for plan unit PRE-2 (142.062-T): generation store
//! opening and the candidate publish facade.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;
use std::path::{Path, PathBuf};

use engram::models::config::CodeGraphConfig;
use engram::services::generations::candidate_build::{
    CandidateBuildError, ReadServerLayout, build_candidate_generation, current_published_revision,
    open_generation_store, publish_sealed_candidate, read_server_layout, seal_candidate_inventory,
};
use engram::services::generations::{
    GenerationActivator, GenerationId, GenerationRevision, GenerationStore,
};
use tempfile::TempDir;

const PRODUCER: &str = "preflight-harness";

fn as_str(path: &Path) -> &str {
    path.to_str().expect("utf8 path")
}

/// Spaced git fixture with a two-function `lib.rs`.
fn fixture() -> (TempDir, PathBuf, ReadServerLayout) {
    let dir = tempfile::Builder::new()
        .prefix("candidate publish ")
        .tempdir()
        .expect("tempdir");
    let root = dir.path().join("work space");
    fs::create_dir_all(root.join(".git")).expect("create .git");
    fs::write(root.join(".git").join("HEAD"), "ref: refs/heads/main\n").expect("write HEAD");
    fs::create_dir_all(root.join("src")).expect("create src");
    fs::write(
        root.join("src").join("lib.rs"),
        "pub fn alpha() -> u32 {\n    1\n}\n\npub fn beta() -> u32 {\n    2\n}\n",
    )
    .expect("write lib.rs");
    let layout = read_server_layout(as_str(&root)).expect("layout");
    (dir, root, layout)
}

async fn build_and_seal(
    layout: &ReadServerLayout,
    store: &GenerationStore,
    id: &str,
) -> engram::services::generations::candidate_build::SealedCandidate {
    let id = GenerationId::new(id).expect("id");
    let built = build_candidate_generation(layout, store, &id, &CodeGraphConfig::default())
        .await
        .expect("build");
    seal_candidate_inventory(built).expect("seal")
}

fn function_count(db: &cozo::DbInstance) -> i64 {
    let rows = db
        .run_script(
            "?[count(id)] := *function_meta { id }",
            BTreeMap::new(),
            cozo::ScriptMutability::Immutable,
        )
        .expect("count script");
    rows.rows[0][0].get_int().expect("integer count")
}

/// Create a directory link (symlink, or junction on Windows). Returns `false`
/// with a logged reason when the link cannot be created.
fn make_dir_link(link: &Path, target: &Path) -> bool {
    #[cfg(windows)]
    {
        let status = std::process::Command::new("cmd")
            .args(["/c", "mklink", "/J"])
            .arg(link)
            .arg(target)
            .output();
        match status {
            Ok(out) if out.status.success() => true,
            Ok(out) => {
                eprintln!("skip: mklink /J failed: {:?}", out.status);
                false
            }
            Err(e) => {
                eprintln!("skip: cannot spawn cmd: {e}");
                false
            }
        }
    }
    #[cfg(unix)]
    {
        match std::os::unix::fs::symlink(target, link) {
            Ok(()) => true,
            Err(e) => {
                eprintln!("skip: symlink failed: {e}");
                false
            }
        }
    }
    #[cfg(not(any(windows, unix)))]
    {
        let _ = (link, target);
        eprintln!("skip: no directory link support");
        false
    }
}

fn dir_is_empty(dir: &Path) -> bool {
    fs::read_dir(dir).expect("read_dir").next().is_none()
}

#[tokio::test]
async fn first_publish_and_activation() {
    let (_dir, _root, layout) = fixture();

    let store = open_generation_store(&layout, true).expect("open store");
    let sealed = build_and_seal(&layout, &store, "cand-pub-1").await;
    let revision = publish_sealed_candidate(&layout, &store, sealed, PRODUCER).expect("publish");
    assert_eq!(revision, GenerationRevision::new(1));
    assert_eq!(
        current_published_revision(&store).expect("current"),
        GenerationRevision::new(1)
    );

    let activator = GenerationActivator::new(
        store,
        layout.runtime_root.clone(),
        layout.expected_identity(),
        layout.activation_deadline,
    );
    let context = activator.activate_initial().await.expect("activate");
    assert_eq!(context.generation_id().as_str(), "cand-pub-1");
    assert_eq!(function_count(context.opened_generation().db()), 2);
}

#[tokio::test]
async fn second_publish_and_store_containment() {
    let (_dir, root, layout) = fixture();

    // Absent store with create == false: typed error, nothing created.
    let err = open_generation_store(&layout, false).expect_err("absent store");
    assert!(matches!(err, CandidateBuildError::StoreAbsent), "{err:?}");
    assert!(!root.join(".engram").exists(), "nothing may be created");

    // A generations root outside the workspace is rejected before creation.
    let outside = TempDir::new().expect("outside dir");
    let missing = outside.path().join("nested").join("generations");
    let mut escaping = layout.clone();
    escaping.generations_root = missing.clone();
    let err = open_generation_store(&escaping, true).expect_err("outside root");
    assert!(
        matches!(err, CandidateBuildError::ContainmentEscape),
        "{err:?}"
    );
    assert!(!missing.exists(), "outside directory must not be created");
    assert!(!outside.path().join("nested").exists());
    let mut dotdot = layout.clone();
    dotdot.generations_root = root.join("..").join("escape-gen");
    let err = open_generation_store(&dotdot, true).expect_err("dotdot root");
    assert!(
        matches!(err, CandidateBuildError::ContainmentEscape),
        "{err:?}"
    );

    // Second publish switches the activator.
    let store = open_generation_store(&layout, true).expect("open store");
    let first = build_and_seal(&layout, &store, "cand-pub-a").await;
    assert_eq!(
        publish_sealed_candidate(&layout, &store, first, PRODUCER).expect("publish 1"),
        GenerationRevision::new(1)
    );
    let activator = GenerationActivator::new(
        store.clone(),
        layout.runtime_root.clone(),
        layout.expected_identity(),
        layout.activation_deadline,
    );
    activator.activate_initial().await.expect("activate");
    let second = build_and_seal(&layout, &store, "cand-pub-b").await;
    assert_eq!(
        publish_sealed_candidate(&layout, &store, second, PRODUCER).expect("publish 2"),
        GenerationRevision::new(2)
    );
    let switched = activator
        .maybe_activate_newer()
        .await
        .expect("reconcile")
        .expect("newer generation must activate");
    assert_eq!(switched.generation_id().as_str(), "cand-pub-b");

    // A store root reported in verbatim form passes containment.
    let mut verbatim = layout.clone();
    verbatim.generations_root = fs::canonicalize(&layout.generations_root).expect("canonical");
    open_generation_store(&verbatim, false).expect("verbatim root must pass within_root");

    assert_linked_store_paths_contained();
}

/// Linked `.engram`, generations root, and runtime root are all rejected.
fn assert_linked_store_paths_contained() {
    // Linked `.engram`.
    let (dir, root, layout) = fixture();
    let outside = dir.path().join("outside engram");
    fs::create_dir_all(&outside).expect("outside");
    if make_dir_link(&root.join(".engram"), &outside) {
        for create in [true, false] {
            let err = open_generation_store(&layout, create).expect_err("linked .engram");
            assert!(
                matches!(err, CandidateBuildError::ContainmentEscape),
                "{err:?}"
            );
        }
        assert!(dir_is_empty(&outside), "nothing may be written through it");
    }

    // Linked generations root.
    let (dir, root, layout) = fixture();
    let outside = dir.path().join("outside generations");
    fs::create_dir_all(&outside).expect("outside");
    fs::create_dir_all(root.join(".engram")).expect("engram");
    if make_dir_link(&layout.generations_root, &outside) {
        for create in [true, false] {
            let err = open_generation_store(&layout, create).expect_err("linked generations");
            assert!(
                matches!(err, CandidateBuildError::ContainmentEscape),
                "{err:?}"
            );
        }
        assert!(dir_is_empty(&outside), "nothing may be written through it");
    }

    // Linked runtime root.
    let (dir, _root, layout) = fixture();
    let outside = dir.path().join("outside runtime");
    fs::create_dir_all(&outside).expect("outside");
    fs::create_dir_all(&layout.generations_root).expect("generations");
    if make_dir_link(&layout.runtime_root, &outside) {
        for create in [true, false] {
            let err = open_generation_store(&layout, create).expect_err("linked runtime");
            assert!(
                matches!(err, CandidateBuildError::ContainmentEscape),
                "{err:?}"
            );
        }
        assert!(dir_is_empty(&outside), "nothing may be written through it");
    }
}

#[tokio::test]
async fn corrupt_manifest_fails_typed_and_is_untouched() {
    let (_dir, _root, layout) = fixture();
    let store = open_generation_store(&layout, true).expect("open store");
    let sealed = build_and_seal(&layout, &store, "cand-pub-corrupt").await;
    let garbage: &[u8] = b"\x00\xffnot a manifest{{{";
    fs::write(store.active_manifest_path(), garbage).expect("write garbage");

    let err = current_published_revision(&store).expect_err("corrupt manifest");
    assert!(
        matches!(
            err,
            CandidateBuildError::ManifestUnreadable | CandidateBuildError::Publish(_)
        ),
        "{err:?}"
    );
    let err = publish_sealed_candidate(&layout, &store, sealed, PRODUCER)
        .expect_err("publish over corrupt manifest");
    assert!(
        matches!(
            err,
            CandidateBuildError::ManifestUnreadable | CandidateBuildError::Publish(_)
        ),
        "{err:?}"
    );
    assert_eq!(
        fs::read(store.active_manifest_path()).expect("re-read"),
        garbage
    );
}
