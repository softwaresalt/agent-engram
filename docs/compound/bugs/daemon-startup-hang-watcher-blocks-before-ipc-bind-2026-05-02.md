---
title: "Daemon Startup Hang: File Watcher Must Not Block Before IPC Bind"
description: "On large workspaces, calling start_watcher before binding the IPC listener causes ReadDirectoryChangesW/inotify_add_watch to block >2s, exceeding the shim health probe timeout; fix is to bind the IPC listener first, then start the watcher in spawn_blocking with a timeout"
problem_type: "design_flaw"
category: "bugs"
component: "daemon/ipc_server"
root_cause: "start_watcher calls new_debouncer + debouncer.watch(RecursiveMode::Recursive), which blocks on Windows via ReadDirectoryChangesW and on Linux via inotify_add_watch; on large workspaces this takes >2s, exceeding the shim 2000ms health probe deadline before the IPC listener ever binds"
resolution_type: "code_fix"
severity: "high"
message: "Daemon failed to reach Ready state within 2000ms"
file_path: "src/daemon/ipc_server.rs"
citations:
  - "src/daemon/ipc_server.rs — run_with_shutdown_v2"
  - "src/daemon/mod.rs — daemon::run()"
  - "tests/integration/daemon_startup_order_test.rs"
  - "docs/closure/2026-05-02-025-F-daemon-startup-fix-closure.md"
  - ".backlogit/archive/025-F.md"
tags:
  - "daemon"
  - "startup"
  - "ipc"
  - "watcher"
  - "windows"
  - "ReadDirectoryChangesW"
  - "inotify"
  - "shim"
  - "health-probe"
  - "025-F"
---

## Problem

The engram daemon fails to reach Ready state when the shim spawns it on a large workspace.
The shim sends a health probe 2000ms after spawning — if the daemon does not respond within
that window, the shim reports "Daemon failed to reach Ready state" and exits.

On a workspace with many directories (e.g., the `agent-engram` repo itself), the old startup
sequence called `start_watcher` **before** `run_with_shutdown` ever bound the IPC listener.
`start_watcher` calls `notify::new_debouncer` then `debouncer.watch(path, RecursiveMode::Recursive)`.
On Windows this triggers `ReadDirectoryChangesW` registration; on Linux it triggers `inotify_add_watch`.
Both are synchronous and can take >2s on large workspaces, exhausting the probe window.

## Root Cause

```text
OLD ordering (daemon::run):
  1. start_watcher()          ← BLOCKS 2–5s on large workspaces
  2. ipc_server::run_with_shutdown()   ← IPC never bound during shim probe window
```

The shim's 2000ms timeout expired before step 2 could complete.

## Resolution (025-F)

Introduced `run_with_shutdown_v2` with bind-first ordering:

```text
NEW ordering (run_with_shutdown_v2):
  1. bind IPC listener         ← completes in <50ms
  2. daemon is now reachable
  3. spawn_blocking(start_watcher, timeout=5s)   ← non-blocking from daemon perspective
  4. event loop (conditional on watcher success)
```

Key implementation details:
- `mpsc` channel (`event_tx`/`event_rx`) created INSIDE `spawn_blocking`, not outside — the
  sender end must live in the same task that returns the watcher handle
- `WatcherHandle` kept in the OUTER `run_with_shutdown_v2` scope so watcher lifetime equals
  daemon lifetime (not task lifetime)
- `event_rx` loop only spawned when `watcher_result` is `Some` — graceful degradation if
  watcher init fails or times out

## Prevention

- **Always bind the IPC listener before any blocking I/O** in the daemon startup sequence.
- The `run_with_shutdown_v2` entry point is now the authoritative startup path.
  `run_with_shutdown` is kept for reference. Do not restore the old call order.
- Integration test `run_with_shutdown_v2_exits_cleanly_on_ttl_expiry` verifies bind-first
  ordering holds on each CI run.
- If adding new startup initialization steps, insert them AFTER the `listener.accept()` loop
  begins (or make them async/non-blocking) so they cannot delay the health probe response.
