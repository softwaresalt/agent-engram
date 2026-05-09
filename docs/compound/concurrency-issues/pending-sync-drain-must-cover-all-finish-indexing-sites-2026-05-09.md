---
title: "All finish_indexing() call sites must drain pending_sync"
description: "A queued-sync design that drains in only one finish_indexing path silently drops syncs triggered from other indexing paths"
problem_type: "missing drain coverage"
category: "concurrency-issues"
component: "src/tools/ and src/daemon/ipc_server.rs"
root_cause: "finish_indexing() is called from multiple code paths (background hydration, index_workspace, sync_workspace, startup auto-sync, file-watcher auto-sync). Only the background_db_hydration path had the drain; the others silently dropped queued syncs."
resolution_type: "code_fix"
severity: "high"
message: "sync_workspace returns status:queued but the queued sync is never run unless background_db_hydration was the active indexer"
file_path: "src/tools/lifecycle.rs"
citations:
  - "PR #101 — pre-merge review P1-02, P1-03, P1-04, P2-01"
  - "src/tools/lifecycle.rs — drain_pending_sync()"
  - "src/tools/write.rs — index_workspace, sync_workspace"
  - "src/daemon/ipc_server.rs — startup hydration, file-watcher auto-sync"
tags:
  - "atomicbool"
  - "drain"
  - "pending-sync"
  - "finish-indexing"
  - "sqlite-busy"
  - "queued-sync"
---

## Problem

When implementing a "queued sync" pattern (where `sync_workspace` defers itself
by setting an `AtomicBool` flag when another indexer holds the lock), the drain
logic was only implemented in `background_db_hydration`. Three other code paths
that call `finish_indexing()` were missing the drain:

- `index_workspace` in `write.rs` (explicit user-triggered full index)
- `sync_workspace` in `write.rs` (concurrent sync after one finishes)
- Startup auto-embedding backfill in `ipc_server.rs`
- File-change auto-sync loop in `ipc_server.rs`

The symptom: a user calls `sync_workspace`, gets `{"status":"queued"}` back, but
the sync never actually runs because `index_workspace` (or the file-watcher)
called `finish_indexing()` without checking `take_pending_sync()`.

## Root Cause

`finish_indexing()` is a primitive that only clears the `indexing_in_progress`
flag and records the completion timestamp. The drain responsibility was not
co-located with `finish_indexing()` — it was embedded in one specific caller.

This breaks the **"contract at the call site"** principle: every caller of
`finish_indexing()` must be aware of and fulfill the drain obligation.

## Resolution

1. **Extract `drain_pending_sync(state: &AppState)`** as a standalone `pub async fn`
   in `lifecycle.rs` (where `sync_code_graph` is already imported). This function:
   - Checks `take_pending_sync()` as a fast-path no-op
   - Acquires the indexing lock via `try_start_indexing()`
   - Runs `sync_code_graph()` and calls `finish_indexing()` on completion

2. **Call `drain_pending_sync()` after every `finish_indexing()` site**:

   ```rust
   // write.rs — index_workspace
   state.finish_indexing().await;
   drain_pending_sync(&state).await;

   // write.rs — sync_workspace
   state.finish_indexing().await;
   drain_pending_sync(&state).await;

   // ipc_server.rs — startup hydration
   state_auto.finish_indexing().await;
   crate::tools::lifecycle::drain_pending_sync(&state_auto).await;

   // ipc_server.rs — file-watcher auto-sync
   state_watcher.finish_indexing().await;
   crate::tools::lifecycle::drain_pending_sync(&state_watcher).await;
   ```

## Prevention

- When implementing an `AtomicBool` "pending work" flag, trace ALL callers
  of the "release" function (here: `finish_indexing()`) and ensure each one
  drains the flag.
- Prefer co-locating the drain inside a `finish_and_drain()` helper so
  the obligation cannot be forgotten at a future call site.
- Write a contract test that queues `pending_sync` and then calls
  `finish_indexing()` from the specific path you're testing — not just the
  happy-path background_db_hydration path.
