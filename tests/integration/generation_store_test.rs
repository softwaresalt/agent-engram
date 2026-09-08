#![forbid(unsafe_code)]

use std::fs;
use std::path::{Path, PathBuf};

use engram::services::generations::{GenerationId, GenerationStore, IndexTargetKind, StoreError};

fn generation_id() -> GenerationId {
    GenerationId::new("gen-1").expect("generation id")
}

fn create_store(root: &Path) -> GenerationStore {
    GenerationStore::new(root).expect("generation store")
}

fn write_file(path: &Path) {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("create parent directories");
    }
    fs::write(path, b"payload").expect("write test file");
}

#[test]
fn seal_legacy_direct_canonicalizes_a_contained_file() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let file_path = root.path().join("sealed").join("index.db");
    write_file(&file_path);

    let store = create_store(root.path());
    let generation_id = generation_id();

    let target = store
        .seal_legacy_direct(generation_id.clone(), Path::new("sealed").join("index.db"))
        .expect("seal contained file");

    assert_eq!(target.kind(), IndexTargetKind::LegacyDirect);
    assert_eq!(target.generation_id(), &generation_id);
    assert_eq!(
        target.path(),
        &file_path.canonicalize().expect("canonical contained file")
    );
}

#[test]
fn seal_legacy_direct_rejects_parent_traversal() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let store = create_store(root.path());

    let error = store
        .seal_legacy_direct(
            generation_id(),
            PathBuf::from("nested").join("..").join("escape.db"),
        )
        .expect_err("parent traversal must be rejected");

    assert!(matches!(error, StoreError::InvalidRelativePath { .. }));
}

#[test]
fn seal_legacy_direct_rejects_absolute_path_escape() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_file = outside.path().join("outside.db");
    write_file(&outside_file);

    let store = create_store(root.path());
    let error = store
        .seal_legacy_direct(generation_id(), outside_file)
        .expect_err("absolute paths must be rejected");

    assert!(matches!(error, StoreError::InvalidRelativePath { .. }));
}

#[cfg(unix)]
#[test]
fn seal_legacy_direct_rejects_symlink_escape() {
    use std::os::unix::fs::symlink;

    let root = tempfile::tempdir().expect("generation root tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_file = outside.path().join("outside.db");
    let link_path = root.path().join("escape-link");
    write_file(&outside_file);
    symlink(&outside_file, &link_path).expect("create symlink escape");

    let store = create_store(root.path());
    let error = store
        .seal_legacy_direct(generation_id(), Path::new("escape-link"))
        .expect_err("symlink escape must be rejected");

    assert!(matches!(error, StoreError::PathEscape { .. }));
}

/// Create a directory link (Windows directory junction) at `at` pointing to
/// `target`. Junctions require no elevation, unlike Windows file symlinks,
/// which is why this mirrors the Unix symlink-escape scenario above via a
/// linked *directory* rather than a linked file. Convention matches
/// `tests/unit/workspace_reparse_test.rs::make_dir_link`.
#[cfg(windows)]
fn make_dir_link(at: &Path, target: &Path) -> std::io::Result<()> {
    use std::os::windows::process::CommandExt as _;
    use std::process::Command;

    let status = Command::new("cmd")
        .arg("/c")
        .raw_arg(format!(
            "mklink /J \"{}\" \"{}\"",
            at.display(),
            target.display()
        ))
        .status()?;
    if status.success() {
        Ok(())
    } else {
        Err(std::io::Error::other("mklink /J failed"))
    }
}

#[cfg(windows)]
#[test]
fn seal_legacy_direct_rejects_directory_junction_escape() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let outside = tempfile::tempdir().expect("outside tempdir");
    let outside_dir = outside.path().join("outside-dir");
    fs::create_dir(&outside_dir).expect("create outside directory");
    let outside_file = outside_dir.join("outside.db");
    write_file(&outside_file);

    let link_dir = root.path().join("escape-link");
    match make_dir_link(&link_dir, &outside_dir) {
        Ok(()) => {}
        Err(error) => {
            // Some locked-down CI/dev environments disable junction creation
            // entirely; skip rather than fail in that narrow case, matching
            // the `workspace_reparse_test.rs` convention of an explicit
            // `SKIPPED:` line instead of a silent no-op.
            println!("SKIPPED: could not create directory junction: {error}");
            return;
        }
    }

    let store = create_store(root.path());
    let error = store
        .seal_legacy_direct(generation_id(), Path::new("escape-link").join("outside.db"))
        .expect_err("directory junction escape must be rejected");

    assert!(matches!(error, StoreError::PathEscape { .. }));
}

#[test]
fn seal_legacy_direct_rejects_non_regular_files() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let directory_path = root.path().join("candidate-dir");
    fs::create_dir(&directory_path).expect("create directory");

    let store = create_store(root.path());
    let error = store
        .seal_legacy_direct(generation_id(), Path::new("candidate-dir"))
        .expect_err("directories must not seal as regular files");

    assert!(matches!(error, StoreError::NonRegularFile { .. }));
}

/// GIVEN a candidate relative path that collides with the store's own
/// reserved `active.json` manifest name at the generation root
/// WHEN `seal_candidate` is called
/// THEN it must be rejected rather than minting a directory at the manifest
/// authority path -- Copilot review round 4 flagged that sealing a candidate
/// named `active.json` would create a directory there, after which every
/// subsequent publication would fail to read the current manifest (recovery
/// would require deleting the candidate).
#[test]
fn seal_candidate_rejects_the_reserved_active_manifest_name() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let store = create_store(root.path());

    let error = store
        .seal_candidate(generation_id(), Path::new("active.json"))
        .expect_err("candidate must not claim the reserved active.json name");

    assert!(
        matches!(error, StoreError::ReservedName { .. }),
        "expected StoreError::ReservedName, got: {error:?}"
    );
    assert!(
        !root.path().join("active.json").exists(),
        "no directory must be minted at the reserved active.json path"
    );
}

/// GIVEN a candidate relative path that collides with the store's own
/// reserved `.publisher.lock` advisory-lock name at the generation root
/// WHEN `seal_candidate` is called
/// THEN it must be rejected for the same reason as the `active.json` case.
#[test]
fn seal_candidate_rejects_the_reserved_publisher_lock_name() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let store = create_store(root.path());

    let error = store
        .seal_candidate(generation_id(), Path::new(".publisher.lock"))
        .expect_err("candidate must not claim the reserved .publisher.lock name");

    assert!(
        matches!(error, StoreError::ReservedName { .. }),
        "expected StoreError::ReservedName, got: {error:?}"
    );
    assert!(
        !root.path().join(".publisher.lock").exists(),
        "no directory must be minted at the reserved .publisher.lock path"
    );
}

/// GIVEN a candidate relative path that collides with a reserved root name
/// only after ASCII case-folding (e.g. `ACTIVE.JSON` vs. `active.json`)
/// WHEN `seal_candidate` is called
/// THEN it must still be rejected -- on case-insensitive filesystems
/// (default Windows/macOS) `ACTIVE.JSON` aliases `active.json` at the OS
/// level, so a case-sensitive-only comparison would let a candidate mint a
/// directory that occupies the manifest authority path, reproducing the
/// exact invalid store state the reservation exists to prevent (Copilot
/// review finding, PR #385 thread `PRRT_kwDORJEduc6gF5mI`, stash `3D2B167C`).
#[test]
fn seal_candidate_rejects_the_reserved_active_manifest_name_case_insensitively() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let store = create_store(root.path());

    for variant in ["ACTIVE.JSON", "Active.Json", "aCtIvE.jSoN"] {
        let result = store.seal_candidate(generation_id(), Path::new(variant));
        let Err(error) = result else {
            panic!("candidate {variant:?} must not bypass the reserved active.json name")
        };

        assert!(
            matches!(error, StoreError::ReservedName { .. }),
            "expected StoreError::ReservedName for {variant:?}, got: {error:?}"
        );
        assert!(
            !root.path().join(variant).exists(),
            "no directory must be minted at the case-variant reserved path {variant:?}"
        );
    }
}

/// GIVEN a candidate relative path that collides with the reserved
/// `.publisher.lock` name only after ASCII case-folding
/// WHEN `seal_candidate` is called
/// THEN it must be rejected for the same reason as the `active.json` case.
#[test]
fn seal_candidate_rejects_the_reserved_publisher_lock_name_case_insensitively() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    let store = create_store(root.path());

    for variant in [".PUBLISHER.LOCK", ".Publisher.Lock"] {
        let result = store.seal_candidate(generation_id(), Path::new(variant));
        let Err(error) = result else {
            panic!("candidate {variant:?} must not bypass the reserved .publisher.lock name")
        };

        assert!(
            matches!(error, StoreError::ReservedName { .. }),
            "expected StoreError::ReservedName for {variant:?}, got: {error:?}"
        );
        assert!(
            !root.path().join(variant).exists(),
            "no directory must be minted at the case-variant reserved path {variant:?}"
        );
    }
}

/// GIVEN a candidate relative path that merely happens to end in a reserved
/// leaf name but is NOT located directly at the generation root (e.g. inside
/// a nested candidate parent directory)
/// WHEN `seal_candidate` is called
/// THEN it must still succeed -- the reservation only protects the store's
/// own root-level namespace, not every occurrence of the name anywhere in
/// the tree.
#[test]
fn seal_candidate_permits_a_nested_leaf_matching_a_reserved_name() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    fs::create_dir(root.path().join("candidates")).expect("create candidate parent");
    let store = create_store(root.path());

    let target = store
        .seal_candidate(generation_id(), Path::new("candidates").join("active.json"))
        .expect("nested leaf matching a reserved name outside the root is not reserved");

    assert_eq!(target.kind(), IndexTargetKind::Candidate);
}

#[test]
fn seal_candidate_refuses_a_second_concurrent_candidate_directory() {
    let root = tempfile::tempdir().expect("generation root tempdir");
    fs::create_dir(root.path().join("candidates")).expect("create candidate parent");

    let store = create_store(root.path());
    let generation_id = generation_id();
    let candidate_path = Path::new("candidates").join("gen-1");

    let first = store
        .seal_candidate(generation_id.clone(), candidate_path.clone())
        .expect("first candidate seal");
    let second = store
        .seal_candidate(generation_id.clone(), candidate_path)
        .expect_err("second concurrent candidate must fail");

    assert_eq!(first.kind(), IndexTargetKind::Candidate);
    assert_eq!(first.generation_id(), &generation_id);
    assert!(matches!(second, StoreError::CandidateAlreadyExists { .. }));
}
