<!-- markdownlint-disable-file -->
# PR Review Status: 008-optimize-surrealdb-usage

## Review Status

* Phase: 4 — Finalized Handoff
* Last Updated: 2026-03-25T04:56:46Z
* Summary: 14-commit PR adding file hash tracking, branch-aware DB isolation, MTREE KNN search, glob ingestion filtering, embedding hardening, and query observability

## Branch and Metadata

* Normalized Branch: `008-optimize-surrealdb-usage`
* Source Branch: `008-optimize-surrealdb-usage`
* Base Branch: `main`
* Linked Work Items: TASK-008.01 through TASK-008.09 (per `.backlog/` task files)
* Commits: 14 ahead of main
* Files Changed (src/ + tests/ + Cargo.toml): 41

## Diff Mapping

| File | Type | New Line Range | Notes |
|------|------|----------------|-------|
| `src/services/file_tracker.rs` | New | 1–302 | Core file hash tracking service |
| `src/db/schema.rs` | Modified | 103–116 | Added `DEFINE_FILE_HASH` table |
| `src/db/mod.rs` | Modified | 93–95 | Wired `DEFINE_FILE_HASH` into `ensure_schema` |
| `src/db/queries.rs` | Modified | 2961–3060 | Added `FileHashRecord`, `FileHashRow`, three CRUD methods |
| `src/db/workspace.rs` | Modified | 1–104 | Windows `\\?\` normalization, `resolve_git_branch`, `resolve_data_dir` |
| `src/tools/lifecycle.rs` | Modified | 1–181 | Branch-aware DB binding, `db_path` in workspace status |
| `src/tools/write.rs` | Modified | 100–220 | Ingestion wired into `index_workspace_inner` |
| `src/services/ingestion.rs` | Modified | Throughout | Glob pattern filtering, `ingest_single_file`, embedding backfill |
| `src/daemon/mod.rs` | Modified | 158–206 | Watcher event_rx forwarded to `run_with_shutdown` |
| `src/services/embedding.rs` | Modified | 1–60+ | Model switch bge-small-en-v1.5, model cache dir |
| `src/services/mod.rs` | Modified | 6 | Added `pub mod file_tracker` |
| `tests/integration/file_tracker_test.rs` | New | 1–248 | S067–S074 integration tests |
| `tests/integration/security_test.rs` | Modified | Various | Added `pattern: None` to `ContentSource` struct literals |

## Instruction Files Reviewed

* `.github/instructions/*.instructions.md` (Rust conventions): All applicable — `#![forbid(unsafe_code)]`, pedantic clippy, `Result`/`EngramError` pattern, `#[tracing::instrument]`, test-first
* `AGENTS.md` / Constitution Principles I–VIII: Applied as review criteria

## Phase 1 Log

* Tracking directory created: `.copilot-tracking/pr/review/008-optimize-surrealdb-usage/`
* PR reference generated via `git diff main..HEAD -- src/ tests/ Cargo.toml` (154 281 bytes)
* `scripts/dev-tools/pr-ref-gen.sh` not present — native git diff used instead
* Phase 1→2 transition: automated

---

## Review Items

### ✅ Approved for PR Comment

*(none yet — in Phase 3)*

### ❌ Rejected / No Action

*(none yet)*

### 🔍 In Review

#### RI-01: `compute_file_hash` misuses `SystemError::DatabaseError` for file I/O

* File: `src/services/file_tracker.rs`
* Lines: 64–72
* Category: Code Quality / Reliability
* Severity: Medium

**Description**

`compute_file_hash` maps a file-read `io::Error` to `SystemError::DatabaseError`. This is semantically wrong — a file that cannot be read is not a database error. The codebase already has `IngestionError::Failed { path, reason }` which is a precise fit. Consumers that observe the error cannot distinguish "disk I/O problem" from "SurrealDB query failed," which harms diagnostics.

**Current Code**

```rust
let content = std::fs::read(path).map_err(|e| {
    EngramError::System(SystemError::DatabaseError {
        reason: format!("cannot read file for hashing ({}): {e}", path.display()),
    })
})?;
```

**Suggested Resolution**

```rust
let content = std::fs::read(path).map_err(|e| {
    EngramError::Ingestion(IngestionError::Failed {
        path: path.display().to_string(),
        reason: format!("cannot read file for hashing: {e}"),
    })
})?;
```

Same fix applies to `record_file_hash`'s `metadata` call (line 91).

**User Decision**: Pending

---

#### RI-02: `detect_offline_changes` performs blocking synchronous I/O inside async context

* File: `src/services/file_tracker.rs`
* Lines: 118–205, plus `collect_workspace_files` / `collect_recursive` (216–255)
* Category: Reliability / Performance
* Severity: High

**Description**

`collect_workspace_files` and `compute_file_hash` both call blocking `std::fs` APIs (`read_dir`, `metadata`, `read`) directly from the async `detect_offline_changes` function. Under Tokio's work-stealing scheduler, each blocking call can stall the current worker thread, starving other tasks sharing that thread. For a workspace with thousands of files this can freeze the MCP server's event loop for seconds.

The established Tokio pattern for CPU-bound or blocking I/O is `tokio::task::spawn_blocking`.

**Suggested Resolution**

```rust
pub async fn detect_offline_changes(
    workspace_root: &Path,
    queries: &CodeGraphQueries,
) -> Result<Vec<FileChange>, EngramError> {
    let stored = queries.get_all_file_hashes().await?;
    let stored_map: HashMap<String, String> = stored
        .into_iter()
        .map(|r| (r.file_path, r.content_hash))
        .collect();

    let root = workspace_root.to_path_buf();
    let (disk_files, stored_map) = tokio::task::spawn_blocking(move || {
        let files = collect_workspace_files(&root);
        (files, stored_map)
    })
    .await
    .map_err(|e| EngramError::System(SystemError::DatabaseError {
        reason: format!("file scan task panicked: {e}"),
    }))?;

    // ... hash comparison loop ...
}
```

`compute_file_hash` can also be wrapped in `spawn_blocking` per call, or the entire hash-comparison loop can be moved inside a single `spawn_blocking` closure.

**User Decision**: Pending

---

#### RI-03: `size_bytes` stored but never used — missed optimization for large workspaces

* File: `src/services/file_tracker.rs` (lines 140–145), `src/db/schema.rs` (line 113)
* Category: Performance / Maintainability
* Severity: Low–Medium

**Description**

`file_hash` stores `size_bytes` in the schema and the `record_file_hash` function correctly writes it, but `detect_offline_changes` never reads it for a quick pre-check. The natural optimization is: if the current file's `metadata.len()` differs from the stored `size_bytes`, skip the full SHA-256 and go straight to `Modified`. If sizes match, proceed to hash verification. This is the same two-level heuristic used by `sync_workspace` (file-level `content_hash` → symbol-level `body_hash`) and would significantly reduce I/O on large workspaces where most files haven't changed.

**Suggested Resolution**

Extend `stored_map` to store `(content_hash, size_bytes)` and add a fast-path in the comparison loop:

```rust
// If sizes match, proceed to hash; if sizes differ, immediately mark Modified.
let meta = std::fs::metadata(&abs_path)?;
if let Some((stored_hash, stored_size)) = stored_map.remove(&rel) {
    if meta.len() != stored_size {
        // Fast path: size changed → Modified without hashing.
        changes.push(FileChange { kind: FileChangeKind::Modified, .. });
        continue;
    }
    let current_hash = compute_file_hash(&abs_path)?;
    if stored_hash != current_hash {
        changes.push(FileChange { kind: FileChangeKind::Modified, .. });
    }
}
```

**User Decision**: Pending

---

#### RI-04: `FileHashRecord` is defined in `db/queries.rs` — breaks established model layering

* File: `src/db/queries.rs` (lines 2961–2994)
* Category: Architecture / Conventions
* Severity: Medium

**Description**

Every other domain model type (`CodeFile`, `Function`, `Class`, `Interface`, `ContentRecord`, `WatcherEvent`, etc.) lives in `src/models/`. `FileHashRecord` is public and semantically a domain model, yet it was placed in `src/db/queries.rs` alongside internal DB row types. This breaks the established layering pattern where `models/` owns public domain types and `db/` owns DB-specific row structs.

`FileHashRow` (the private SurrealDB deserialization struct) belongs in `db/queries.rs`, but `FileHashRecord` should be extracted to `src/models/file_hash.rs` and re-exported from `src/models/mod.rs`.

**Suggested Resolution**

Create `src/models/file_hash.rs`:

```rust
//! File hash model — a stored content hash for a tracked workspace file.

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct FileHashRecord {
    pub file_path: String,
    pub content_hash: String,
    pub size_bytes: u64,
    pub recorded_at: DateTime<Utc>,
}
```

Then in `src/models/mod.rs` add `pub mod file_hash;` and re-export `FileHashRecord`.

**User Decision**: Pending

---

#### RI-05: `EXCLUDED_PREFIXES` duplicates `WatcherConfig::exclude_patterns` — violates DRY

* File: `src/services/file_tracker.rs` (lines 29–31)
* Category: Maintainability
* Severity: Medium

**Description**

`EXCLUDED_PREFIXES` is an independent hardcoded constant that mirrors the default value of `WatcherConfig::exclude_patterns` in `src/daemon/watcher.rs`. When a new exclusion is added to one (e.g., `.DS_Store`, `dist/`), the other will be silently missed. The single source of truth should be in one place.

**Suggested Resolution**

Option A — expose the default list as a `pub const` from `daemon/watcher.rs` and reference it in `file_tracker.rs`:

```rust
// in daemon/watcher.rs
pub const DEFAULT_EXCLUDE_PREFIXES: &[&str] =
    &[".engram/", ".git/", "node_modules/", "target/", ".env"];

impl Default for WatcherConfig {
    fn default() -> Self {
        Self {
            exclude_patterns: DEFAULT_EXCLUDE_PREFIXES
                .iter()
                .map(|s| s.to_string())
                .collect(),
            ..
        }
    }
}
```

Option B — accept a `&[String]` exclusion list as a parameter to `detect_offline_changes`, defaulting to `WatcherConfig::default().exclude_patterns`.

**User Decision**: Pending

---

#### RI-06: `detect_offline_changes` is not wired into the daemon lifecycle — feature is unreachable

* File: `src/tools/lifecycle.rs` (set_workspace, lines 58–106); `src/tools/write.rs` (index_workspace_inner)
* Category: Functional Correctness
* Severity: High

**Description**

`detect_offline_changes` and `record_file_hash` are implemented and tested in isolation, but neither is called from anywhere in the daemon's actual lifecycle:

- `set_workspace` (lifecycle.rs) calls `hydrate_workspace` + `hydrate_code_graph` but never calls `detect_offline_changes`
- `index_workspace_inner` (write.rs) indexes code but never calls `record_file_hash` to stamp the hashes after indexing
- The watcher event loop in `daemon/mod.rs` was simplified in this PR (removed the debounce loop) but doesn't call `record_file_hash` either

The feature as merged is entirely passive — the `file_hash` table will always be empty in production because nothing populates it, meaning `detect_offline_changes` will always return every file as `Added`.

**Suggested Resolution**

At minimum, `record_file_hash` should be called after each successful code-graph index for each indexed file, and `detect_offline_changes` should be called during `set_workspace` (after `hydrate_code_graph`) with results surfaced in `WorkspaceBinding` or logged. This is explicitly documented in the module-level comment of `file_tracker.rs` but not implemented.

**User Decision**: Pending

---

#### RI-07: `upsert_file_hash` records incorrect metrics — always 0 duration and count 1

* File: `src/db/queries.rs` (lines 3040–3042)
* Category: Observability
* Severity: Low

**Description**

```rust
record_query_metrics("crud", "file_hash", 1, std::time::Duration::ZERO);
```

This call hard-codes `1` as the result count and `Duration::ZERO` as elapsed, so the observability infrastructure always sees a zero-latency upsert regardless of how long it actually took. Every other `upsert_*` method either skips `record_query_metrics` entirely or times the operation. Omitting timing here produces misleading data in `get_health_report`.

**Suggested Resolution**

```rust
let start = std::time::Instant::now();
// ... db.query(...) ...
record_query_metrics("crud", "file_hash", 1, start.elapsed());
```

Or simply remove the call (consistent with other upsert methods that don't record metrics).

**User Decision**: Pending

---

#### RI-08: `detect_offline_changes` iterates `changes` three times for the final log event

* File: `src/services/file_tracker.rs` (lines 195–208)
* Category: Code Quality
* Severity: Low

**Description**

The tracing `info!` call at the end of `detect_offline_changes` counts added, modified, and deleted by running three separate iterator chains over the same `Vec`. This is a minor inefficiency but also a missed opportunity for readability.

**Current Code**

```rust
tracing::info!(
    added = changes.iter().filter(|c| c.kind == FileChangeKind::Added).count(),
    modified = changes.iter().filter(|c| c.kind == FileChangeKind::Modified).count(),
    deleted = changes.iter().filter(|c| c.kind == FileChangeKind::Deleted).count(),
    "file_tracker: offline change detection complete"
);
```

**Suggested Resolution**

Maintain counters during the loop itself rather than recomputing:

```rust
let (added, modified, deleted) = changes.iter().fold((0usize, 0, 0), |(a, m, d), c| {
    match c.kind {
        FileChangeKind::Added    => (a + 1, m, d),
        FileChangeKind::Modified => (a, m + 1, d),
        FileChangeKind::Deleted  => (a, m, d + 1),
    }
});
tracing::info!(added, modified, deleted, "file_tracker: offline change detection complete");
```

**User Decision**: Pending

---

## Next Steps

* [x] Phase 1: Initialize (tracking dir, diff, seed document)
* [x] Phase 2: Analyze (diff mapped, instruction files checked, 8 review items queued)
* [ ] Phase 3: Walk user through RI-01 → RI-08 sequentially, capture decisions
* [ ] Phase 4: Generate `handoff.md` with approved comments and instruction compliance table
