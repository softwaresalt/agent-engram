//! F09 existing-generation open tests.
//!
//! F01 proved Cozo 0.7.6 exposes no construction path that can *prove* an
//! existing `SQLite` database was opened read-only. The production strategy is
//! therefore intentionally single-path: copy the published generation database
//! into a private runtime location, then open the copy. These tests exercise
//! only that proven path; there is no alternate "open published DB read-only"
//! branch to validate because the spike rejected it.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, fs, io::Write, path::Path};

use cozo::ScriptMutability;
use engram::db::cozo_backend::{ExistingDbLocation, open_existing_generation_via_runtime_copy};

fn create_seeded_db(db_path: &Path) -> Vec<u8> {
    {
        let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
            .expect("create db");
        db.run_default(":create probe_row {id => val}")
            .expect("create relation");
        db.run_default("?[id, val] <- [[1, 'seed']] :put probe_row {id => val}")
            .expect("seed row");
    }

    fs::read(db_path).expect("read seeded db bytes")
}

fn insert_runtime_only_row(db: &cozo::DbInstance) {
    db.run_script(
        "?[id, val] <- [[2, 'runtime-only']] :put probe_row {id => val}",
        BTreeMap::new(),
        ScriptMutability::Mutable,
    )
    .expect("mutate runtime copy");
}

fn count_probe_rows(db: &cozo::DbInstance) -> usize {
    db.run_script(
        "?[id, val] := *probe_row{id, val}",
        BTreeMap::new(),
        ScriptMutability::Immutable,
    )
    .expect("read probe rows")
    .rows
    .len()
}

#[test]
fn fresh_open_creates_runtime_copy_under_runtime_root() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    let published_bytes = create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    let opened =
        open_existing_generation_via_runtime_copy(&location, runtime_root.path(), "generation-001")
            .expect("open runtime copy");

    let expected_runtime_path = runtime_root.path().join("generation-001").join("engram.db");
    assert_eq!(
        opened.runtime_copy().path(),
        expected_runtime_path.as_path()
    );
    assert!(expected_runtime_path.exists(), "runtime copy must exist");
    assert_eq!(
        fs::read(&expected_runtime_path).expect("read runtime copy bytes"),
        published_bytes,
        "fresh runtime copy must begin as a byte-for-byte copy of the published DB"
    );
    assert_eq!(
        count_probe_rows(opened.db()),
        1,
        "fresh open must see the seeded row"
    );
}

/// GIVEN a published database that genuinely exists and is a regular file
/// WHEN it is located OUTSIDE the supplied `generation_root` (a sibling
/// directory, not an ancestor-contained path)
/// THEN `ExistingDbLocation::new` must still reject it -- this is the
/// specific containment guarantee a Copilot review round flagged as
/// missing when the constructor only checked existence/regular-file-ness.
/// Regression coverage so a future refactor that weakens the
/// `starts_with` containment check fails this test instead of silently
/// reintroducing the gap.
#[test]
fn existing_db_location_rejects_a_path_outside_the_generation_root() {
    let generation_root = tempfile::tempdir().expect("tempdir");
    let outside_dir = tempfile::tempdir().expect("tempdir");
    let outside_db_path = outside_dir.path().join("engram.db");
    create_seeded_db(&outside_db_path);

    let error = ExistingDbLocation::new(generation_root.path(), outside_db_path)
        .expect_err("a database path outside generation_root must be rejected");

    let message = error.to_string();
    assert!(
        message.contains("escapes generation root"),
        "expected a containment-escape error, got: {message}"
    );
}

/// GIVEN a `generation_root` argument that is itself a regular file (not a
/// directory)
/// WHEN the same file path is also supplied as `published_db_path`
/// THEN `ExistingDbLocation::new` must reject it rather than minting a valid
/// location -- Copilot review flagged that `Path::starts_with` treats an
/// identical path as trivially contained, so a non-directory "root" let an
/// arbitrary file mint an `ExistingDbLocation` for itself, bypassing the
/// containment check `GenerationStore::new` already enforces for its own
/// root argument.
#[test]
fn existing_db_location_rejects_a_non_directory_generation_root() {
    let parent_dir = tempfile::tempdir().expect("tempdir");
    let file_masquerading_as_root = parent_dir.path().join("not-a-directory.db");
    create_seeded_db(&file_masquerading_as_root);

    let error = ExistingDbLocation::new(&file_masquerading_as_root, &file_masquerading_as_root)
        .expect_err("a non-directory generation root must be rejected");

    let message = error.to_string();
    assert!(
        message.contains("must be a directory"),
        "expected a non-directory generation root error, got: {message}"
    );
}

fn sidecar_path(main_path: &Path, suffix: &str) -> std::path::PathBuf {
    let mut name = main_path.file_name().expect("file name").to_os_string();
    name.push(suffix);
    main_path.with_file_name(name)
}

/// GIVEN a runtime copy directory that already holds a prior generation's
/// main-file copy plus leftover `-wal`/`-shm` sidecar files (simulating a
/// process that crashed before checkpointing WAL frames into the main file)
/// WHEN the same generation is (re)opened via a fresh runtime copy
/// THEN the stale sidecars must not survive the reseal, and the freshly
/// opened database must reflect only the newly copied published snapshot --
/// never state reconstructed by replaying a leftover WAL/SHM pair from a
/// prior incarnation of this runtime copy path. Regression coverage for the
/// Copilot review finding that `publish_runtime_copy` replaced only
/// `engram.db` and left crash-orphaned sidecars in place next to it.
#[test]
fn reopening_removes_stale_wal_and_shm_sidecars_left_by_a_crash() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    let published_bytes = create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    let first_open = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-crash-sidecar",
    )
    .expect("first open runtime copy");
    let runtime_copy_path = first_open.runtime_copy().path().to_path_buf();
    drop(first_open);

    // Simulate a crash that left stale WAL/SHM sidecars next to the runtime
    // copy's main file (e.g. the process died mid-checkpoint after a
    // runtime-only mutation).
    let wal_path = sidecar_path(&runtime_copy_path, "-wal");
    let shm_path = sidecar_path(&runtime_copy_path, "-shm");
    fs::write(&wal_path, b"stale-wal-bytes-from-a-crashed-run").expect("write stale wal sidecar");
    fs::write(&shm_path, b"stale-shm-bytes-from-a-crashed-run").expect("write stale shm sidecar");

    let second_open = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-crash-sidecar",
    )
    .expect("second open must succeed and isolate the fresh copy from stale sidecars");

    assert!(
        !wal_path.exists(),
        "stale -wal sidecar from a crashed prior run must not survive a fresh runtime copy reseal"
    );
    assert!(
        !shm_path.exists(),
        "stale -shm sidecar from a crashed prior run must not survive a fresh runtime copy reseal"
    );
    assert_eq!(
        count_probe_rows(second_open.db()),
        1,
        "second open must reflect only the freshly copied published snapshot, never stale WAL state"
    );
    assert_eq!(
        fs::read(second_open.runtime_copy().path()).expect("read runtime copy bytes"),
        published_bytes
    );
}

/// GIVEN a published database whose declared size (captured via metadata
/// immediately before the copy begins) no longer matches its actual size by
/// the time the copy completes
/// WHEN the runtime copy is opened
/// THEN the mismatch must be detected and the copy must be rejected rather
/// than sealed/opened unverified -- `std::fs::copy` alone enforces no bound
/// against a growing or shrinking source, so this proves the copy path
/// revalidates size/digest before sealing. This test truncates the
/// published database file to a shorter length than a stale
/// `ExistingDbLocation` still believes was validated, reproducing a source
/// that shrank between validation and copy.
#[test]
fn open_rejects_a_published_database_that_shrank_after_validation() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    // Truncate the published file after `ExistingDbLocation` validated it,
    // simulating a concurrent shrink between validation and the runtime
    // copy's own bounded read.
    let original_len = fs::metadata(&published_db_path)
        .expect("read original metadata")
        .len();
    assert!(
        original_len > 4,
        "seeded db must be large enough to truncate meaningfully"
    );
    let truncated_file = fs::OpenOptions::new()
        .write(true)
        .open(&published_db_path)
        .expect("open published db for truncation");
    truncated_file
        .set_len(original_len - 4)
        .expect("truncate published db");
    drop(truncated_file);

    let error = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-shrink-detect",
    )
    .expect_err("a source that shrank after validation must be rejected, never sealed unverified");

    let message = error.to_string();
    assert!(
        message.contains("truncated") || message.contains("changed"),
        "expected a truncation/mutation-detection error, got: {message}"
    );
}

/// GIVEN a published database that grew (had bytes appended) after
/// `ExistingDbLocation` validated and snapshotted its length
/// WHEN the runtime copy is opened
/// THEN the growth must be detected and the copy rejected -- the companion
/// case to the shrink test above, together covering the "growing/oversized"
/// source scenario the Copilot review finding described.
#[test]
fn open_rejects_a_published_database_that_grew_after_validation() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    // Append bytes after validation, simulating a concurrent writer growing
    // the file between validation and the runtime copy's bounded read.
    let mut file = fs::OpenOptions::new()
        .append(true)
        .open(&published_db_path)
        .expect("open published db for growth append");
    file.write_all(b"unexpected-growth-after-validation")
        .expect("append growth bytes");
    drop(file);

    let error = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-growth-detect",
    )
    .expect_err("a source that grew after validation must be rejected, never sealed unverified");

    let message = error.to_string();
    assert!(
        message.contains("grew") || message.contains("changed"),
        "expected a growth/mutation-detection error, got: {message}"
    );
}

#[test]
fn mutating_runtime_copy_never_changes_published_db_bytes() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    let published_bytes_before = create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    let opened = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-immutability",
    )
    .expect("open runtime copy");
    insert_runtime_only_row(opened.db());
    assert_eq!(
        count_probe_rows(opened.db()),
        2,
        "runtime copy must observe both the seed row and the runtime-only mutation"
    );
    drop(opened);

    let published_bytes_after = fs::read(&published_db_path).expect("re-read published bytes");
    assert_eq!(
        published_bytes_before, published_bytes_after,
        "mutating the runtime copy must not change the published DB file"
    );
}

/// GIVEN a `runtime_root` whose absolute path contains non-UTF-8 bytes
/// (constructible on Unix since `OsStr`/`Path` do not require valid UTF-8,
/// but `CozoDB`'s `sqlite` backend requires a `&str` path)
/// WHEN a generation is opened via the runtime-copy path
/// THEN the open must fail with a UTF-8 validation error *before* any
/// filesystem mutation -- not after `publish_runtime_copy` has already
/// copied the published database into place, sealed it as the runtime
/// copy's `engram.db`, and removed any stale sidecars. Regression coverage
/// for PR #385 review thread `PRRT_kwDORJEduc6gJLCP`: the pre-fix code
/// validated `final_path`'s UTF-8-ness only via
/// `runtime_copy.path().to_str()` *after* `publish_runtime_copy` returned,
/// so a non-UTF-8 runtime root still mutated the runtime directory
/// (created it, copied/sealed `engram.db`, removed sidecars) before the
/// open ultimately failed.
#[cfg(unix)]
#[test]
fn open_rejects_a_non_utf8_runtime_root_before_any_runtime_copy_side_effects() {
    use std::os::unix::ffi::OsStrExt;

    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root_parent = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    // Build a runtime root whose final path component is not valid UTF-8.
    // `Path`/`OsStr` on Unix wrap raw bytes with no UTF-8 requirement, so
    // this is constructible without `unsafe` and without touching the
    // filesystem yet.
    let non_utf8_component = std::ffi::OsStr::from_bytes(b"runtime-root-\xFF\xFE-non-utf8");
    let runtime_root = runtime_root_parent.path().join(non_utf8_component);
    assert!(
        runtime_root.to_str().is_none(),
        "precondition: the constructed runtime root must not be valid UTF-8"
    );
    assert!(
        !runtime_root.exists(),
        "precondition: the non-UTF-8 runtime root must not already exist"
    );

    let generation_id = "generation-non-utf8-root";
    let expected_final_path = runtime_root.join(generation_id).join("engram.db");

    let error = open_existing_generation_via_runtime_copy(&location, &runtime_root, generation_id)
        .expect_err("a non-UTF-8 runtime root must be rejected");
    let message = error.to_string();
    assert!(
        message.contains("not valid UTF-8"),
        "expected a UTF-8 validation error, got: {message}"
    );

    // The failure must be side-effect-free: rejecting a non-UTF-8 runtime
    // root before any mutation means the runtime directory tree under it
    // (including the final engram.db, any sidecars, staging files, and
    // lock files) must never be created.
    assert!(
        !runtime_root.exists(),
        "rejecting a non-UTF-8 runtime root must not create the runtime directory tree"
    );
    assert!(
        !expected_final_path.exists(),
        "rejecting a non-UTF-8 runtime root must not create/replace the final engram.db"
    );
}

#[test]
fn reopening_same_generation_replaces_existing_runtime_copy() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_root = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    let published_bytes = create_seeded_db(&published_db_path);
    let location = ExistingDbLocation::new(published_dir.path(), published_db_path.clone())
        .expect("published database path must validate");

    let first_open = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-reopen",
    )
    .expect("first open runtime copy");
    insert_runtime_only_row(first_open.db());
    assert_eq!(count_probe_rows(first_open.db()), 2);
    let runtime_copy_path = first_open.runtime_copy().path().to_path_buf();
    drop(first_open);

    let second_open = open_existing_generation_via_runtime_copy(
        &location,
        runtime_root.path(),
        "generation-reopen",
    )
    .expect("second open runtime copy");

    assert_eq!(
        second_open.runtime_copy().path(),
        runtime_copy_path.as_path()
    );
    assert_eq!(
        count_probe_rows(second_open.db()),
        1,
        "second open must replace the prior runtime copy with a fresh copy of the published DB"
    );
    assert_eq!(
        fs::read(second_open.runtime_copy().path()).expect("read replaced runtime copy bytes"),
        published_bytes,
        "reopened runtime copy must be republished from the original bytes"
    );
}
