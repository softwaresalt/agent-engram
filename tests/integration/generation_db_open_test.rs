//! F09 existing-generation open tests.
//!
//! F01 proved Cozo 0.7.6 exposes no construction path that can *prove* an
//! existing `SQLite` database was opened read-only. The production strategy is
//! therefore intentionally single-path: copy the published generation database
//! into a private runtime location, then open the copy. These tests exercise
//! only that proven path; there is no alternate "open published DB read-only"
//! branch to validate because the spike rejected it.

#![forbid(unsafe_code)]

use std::{collections::BTreeMap, fs, path::Path};

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
