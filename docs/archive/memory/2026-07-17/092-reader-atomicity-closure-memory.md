---
title: "092.002-T reader-side workspace+config atomicity - session memory"
type: session-memory
date: 2026-07-17
task: 092.002-T
feature: 092-F
pr: 263
merge_commit: 4436d534b7fd625ebea383d2e63a9473470ad829
status: done
---

## What shipped

092.002-T - reader-side workspace+config atomicity. New None-gating atomic reader
`AppState::snapshot_workspace_and_config(&self) -> Option<(WorkspaceSnapshot, WorkspaceConfig)>`
in `src/server/state.rs`; migrated `background_db_hydration` and `drain_pending_sync`
in `src/tools/lifecycle.rs` off separate-await reads. Closes the reader-side
(workspace_i, config_j) tear in the two background paths (086-S Gemini C1/C2, P2).

## Files modified

* `src/server/state.rs` - added `snapshot_workspace_and_config`; updated the module-level
  lock-order doc comment to list all three paired-lock sites.
* `src/tools/lifecycle.rs` - migrated both background readers to the new atomic reader.
* `tests/integration/get_workspace_status_atomicity_test.rs` - added a non-vacuous
  reader-side torn-pair test (red harness models the old two-await reader with a 50 us
  sleep; 524 torn pairs red, 0 green) and a None-gating (config-absent) test.

## Key decisions

* **None-gating, not config-defaulting.** The reader returns `None` if either value is
  absent, preserving the background paths' skip-if-either-none behavior. It is deliberately
  NOT `snapshot_dispatch_context` (which defaults config via `unwrap_or_default()`); a naive
  swap would have changed skip -> run-with-default-config.
* **Lock order.** `active_workspace` then `workspace_config`, matching the writer
  (`set_workspace_and_config`) and dispatch reader (`snapshot_dispatch_context`). Three
  paired-lock sites, all same order, no reverse-order site -> deadlock-free by lock order.
  tokio guards are `Send` (holding first read guard across the second lock's await keeps the
  future Send) - a distinct property from deadlock-freedom. No I/O `.await` under guards;
  both guards drop before the reader returns.

## Review

* Pre-Copilot adversarial: GPT-5.6 Sol @ xhigh (key rust/correctness) + Gemini-3.1 @ high
  (concurrency lens). No P0/P1. One out-of-scope P2 deferred -> 092.003-T.
* Copilot: `COMMENTED` at HEAD 53a68ae, no inline findings, 0 threads. Clean first review,
  0 review-fix cycles (adversarial-before-Copilot met the operator's goal).
* Gates: fmt + clippy pedantic clean; atomicity 5/5; lifecycle 4, multi_workspace 6,
  daemon_lifecycle 7.

## Deferred follow-up (created at closure)

* **092.003-T** (queued, parent 092-F) - daemon `ipc_server.rs` (`run_with_shutdown` /
  `run_with_shutdown_v2`) auto-sync/background closures still read the pair via separate
  awaits: same tear window, third code path. Mechanical migration to
  `snapshot_workspace_and_config()`.

## Next steps

* Drive the closure PR through the 4-point Copilot merge gate; merge.
* Continue the safe queue: 091.015-T (ID-preserving canonical_path backfill; heightened
  UUID-preservation scrutiny) -> 091.020-T -> 091.016-T -> 091.019-T -> 091.017-T ->
  090.005-T. Keep deferred: 087.005-T / 087.006-T (PowerBI durability), 025-S/041-F cluster
  (CozoDB major upgrade), operator branch cluster (081-S/088-F), blocked/upstream items.
