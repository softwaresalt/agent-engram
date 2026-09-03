# Decision: Generation Storage Feasibility Spike (F01 / S1)

* **Status**: DECIDED — GO
* **Date**: 2026-09-02 (spike executed and recorded 2026-09-03 as part of shipment 133-S)
* **Feature**: F01 — Storage feasibility spike (task `142.004-T`)
* **Plan reference**: `docs/exec-plans/2026-09-02-separate-indexer-read-server-plan.md`,
  "Mandatory feasibility spike / S1 — Prove storage and publication primitives"
  (Remediation Revision 2, requirement R49).
* **Probe test**: `tests/integration/generation_storage_probe_test.rs`
  (4 tests, all passing; see verification evidence below).

## Context

Before any generation open/publication implementation work (F06–F09, later
shipments) proceeds, the plan requires this spike to empirically settle four
questions about the storage and publication primitives the read-server will
rely on:

1. Can Cozo's SQLite backend open an existing database in a way that is
   provably read-only (no bootstrap/sidecar mutation on open)?
2. If (1) is disproven, does a private-runtime-copy fallback preserve true
   immutability of the original (published) generation's on-disk file?
3. Is replace-existing atomicity/durability achievable using only safe
   `std::fs` primitives, and is it byte-stable (no truncation/partial-write
   torn state) on this platform?
4. Is any `unsafe` code required for the above?

## Findings

### 1. Cozo SQLite read-only open — NOT POSSIBLE (negative result)

Direct source inspection of the `cozo` 0.7.6 crate
(`cozo-0.7.6/src/storage/sqlite.rs`, `new_cozo_sqlite`) shows that the
`options` string argument to `DbInstance::new("sqlite", path, options)` is
**completely ignored**: every open unconditionally executes
`create table if not exists cozo (...)` DDL and calls `Db::initialize()`
(which in turn calls `load_last_ids()`). There is no read-only variant of
this constructor anywhere in the public API.

This was confirmed empirically by
`cozo_sqlite_open_is_not_provably_read_only`: a database seeded once and
then re-opened with a fresh `DbInstance::new` call still accepts a mutable
`run_default` script without error. Cozo 0.7.6 grants no enforced read-only
capability at the API level — a caller cannot prove read-only-ness of an
open by construction, only by discipline (never issuing a mutable script).

**Conclusion**: experiment 1 is a hard negative. The private-runtime-copy
fallback (experiment 2) is required and is the selected strategy — this also
matches the plan's own F09 unit description, which already commits to
runtime copies under `.engram/generations/runtime`.

### 2. Private-runtime-copy fallback — PROVEN

`private_runtime_copy_preserves_original_byte_stability` copies a seeded
Cozo SQLite database file byte-for-byte to an isolated runtime location,
opens and further mutates **only the copy**, and then re-reads the
**original** published file's bytes. The original is byte-identical
before and after the copy is created and independently mutated, while the
copy correctly observes both the original seed row and the runtime-only
mutation.

**Conclusion**: a plain `fs::copy` into a private runtime path, followed by
opening/mutating only the copy, gives true read-side immutability for the
published generation, regardless of what Cozo does internally to the
runtime copy on open. This is the selected strategy for F06–F09.

### 3. Atomic replace-existing via safe `std::fs::rename` — PROVEN, byte-stable

`atomic_replace_existing_rename_is_byte_stable` writes a replacement file
to a co-located staging path, `fsync`s it (`File::sync_all`), then calls
`std::fs::rename(staging, destination)` where `destination` already exists.
The rename succeeds without requiring a prior unlink of the destination,
the staging name no longer exists afterward, and the destination contains
the replacement's exact byte content with no truncation or partial-write
torn state.

`std::fs::rename` is documented by the Rust standard library to behave as
POSIX `rename(2)` on Unix and as `MoveFileExW` with
`MOVEFILE_REPLACE_EXISTING` on Windows — both are atomic replace-existing
operations at the OS level. This probe was executed and passed on Windows
(the current development platform); the cross-platform guarantee for Unix
comes from the standard library's documented contract plus the identical
POSIX semantics `rename(2)` provides, which this probe's assertions do not
depend on any Windows-specific behavior to satisfy.

**Conclusion**: `std::fs::rename`, preceded by a co-located
write-then-`fsync` of the replacement, is the selected safe-Rust
replace-existing primitive for generation publication (F07/F08).

### 4. No `unsafe` code required — CONFIRMED

None of the three probes above required an `unsafe` block. The probe file
contains no `unsafe` keyword. The `engram` crate root already carries
`#![forbid(unsafe_code)]` (`src/lib.rs`), which makes any future regression
a hard compile error across the whole workspace, not merely a lint warning.

**Conclusion**: the safe-Rust dependency set for the selected primitives is
exactly `std::fs`, `std::io::Write`, and the existing `cozo` dependency
(already a default workspace dependency via the `cozo-backend` feature),
plus the `tempfile` dev-dependency used only by this probe's test
scaffolding.

## Verification evidence

* `cargo test --test integration_generation_storage_probe` — 4/4 passed.
* `cargo clippy --all-targets -- -D warnings -D clippy::pedantic` — clean.
* `cargo fmt --all -- --check` — clean.
* Full `cargo dev-test` — clean (see shipment 133-S build evidence).
* Source inspection: `~/.cargo/registry/src/index.crates.io-1949cf8c6b5b557f/cozo-0.7.6/src/storage/sqlite.rs`
  (lines ~37–60, `new_cozo_sqlite`).

## Verdict: GO

All four S1 experiments are settled. F06–F09 (later shipments) should
proceed on the following basis:

* **Do not** attempt to rely on any Cozo-level read-only open guarantee —
  none exists in 0.7.6.
* **Do** use a private-runtime-copy (`fs::copy` into
  `.engram/generations/runtime/...` or equivalent) as the sole mechanism for
  giving the read-server a mutable working set without touching the
  published generation's on-disk file.
* **Do** use write-to-staging + `fsync` + `std::fs::rename` as the
  replace-existing publication primitive — no unsafe code, no third-party
  atomic-file crate required.
* No `unsafe` code is needed anywhere in the storage/publication path
  established by this spike.

## Forward pointers

* F06 (open path) and F09 (runtime copy management) apply the
  private-runtime-copy fallback established here.
* F07/F08 (publication path) apply the `fsync` + `rename` replace-existing
  primitive established here.
* This decision does not implement any of F06–F09; it only proves the
  primitives are viable ahead of that (later-shipment) implementation work,
  per the plan's explicit sequencing.
