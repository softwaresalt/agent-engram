---
title: "Operational Closure - 086-S writer-side workspace+config atomicity"
doc_type: closure
source: "086-S shipment (feature 092-F; task 092.001-T)"
description: >-
  Post-merge closure for shipment 086-S. Records the atomic set_workspace_and_config
  writer fix that closes the new-workspace/old-config publish tear (082-S adversarial
  review F4), the lock-order proof, the pre-Copilot adversarial review (Sol xhigh key +
  Gemini concurrency lens), the single Copilot cycle, the deferred reader-side follow-up,
  and the runtime rollback posture for the bind publish path.
topic: "Writer-side workspace+config atomicity on the bind publish path"
depth: "closure"
decision_status: "SHIPPED - merged to main as merge commit 106be1d via PR #261"
author: ship
date: 2026-07-17
verdict: SHIPPED
pr: 261
merge_commit: 106be1dcbb811ac9f3504a77e8512d431b807fb5
target_commit: 106be1dcbb811ac9f3504a77e8512d431b807fb5
branch: feat/086-writer-atomicity
scope: "Make the bind publish path publish workspace and config atomically so an atomic reader never observes a mismatched (workspace_i, config_j) pair"
reviewers:
  - gpt-5.6-sol
  - gemini-3.1-pro-preview
  - copilot
linked_artifacts:
  - "086-S"
  - "092-F"
  - "092.001-T"
  - "092.002-T"
---

## Summary

engram binds a workspace by publishing two pieces of state: the active workspace snapshot
(`active_workspace`) and the workspace configuration (`workspace_config`). The reader path
was already made atomic in 086.004-T (`snapshot_dispatch_context` acquires both read guards
in a fixed order). The writer, however, still published in two separate awaits -
`set_workspace(..).await` then `set_workspace_config(..).await` - leaving a narrow
new-workspace/old-config window that even an atomic reader could observe. The 082-S
adversarial review flagged this as residual finding F4.

Feature 092-F closes that tear with a single writer-side change: a new atomic
`AppState::set_workspace_and_config` that acquires both write locks in the reader's lock
order, runs the capacity check first, and publishes both values or neither. The bind flow in
`src/tools/lifecycle.rs` was migrated to the single atomic call. This shipment changes no
schema, no on-disk format, and no command routing; it narrows a concurrency window on the
bind publish path.

## Tasks shipped

* `092.001-T` - Atomic `set_workspace_and_config`. Adds
  `AppState::set_workspace_and_config(&self, snapshot, config) -> Result<(), WorkspaceError>`
  in `src/server/state.rs`. It acquires `active_workspace.write()` then
  `workspace_config.write()` (the same order the reader `snapshot_dispatch_context` uses),
  performs the `LimitReached` capacity check first while holding both guards, and returns
  `Err` with no partial publish on failure. The `lifecycle.rs` bind flow was migrated from
  the two-await sequence to this single call, preserving `query_stats` timing reset, scan
  generation, and background hydration spawn.

## Key decisions

### Atomic publish under the reader's lock order

The fix holds both write guards for the whole publish. Correctness rests on a lock-order
proof: crate-wide there are exactly two sites that acquire both locks - the reader
`snapshot_dispatch_context` (read/read) and the new writer (write/write) - and both acquire
`active_workspace` before `workspace_config`. No site acquires them in the reverse order, so
the paired-lock exception cannot deadlock. That deadlock-freedom follows solely from the
consistent lock order. Separately, `tokio::sync::RwLock` guards are `Send`, so holding the
first guard across the second lock's `.await` keeps the resulting future `Send` and schedulable
on the multithreaded runtime - a distinct property from deadlock-freedom, not its cause.

### Capacity check first, no partial publish

The `LimitReached` capacity check runs before either value is mutated. On failure the method
returns `Err` while both guards are still held and nothing has been written, so a rejected
bind can never leave a half-published (new-workspace, old-config) pair. The capacity check is
byte-identical to the one in the pre-existing `set_workspace`.

### Non-vacuous atomicity test

The atomicity test was extended to drive a real A->B / B->A writer transition (bind
workspace A / config A, then workspace B / config B) rather than routing through a neutral
binding. A concurrent atomic reader sampled across the transition must never observe a
mismatched pair. The test fails against the old two-await writer (proving non-vacuity) and
passes after the fix. Target `integration_get_workspace_status_atomicity` runs 3/3.

## Review resolution

### Adversarial review (pre-Copilot, per operator key-reviewer directive)

* **Key reviewer - GPT-5.6 Sol @ xhigh (rust/correctness):** rated the core fix deadlock-free,
  no torn read, no partial publish, bounded critical section. Raised 2 P2 documentation-accuracy
  findings: the T041 deadlock-audit comment was stale (it predated the intentional paired-lock
  exception and wrongly claimed a `!Send` compile-time net catches held-guard-across-await), and
  the `snapshot_dispatch_context` / `DispatchSnapshot` rustdoc did not mention the new atomic
  writer.
* **Cross-model concurrency lens - Gemini-3.1-pro-preview @ high:** independently rated the core
  fix deadlock-free / no-torn-read / no-partial-publish / bounded. Flagged 2 P2 pre-existing
  readers plus 1 P3 advisory (see below).

Both P2 documentation findings were applied and folded into the amended commit `81ba27d`: the
T041 audit comment was rewritten to document the intentional paired-lock exception and drop the
inaccurate `!Send` claim, and the reader rustdoc now references `set_workspace_and_config`.

### Copilot - 1 review pass, resolved

* `81ba27d` - 1 finding: the note at `get_workspace_status` (`lifecycle.rs`) was stale - it still
  described the old two-await publish and called strict atomicity an 082-S F4 follow-up, when this
  change **is** that follow-up. Fixed in `4888935` (the note now references the atomic
  `set_workspace_and_config`); replied and resolved the thread.
* `4888935` - fresh Copilot review clean; 4-point merge gate CLEAN; merged.

## Deferred follow-ups

* `092.002-T` (queued) - Reader-side atomicity for the background readers. Two pre-existing
  background paths in `lifecycle.rs` - `background_db_hydration` and `drain_pending_sync` - still
  read the (workspace, config) pair non-atomically via separate `snapshot_workspace()` and
  `workspace_config()` awaits (Gemini C1/C2, P2). These lines are unchanged by 092.001-T and out
  of scope for the plan-review-gated writer-side task, so deferring them cost no Copilot cycle
  (Copilot reviews the diff). Both paths are opt-in and self-healing (they skip work when either
  value is absent), so the observable risk is a single stale-config hydration or sync pass, not
  data loss. The correct fix needs a new None-gating atomic reader that returns
  `Option<(WorkspaceSnapshot, WorkspaceConfig)>` and preserves the current skip-if-either-None
  semantics - a naive `snapshot_dispatch_context()` swap would default the config instead of
  skipping, which is a semantic change. Tracked as a queued follow-up under 092-F.

## Rejected

* Privatize `set_workspace` / `set_workspace_config` (Gemini C3, P3). Both are `pub` API used by
  roughly 35 test call sites; privatizing them breaks the test suite. Tests are the specification,
  so the public surface stays.

## Release observability

Feature 092-F changes the bind publish path - a runtime surface - so it carries a monitoring and
rollback posture. It changes no DB schema, no JSONL format, and no command routing or tool schema.

### Healthy signals

* `integration_get_workspace_status_atomicity` passes (3/3) in CI. Because the test drives a real
  A->B / B->A writer transition against a concurrent atomic reader, a green run is direct evidence
  that the writer publishes atomically.
* The 029-F WS-6 bind-latency SLA test passes (relaxed 2,000 ms threshold in debug CI; the 500 ms
  target applies to release builds). The atomic method holds both write guards only for in-memory
  swaps with no I/O `.await` inside the critical section, so the critical section stays bounded. Its
  contention profile is not identical to the old two-await sequence - the writer now holds
  `active_workspace` while awaiting `workspace_config` - so this records the green SLA contract
  rather than claiming an unchanged publish cost.

### Failure signals

* `integration_get_workspace_status_atomicity` fails - the writer or reader lock order changed, or
  a partial publish was reintroduced. The failing assertion names the mismatched pair.
* A status read reports a new-workspace / old-config pair after a bind - the exact tear this fix
  closes. This would indicate the atomic writer was bypassed on some bind path.

### Monitoring method, baseline, threshold

* Method: the `integration_get_workspace_status_atomicity` integration test in CI, plus the
  affected lifecycle/config/multi-workspace suites.
* Baseline at merge (`106be1d`): atomicity 3/3; affected suites green (contract_lifecycle 9,
  integration_config 5, integration_daemon_lifecycle 7, integration_multi_workspace 6,
  integration_workspace_id_drift 2, integration_workspace_lifecycle_workflow 4, unit_branch_workspace 6,
  unit_workspace_config_policy 9).
* Threshold to investigate: any failure of the atomicity test; any observed
  new-workspace/old-config status read; or a WS-6 bind-latency SLA breach - bind latency exceeding
  the 500 ms release target, or the relaxed 2,000 ms threshold the debug-CI SLA test enforces.

### Owner and observation window

* Owner: ship and repository maintainer.
* Duration: passive. The guard runs on every code-touching PR. No timed production window is
  required because engram is a local single-binary daemon with no fleet rollout; the change takes
  effect on the next binary the operator runs.
* Outcome (pre-release validation): local gates green (fmt, clippy pedantic); atomicity 3/3;
  affected suites green; adversarial review (Sol xhigh + Gemini) clean on the core fix; 1 Copilot
  pass resolved; 4-point merge gate CLEAN at `4888935`.

### Rollback trigger and procedure

* Trigger: the atomic publish is later found to deadlock, regress bind latency past the WS-6 SLA,
  or otherwise destabilize the bind path.
* Procedure: revert the merge with `git revert -m 1 106be1d`. This restores the two-await publish
  sequence (`set_workspace` then `set_workspace_config`) and removes the atomicity test additions.
  Runtime blast radius is contained to the bind publish path: no schema, format, or routing change
  is reversed, so no data migration is involved. The revert reopens the F4 tear window but does not
  corrupt state. Backlog state is unaffected by the code revert: the archival of 086-S / 092-F /
  092.001-T and the creation of 092.002-T live in this separate closure PR (#262), not in merge
  `106be1d`, so reverting `106be1d` leaves the backlog unchanged. If the backlog archival itself
  should be undone, revert this closure PR instead.

## Verdict

SHIPPED. Merged to main as merge commit `106be1d` via PR #261. Shipment 086-S and feature 092-F are
archived; task 092.001-T is done. One deferred follow-up remains queued: 092.002-T (reader-side
atomicity for `background_db_hydration` and `drain_pending_sync`).
