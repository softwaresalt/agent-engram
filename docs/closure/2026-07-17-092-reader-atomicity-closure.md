---
title: "Operational Closure - 092.002-T reader-side workspace+config atomicity"
doc_type: closure
source: "092.002-T (feature 092-F, archived)"
description: >-
  Post-merge closure for task 092.002-T. Records the None-gating atomic reader
  snapshot_workspace_and_config that closes the reader-side (workspace_i, config_j)
  tear in the two background paths (background_db_hydration, drain_pending_sync),
  the lock-order proof across all three paired-lock sites, the pre-Copilot
  adversarial review (Sol xhigh key + Gemini concurrency lens) that produced a
  clean first Copilot review, the deferred daemon-reader follow-up, and the
  runtime rollback posture for the background read path.
topic: "Reader-side workspace+config atomicity on the background hydration/sync paths"
depth: "closure"
decision_status: "SHIPPED - merged to main as merge commit 4436d53 via PR #263"
author: ship
date: 2026-07-17
verdict: SHIPPED
pr: 263
merge_commit: 4436d534b7fd625ebea383d2e63a9473470ad829
target_commit: 4436d534b7fd625ebea383d2e63a9473470ad829
branch: feat/092-reader-atomicity
scope: "Give the two background readers a None-gating atomic snapshot so a concurrent bind is never observed as a mismatched (workspace_i, config_j) pair, without changing the skip-if-either-None semantics"
reviewers:
  - gpt-5.6-sol
  - gemini-3.1-pro-preview
  - copilot
linked_artifacts:
  - "092-F"
  - "092.001-T"
  - "092.002-T"
  - "092.003-T"
---

## Summary

engram binds a workspace by publishing two pieces of state: the active workspace
snapshot (`active_workspace`) and the workspace configuration (`workspace_config`).
The writer was made atomic in 086-S (`set_workspace_and_config` holds both write
guards), and the dispatch-entry reader was already atomic
(`snapshot_dispatch_context` holds both read guards). Two background paths, however,
still read the pair non-atomically: `background_db_hydration` and `drain_pending_sync`
in `src/tools/lifecycle.rs` each sampled `snapshot_workspace().await` and
`workspace_config().await` as two separate awaits. A concurrent atomic bind landing
between those two awaits could be observed as a mismatched (workspace_i, config_j)
pair. The 086-S Gemini concurrency review flagged this as findings C1/C2 (P2).

Task 092.002-T closes that reader-side tear with a single new reader:
`AppState::snapshot_workspace_and_config` acquires both read guards in the reader's
lock order, clones both values while both guards are held, and returns
`Some((workspace, config))` or `None`. Both background paths were migrated to it.
This change touches no schema, no on-disk format, and no command routing; it narrows
a concurrency window on the background read path.

## Tasks shipped

* `092.002-T` - None-gating atomic reader. Adds
  `AppState::snapshot_workspace_and_config(&self) -> Option<(WorkspaceSnapshot, WorkspaceConfig)>`
  in `src/server/state.rs`. It acquires `active_workspace.read()` then
  `workspace_config.read()` (the same order the writer `set_workspace_and_config` and
  the reader `snapshot_dispatch_context` use), clones both values while both guards are
  held, and returns `None` when either value is absent. `background_db_hydration` and
  `drain_pending_sync` in `src/tools/lifecycle.rs` were migrated from the two-await
  sequence to this single call, preserving their existing skip-if-either-None behavior,
  the `try_start_indexing` gate, and all surrounding tracing and sync logic.

## Key decisions

### Atomic snapshot under the reader's lock order

The reader holds both read guards for the whole snapshot. Correctness rests on the
same lock-order proof established in 086-S, now covering three paired-lock sites:
`snapshot_dispatch_context` (read/read), the new `snapshot_workspace_and_config`
(read/read), and `set_workspace_and_config` (write/write). All three acquire
`active_workspace` before `workspace_config`, and no site acquires them in the reverse
order, so the paired-lock exception cannot deadlock. That deadlock-freedom follows
solely from the consistent lock order. Separately, `tokio::sync::RwLock` guards are
`Send`, so holding the first read guard across the second lock's `.await` keeps the
resulting future `Send` and schedulable on the multithreaded runtime - a distinct
property from deadlock-freedom, not its cause. No I/O `.await` runs inside the reader:
both guards are dropped before it returns, so the callers perform their DB and sync I/O
only after the guards are released.

### None-gating, not config-defaulting

The reader returns `None` when either value is absent. This preserves the exact
skip-if-either-None semantics both background paths already had: they skip their pass
until both the workspace and its config are loaded. This is deliberately different from
`snapshot_dispatch_context`, which substitutes `WorkspaceConfig::default()` when config
is absent. A naive swap of the background paths to `snapshot_dispatch_context` would
have changed their behavior from "skip" to "run with a default config", so a dedicated
None-gating reader was required rather than reusing the dispatch reader.

### Non-vacuous reader-side atomicity test

The test drives a concurrent writer alternating A->B / B->A binds against the new
reader, with each workspace bound to a config whose `retrieval_eval.enabled` flag is
tied to the workspace identity (true for A, false for B) so a torn pair is detectable.
A red-phase harness models the old two-await reader with a 50 us sleep between the two
awaits to widen the tear window; against that harness the assertion observes 524 torn
pairs, proving non-vacuity. Against the atomic reader it observes zero torn pairs. A
vacuity guard asserts both A and B states were actually sampled. A second test asserts
the reader returns `None` when config is absent. Target
`integration_get_workspace_status_atomicity` runs 5/5.

## Review resolution

### Adversarial review (pre-Copilot, per operator key-reviewer directive)

* **Key reviewer - GPT-5.6 Sol @ xhigh (rust/correctness):** rated the reader
  deadlock-free, no torn read, correct None-gating (skip preserved, config never
  defaulted), and no I/O under guard. No P0/P1 findings.
* **Cross-model concurrency lens - Gemini-3.1-pro-preview @ high:** independently
  confirmed lock-order correctness and non-vacuity of the test. Raised 1 out-of-scope P2
  (see deferred follow-ups). No P0/P1 findings.

### Copilot - clean first review, 0 review-fix cycles

* `53a68ae` - Copilot review returned `COMMENTED` at HEAD with no inline findings and no
  review threads. The pre-Copilot adversarial review achieved the operator's goal of a
  clean Copilot pass with no fix cycle. 4-point merge gate CLEAN at `53a68ae`; merged.

## Deferred follow-ups

* `092.003-T` (queued) - Daemon-reader atomicity. The adversarial review found that the
  auto-sync / background closures in `src/daemon/ipc_server.rs` (`run_with_shutdown` and
  `run_with_shutdown_v2`) still read the (workspace, config) pair non-atomically via
  separate `snapshot_workspace()` and `workspace_config()` awaits - the same tear window,
  in a third code path. It was out of scope for 092.002-T, which was constrained to the
  two `lifecycle.rs` background paths plus `state.rs` and one integration test. The fix is
  mechanical: migrate those daemon readers to `snapshot_workspace_and_config()`. Tracked as
  a queued follow-up under 092-F.

## Release observability

Task 092.002-T changes the background hydration and coalesced-sync read path - a runtime
surface - so it carries a monitoring and rollback posture. It changes no DB schema, no
JSONL format, and no command routing or tool schema.

### Healthy signals

* `integration_get_workspace_status_atomicity` passes (5/5) in CI. Because the reader-side
  test drives a real A->B / B->A bind transition against the new reader and asserts zero
  torn pairs with a vacuity guard, a green run is direct evidence that the background reader
  snapshots atomically.
* Background hydration and coalesced sync act only on a consistent (workspace, config) pair:
  a bind in flight is either fully observed or skipped, never observed half-applied.

### Failure signals

* `integration_get_workspace_status_atomicity` fails - the reader or writer lock order
  changed, or the None-gating was replaced with config-defaulting. The failing assertion
  names the mismatched pair or the unexpected non-None result.
* A background hydration or sync pass acts on a new-workspace / old-config pair after a
  concurrent bind - the exact tear this fix closes. This would indicate a background path
  was reverted to separate-await reads.

### Monitoring method, baseline, threshold

* Method: the `integration_get_workspace_status_atomicity` integration test in CI, plus the
  affected lifecycle / multi-workspace / daemon-lifecycle suites.
* Baseline at merge (`4436d53`): atomicity 5/5; affected suites green
  (integration_workspace_lifecycle_workflow 4, integration_multi_workspace 6,
  integration_daemon_lifecycle 7).
* Threshold to investigate: any failure of the atomicity test, or any observed
  background pass acting on a new-workspace / old-config pair.

### Owner and observation window

* Owner: repository maintainer (operator); ship hands off the post-deploy check.
* Window: bounded to the first 3 workspace binds under concurrent background hydration or
  sync after the operator next builds and runs the updated binary, or 7 days from that
  first run, whichever comes first. Engram is a local single-binary daemon with no runtime
  telemetry or fleet rollout, so observation is a manual check, not a dashboard.
* Active check: after a bind that overlaps a background hydration or sync in the window,
  run `get_workspace_status` (or the CLI status equivalent) and confirm the reported
  workspace and config form a consistent pair. Silence is not treated as success - the
  check is performed, not assumed.
* Closeout: at window end, record the outcome (healthy / degraded / rolled back) in this
  record.
* Pre-release validation (already complete): local gates green (fmt, clippy pedantic);
  atomicity 5/5; affected suites green; adversarial review (Sol xhigh key + Gemini
  concurrency lens) clean with no P0/P1; clean first Copilot review with 0 review-fix
  cycles; 4-point merge gate CLEAN at `53a68ae`.
* Post-deploy outcome: PENDING the operator's next build/run of the updated binary (operator
  is currently AFK). To be recorded at window close per the active check above.

### Rollback trigger and procedure

* Trigger: the atomic reader is later found to deadlock, to skip a pass it should have run
  (a None-gating regression), or to otherwise destabilize background hydration or sync.
* Procedure: revert the merge with `git revert -m 1 4436d53`. This restores the two-await
  reads in `background_db_hydration` and `drain_pending_sync` and removes the reader-side
  test additions. Runtime blast radius is contained to the background read path: no schema,
  format, or routing change is reversed, so no data migration is involved. The revert
  reopens the reader-side tear window but does not corrupt state. Backlog state is
  unaffected by the code revert: the archival of 092.002-T and the creation of 092.003-T
  live in this separate closure PR, not in merge `4436d53`, so reverting `4436d53` leaves
  the backlog unchanged. If the backlog archival itself should be undone, revert this
  closure PR instead.

## Verdict

SHIPPED. Merged to main as merge commit `4436d53` via PR #263. Task 092.002-T is done
(feature 092-F was already archived in 086-S closure). One deferred follow-up remains
queued: 092.003-T (daemon-reader atomicity for `run_with_shutdown` / `run_with_shutdown_v2`
in `ipc_server.rs`).
