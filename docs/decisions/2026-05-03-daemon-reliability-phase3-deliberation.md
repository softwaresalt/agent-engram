---
title: "Daemon Reliability Phase 3 — Subprocess Stability & Retry Observability"
description: "Deliberation on resolving the CozoDB subprocess spawn panic on Windows and adding retry observability to the SQLITE_BUSY retry helper"
topic: "CozoDB subprocess open panic (Windows) and retry tracing"
depth: "standard"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/closure/2026-05-03-038-F-daemon-reliability-phase2-closure.md"
  - "docs/compound/concurrency-issues/cozodb-sqlite-lock-panic-2026-05-01.md"
  - "docs/compound/data-plane/sqlite-busy-retry-granularity-2026-05-03.md"
tags:
  - "cozo"
  - "sqlite"
  - "daemon"
  - "subprocess"
  - "windows"
  - "observability"
  - "tracing"
---

## Problem Frame

After 038-F (Daemon Reliability Phase 2), two follow-up items remain:

1. **Subprocess spawn timeout** (stash `100EACD8`, medium priority): Integration
   tests `smoke_full_tool_chain_over_ipc` and
   `daemon_rehydrates_graph_and_vector_state_after_db_directory_is_deleted` fail
   on Windows because CozoDB 0.7.6 panics at `sqlite.rs:49`
   (`conn.prepare().unwrap()`) when opened in a daemon subprocess context. The
   daemon never reports Ready. This is a pre-existing failure on `main` — not a
   regression from 038-F.

2. **Retry observability** (stash `1BA885AF`, low priority): The
   `run_script_busy_retry_mutable` helper in `src/db/cozo_queries.rs` retries
   silently. Each retry should emit a `tracing::warn` span with attempt count
   and delay for observability.

**Additionally:** Queue item `002-D` (fd-lock scope extension deliberation)
is already resolved — 037-F implemented the chosen direction (extend fd-lock
to cover schema bootstrap). That item should be archived, not re-staged.

**Success criteria:**

- Integration tests that spawn daemon subprocesses either pass or are
  gracefully skipped with a documented root-cause annotation (not silently
  failing)
- `run_script_busy_retry_mutable` emits tracing::warn on each retry attempt
- `cargo test` passes (excluding pre-existing unrelated failures)
- `cargo clippy -- -D warnings -D clippy::pedantic` exits 0

## Research Findings

### Subprocess spawn failure (100EACD8)

- Root cause: CozoDB 0.7.6 `storage/sqlite.rs:49` calls `unwrap()` on
  `conn.prepare()` during database open. In subprocess contexts on Windows,
  this prepare call fails (possibly due to inherited file handles, working
  directory issues, or SQLite DLL initialization in a fresh process).
- The fd-lock workaround (037-F) only protects against concurrent opens — it
  does not prevent the first open from panicking in subprocess context.
- Cozo 0.8+ does not exist on crates.io (latest: 0.7.6).
- WAL mode (`PRAGMA journal_mode=WAL`) helps concurrent access but does not
  prevent the initial `prepare()` failure — WAL is set after open succeeds.
- The daemon startup sequence (025-F) binds IPC first and starts watcher
  after — this is correct, but the daemon never reaches IPC bind because
  `connect_db` panics during CozoDB open.

### Retry observability (1BA885AF)

- `run_script_busy_retry_mutable` is at `src/db/cozo_queries.rs:300-327`.
- Currently retries are completely silent — no logging, no metrics.
- The fix is mechanical: add `tracing::warn!` inside the retry branch.
- Compound learning `sqlite-busy-retry-granularity-2026-05-03.md` documents
  the per-statement retry design rationale.

## Options Evaluated

### Option A: Investigate & Mitigate Subprocess Panic

Apply a pre-open mitigation in `connect_db` for the subprocess case:

1. Set `PRAGMA journal_mode=WAL` via environment or connection option (if cozo
   supports it)
2. Add a retry loop around `DbInstance::new` with exponential back-off
3. If neither works, catch the panic with `std::panic::catch_unwind` and convert
   to an `EngramError`

**Pros:** Addresses the root cause as much as possible without an upstream fix.
**Cons:** `catch_unwind` requires `UnwindSafe` (may conflict with `forbid(unsafe_code)`);
cozo may not support WAL mode configuration pre-open.
**Effort:** Medium

### Option B: Graceful Skip with Root-Cause Documentation

Accept the upstream cozo 0.7.6 limitation. In the affected integration tests,
detect the subprocess environment (Windows + cozo 0.7.6) and skip gracefully
with `#[ignore]` or a runtime skip annotation that documents the root cause.
Remove `continue-on-error: true` from CI for all other tests.

**Pros:** Pragmatic, immediate, no risky workarounds. CI becomes reliable for
all non-affected tests. Root cause is documented for future resolution when
cozo ships a fix.
**Cons:** Two tests remain disabled. Does not fix the underlying issue.
**Effort:** Low

### Option C: Subprocess Isolation via Fresh DB Path

Ensure each subprocess test uses a completely unique temporary DB path with no
possibility of path collision or inherited file handles. If the panic is
triggered by stale handles from the parent process, isolating paths may avoid it.

**Pros:** May sidestep the panic if path collision is the trigger.
**Cons:** Speculative — we don't know if path collision causes this. Tests
may already use temp paths. Does not help if the issue is OS-level subprocess
initialization.
**Effort:** Low-Medium

## Trade-off Comparison

| Criterion | A: Mitigate | B: Graceful Skip | C: Path Isolation |
|---|---|---|---|
| Fixes subprocess tests | Maybe | No (skips them) | Maybe |
| Risk | Medium (catch_unwind, unsafe boundary) | Very low | Low |
| Effort | Medium | Low | Low-Medium |
| Actionable now | Yes | Yes | Yes |
| Long-term clean | Partial | No (waits for upstream) | Partial |

## Chosen Direction

**Option B (Graceful Skip)**: Path isolation (Option C) is already in place —
`DaemonHarness::spawn` uses `TempDir::new()` for every invocation. The panic
still occurs. The upstream cozo 0.7.6 `conn.prepare().unwrap()` panic cannot be
fixed without `catch_unwind` (violates `#![forbid(unsafe_code)]`) or an upstream
release. Apply Option B: annotate affected tests with conditional skip and
root-cause documentation, add retry observability, and evaluate removing CI
`continue-on-error` now that the other pre-existing failures (038.002-T,
038.003-T) are fixed.

**Rationale:** The only pragmatic path given `forbid(unsafe_code)` and no
upstream fix available. Documenting the root cause and making CI a proper gate
for non-affected tests delivers immediate value.

**Tasks:**

1. Investigate subprocess DB path isolation — attempt fresh temp path per
   subprocess invocation and verify whether panic persists
2. If panic persists: annotate affected tests with root-cause documentation
   and structured skip conditions
3. Add `tracing::warn` to `run_script_busy_retry_mutable` retry attempts

## Open Questions

- Does the subprocess panic occur on Linux/macOS or only Windows?
- Is the panic triggered by inherited file descriptors from the parent process?
- When will cozo 0.8+ ship? (Same question as 002-D — monitor upstream)

## Notes

- Stash `1092D3D6` (upgrade cozo 0.8+) remains deferred — upstream blocked.
- Queue item `002-D` should be archived — its deliberation target (fd-lock
  scope extension) was shipped in 037-F.
- This is Phase 3 of the daemon reliability series (following 037-F Phase 1,
  038-F Phase 2).
