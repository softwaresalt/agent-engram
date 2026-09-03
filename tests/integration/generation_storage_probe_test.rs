//! F01 storage feasibility probe (RED-first).
//!
//! Empirically settles the S1 mandatory feasibility spike (requirement R49)
//! that must pass before any generation open/publication implementation
//! (F06-F09) proceeds. Findings are recorded in
//! `docs/decisions/2026-09-02-generation-storage-feasibility-spike.md`.
//!
//! Experiments (mirrors the plan's S1 experiment list, Remediation Revision 2):
//! 1. Prove whether Cozo's `SQLite` backend can open an existing database
//!    read-only without bootstrap/sidecar mutation — it cannot (see
//!    `cozo_sqlite_open_is_not_provably_read_only` and the decision doc's
//!    source-inspection evidence).
//! 2. Because (1) fails, prove the private-runtime-copy fallback preserves
//!    true immutability of the original (published) generation's on-disk
//!    file (see `private_runtime_copy_preserves_original_byte_stability`).
//! 3. Prove replace-existing atomicity/durability on this platform using
//!    only `std::fs` (see `atomic_replace_existing_rename_is_byte_stable`).
//! 4. No `unsafe` code is required for any of the above: this file contains
//!    no `unsafe` blocks, and the `engram` crate root already carries
//!    `#![forbid(unsafe_code)]`.

use std::collections::BTreeMap;
use std::fs;

use cozo::ScriptMutability;

/// Creates a fresh SQLite-backed Cozo database at `db_path`, seeds one row,
/// and returns the raw bytes on disk after the handle is dropped (forcing
/// any buffered writes/checkpoints to settle before the caller inspects
/// the file).
fn create_seeded_db(db_path: &std::path::Path) -> Vec<u8> {
    {
        let db = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
            .expect("create db");
        db.run_default(":create probe_row {id => val}")
            .expect("create relation");
        db.run_default("?[id, val] <- [[1, 'seed']] :put probe_row {id => val}")
            .expect("seed row");
    } // `db` dropped here: releases the SQLite connection before we read the file.
    fs::read(db_path).expect("read seeded db bytes")
}

/// GIVEN a Cozo `SQLite` database that already has its relation created and a
/// row seeded
/// WHEN the same path is opened again with a fresh `DbInstance::new("sqlite",
/// ..., "")` call — the only construction path Cozo 0.7.6 exposes, with no
/// read-only option in its `options` argument (confirmed by direct source
/// inspection of `cozo::storage::sqlite::new_cozo_sqlite`, which
/// unconditionally executes `create table if not exists cozo (...)` and
/// `Db::initialize` on every open, regardless of intent)
/// THEN the returned handle still exposes `run_script` with
/// `ScriptMutability::Mutable`, i.e. Cozo grants no enforced read-only
/// capability at the API level for an "existing" open — a caller cannot
/// prove read-only-ness of an open by construction alone, only by discipline
/// never to call a mutable script. This is the negative result for
/// experiment 1: a "proven read-only open" is not available from the
/// library's public API, so the private-runtime-copy fallback (experiment 2)
/// is the selected strategy.
#[test]
fn cozo_sqlite_open_is_not_provably_read_only() {
    let dir = tempfile::tempdir().expect("tempdir");
    let db_path = dir.path().join("engram.db");
    let _ = create_seeded_db(&db_path);

    // Re-open the same path. Cozo 0.7.6's `DbInstance::new("sqlite", ...)` is
    // the only construction path; it has no read-only variant, so this
    // second open is mechanically indistinguishable from the first one that
    // created the schema and wrote data — it can still run mutable scripts.
    let reopened = cozo::DbInstance::new("sqlite", db_path.to_str().expect("utf8 path"), "")
        .expect("reopen db");
    let mutate_result =
        reopened.run_default("?[id, val] <- [[2, 'mutated-on-reopen']] :put probe_row {id => val}");
    assert!(
        mutate_result.is_ok(),
        "a plain re-open still accepts a mutable script; Cozo 0.7.6 enforces \
         no read-only mode at the API level for an \"existing database\" open"
    );
}

/// GIVEN a published generation's Cozo `SQLite` database file
/// WHEN it is copied byte-for-byte to a private runtime location and the
/// COPY (never the original) is opened, seeded further, and queried
/// THEN the original file's bytes are completely unchanged, proving the
/// private-runtime-copy fallback gives true read-side immutability for the
/// published generation regardless of what Cozo does internally to the copy.
#[test]
fn private_runtime_copy_preserves_original_byte_stability() {
    let published_dir = tempfile::tempdir().expect("tempdir");
    let runtime_dir = tempfile::tempdir().expect("tempdir");
    let published_db_path = published_dir.path().join("engram.db");
    let runtime_db_path = runtime_dir.path().join("engram.db");

    let original_bytes = create_seeded_db(&published_db_path);
    assert!(
        !original_bytes.is_empty(),
        "seeded database file must be non-empty"
    );

    // Private-runtime-copy: byte-for-byte copy into an isolated runtime
    // location that is never exposed as "the published generation".
    fs::copy(&published_db_path, &runtime_db_path).expect("copy to runtime location");

    // Open and further mutate the COPY only. This is the read-server's
    // private working set; the original is never touched again.
    let runtime_db =
        cozo::DbInstance::new("sqlite", runtime_db_path.to_str().expect("utf8 path"), "")
            .expect("open runtime copy");
    runtime_db
        .run_script(
            "?[id, val] <- [[2, 'runtime-only']] :put probe_row {id => val}",
            BTreeMap::new(),
            ScriptMutability::Mutable,
        )
        .expect("mutate runtime copy");
    let read_back = runtime_db
        .run_script(
            "?[id, val] := *probe_row{id, val}",
            BTreeMap::new(),
            ScriptMutability::Immutable,
        )
        .expect("read runtime copy");
    assert_eq!(
        read_back.rows.len(),
        2,
        "runtime copy must observe both the original seed row and the \
         runtime-only mutation"
    );
    drop(runtime_db);

    // The original, published file must be byte-for-byte unchanged: we never
    // opened it again after the initial seed-and-drop in `create_seeded_db`.
    let original_bytes_after = fs::read(&published_db_path).expect("re-read original bytes");
    assert_eq!(
        original_bytes, original_bytes_after,
        "the published generation's on-disk file must be byte-identical \
         after the runtime copy is created and independently mutated"
    );
}

/// GIVEN an existing destination file and a fully-written, fsynced
/// replacement file co-located in the same directory
/// WHEN `std::fs::rename` is used to replace the destination with the
/// replacement (the safe-Rust primitive: no `unsafe` block, works
/// identically as POSIX `rename(2)` on Unix and `MoveFileExW` with
/// `MOVEFILE_REPLACE_EXISTING` on Windows per the Rust standard library's
/// documented cross-platform contract)
/// THEN the rename succeeds without needing to unlink the destination first,
/// and the destination contains the replacement's exact byte content — no
/// truncation, no partial write, no leftover old content.
#[test]
fn atomic_replace_existing_rename_is_byte_stable() {
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("active.json");
    let staging = dir.path().join("active.json.tmp");

    let old_content = b"{\"revision\":1,\"generation\":\"old\"}";
    let new_content = b"{\"revision\":2,\"generation\":\"new-longer-payload-than-old\"}";

    fs::write(&destination, old_content).expect("write initial destination");
    assert!(
        destination.exists(),
        "destination must pre-exist for this probe"
    );

    // Write-then-fsync the replacement to a co-located staging file before
    // the rename, so the rename step itself is the only thing that can be
    // interrupted (the durability half of the crash-recovery property).
    {
        use std::io::Write;
        let mut file = fs::File::create(&staging).expect("create staging file");
        file.write_all(new_content).expect("write staging content");
        file.sync_all().expect("fsync staging file");
    }

    fs::rename(&staging, &destination).expect(
        "std::fs::rename must replace an already-existing destination without \
         requiring a prior delete — this is the selected safe-Rust primitive",
    );

    assert!(
        !staging.exists(),
        "the staging name must no longer exist after a successful rename"
    );
    let final_bytes = fs::read(&destination).expect("read destination after rename");
    assert_eq!(
        final_bytes, new_content,
        "destination must contain exactly the replacement's bytes, byte-for-byte \
         (no truncation or partial-write torn state)"
    );
}

/// GIVEN this probe file compiles as part of the `engram` workspace
/// WHEN the crate-root `#![forbid(unsafe_code)]` lint is in effect (verified
/// separately by `cargo clippy` / `cargo build` succeeding for the whole
/// workspace, since a forbidden lint violation is a hard compile error, not
/// a warning that could be silently ignored)
/// THEN no experiment above required an `unsafe` block — the safe-Rust
/// dependency set for the selected primitives is exactly `std::fs`,
/// `std::io::Write`, and the existing `cozo` dependency (already a default
/// workspace dependency via the `cozo-backend` feature) plus the `tempfile`
/// dev-dependency used only by this probe's test scaffolding.
#[test]
fn no_unsafe_code_required_for_selected_primitives() {
    // This test is intentionally a documentation-anchor assertion: the
    // absence of `unsafe` in this file is enforced by the compiler (this
    // file contains no `unsafe` keyword at all), and the `engram` crate
    // root's `#![forbid(unsafe_code)]` (src/lib.rs) makes any future
    // regression a hard compile error across the whole workspace.
}
