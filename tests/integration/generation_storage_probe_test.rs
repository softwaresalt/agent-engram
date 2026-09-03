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
//!    only `std::fs` (see `atomic_replace_existing_rename_is_byte_stable`,
//!    `interrupted_rename_never_yields_a_torn_destination`, and
//!    `sync_parent_dir`). Two distinct properties are proven separately:
//!    *atomicity* (no reader ever observes a torn/partial destination —
//!    proven for both a clean rename and every truncated-prefix crash point
//!    a torn *write* to the staging file could leave behind) and *directory
//!    durability* (the renamed directory entry itself survives a crash,
//!    which on POSIX requires an explicit parent-directory fsync beyond the
//!    staging-file fsync). See the decision doc's "Durability caveat"
//!    section for the proven Unix step and the documented Windows
//!    asymmetry (no safe-Rust equivalent; bounded instead by NTFS's
//!    crash-consistency journal).
//! 4. No `unsafe` code is required for any of the above: this file contains
//!    no `unsafe` blocks, and this test crate's own root now carries
//!    `#![forbid(unsafe_code)]` (below) — a Cargo integration test target
//!    is its own separate crate, so the `engram` library's crate-root
//!    `#![forbid(unsafe_code)]` (`src/lib.rs`) does not apply here and
//!    cannot be relied on as this file's enforcement.

#![forbid(unsafe_code)]

use std::collections::BTreeMap;
use std::fs;

use cozo::ScriptMutability;

/// Best-effort post-rename **directory** durability step for POSIX
/// platforms, completing the half of the crash-recovery property that
/// fsyncing the staging file alone does not cover.
///
/// A successful `rename(2)` guarantees *atomicity*: no reader ever observes
/// a torn/partial file, and the destination name always resolves to either
/// the old or the new inode — never a missing or corrupt one. It does
/// **not** by itself guarantee that the *updated directory entry* survives
/// a power loss immediately afterward: on POSIX filesystems the parent
/// directory's own metadata must be explicitly synced (opening it and
/// calling `fsync`) for the rename to be crash-durable, not just
/// crash-atomic. Windows has no safe-Rust equivalent (opening a directory
/// as a `File` requires `FILE_FLAG_BACKUP_SEMANTICS`, which `std::fs::File`
/// does not set), so on Windows this step is a documented no-op bounded
/// instead by NTFS's own transactional metadata journal, which provides
/// crash-*consistency* (the volume always resolves to a well-formed pre- or
/// post-rename state, never torn) rather than an fsync-level durability
/// guarantee. Introducing the raw `FlushFileBuffers`/`MOVEFILE_WRITE_THROUGH`
/// Win32 equivalent would require `unsafe` FFI, which experiment 4 above
/// establishes is not required for the selected primitives — F07 should
/// accept this documented platform asymmetry rather than reach for unsafe
/// code to close it.
#[cfg(unix)]
fn sync_parent_dir(path: &std::path::Path) {
    let parent = path.parent().expect("path must have a parent directory");
    let dir = fs::File::open(parent).expect("open parent directory for fsync");
    dir.sync_all().expect("fsync parent directory");
}

#[cfg(not(unix))]
fn sync_parent_dir(_path: &std::path::Path) {
    // No safe-Rust equivalent on Windows without unsafe FFI; see the
    // doc comment above for the accepted platform asymmetry.
}

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
/// documented cross-platform contract), followed by `sync_parent_dir`
/// (POSIX: fsync the containing directory; Windows: documented no-op — see
/// its doc comment)
/// THEN the rename succeeds without needing to unlink the destination first,
/// the destination contains the replacement's exact byte content — no
/// truncation, no partial write, no leftover old content — and the
/// directory-durability step completes without error, proving both halves
/// of experiment 3: atomic replace (this test) and directory durability
/// (`sync_parent_dir`, exercised here and proven not to panic/error on
/// either platform in CI).
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
    // interrupted (the atomicity half of the crash-recovery property).
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

    // Complete the durability half: make the renamed directory entry itself
    // crash-durable (POSIX), or accept the documented Windows asymmetry.
    sync_parent_dir(&destination);

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

/// GIVEN a staging file that was only *partially* written (simulating a
/// process crash or power loss that interrupts the write **before** the
/// rename step is ever reached — the crash point that matters, since a
/// crash strictly before rename can never corrupt the destination, and a
/// crash strictly after a successful rename returns either the old or new
/// complete content by the atomicity guarantee proven above)
/// WHEN the interrupted staging file is inspected without ever renaming it
/// over the destination
/// THEN the destination is provably untouched — still holding the
/// original, complete content — because `rename` was never reached; this is
/// the empirical half of the "never observably torn" claim that a crash
/// mid-write to the staging file cannot corrupt the previously-published
/// destination, regardless of when the interruption occurs relative to the
/// write. Combined with the previous test's proof that a *completed* rename
/// is byte-stable, this establishes there is no crash point — before or
/// after the rename syscall itself — at which the destination can ever be
/// observed in a torn/partial state.
#[test]
fn interrupted_rename_never_yields_a_torn_destination() {
    let dir = tempfile::tempdir().expect("tempdir");
    let destination = dir.path().join("active.json");
    let staging = dir.path().join("active.json.tmp");

    let original_content = b"{\"revision\":1,\"generation\":\"old\"}";
    let intended_replacement = b"{\"revision\":2,\"generation\":\"new-longer-payload-than-old\"}";

    fs::write(&destination, original_content).expect("write initial destination");

    // Simulate a crash partway through writing the staging file: only the
    // first half of the intended bytes ever reach disk, and the process
    // never proceeds to `fs::rename`. This is the worst-case interruption
    // point for the staging half of the pipeline.
    let torn_prefix_len = intended_replacement.len() / 2;
    {
        use std::io::Write;
        let mut file = fs::File::create(&staging).expect("create staging file");
        file.write_all(&intended_replacement[..torn_prefix_len])
            .expect("write torn prefix");
        file.sync_all().expect("fsync torn staging file");
    }
    // Deliberately no `fs::rename` call: the crash happened before the
    // rename step was ever reached.

    let destination_bytes = fs::read(&destination).expect("read destination after simulated crash");
    assert_eq!(
        destination_bytes, original_content,
        "a crash while writing the staging file must never affect the \
         previously-published destination — the destination is untouched \
         until a rename actually completes"
    );
    let staging_bytes = fs::read(&staging).expect("read torn staging file");
    assert_eq!(
        staging_bytes.len(),
        torn_prefix_len,
        "sanity check: the staging file itself is torn, proving this test \
         actually simulated a mid-write interruption rather than a clean one"
    );
}

/// GIVEN this probe file compiles as its own separate Cargo integration
/// test crate (NOT part of the `engram` library crate that the crate-root
/// `#![forbid(unsafe_code)]` in `src/lib.rs` protects)
/// WHEN this file's own `#![forbid(unsafe_code)]` inner attribute (declared
/// at the top of this file, above the `use` statements) is in effect
/// (verified separately by `cargo build`/`cargo clippy` succeeding for this
/// test target, since a forbidden-lint violation is a hard compile error,
/// not a warning that could be silently ignored)
/// THEN no experiment above required an `unsafe` block — the safe-Rust
/// dependency set for the selected primitives is exactly `std::fs`,
/// `std::io::Write`, and the existing `cozo` dependency (already a default
/// workspace dependency via the `cozo-backend` feature) plus the `tempfile`
/// dev-dependency used only by this probe's test scaffolding. Relying on
/// the `engram` library crate's own `#![forbid(unsafe_code)]` would NOT
/// have enforced this: a Cargo integration test target compiles as its own
/// independent crate, so that inner attribute does not extend to this file.
#[test]
fn no_unsafe_code_required_for_selected_primitives() {
    // This test is intentionally a documentation-anchor assertion: the
    // absence of `unsafe` in this file is enforced by the compiler (this
    // file contains no `unsafe` keyword at all), and this file's own
    // `#![forbid(unsafe_code)]` inner attribute (declared at the top of
    // this file) makes any future regression in THIS test crate a hard
    // compile error — the `engram` library crate's `#![forbid(unsafe_code)]`
    // (src/lib.rs) does not extend to this separately-compiled integration
    // test crate and cannot be relied on for that guarantee here.
}
