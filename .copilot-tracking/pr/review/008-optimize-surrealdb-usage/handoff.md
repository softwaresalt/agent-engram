<!-- markdownlint-disable-file -->
# PR Review Handoff: 008-optimize-surrealdb-usage

## PR Overview

This branch adds file hash tracking for offline change detection, MTREE KNN vector search,
glob-filtered content ingestion, and various observability and reliability improvements.

* Branch: `008-optimize-surrealdb-usage`
* Base Branch: `main`
* Total Files Changed (src/ + tests/ + Cargo.toml): 41
* Total Review Comments: 8 (all resolved and committed in `9416c5a`)

## Changes Applied (commit 9416c5a)

All review items were fixed directly in the branch before PR submission.

### File: `src/services/file_tracker.rs`

#### Comment 1 — Wrong error variant for file I/O (RI-01)

* Category: Code Quality / Reliability
* Severity: Medium
* Status: ✅ Fixed

`compute_file_hash` and `record_file_hash` were mapping `io::Error` to
`SystemError::DatabaseError`, making it impossible for callers to distinguish
disk failures from database failures. Changed to `IngestionError::Failed { path, reason }`,
which is semantically precise and consistent with how ingestion failures are reported
elsewhere in the codebase.

#### Comment 2 — Blocking I/O inside async context (RI-02)

* Category: Reliability / Performance
* Severity: High
* Status: ✅ Fixed

`detect_offline_changes` was calling `std::fs::read_dir`, `std::fs::metadata`, and
`std::fs::read` directly in an async function, potentially stalling the Tokio event loop
for seconds on large workspaces. All file-system work is now performed inside
`tokio::task::spawn_blocking` via the extracted `compare_disk_to_stored` function.

#### Comment 3 — `size_bytes` stored but never used for optimization (RI-03)

* Category: Performance
* Severity: Low–Medium
* Status: ✅ Fixed

Added a two-level fast-path in `compare_disk_to_stored`: when the current file size
differs from the stored `size_bytes`, the file is immediately classified as Modified
without paying for a full SHA-256 read. Only size-equal files proceed to hash
verification. This mirrors the two-level heuristic already used by `sync_workspace`.

#### Comment 8 — Three-pass iterator for log counters (RI-08)

* Category: Code Quality
* Severity: Low
* Status: ✅ Fixed

Replaced the three separate `iter().filter().count()` calls at the end of
`detect_offline_changes` with a single `fold` pass that accumulates all three
counters simultaneously.

---

### File: `src/models/file_hash.rs` (new) + `src/models/mod.rs`

#### Comment 4 — `FileHashRecord` in wrong layer (RI-04)

* Category: Architecture / Conventions
* Severity: Medium
* Status: ✅ Fixed

`FileHashRecord` is a public domain model type and belongs in `src/models/` alongside
`CodeFile`, `ContentRecord`, etc. Created `src/models/file_hash.rs`, re-exported
`FileHashRecord` from `src/models/mod.rs`, and changed `src/db/queries.rs` to
`pub use crate::models::FileHashRecord` so all existing callers compile unchanged.

---

### File: `src/daemon/watcher.rs` + `src/services/file_tracker.rs`

#### Comment 5 — Duplicate exclusion list (RI-05)

* Category: Maintainability
* Severity: Medium
* Status: ✅ Fixed

Extracted `DEFAULT_EXCLUDE_PREFIXES: &[&str]` as a `pub const` in `daemon/watcher.rs`
and updated `WatcherConfig::default()` to use it. `file_tracker.rs` now imports and uses
this single source of truth, eliminating the duplicate hardcoded `EXCLUDED_PREFIXES`
constant.

---

### File: `src/tools/lifecycle.rs`

#### Comment 6 — `detect_offline_changes` never called (RI-06)

* Category: Functional Correctness
* Severity: High
* Status: ✅ Fixed

`detect_offline_changes` and `record_file_hash` were fully implemented and tested but
never called from the daemon lifecycle, meaning the `file_hash` table was always empty
in production. Added a call to `detect_offline_changes` in `set_workspace` immediately
after `hydrate_code_graph`. Results are logged:

* `info` with count when offline changes are found (prompting a `sync_workspace` call)
* `debug` when no changes are detected
* `warn` when detection itself fails (best-effort; workspace binding continues)

---

### File: `src/db/queries.rs`

#### Comment 7 — Incorrect metrics in `upsert_file_hash` (RI-07)

* Category: Observability
* Severity: Low
* Status: ✅ Fixed

`upsert_file_hash` was calling `record_query_metrics("crud", "file_hash", 1, Duration::ZERO)`,
hard-coding zero elapsed time regardless of actual latency. Added
`let start = std::time::Instant::now()` before the DB call and replaced `Duration::ZERO`
with `start.elapsed()`, consistent with every other timed query method in the file.

---

## Review Summary by Category

| Category | Count |
|----------|-------|
| Functional Correctness | 1 (RI-06) |
| Reliability / Performance | 2 (RI-02, RI-03) |
| Architecture / Conventions | 2 (RI-04, RI-05) |
| Code Quality | 2 (RI-01, RI-08) |
| Observability | 1 (RI-07) |

## Instruction Compliance

| Instruction File | Status | Notes |
|-----------------|--------|-------|
| Rust coding conventions (`**/*.rs`) | ✅ | `#[forbid(unsafe_code)]`, pedantic clippy clean, `Result`/`EngramError`, `#[tracing::instrument]` |
| MCP Protocol Fidelity (Constitution II) | ✅ | No tool schema changes in this fix commit |
| Test-First Development (Constitution III) | ✅ | All 8 integration tests remain green after fixes |
| Workspace Isolation (Constitution IV) | ✅ | No path traversal risks introduced |
| Structured Observability (Constitution V) | ✅ | Tracing added to `set_workspace` offline detection path |
| Single-Binary Simplicity (Constitution VI) | ✅ | No new dependencies added |

## Outstanding Risks / Follow-up

* **TASK-010** (`009-09: Wire record_file_hash into indexing and watcher pipeline`) — tracked in the 009 milestone backlog. `record_file_hash` is not yet called from `index_workspace_inner` or the watcher event loop. The `file_hash` table will be populated on startup detection but hashes won't stay current between daemon runs until those call sites are wired in. See `TASK-010` for the full work breakdown and acceptance criteria.
* On the very first run (empty `file_hash` table), `detect_offline_changes` returns every workspace file as Added. Callers should treat a large first-run Added list as expected behavior rather than an error.
