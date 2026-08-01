---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
status: "blocked (final Stage concurrency review; GATE: BLOCK)"
source: "docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue and the R2 producer/finisher backstop. Concurrency re-review found that the first remediation named the wrong production producer and left startup generation capture underspecified.

- **G1 / FF55E51A:** `src/tools/lifecycle.rs::set_workspace` calls `set_workspace_and_config` and later `begin_scan_generation`. Today a queued sync can observe the new binding before the generation transition. The real queued-sync producer is `src/tools/write.rs::sync_workspace`, not `lifecycle.rs`. It snapshots graph handler context, loses `try_start_indexing`, and publishes a queued request. It must carry a generation proven consistent with that snapshot; a mismatch must retry within a fixed private budget and then fail closed, never be relabeled at publication.
- **G2 / stale publisher:** a generation-G producer can pause after its snapshot and resume after G+1 owns the queue. Publication must compare the explicit G under `pending_sync`: newer replaces, equal OR-coalesces, and older is ignored.
- **S1 / 88EB5FB1:** `src/daemon/ipc_server.rs::try_start_startup_sync` still fails the startup indexing-lock attempt and calls `set_pending_sync`. It must capture generation before that initial attempt and pass that exact token to the explicit-generation R2 publisher. The unqualified two-argument publisher is forbidden.
- **D1 / 1E70A289:** comments in the same `state.rs` width overclaim same-lock generation capture and misuse a SeqCst total-order edge as a happens-before proof. Correct them without widening the task.

Scope excludes 015-D IPC response behavior and all Python, Spark, SQL, PowerBI, deletion, schema, CLI-response, persistence, and Cozo work.

## Final Stage Disposition

**GATE: BLOCK. Do not execute or claim 104-S under the current caps.** The Concurrency Reviewer found that `src/tools/lifecycle.rs::drain_pending_sync` lost-lock re-arm is not generation-neutral. After G -> G+1 it can preserve old heavy companion bits, or relabel old intent through unqualified `set_pending_sync` as the current generation. A complete generation fix therefore requires `src/server/state.rs`, `src/tools/write.rs`, and `src/tools/lifecycle.rs`.

That three-production-file fix exceeds 109.002-T / Unit 2's hard two-production-file generation GREEN cap. Moving the lifecycle correction into 109.004-T / Unit 4 would violate that task's `src/daemon/ipc_server.rs`-only startup GREEN cap. The shipment stop condition is met; Stage must not widen, split, or invent scope. Shipment 104-S, feature 109-F, and all four unstarted child tasks are blocked. A future operator-directed replan must explicitly decide whether to authorize a three-file generation GREEN cap or approve a different task/shipment decomposition before Stage can review and re-queue this work.
## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| G1a: binding cannot be observed with a stale producer generation | Privately refactor state-owned `set_workspace_and_config` / `begin_scan_generation` semantics while leaving the lifecycle call sequence unchanged | Deterministic binding-visible-before-generation RED becomes GREEN |
| G1b: stale producer cannot coalesce into a newer owner | `write.rs` validates generation before/after `snapshot_graph_handler_context`, retries mismatch, fails closed on exhaustion, and publishes accepted G explicitly | Paused G / owning G+1 / resumed G scenario has no heavy leak |
| G1c: equal-generation behavior is preserved | Explicit publisher OR-coalesces equal-generation bits | One same-generation sticky control |
| S1: startup producer has a guaranteed finisher | Capture G before startup lock attempt; on failure call explicit-generation publish/reacquire | Final-peek race and no-contention control |
| Lock safety | Finish capacity validation and all other awaits before acquiring `scan_cancel` or `pending_sync`; no synchronous guard across await | Targeted review plus deterministic tests |
| Publication completeness | Inventory every production publication/re-arm site and classify migration or neutrality | **Failed:** lifecycle re-arm is generation-sensitive and requires a third generation GREEN production file |
| Preserve contracts | No public API/schema/model/CLI/MCP response change | Diff and structural review |

## Implementation Units

### Unit 1 — RED generation-transition harness

**Backlog shape:** tests-only task; target 70–90 minutes.

Add exactly three deterministic scenarios, never sleeps:

1. **G1a stale transition order:** expose the new workspace/config binding while the old generation still owns the queue, publish through the queued-sync producer seam, run old-generation clear, and show the new-binding request is lost pre-fix.
2. **G1b stale publisher:** capture explicit G with the producer binding snapshot, pause before publication, establish a G+1 owner, resume G, and show pre-fix relabel/heavy leakage; post-fix G is ignored.
3. **G1c same-generation control:** two requests carrying the same generation preserve sticky OR-coalescing.

Exercise the `state.rs` transition/publisher seam and the real `write.rs` queued producer seam. Private/unit access or a narrow `cfg(test)` seam is allowed; no public test API.

**Files cap:** at most two test surfaces (`src/server/state.rs`, `src/tools/write.rs`); no production behavior change.
**Function/scenario cap:** at most three helpers and exactly three scenarios.
**Stop trigger:** return blocked for a third surface, fourth scenario, wall-clock sleep, process-global mutation, daemon/IPC harness, public hook, or 110 minutes.

### Unit 2 — GREEN binding/generation/publish linearization

**Backlog shape:** source-only task; target 80–110 minutes; depends on Unit 1.

**BLOCKED — do not execute.** The current cap permits only `src/server/state.rs` and `src/tools/write.rs`, but the complete fix also requires `src/tools/lifecycle.rs` to make drain re-arm generation-qualified. The prior two-file design below is superseded by the final review finding; it is retained only to show why the cap cannot be met.

1. **State-owned transition with unchanged lifecycle caller.** Privately refactor `set_workspace_and_config` / `begin_scan_generation` semantics so the existing lifecycle sequence publishes the binding/config, advanced generation, and fresh cancellation ownership as one state transition. `lifecycle.rs` is not edited. Acquire workspace then config; complete their awaited acquisitions and the workspace-capacity validation before awaiting `scan_cancel`. Once `scan_cancel` is acquired, perform no further await before the transition is complete. Acquire `pending_sync`, if required by the chosen private implementation, only after every await. No `std::sync::MutexGuard` may cross `.await`. `begin_scan_generation` returns the generation/receiver prepared by that transition rather than creating a binding-visible gap.
2. **Validated producer snapshot in `write.rs`.** Before `try_start_indexing`, read generation, await `snapshot_graph_handler_context`, then read generation again. Accept only equal before/after values. On mismatch, retry using a fixed small private budget; on exhaustion return an existing internal error before taking the indexing lock or publishing. Never continue with the later value and never reread generation to relabel after contention.
3. **Explicit publication.** Carry accepted G through the failed-lock/pause path into an explicit-generation `publish_pending_sync_and_try_reacquire(G, revalidate, backfill)` path. Under `pending_sync`, newer replaces, equal OR-coalesces, and older is ignored. Preserve generation-scoped clear and companion+pending atomicity.
4. **Proof comments.** State that the generation is validated with the producer binding snapshot and supplied explicitly. Restate R2 through `pending_sync` mutex critical-section order; SeqCst remains only the indexing-flag ordering.

The four-production-function budget is hard and covers the state transition pair, the explicit publish/reacquire path, and the `write.rs` queued-sync flow. Existing unqualified helpers with no production callers are not widened or adopted; their cleanup is deferred.

**Files cap:** `src/server/state.rs` + `src/tools/write.rs` only.
**Function cap:** at most four production functions plus nearby comments.
**Stop trigger:** return blocked at a third production file, fifth production function, any `lifecycle.rs` edit, mismatch fallback/relabel, unbounded retry, unresolved lock order, synchronous guard across await, public contract/model change, second queue/state machine, unsafe code, or 110 minutes.

### Unit 3 — RED startup final-peek harness

**Backlog shape:** tests-only task; target 60–80 minutes; depends on Unit 2.

Add one deterministic startup race in the private `ipc_server` test surface. Startup captures G before the initial indexing-lock attempt; the attempt fails; publication pauses; the holder releases and completes its final empty pending-sync peek; then startup resumes publication with the same G. Assert that the same-generation startup path guarantees a finisher without an external watcher/index/sync/timer tick. Include one no-contention control.

**Files cap:** one test surface in `src/daemon/ipc_server.rs` or one focused test file, not both.
**Scenario cap:** one race plus one control.
**Stop trigger:** return blocked for a third scenario, real daemon, IPC timing, sleep, public seam, or 110 minutes.

### Unit 4 — GREEN startup producer backstop

**Backlog shape:** source-only task; target 60–90 minutes; depends on Unit 3.

**BLOCKED by Unit 2 and may not absorb the lifecycle correction.** Doing so would violate this unit's `src/daemon/ipc_server.rs`-only production cap.

In `src/daemon/ipc_server.rs`, capture `current_sync_generation` before the initial `try_start_indexing` attempt. If the attempt fails, pass that exact token to `publish_pending_sync_and_try_reacquire(G, false, false)`. Never call `set_pending_sync` or an unqualified two-argument publisher from production startup. If G became stale before publication, the state publisher ignores it; it must not stamp the current generation.

Distinguish only initial owner, queued-under-holder, and producer-reacquired outcomes. A reacquiring producer releases the indexing flag, drains exactly once, and returns without running the normal startup body twice. Complete awaits before any `pending_sync` guard and never carry a synchronous guard across await.

**Files cap:** `src/daemon/ipc_server.rs` only.
**Function cap:** at most three private functions.
**Stop trigger:** return blocked on post-failure generation capture, unqualified publisher use, recursion/double-drain, second state machine, synchronous guard across await, behavior outside startup coordination, or 110 minutes.

## Production Publication Inventory

Planning-time targeted source inventory found these production sites:

| Production site | Current role | Disposition |
|---|---|---|
| `src/tools/write.rs::sync_workspace` failed-lock branch | Publishes queued sync plus revalidate/backfill companions through publish/reacquire | **Migrate in 109.002-T.** Validate G around `snapshot_graph_handler_context`; retry/fail closed on mismatch; pass explicit G. |
| `src/daemon/ipc_server.rs::try_start_startup_sync` failed-lock branch | Calls `set_pending_sync` for startup intent | **Migrate in 109.004-T.** Capture G before the lock attempt and call only explicit-generation publish/reacquire. |
| `src/tools/lifecycle.rs::drain_pending_sync` lost-lock branch | Re-arms a pending bit already consumed by the drain | **BLOCKER; not generation-neutral.** After G -> G+1 it can preserve old heavy companions or relabel old intent through unqualified `set_pending_sync` as the current generation. Correctness requires a `lifecycle.rs` production edit in the same generation fix as `state.rs` + `write.rs`. |
| `src/server/state.rs` unqualified publish/companion helpers | Primitive/test compatibility surface; no additional production caller found | **Deferred.** Do not use them from 109.002-T or 109.004-T; cleanup would exceed this remediation. |

Consumers (`take_*`, `clear_pending_sync_for_generation`, `has_pending_sync`) are not publication sites. Test-only calls are not production inventory. Final review failed the lifecycle-neutrality proof: the required three-file generation fix breaches Unit 2, while assigning the lifecycle edit to Unit 4 breaches its startup-only cap. This is an activated stop condition, not an advisory; no implementation may start until an operator-directed replan and explicit cap/decomposition decision passes review.

## Dependency Graph

```text
Unit 1 RED generation harness (2 stale-order + 1 same-G control)
  -> Unit 2 GREEN state.rs + write.rs
     -> Unit 3 RED startup harness
        -> Unit 4 GREEN ipc_server.rs
```

The Unit 3 dependency on Unit 2 is semantic: startup must consume the corrected explicit-generation primitive. Shipments 102-S and 103-S precede 104-S only by operator order, not technical dependency.

## Decisions and Rationale

1. **Use the real producer.** Unit 2 targets `write.rs`, not `lifecycle.rs`; the latter caller sequence remains unchanged while state owns transition semantics.
2. **Bound mismatch handling.** A torn before/after generation snapshot retries within a fixed private budget and then fails closed before lock/publish. Relabeling is forbidden.
3. **Capture startup generation before contention.** The startup token identifies the producer before the failed attempt; publication cannot choose a later generation.
4. **Keep three RED scenarios.** The two stale-order cases and one same-generation control satisfy 109.001-T without exceeding its cap.
5. **Block on the non-neutral lifecycle remainder.** Lifecycle drain re-arm can carry stale heavy companions or relabel old intent after G -> G+1. The complete fix needs `state.rs` + `write.rs` + `lifecycle.rs`, which activates the Unit 2 third-file stop condition.
6. **Do not reopen 105-F or fold 015-D.** Archived predecessor evidence remains immutable and IPC response behavior is a different width.

## Risks and Caveats

- **Atomicity gap:** merely moving the generation read does not bind it to the workspace snapshot. Mitigation: state transition invariant plus before/after validation, bounded retry, fail closed.
- **Reverse stale leak:** publication-time reread can relabel G as G+1. Mitigation: explicit token only; older ignored under mutex.
- **Deadlock:** async workspace/config/scan-cancel locks interact with synchronous pending mutex. Mitigation: workspace -> config -> scan_cancel -> pending_sync; capacity validation and all other awaits before scan_cancel/pending; no synchronous guard across await.
- **Startup double execution:** reacquiring producer could drain and continue normal startup. Mitigation: explicit outcome and exactly-one-finisher assertion.
- **Lifecycle re-arm is generation-sensitive:** it can preserve stale heavy companions or relabel through unqualified `set_pending_sync` after G -> G+1. Disposition: GATE BLOCK; future operator replan/cap decision required.
- **Scope expansion:** helper cleanup could consume the function cap. Mitigation: defer unqualified no-production-caller helpers and prohibit their use.

## Plan Hardening Signals

| Signal | Present | Reason |
|---|---|---|
| Public API, schema, or contract change | no | All changes are private; serialized and CLI/MCP contracts are frozen |
| Security/auth/permission/compliance | no | No trust boundary or credential behavior |
| Migration/backfill/destructive/irreversible | no | In-memory coordination only |
| External integration/operator checkpoint | no | No external dependency |
| High runtime, rollout, or rollback risk | yes | Concurrency ordering can silently drop, relabel, or duplicate work |

**Requires plan hardening: yes.**

## Plan Hardening

### Protected invariants

- The unchanged lifecycle call sequence cannot expose a new binding with an old generation.
- Every binding-specific producer carries a generation validated with its snapshot; mismatch retries then fails closed.
- Under `pending_sync`, newer replaces, equal OR-coalesces, and older is ignored.
- Startup captures generation before the initial lock attempt and never uses the unqualified publisher.
- Every failed producer/holder handoff has exactly one guaranteed finisher.
- Lifecycle lost-lock re-arm preserves the original generation and companion ownership; it never relabels via unqualified `set_pending_sync` after G -> G+1.
- Drain remains bounded; no synchronous mutex guard crosses await; no public behavior or persistence changes.

### Risk actions

**ProposedAction PA-1**
summary: state/write generation linearization plus generation-qualified lifecycle drain re-arm
targets: `src/server/state.rs`, `src/tools/write.rs`, `src/tools/lifecycle.rs`
change_kind: shared runtime coordination edit
ActionRisk: moderate
rollback: revert feature commit and restart daemon; no data repair/reindex
approval_required: operator replan and explicit generation GREEN cap/decomposition decision
ActionResult: blocked by the current two-production-file cap

**ProposedAction PA-2**
summary: pre-lock startup generation capture and explicit-token R2 handoff
targets: `src/daemon/ipc_server.rs`
change_kind: daemon startup coordination edit
ActionRisk: moderate
rollback: revert feature commit and restart daemon
approval_required: blocked pending the PA-1 operator replan/cap decision
ActionResult: blocked by the invalid upstream generation GREEN

### Dark-mode execution order and stop triggers

1. Keep 102-S and 103-S queued in existing operator order; this disposition does not modify them.
2. Do not claim or execute 104-S. Block 104-S, 109-F, and all unstarted child tasks.
3. Resume only after an operator-directed replan explicitly authorizes a three-file generation GREEN cap or approves different task/shipment decomposition and a new review returns PASS.

Activated stop condition: the complete generation fix requires a third Unit-2 production file (`lifecycle.rs`). Immediate stop conditions: 110 minutes on a card; Unit 2 exceeds two production files/four production functions; Unit 1 exceeds two test surfaces/three scenarios; Unit 4 exceeds one production file/three private functions; any lifecycle production edit; any public contract/schema/model change; unbounded retry; snapshot mismatch relabel/fallback; generation captured after startup lock failure; production use of an unqualified publisher; synchronous guard across await; unresolved inventory/lock order; timing RED; second queue/state machine; unsafe code; recursion/double-drain; or unprovable exactly-one-finisher.

### Monitoring, rollback, and observation

| SLI | Healthy | Rollback/block trigger | Owner/window |
|---|---|---|---|
| generation fixtures | G1a survives old clear; resumed G ignored after G+1; same-G sticky OR preserved | any drop, relabel, heavy leak, or control regression | Ship; targeted gate + 15 minutes |
| snapshot validation | mismatch retries within fixed budget; exhaustion publishes nothing | fallback to later generation, lock acquisition, or publication after exhaustion | Ship; targeted gate |
| startup fixture | one guaranteed finisher; zero duplicate startup bodies | stranded request or duplicate body | Ship; targeted gate + three restarts |
| publication inventory | all production sites match the table | any unclassified binding-specific producer/unqualified startup publisher | Ship; pre-merge |
| drain/startup runtime | zero bound warnings; completion within 30-second debug budget | attributable warning or one restart over budget | Ship; three restarts + 15 minutes |

Rollback is commit revert plus daemon restart. No migration reversal, cache repair, or full reindex is expected.

## Runtime Verification and Closure

No runtime verification or implementation is authorized under this blocked plan. The future reviewed plan must cover generation-qualified lifecycle re-arm together with the state/write changes, retain deterministic stale-order and startup controls, and restate monitoring/rollback. Validation in this Stage cycle is backlog/plan structure only.

## Harvest Shape

Use the existing medium-priority feature 109-F and queued tasks 109.001-T through 109.004-T in strict dependency order. Keep shipment 104-S queued. Do not create tasks, alter unrelated stash, or change shipment lifecycle. Unit 2 references only `state.rs` + `write.rs`; Unit 4 references `## Harvest Shape

Existing shipment 104-S, feature 109-F, and tasks 109.001-T through 109.004-T are blocked; no new task or shipment is created. Keep 102-S and 103-S queued with their existing order metadata. Stage must not widen, split, or invent remediation scope. A future operator-directed replan must make the cap/decomposition decision before any item can return to queued.

## Plan Review — GATE: BLOCK

**Review mode:** final narrow Stage concurrency review-cycle disposition for 104-S/109-F only. No implementation, build, test, lint, commit, push, task/shipment creation, shipment claim, or stash mutation occurred.

**Hardening gate:** failed under the current caps. The lifecycle lost-lock re-arm is generation-sensitive, so the complete generation fix requires three production files while Unit 2 permits two. Unit 4 cannot absorb `lifecycle.rs` because it is capped to `src/daemon/ipc_server.rs` only.

### Findings

- **P0:** none.
- **P1 BLOCK — lifecycle drain re-arm is not generation-neutral:** after G -> G+1 it can preserve old heavy companions or relabel old intent through unqualified `set_pending_sync` as the current generation.
- **P1 BLOCK — no cap-compliant complete fix:** correcting the invariant requires `src/server/state.rs` + `src/tools/write.rs` + `src/tools/lifecycle.rs`, exceeding 109.002-T / Unit 2's two-production-file cap.
- **P1 BLOCK — startup GREEN cannot absorb the edit:** moving `lifecycle.rs` into 109.004-T / Unit 4 violates its `src/daemon/ipc_server.rs`-only cap.
- **Disposition:** shipment stop conditions apply. Do not widen, split, or invent scope; block the existing shipment/feature/tasks and require a future operator replan/cap decision.

### Persona decisions

- **Constitution Reviewer:** PASS on Stage role boundaries and the decision not to implement or invent scope.
- **Rust/Concurrency Reviewer:** **BLOCK** — the prior lifecycle-neutrality proof is invalid.
- **Scope Boundary Auditor:** **BLOCK** — the complete three-file fix breaches Unit 2, and transferring it breaches Unit 4.
- **Architecture Strategist:** **BLOCK** pending a generation-qualified lifecycle re-arm design under operator-approved caps.

**Decision:** **GATE BLOCK.** 104-S and 109-F remain non-executable. A future operator must explicitly choose whether to authorize a three-production-file generation GREEN cap or approve different task/shipment decomposition; Stage then must replan and re-review before re-queueing.
