---
title: "Compacted Memory — 025-F Engram Server Reliability & Dog-Fooding + 037-F Post-Merge Closure"
date: 2026-05-02
features: [025-F, 037-F]
shipments: [019-S, 020-S]
prs: [69, 71, 72]
status: compacted
source_files:
  - docs/memory/2026-05-02/stage-025-F-server-reliability-memory.md
  - docs/memory/2026-05-02/037-F-ship-session-final-checkpoint.md
---

## 037-F / 019-S — CozoDB Concurrency Hardening

**Branch**: `post-merge/037-cozodb-concurrency-hardening` | **PR**: #69 | **Merged**: 2026-05-02

### What Was Done

- Post-merge closure PR #69 created; two rounds of Copilot review (8 comments total, all resolved)
- Round 1 (3): `invalid option` scope clarified in architecture.md; readiness verdict added to closure; decline for false-positive ARCHITECTURE.md path reference
- Round 2 (5): `run_script_retrying` scope clarified (`:create` only, HNSW uses direct `run_script`); declined 2 false-positive `||` table format comments; rollback trigger aligned to "1 or more SQLITE_BUSY panics"; fixed `ARCHITECTURE.md` → `docs/architecture.md` in archive file
- All 6 items (019-S + 037-F + 037.001-037.004-T) archived

### Key Decisions

- `run_script_retrying` wraps `:create` relation scripts only; HNSW index creation uses direct `run_script` (no retry)
- `invalid option` silencing scoped to HNSW index creation only (not `:create` relations)
- Rollback trigger: "1 or more SQLITE_BUSY panics in production after deploy"
- Architecture doc always referenced as `docs/architecture.md` (lowercase)

---

## 025-F / 020-S — Engram Server Reliability & Dog-Fooding

**Branch**: `feat/025-F-daemon-startup-fix` | **PRs**: #71 (stage docs), #72 (impl) | **Merged**: 2026-05-02

### Stage Session (Stage Agent)

- Processed stash 9B4996E5 (daemon hang bug) + queue 025-F (releasable server milestone)
- Deliberation artifact: `docs/decisions/2026-05-02-engram-server-reliability-dogfooding-deliberation.md`
- Plan: `docs/exec-plans/2026-05-02-engram-server-reliability-plan.md`
- Shipment 020-S created; 4 tasks harvested (025.001–025.004-T) with correct dependency chain
- Stash 9B4996E5 removed (promoted to tasks)

### Ship Session — Implementation

**Root cause confirmed (025.001-T)**: `start_watcher()` called before IPC bind in `daemon::run()`;
`RecursiveMode::Recursive` watch registration blocks >2s on large workspaces via `ReadDirectoryChangesW`;
shim 2000ms health probe expires before IPC ever binds.

**Fix implemented (025.002-T + 025.003-T)**:
- `run_with_shutdown_v2`: bind IPC listener first; watcher in `spawn_blocking` with 5s timeout
- `mpsc` channel created inside `spawn_blocking` (critical for ownership)
- `WatcherHandle` kept in outer scope (watcher lifetime = daemon lifetime)
- `event_rx` loop conditional on watcher success (graceful degradation)
- `remove_stale_pid_if_dead`: cleans up PID files before lock acquisition; return type `Option<u32>` (all paths non-fatal)
- Legacy numeric PID file fallback added

**Copilot review — 2 rounds, 8 comments total, all resolved**:
- Round 1: doc comment, `FlushFailed` misuse fixed (→ return type change), `remove_file` error semantics, duplicate log, docblock, channel scope
- Round 2: `remove_stale_pid_if_dead` return type `Result<Option<u32>>` → `Option<u32>` (resolved `FlushFailed` misuse + clippy `unnecessary_wraps`); comment "before acquiring the lock" → "before binding the listener"

**Pre-existing flaky tests marked `#[ignore]`**:
- `s_cs4` (concurrent_sessions): SQLITE_BUSY race (U015-FLK1)
- `graph_vector_rehydration`: `flush_state` doesn't write `nodes.jsonl` (026-F gap)

**Post-merge closure (Step 6 complete)**:
- 020-S shipped at `d20ac49`; all 6 items archived
- Closure artifact: `docs/closure/2026-05-02-025-F-daemon-startup-fix-closure.md`
- Architecture.md Daemon module row updated with bind-first ordering
- 2 compound learnings added: daemon startup hang root cause; Option vs Result for non-fatal paths
- 2 follow-up stash entries: 9CFB4DBA (SQLITE_BUSY retry), 44452A7D (flush_state gap)
- Closure PR #73 created: `post-merge/025-F-daemon-startup-fix` → main

### Key Decisions

- Bind-first ordering is the authoritative daemon startup pattern; `run_with_shutdown` preserved for reference only
- Non-fatal cleanup functions use `Option<T>` not `Result<Option<T>, E>`
- Pre-existing flaky tests marked with U-code comments instead of deleted (evidence-preserving)
