---
title: "Writer-side workspace+config atomicity (086-F residual F4) — implementation plan"
type: exec-plan
doc_type: plan
source: "stash 32DAA85B (082-S adversarial review F4)"
date: 2026-07-15
status: reviewed — ready for harvest
author: stage
source_stash: 32DAA85B
origin_review: docs/closure/2026-07-15-082-s-runtime-reliability-adversarial-review.md
covering_feature: 092-F
requires_plan_hardening: yes
plan_review_gate: PASS
plan_review_evidence: docs/closure/2026-07-15-stage-followups-adversarial-review.md
---

# Writer-side workspace+config atomicity (092-F / 092.001-T)

## Objective

Close the **writer-side** tear identified as **F4** in the 082-S adversarial review. The
**reader** path was made atomic in 086.004-T (`snapshot_dispatch_context` holds both read guards),
but the **writer** still publishes workspace then config in two separate awaits, so an atomic reader
can still observe a `new-workspace / old-config` pair during a bind transition.

## Grounding (authoritative origin/main @ df77584)

- `src/server/state.rs`
  - `snapshot_dispatch_context` (L245): reader — acquires `active_workspace.read()` **then**
    `workspace_config.read()`. Establishes the canonical lock order **workspace → config**.
  - `set_workspace` (L253): acquires only `active_workspace.write()`; performs the
    `LimitReached` capacity check; publishes the workspace snapshot.
  - `set_workspace_config` (L361): acquires only `workspace_config.write()`; publishes the config.
- `src/tools/lifecycle.rs` `set_workspace` (L88): the bind flow calls
  `state.set_workspace(snapshot).await?;` immediately followed by
  `state.set_workspace_config(Some(ws_config.clone())).await;` (L159–160) — the two-await tear.

## Design

1. Add `AppState::set_workspace_and_config(&self, snapshot: WorkspaceSnapshot, config:
   Option<WorkspaceConfig>) -> Result<(), WorkspaceError>` that:
   - acquires `active_workspace.write()` **then** `workspace_config.write()` — **same order as
     `snapshot_dispatch_context`** (mandatory; see plan-harden);
   - performs the existing `LimitReached` capacity check **first**, while holding both guards, and
     returns `Err` **without** mutating either value on failure (no partial publish);
   - assigns `*workspace = Some(snapshot)` and `*config = config`, then drops both guards.
2. Migrate the `lifecycle.rs` bind flow to a single
   `state.set_workspace_and_config(snapshot, Some(ws_config.clone())).await?;` call, replacing the
   two separate awaits. Preserve all surrounding behavior (`query_stats::reset_timing`, scan
   generation, background hydration spawn).

## Test-first acceptance (drives the task)

Write the failing test **first**:

- Extend the `get_workspace_status` / dispatch atomicity test to drive **real** `A → B` and
  `B → A` writer ordering (bind workspace A with config A, then bind workspace B with config B),
  **not** routed through a neutral workspace `N`.
- A concurrent atomic reader (`snapshot_dispatch_context`) sampled across the transition must
  **never** observe a mismatched pair `(workspace_i, config_j)` with `j != i`.
- The test **must fail** against the current two-await writer (proving non-vacuity) and **pass**
  after `set_workspace_and_config` lands.

## Plan-harden (elevated blast radius — shared bind publish path)

The new method sits on the shared `AppState` publish path exercised by **every** bind. Hardening
requirements (all encoded into 092.001-T acceptance):

- **Lock order invariant:** acquire `active_workspace` before `workspace_config` everywhere both are
  held (writer matches reader) — prevents lock-order inversion / deadlock. Document the invariant
  in-code at both hold sites.
- **Bounded critical section:** only trivial moves/clones under the two write guards; **no `.await`
  on I/O** while holding them (protects the 500 ms bind-latency SLA, 029-F WS-6).
- **No partial publish:** capacity check precedes any mutation; error path leaves state unchanged.
- **Rollback:** revert to the two-await sequence. No DB schema, JSONL format, or migration change →
  rollback is trivial and side-effect-free.
- **Monitoring / release-observability:** bind latency (WS-6 SLA) unchanged; no new SQLITE_BUSY
  surface; watch that atomic status reads never report a `new-workspace/old-config` mismatch.

## Constitution / safety

- No new dependencies. `#![forbid(unsafe_code)]` intact (safe RwLock usage only).
- No security surface change (internal state atomicity; no auth/crypto/PII path).

## Scope isolation (single width)

Touches **only** `src/server/state.rs` + `src/tools/lifecycle.rs` + the atomicity test. One concern
(concurrency/atomicity). **No** CLI, schema, template, or Power BI work bundled. Independent of
Option C (091-F), 083-S/084-S/085-S.

## Backlog mapping

- Covering feature **092-F**; single executable task **092.001-T**; adversarial + plan-review gate
  **092.001-R** (PASS); shipment **086-S** = `[092-F, 092.001-T]` (queued, medium).
