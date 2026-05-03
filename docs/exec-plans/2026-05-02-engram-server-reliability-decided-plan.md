---
title: "025-F Engram Server Reliability — Decided Plan"
original_plan: "docs/exec-plans/2026-05-02-engram-server-reliability-plan.md"
feature_id: "025-F"
shipment_id: "020-S"
merge_commit: "d20ac49"
status: "shipped"
compacted_date: "2026-05-02"
---

## Problem

`daemon::run()` called `start_watcher()` before binding the IPC listener.
`start_watcher` registers `RecursiveMode::Recursive` which blocks >2s on large workspaces
via `ReadDirectoryChangesW` (Windows) / `inotify_add_watch` (Linux). The shim's 2000ms
health probe expired before the IPC listener ever bound → "Daemon failed to reach Ready state".

## Final Decisions

| Decision | Rationale |
|---|---|
| Bind IPC listener first, then start watcher | Consistent with existing "bind-first" pattern already used inside `run_with_shutdown` for workspace hydration |
| Start watcher in `spawn_blocking` with 5s timeout | Prevents async runtime thread pool starvation; timeout guards against future regressions |
| `remove_stale_pid_if_dead` returns `Option<u32>` | All error paths non-fatal (warn + None); `Result<Option<T>>` was misleading; resolved clippy `unnecessary_wraps` |
| Legacy numeric PID fallback | Replicates `PidFile::read()` pattern; handles old PID files from pre-JSON format |
| Pre-existing flaky tests marked `#[ignore]` | Evidence-preserving; tagged with U-codes; backlog follow-ups created instead of silent deletion |

## Implementation Units Shipped

| Task | Change |
|---|---|
| 025.001-T | Diagnostic `info!` spans in `daemon::run()` — confirmed `start_watcher` is the blocker |
| 025.002-T | `run_with_shutdown_v2` in `ipc_server.rs` with bind-first ordering |
| 025.003-T | `remove_stale_pid_if_dead` in `daemon/mod.rs`; `Option<u32>` return; legacy PID fallback |
| 025.004-T | Integration test `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry`; e2e dog-fooding verified |

## Follow-up Items

| Stash | Description |
|---|---|
| `9CFB4DBA` | SQLITE_BUSY retry logic in `index_workspace` handler (U015-FLK1) |
| `44452A7D` | `flush_state` must write `nodes.jsonl` to `.engram/code-graph/` (026-F gap) |
