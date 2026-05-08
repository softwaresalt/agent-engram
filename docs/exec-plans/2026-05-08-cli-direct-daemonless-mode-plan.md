---
title: "CLI-Direct Daemonless Mode — Implementation Plan"
type: impl-plan
date: 2026-05-08
feature: 045-F
shipment: 030-S
source: docs/decisions/2026-05-08-cli-direct-daemonless-mode-deliberation.md
status: decided
---

## Problem Frame

All engram CLI subcommands route through the daemon via IPC (`run_tool()` in `src/cli/runner.rs` → `send_request()` → daemon handler). For indexing commands (`sync`, `index`), this forces a persistent daemon process, conflicts with ongoing `background_db_hydration`, and creates 30s IPC timeout failures on large workspaces.

The chosen solution (deliberation Option D) adds a `--direct` flag to `sync` and `index` that bypasses the daemon entirely, calling the service-layer functions directly. `DaemonLock` provides mutual exclusion — CLI-direct and daemon cannot operate on the same workspace simultaneously. Freshness is derived from file-hash tracking: `sync_workspace()` records per-file hashes (matching what `index_workspace()` already does), so the daemon's `detect_offline_changes()` returns 0 changes after a `--direct` run and skips re-indexing.

## Requirements Trace

| Requirement (from deliberation) | Implementation Unit |
|---|---|
| `--direct` flag on `sync` and `index` | Unit 2 |
| `ENGRAM_DIRECT=1` env var support | Unit 2 |
| `DaemonLock` mutual exclusion | Unit 1 |
| Service-layer direct invocation | Unit 1 |
| `sync_workspace()` records file hashes | Unit 3 |
| Index freshness marker | Unit 3 |
| Daemon startup fast-path | Unit 3 |
| Integration tests | Unit 4 |

## Implementation Units

### Unit 1: Direct Runner Module (045.001-T)

**What**: Create `src/cli/direct.rs` — a lightweight runner that opens CozoDB directly, calls service-layer functions, and exits.

**Files affected**:
- `src/cli/direct.rs` (new — ~80 lines)
- `src/cli/mod.rs` (add `pub mod direct;`)

**Implementation**:

```rust
// src/cli/direct.rs
pub async fn run_direct_sync(
    workspace: &Path,
    full: bool,
    formatter: &OutputFormatter,
) -> i32 {
    // 1. Acquire DaemonLock — exit 2 if daemon is running
    let _lock = match DaemonLock::acquire(workspace) {
        Ok(lock) => lock,
        Err(EngramError::Lock(LockError::AlreadyHeld { pid })) => {
            return formatter.cli_error(&format!(
                "daemon (PID {pid}) is running in this workspace. \
                 Stop it first or omit --direct."
            ));
        }
        Err(e) => return formatter.cli_error(&format!("lock error: {e}")),
    };

    // 2. Resolve workspace metadata (all errors → cli_error exit)
    let ws_str = match workspace.to_str() {
        Some(s) => s,
        None => return formatter.cli_error("workspace path is not valid UTF-8"),
    };
    let canonical = match canonicalize_workspace(ws_str) {
        Ok(p) => p,
        Err(e) => return formatter.cli_error(&format!("workspace error: {e}")),
    };
    let data_dir = resolve_data_dir(&canonical);
    let branch = resolve_git_branch(&canonical)
        .unwrap_or_else(|_| "main".to_owned());
    let config = match parse_config(&canonical) {
        Ok(c) => c,
        Err(e) => {
            warn!(error = %e, "config parse failed, using defaults");
            WorkspaceConfig::default()
        }
    };

    // 3. Dispatch to service layer
    let result = if full {
        index_workspace(&canonical, &data_dir, &branch, &config.code_graph, true).await
    } else {
        sync_workspace(&canonical, &data_dir, &branch, &config.code_graph).await
    };
    match result {
        Ok(r) => formatter.success(None, serde_json::json!(r)),
        Err(e) => formatter.cli_error(&format!("{e}")),
    }
    // 4. DaemonLock dropped here — released automatically
}
```

**Key decisions**:
- Reuse `DaemonLock` as-is — no new locking code
- `parse_config()` called directly (already standalone, returns defaults if missing)
- `resolve_git_branch()` with fallback to `"main"` for non-git workspaces
- Error handling via `formatter.cli_error()` for consistency with `run_tool()` exit codes

**Tests**: Compilation test only — integration tests in Unit 4.

**Execution posture**: Test-first (harness in Unit 4, implementation here).

---

### Unit 2: Wire `--direct` Flag and Env Var (045.002-T)

**What**: Add `--direct` flag to `Sync` and `Index` subcommands; add `ENGRAM_DIRECT` env var check; dispatch to `direct::run_direct_sync()` when active.

**Files affected**:
- `src/bin/engram.rs` (add `direct` field to `Sync` and `Index` variants — ~10 lines)
- `src/cli/commands/indexing.rs` (add `direct` parameter, dispatch logic — ~15 lines)

**Implementation in `engram.rs`**:

```rust
Sync {
    #[arg(long)]
    full: bool,
    /// Bypass the daemon and index directly (no daemon required).
    /// Also enabled via ENGRAM_DIRECT=1 env var.
    #[arg(long, env = "ENGRAM_DIRECT")]
    direct: bool,
},
Index {
    /// Bypass the daemon and index directly (no daemon required).
    #[arg(long, env = "ENGRAM_DIRECT")]
    direct: bool,
},
```

**Implementation in `indexing.rs`**:

```rust
pub async fn run_sync(
    full: bool,
    direct: bool,
    flags: &GlobalFlags,
    formatter: &OutputFormatter,
) -> i32 {
    if direct {
        let workspace = match flags.resolve_workspace() {
            Ok(p) => p,
            Err(e) => return formatter.cli_error(&e),
        };
        return direct::run_direct_sync(&workspace, full, formatter).await;
    }
    // existing IPC path unchanged
    if full {
        run_tool("index_workspace", None, flags, formatter).await
    } else {
        run_tool("sync_workspace", None, flags, formatter).await
    }
}
```

**Key decisions**:
- `#[arg(long, env = "ENGRAM_DIRECT")]` uses clap's built-in env var support — `ENGRAM_DIRECT=1` auto-sets `direct = true`
- Flag is per-subcommand (not global) — only `Sync` and `Index` support it
- `run_sync()` and `run_index()` signature change is backward-compatible (callers updated in same commit)

**Tests**: Existing `sync_no_full_uses_sync_workspace` tests remain valid; flag routing tested in Unit 4.

**Execution posture**: Implementation-first (straightforward wiring).

---

### Unit 3: Integration Tests (045.003-T)

**What**: Ensure the daemon skips re-indexing when the DB is already current after a `--direct` run.

**Files affected**:
- `src/services/code_graph.rs` — add `record_file_hash` calls to `sync_workspace()` (~5 lines per site)
- `src/tools/lifecycle.rs` — fast-path `background_db_hydration` when DB has current data (~15 lines)
- `src/services/hydration.rs` — skip JSONL reload when DB is already populated (~10 lines)

**Implementation — Gap 1 (sync records hashes)**:

In `sync_workspace()`, after each successfully synced file (around line 718, after `upsert_code_file`):

```rust
if let Err(e) = crate::services::file_tracker::record_file_hash(
    &rel_path, file_path, &queries
).await {
    debug!(error = %e, path = %rel_path,
        "code graph sync: file hash recording failed");
}
```

This matches what `index_workspace()` already does (line 448).

**Implementation — Gap 2 (skip JSONL re-load)**:

In `hydrate_code_graph()` (`src/services/hydration.rs`), before loading JSONL files:

```rust
// If the DB already has code files, skip JSONL re-loading.
// The JSONL files are dehydration artifacts for persistence; the DB
// is the source of truth once populated.
let existing_count = match cg_queries.count_code_files().await {
    Ok(n) => n,
    Err(e) => {
        warn!(error = %e, "count_code_files failed, falling back to JSONL reload");
        0
    }
};
if existing_count > 0 {
    info!(existing = existing_count, "code graph already populated, skipping JSONL reload");
    return Ok(CodeGraphHydrationResult::default());
}
```

Requires adding `count_code_files()` to `CodeGraphQueries` — a simple `?[count(id)] := *code_files{id}` Datalog query.

**Implementation — Gap 3 (fast-path offline detection)**:

The daemon already skips re-indexing when `offline_count == 0` (line 325 in `lifecycle.rs`). With Gap 1 fixed, this path fires correctly after a `--direct` sync. No additional code needed for the basic case.

**Key decisions**:
- Skip JSONL reload based on `code_files` count, not a freshness timestamp — simpler and avoids clock skew
- `record_file_hash` in `sync_workspace` is a bug fix (file hashes should always be recorded)
- The `detect_offline_changes` scan still runs (~100ms) but returns 0 changes; a future optimization could skip it entirely via a "last indexed at" marker, but that's not needed for this shipment

**Tests**: Existing `detect_offline_changes` tests validate the hash comparison. Integration test in Unit 4 verifies the daemon startup fast-path.

**Execution posture**: Test-first for `record_file_hash` in sync; characterization-first for hydration skip.

---

### Unit 4: Index Freshness Detection (045.004-T)

**What**: End-to-end tests verifying CLI-direct mode and daemon freshness detection.

**Files affected**:
- `tests/integration/cli_direct_test.rs` (new — ~120 lines)

**Test cases**:

1. **`direct_sync_produces_valid_result`**: Run `engram sync --direct --json` on a temp workspace with Rust files. Assert exit 0, parse `SyncResult` from stdout JSON.

2. **`direct_index_produces_valid_result`**: Run `engram index --direct --json` on same temp workspace. Assert exit 0, parse `IndexResult`.

3. **`direct_mode_mutex_with_daemon`**: Start daemon via `DaemonHarness`, then run `engram sync --direct`. Assert exit 2 and stderr contains "daemon (PID".

4. **`env_var_activates_direct_mode`**: Set `ENGRAM_DIRECT=1`, run `engram sync --json` without `--direct` flag. Assert exit 0 (direct mode activated by env var).

5. **`daemon_skips_reindex_after_direct`**: Run `engram index --direct --json`, then start daemon. Assert daemon logs contain "offline changes detected" with count 0 (or "index is current"). Assert daemon reaches ready state without re-indexing.

**Key decisions**:
- Tests use `env!("CARGO_BIN_EXE_engram")` for binary location (per test harness convention)
- `.env_remove("ENGRAM_DATA_DIR")` in all subprocess spawns (per compound learning)
- Deadline-based polling for daemon readiness (per 028-S test fix pattern)
- Temp workspace created with `tempfile::tempdir()` containing a small Rust file

**Execution posture**: Test-first (harness written before implementation units).

## Dependency Graph

```
045.001-T (direct runner)
    ↓
045.002-T (--direct flag)     045.004-T (freshness detection)
    ↓                              ↓
         045.003-T (integration tests)
```

**Execution order**: 045.001-T → 045.002-T + 045.004-T (parallel) → 045.003-T

No cycles. 045.002-T and 045.004-T are independent after 045.001-T completes.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Reuse `DaemonLock` for mutual exclusion | Already proven, OS-level, stale-PID recovery built in. No new code. |
| `#[arg(env = "ENGRAM_DIRECT")]` for env var | clap handles parsing; no manual `std::env::var` needed |
| Skip JSONL reload by `code_files` count | Simpler than timestamp comparison; immune to clock skew |
| `record_file_hash` in sync is a bug fix | Index already does it; sync should too for consistency |
| Per-subcommand `--direct` (not global) | Only indexing commands make sense in direct mode |

## Risks and Caveats

| Risk | Likelihood | Impact | Mitigation |
|---|---|---|---|
| `DaemonLock` not visible in `src/cli/` | Low | Low | It's `pub` in `daemon::lockfile` — just import it |
| Config edge cases (missing `.engram/`) | Low | Low | `parse_config()` returns defaults; `resolve_data_dir()` creates dirs |
| Windows path normalization in direct mode | Low | Medium | `canonicalize_workspace()` already handles `\\?\` prefix stripping |
| Test flakiness from timing | Medium | Low | Deadline-based polling per compound learning `engram-data-dir-inherited-by-test-daemon-spawns` |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | **No** | New flag is additive; existing behavior unchanged |
| Security, auth, permission, or compliance | **No** | `DaemonLock` is an existing security boundary; no new attack surface |
| Migration, destructive data/config action | **No** | No data migration; DB schema unchanged |
| External integration or operator checkpoint | **No** | Self-contained within the CLI binary |
| High runtime, rollout, or rollback risk | **No** | Fully backward-compatible; `--direct` is opt-in; rollback = remove flag |

**Requires plan hardening: no**

## Runtime Verification and Closure

| Unit | Runtime Surface | Verification | Closure |
|---|---|---|---|
| Unit 1 (direct runner) | CLI exit codes | `engram sync --direct --json` returns valid JSON, exit 0 | Verify no orphaned processes |
| Unit 2 (flag wiring) | CLI help text, env var | `engram sync --help` shows `--direct`; `ENGRAM_DIRECT=1` works | Document in `--help` |
| Unit 3 (freshness) | Daemon startup latency | Daemon logs show 0 offline changes after direct run | Monitor startup time regression |
| Unit 4 (tests) | CI green | All 5 integration tests pass | CI gate |

## Plan Review

**Gate decision: PASS**

Reviewed by: Constitution Reviewer, Rust Reviewer, Scope Boundary Auditor, Learnings Researcher, Architecture Strategist

### Constitution Check

All constitutional principles satisfied:

- **I. Safety-First Rust**: All proposed code returns `Result<T, EngramError>`. Pseudocode uses illustrative fallback patterns (`unwrap_or_else`, `match` with `warn` + fallback); actual implementation must propagate errors via `?` or explicit `match` with logged fallback — never bare `unwrap()` or `expect()`. `DaemonLock` uses existing error types.
- **II. Test-First Development**: Unit 4 provides integration tests. Execution posture is test-first where appropriate.
- **III. Workspace Isolation**: `canonicalize_workspace()` already validates paths. Direct mode reuses the same validation.
- **IV. CLI Containment**: No out-of-workspace operations introduced.
- **VII. Destructive Approval**: No destructive operations; `--direct` is read/write to the DB only within the workspace `.engram/` directory.
- **X. Context Efficiency**: Plan uses targeted service-layer functions, not bulk scanning.

### Rust Reviewer Findings

No P0 or P1 findings.

- **P3 — Pseudocode uses `unwrap_or_default()` on `parse_config`**: The plan shows `parse_config(&canonical).unwrap_or_default()` but `parse_config` already returns `Ok(default)` on missing file. Implementation should propagate the `Result` properly with `?` or explicit match, not silently swallow config validation errors. The pseudocode is illustrative only; the actual implementation should use `?` after `parse_config`. *Advisory.*

- **P3 — `canonicalize_workspace` param is `&str` not `&Path`**: The plan shows `canonicalize_workspace(workspace.to_str().unwrap_or_default())`. Implementation should use proper `Path` → `str` conversion or extend `canonicalize_workspace` to accept `&Path`. The existing function takes `&str`, so `to_str()` is needed, but `unwrap_or_default()` would silently pass `""` on non-UTF-8 paths. Use `.ok_or_else(|| ...)` instead. *Advisory.*

### Scope Boundary Auditor Findings

No P0 or P1 findings.

- **P2 — Unit 3 `count_code_files()` is new DB query**: Plan proposes a `count_code_files()` method on `CodeGraphQueries`. This is a small addition (~5 lines) but crosses from "wiring" into "new query". Acceptable for this scope since it is a trivial query supporting the freshness skip. *Acknowledged.*

- **P3 — Flush subcommand not in scope**: The deliberation mentions `flush` as in-scope for direct mode, but the plan scopes only `sync` and `index`. This is correct — `flush` writes dehydration artifacts and is already handled by `sync_workspace`. No action needed. *Advisory.*

### Learnings Researcher Findings

No P0 or P1 findings. Relevant learnings identified and already referenced:

- `cozodb-sqlite-lock-panic-2026-05-01.md` — the advisory lock in `connect_db` serializes `DbInstance::new()` + bootstrap. Direct mode reuses `connect_db`, so this mitigation applies automatically. ✓
- `daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md` — not directly relevant (direct mode does not start a watcher). ✓
- `engram-data-dir-inherited-by-test-daemon-spawns-2026-05-08.md` — plan already references `.env_remove("ENGRAM_DATA_DIR")` in test harness. ✓
- `sqlite-busy-retry-granularity-2026-05-03.md` — not relevant because `DaemonLock` prevents concurrent access entirely. ✓

### Architecture Strategist Findings

No P0 or P1 findings.

- **P2 — Dependency graph shows 045.002-T depends on 045.001-T but they could be parallel**: The `--direct` flag wiring (045.002-T) needs `direct::run_direct_sync` to exist (045.001-T), so the dependency is correct. However, the flag wiring could compile against a stub `direct.rs` that returns exit code 1 ("not yet implemented"), allowing parallel development. Not required but could speed up development. *Acknowledged.*

- **P3 — `DaemonLock` is in `daemon` module, imported by `cli`**: This creates a dependency from `cli` → `daemon::lockfile`. The lockfile is a standalone utility with no daemon runtime dependency, so this coupling is acceptable. A future refactor could move it to a shared `util` module, but that is out of scope. *Advisory.*

### Summary

| Severity | Count | Details |
|---|---|---|
| P0 | 0 | — |
| P1 | 0 | — |
| P2 | 2 | New `count_code_files()` DB query; parallel dev opportunity |
| P3 | 3 | Pseudocode `unwrap_or_default`; `&str` conversion; lockfile module location |

**Plan hardening required**: No (confirmed — all signals absent)

**Conclusion**: Plan is sound, well-scoped, and grounded in verified codebase patterns. All requirements from the deliberation are traced to implementation units. No blocking findings. Proceed to harvest.
