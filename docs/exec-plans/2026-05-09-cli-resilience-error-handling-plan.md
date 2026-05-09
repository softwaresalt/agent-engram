---
title: "CLI Resilience & Error Handling"
type: impl-plan
date: 2026-05-09
status: draft
source_stash:
  - A98E9409
  - 3AA1E6DD
  - E0CF06A6
---

## Problem Frame

Three related CLI error-handling and UX defects affect the user experience when
the engram CLI encounters non-happy-path conditions:

1. **`--direct` mode SQLITE_BUSY panic** (`A98E9409`, medium): When
   `ENGRAM_DATA_DIR` points to a directory whose CozoDB database is already
   open by a running daemon, `engram sync --direct` acquires the `DaemonLock`
   successfully (because the lock targets `.engram/run/engram.lock`, not the DB
   file) but then panics inside `cozo-0.7.6/src/storage/sqlite.rs:49` when
   CozoDB's internal `unwrap()` hits `SQLITE_BUSY`. The `connect_db` fd-lock
   mitigates multi-process races on the same `engram.db.lock` file, but when
   `ENGRAM_DATA_DIR` causes the direct-mode process to target a *different*
   data directory than the daemon's lock covers, the advisory lock is bypassed.
   The fix: detect when a daemon is already running *for the target data
   directory* and refuse to proceed, surfacing a clean error message instead of
   panicking.

2. **IndexInProgress detection hardening** (`3AA1E6DD`, low): The IPC runner in
   `src/cli/runner.rs` detects `IndexInProgress` (error code 7003) only via
   `err.data.engram_code`. If future wire format changes omit the `data` field
   or restructure it, the detection fails silently. Adding a fallback check on
   `err.code == -32603` (Internal Error, which is what the JSON-RPC layer wraps
   tool errors into) improves resilience against wire format drift.

3. **Daemon startup progress indicator** (`E0CF06A6`, low): The first CLI
   command in a fresh workspace auto-spawns the daemon via
   `ensure_daemon_running()` in `src/shim/lifecycle.rs`. The `poll_until_ready`
   loop runs for up to 30 seconds with zero user-facing output. Users see a
   frozen terminal. Fix: emit a brief stderr progress message when the CLI
   runner detects it needs to spawn the daemon.

All three touch the CLI error/UX surface. They share no external dependencies
and can be shipped together.

## Requirements Trace

| Requirement | Implementation |
|---|---|
| `--direct` mode must not panic on SQLITE_BUSY | Check for live daemon PID before opening DB in direct mode |
| Clean error message when DB locked by daemon | Return `formatter.cli_error(...)` with actionable message |
| IndexInProgress detection resilient to wire changes | Add fallback check on `err.code == -32603` with `err.message` heuristic |
| Existing IndexInProgress behavior preserved | Primary path unchanged; fallback is additive |
| Startup progress visible to user | Emit "Starting engram daemon..." on stderr before polling |
| No spurious output in non-TTY/JSON mode | Only emit progress in Text mode or when stderr is a TTY |

## Implementation Units

### Unit 1: Guard `--direct` mode against daemon-held database

**Scope**: Before calling `index_workspace`/`sync_workspace` in
`src/cli/direct.rs`, check whether a daemon process is alive for the resolved
workspace. If so, return a clean error instead of proceeding to the DB open
that will panic.

**Files affected**:

- `src/cli/direct.rs` — add daemon-alive check after `DaemonLock::acquire`

**Changes**:

The `DaemonLock::acquire` already returns `AlreadyHeld { pid }` when the daemon
lock file is held. However, when `ENGRAM_DATA_DIR` redirects the data directory,
the `DaemonLock` target and the CozoDB fd-lock target diverge. The fix adds a
secondary check: after resolving the data directory, probe the `engram.db.lock`
file to verify it is not held by another process before calling
`index_workspace`/`sync_workspace`.

```rust
// After DaemonLock::acquire succeeds and data_dir is resolved:
let db_lock_path = data_dir.join("cozo").join(&branch_safe).join("engram.db.lock");
if db_lock_path.exists() {
    let lock_file = std::fs::OpenOptions::new()
        .read(true).write(true).open(&db_lock_path)
        .map_err(|e| /* cli_error */)?;
    let mut flock = fd_lock::RwLock::new(lock_file);
    if flock.try_write().is_err() {
        return formatter.cli_error(
            "workspace database is locked by another process; \
             stop the daemon first or use IPC mode (omit --direct)"
        );
    }
    // Drop the probe lock immediately — connect_db will re-acquire properly.
}
```

**Tests**: Unit test in `src/cli/direct.rs` tests module. Integration test
using binary subprocess that starts a daemon first, then runs
`engram sync --direct` and asserts exit code 2 + error message contains
"locked by another process".

**Execution posture**: Test-first — write the integration test that expects
the clean error, verify it currently panics, then implement the guard.

### Unit 2: Harden IndexInProgress detection fallback

**Scope**: Add a secondary detection path in `src/cli/runner.rs` that
recognizes `IndexInProgress` from the JSON-RPC error code alone when the
structured `engram_code` field is absent.

**Files affected**:

- `src/cli/runner.rs` — extend the `friendly_message` detection block

**Changes**:

After the existing `engram_code` check, add a fallback:

```rust
let friendly_message = err
    .data
    .as_ref()
    .and_then(|d| d.get("engram_code"))
    .and_then(serde_json::Value::as_u64)
    .and_then(|code| {
        if code == u64::from(INDEX_IN_PROGRESS_CODE) {
            Some(/* existing message */)
        } else {
            None
        }
    })
    // Fallback: -32603 Internal Error + message containing "index" hint
    .or_else(|| {
        if err.code == -32603
            && err.message.to_lowercase().contains("index")
            && err.message.to_lowercase().contains("progress")
        {
            Some(
                "Indexing is in progress. \
                 This command will be available once indexing completes. \
                 Try again shortly."
                    .to_owned(),
            )
        } else {
            None
        }
    })
    .unwrap_or(err.message);
```

**Tests**: Add unit tests in `src/cli/runner.rs` tests module that construct
mock error responses with (a) only `err.code == -32603` + matching message,
(b) both `engram_code` and `err.code` (primary path wins), and (c) unrelated
`-32603` error (no false positive).

**Execution posture**: Test-first — write tests for all three cases, verify
they fail, then implement the fallback.

### Unit 3: Daemon startup progress indicator

**Scope**: Emit a single "Starting engram daemon..." message on stderr when
the CLI runner auto-spawns the daemon, so users know the process isn't frozen.

**Files affected**:

- `src/cli/runner.rs` — add progress hint before `ensure_daemon_running` call
- `src/cli/output.rs` — add a `progress_hint` method to `OutputFormatter`

**Changes**:

Add a method to `OutputFormatter`:

```rust
/// Emit a progress hint on stderr when in text mode.
/// Suppressed in JSON mode and when quiet is set.
pub fn progress_hint(&self, message: &str) {
    if self.mode == OutputMode::Text && !self.quiet {
        eprintln!("{message}");
    }
}
```

In `run_tool_timed` in `runner.rs`, after resolving the workspace and before
the `ensure_daemon_running` call, check whether the daemon is already running.
If not (health check fails), emit the progress hint:

```rust
// Quick health probe before spawning.
let endpoint = ipc_endpoint(&workspace_path)
    .map_err(|e| /* ... */)?;
if !check_health(&endpoint).await {
    formatter.progress_hint("Starting engram daemon...");
}

if let Err(e) = ensure_daemon_running(&workspace_path).await {
    return formatter.cli_error(&format!("daemon unavailable: {e}"));
}
```

**Tests**: Unit test for `progress_hint` — verify it emits nothing in JSON
mode and nothing when quiet. Integration test: run a CLI command in a fresh
workspace (no daemon) and verify stderr contains "Starting engram daemon".

**Execution posture**: Test-first for `progress_hint` method. The integration
test verifies the full flow.

## Dependency Graph

```
Unit 1 (--direct guard)     — independent
Unit 2 (IndexInProgress)    — independent
Unit 3 (startup progress)   — independent
```

No inter-unit dependencies. All three can be executed in any order or in
parallel.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Probe `engram.db.lock` with `try_write` rather than attempting DB open | Avoids the CozoDB unwrap panic entirely; fails fast before reaching upstream code |
| Drop the probe lock immediately after checking | `connect_db` re-acquires properly with its own retry loop and 30s timeout |
| Use message substring matching for IndexInProgress fallback | The error message contains "index" and "progress" in all current code paths; matching on both words avoids false positives while remaining resilient to wording changes |
| Emit progress on stderr, not stdout | stdout is reserved for JSON-RPC output; stderr is the correct channel for human-readable diagnostics |
| Suppress progress in JSON/quiet mode | Machine consumers should not see non-JSON output; quiet mode suppresses all optional output |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| `engram.db.lock` might not exist yet (fresh workspace) | Guard the probe with `if db_lock_path.exists()` — no lock file means no competing process |
| Fallback IndexInProgress detection false positive | Requires BOTH "index" AND "progress" in the message; standalone `-32603` errors with different messages are unaffected |
| Progress message timing: daemon may already be running | Pre-check health before emitting; message only appears when a fresh spawn is needed |
| `check_health` in Unit 3 adds one extra IPC roundtrip | The health probe has a 500ms timeout and only runs once per command invocation; negligible cost vs the 30s potential wait it explains |

## Plan Hardening Signals

| Signal | Present? | Justification |
|---|---|---|
| Public API, schema, or contract change | ❌ No | No MCP tool signatures or wire formats change |
| Security, auth, permission, or compliance-sensitive behavior | ❌ No | Error handling and UX only |
| Migration, backfill, destructive data/config action | ❌ No | No data changes |
| External integration, operator checkpoint, or external dependency | ❌ No | All changes are internal to the CLI |
| High runtime, rollout, or rollback risk | ❌ No | Additive error guards; existing happy paths unchanged |

**Requires plan hardening: no**

## Runtime Verification and Closure

### Unit 1 (--direct guard)

- **Runtime surface**: CLI `--direct` mode error path
- **Verification**: Run `engram sync --direct` while a daemon is running; verify
  exit code 2 and stderr message. Run without daemon; verify normal operation.
- **Closure**: No monitoring needed — one-shot CLI command, not a service.

### Unit 2 (IndexInProgress fallback)

- **Runtime surface**: CLI IPC error response handling
- **Verification**: Trigger IndexInProgress condition and verify friendly message
  appears regardless of whether `engram_code` is in the wire response.
- **Closure**: No monitoring needed — error formatting is stateless.

### Unit 3 (startup progress)

- **Runtime surface**: CLI stderr output on first use
- **Verification**: In a fresh workspace, run any CLI command and verify
  "Starting engram daemon..." appears on stderr. Run again (daemon running)
  and verify no message.
- **Closure**: No monitoring needed — cosmetic UX output.

## Constitution Check

| Principle | Status |
|---|---|
| I. Safety-First Rust | ✅ No unsafe code; all errors via `Result` |
| II. Test-First | ✅ Each unit specifies test-first posture |
| III. Workspace Isolation | ✅ No path traversal concerns |
| IV. CLI Containment | ✅ All changes within project |
| V. Structured Observability | ✅ Error paths produce structured output |
| VI. Single Responsibility | ✅ No new dependencies |
| VII. Destructive Approval | N/A — no destructive operations |
| VIII. Safety Modes | N/A — no elevated risk |
| IX. Git-Friendly | ✅ All changes in tracked source files |
| X. Context Efficiency | ✅ No data access pattern changes |
| XI. Merge Commit | ✅ Will use merge commit |

## Plan Review

**Reviewed**: 2026-05-09
**Gate decision**: **PASS** (0 P0, 0 P1, 1 P2, 1 P3)
**Plan hardening required**: No — no hardening signals present.

### Persona: Constitution Reviewer

All three implementation units comply with constitutional principles.
Test-first posture is specified for each unit. Error handling uses
`Result<T, EngramError>` throughout. No unsafe code. No new dependencies
beyond `fd_lock` which is already in the dependency tree.

No findings.

### Persona: Rust Reviewer

**P2-01 (Moderate)**: Unit 1's `try_write` probe acquires and immediately
drops an exclusive write lock. On Windows, `fd-lock` uses `LockFileEx`
which creates a mandatory byte-range lock. The drop releases it, but there
is a brief window between the probe drop and `connect_db`'s own lock
acquisition where another process could claim the lock. This window is
vanishingly small (microseconds) and only affects a race between two
`--direct` processes, not the daemon case (daemon holds the lock
continuously). Acceptable as-is — the probe is a best-effort early exit,
not a guarantee. The `connect_db` fd-lock provides the authoritative
serialization.

**Recommendation**: Document the TOCTOU window in a code comment so future
maintainers understand the probe is advisory.

### Persona: Scope Boundary Auditor

All three units target distinct code locations with no overlap. Unit 1
modifies `direct.rs` only. Unit 2 modifies `runner.rs` only. Unit 3
modifies `runner.rs` and `output.rs` — no overlap with Unit 2's change
site (Unit 2 touches the error response handler; Unit 3 touches the
pre-spawn section).

**P3-01 (Minor)**: Unit 3 proposes calling `ipc_endpoint` before
`ensure_daemon_running`, which is the reverse of the current order.
`ipc_endpoint` is a pure computation (no I/O), so reordering is safe,
but the final implementation should keep the code readable and document
why the endpoint is computed early.

No scope creep detected.

### Persona: Learnings Researcher

Searched `docs/compound/` for relevant prior solutions:

1. **`cozodb-sqlite-lock-panic-2026-05-01.md`**: Directly relevant. Documents
   the `fd-lock` mitigation in `connect_db` and the CozoDB upstream `unwrap()`
   defect (U015-FLK1). The plan's Unit 1 approach is consistent with this
   learning — it adds an early exit *before* reaching the CozoDB code path.
   No conflict.

2. **`daemon-startup-hang-watcher-blocks-before-ipc-bind-2026-05-02.md`**:
   Relevant context for Unit 3. Documents that daemon startup can be slow
   due to file watcher registration. The plan's progress indicator addresses
   the user-facing symptom of this known latency. No conflict.

3. **`sqlite-busy-retry-granularity-2026-05-03.md`** and
   **`sqlite-busy-retry-metrics-observability-2026-05-04.md`**: Background
   context on SQLITE_BUSY retry infrastructure. Not directly relevant to
   Unit 1's approach (which avoids the DB entirely) but confirms the
   project's strategy of mitigating upstream CozoDB defects locally.

No contradictions with existing learnings. No ignored prior solutions.

### Gate Summary

| Severity | Count | Findings |
|---|---|---|
| P0 | 0 | — |
| P1 | 0 | — |
| P2 | 1 | P2-01: TOCTOU window in fd-lock probe (advisory comment recommended) |
| P3 | 1 | P3-01: Code readability note on `ipc_endpoint` reordering |

**Decision**: PASS — no blocking findings. P2-01 is advisory; the TOCTOU
window is inherent to any probe-then-act pattern and is mitigated by the
authoritative lock in `connect_db`. Proceed to harvest.
