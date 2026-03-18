<!-- markdownlint-disable-file -->
# PR Review Status: 006-workspace-content-intelligence

## Review Status

* Phase: 4 — Finalize Handoff
* Last Updated: 2026-03-18T14:13:53Z
* Summary: Three bug-fix commits targeting daemon startup reliability (lockfile PID 0, IPC ready timeout, silent stale cleanup)

## Branch and Metadata

* Normalized Branch: `006-workspace-content-intelligence`
* Source Branch: `006-workspace-content-intelligence`
* Base Branch: `main` (merge-base: `2214319`)
* Linked Work Items: None

## Scope

This review covers **only the 3 bug-fix commits** (a241633, fb064ca, 2ae8456), not the full feature branch.

## Diff Mapping

| File | Type | New Lines | Old Lines | Notes |
|------|------|-----------|-----------|-------|
| `src/daemon/lockfile.rs` | Modified | 1-287 | 1-224 | Split lock/PID files; stale cleanup logging |
| `src/daemon/ipc_server.rs` | Modified | 273-441 | 273-410 | Bind-first, background hydration, health status |
| `src/shim/lifecycle.rs` | Modified | 28, 191-196 | 28, 188-196 | Timeout 10s→30s |
| `tests/unit/lockfile_test.rs` | Modified | 21-68 | 21-60 | Assert both files; readability check |

## Instruction Files Reviewed

* `.github/instructions/*.instructions.md`: Rust conventions, MCP server best practices
* Constitution: Safety-first Rust, test-first development, workspace isolation

## Review Items

### ✅ Approved — No Action Needed

#### RI-01: Lock/PID file separation (a241633)

* Category: Bug Fix / Cross-Platform
* Severity: Critical (was causing PID 0 errors on Windows)
* Verdict: Correctly implemented. `engram.lock` (fd-lock target) and `engram.pid` (plain text) separation resolves Windows `ERROR_LOCK_VIOLATION`. `std::fs::write` avoids the old truncate-seek-write dance. Documentation is thorough.

#### RI-02: IPC bind-before-hydrate (fb064ca)

* Category: Bug Fix / Startup Sequence
* Severity: Critical (was causing NotReady timeout)
* Verdict: Correctly moves `bind_listener` before `set_workspace`. Background `tokio::spawn` unblocks accept loop. `_health` returns `"starting"` / `"ready"` based on workspace snapshot presence. TTL placement after hydration is correct.

#### RI-03: Ready timeout increase (fb064ca)

* Category: Configuration
* Severity: Low
* Verdict: 10s→30s is reasonable. Test name and assertion updated consistently.

#### RI-04: Stale cleanup error logging (2ae8456)

* Category: Observability
* Severity: Low
* Verdict: `NotFound` errors are correctly suppressed; other errors get `warn!` with path and error context.

### ❌ Deferred to Follow-up

#### RI-05: Box::leak on error path leaks FD and memory

* File: `src/daemon/lockfile.rs`
* Lines: 124 through 126
* Category: Resource Management
* Severity: Low
* Decision: **Deferred** — daemon calls `acquire()` once at startup; leak is 1-2 FDs max. Non-blocking.

#### RI-06: No integration test for "starting" → "ready" transition

* File: `tests/` (all test files)
* Category: Testing Gap
* Severity: Medium
* Decision: **Deferred** — would require workspace with slow hydration or delay injection. Non-blocking.

#### RI-07: Tool calls during "starting" phase return generic WorkspaceError::NotSet

* File: `src/tools/read.rs`
* Lines: 66 through 70
* Category: User Experience
* Severity: Low
* Decision: **Deferred** — mitigated by shim gating on `"ready"`. Will be addressed as part of phased startup backlog item.

#### RI-08: Hydration failure sends shutdown but does not flush error state

* File: `src/daemon/ipc_server.rs`
* Lines: 431 through 434
* Category: Reliability / Diagnostics
* Severity: Low
* Decision: **Deferred** — error is logged to stderr. Diagnostic file (`.engram/run/last-error.txt`) is a nice-to-have for follow-up.

## Next Steps

* [x] All review items resolved — 4 approved, 4 deferred to follow-up
* [ ] Merge PR when ready
