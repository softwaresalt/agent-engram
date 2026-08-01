---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
status: "reviewed + hardened (plan-review GATE: PASS)"
source: "docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue and the R2 producer/finisher backstop. PR #302 review found two remaining producer windows on that same primitive.

- **G1 / FF55E51A:** `set_workspace` publishes the new workspace/config pair before `begin_scan_generation` advances `sync_generation`. A concurrent sync can observe the new binding, fail the indexing lock held by the old hydration, publish under the old generation, and then be cleared by that old hydration. Separately, `publish_pending_sync` loads the generation before acquiring the pending-sync mutex; a paused generation-G heavy publish can resume after a G+1 routine publish and OR the stale heavy bit into the newer request.
- **S1 / 88EB5FB1:** `try_start_startup_sync` still performs failed `try_start_indexing` followed by `set_pending_sync` without the R2 re-acquire backstop. A startup producer that resumes after the holder final-peek can strand the intent until an unrelated tick.
- **D1 / 1E70A289:** comments in the exact `state.rs` primitive overclaim same-lock generation capture and use a SeqCst total-order edge as happens-before. Correct them while G1 changes that primitive; no independent documentation task is needed.

Failures are narrow and self-healing, but they can drop explicit queued intent, leak an unnecessary heavy companion, or delay startup sync. Scope excludes 015-D IPC response behavior and all Python, Spark, SQL, PowerBI, deletion, schema, CLI-response, and Cozo work.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| G1a: new binding cannot be observed with a stale queue generation | Linearize binding/generation transition at one internal seam | Deterministic binding-visible-before-bump RED scenario becomes GREEN |
| G1b: stale publisher cannot coalesce into newer owner | Read/validate generation at the pending queue publish linearization point; reject stale ownership | Deterministic G / G+1 / resumed-G heavy-bit scenario |
| D1: comments match the real memory-order proof | Update `PendingSyncState::publish` and R2 proof comments in the same source task | Review checks mutex-CS proof and no false same-lock claim |
| S1: startup producer has a guaranteed finisher | Reuse `publish_pending_sync_and_try_reacquire`; on reacquire, release and drain exactly once | Deterministic intent-after-final-peek RED scenario becomes GREEN |
| Preserve behavior | No public API, schema, CLI/MCP result, migration, second queue, or unbounded loop | Existing queue and startup controls remain unchanged |

## Implementation Units

### Unit 1 — RED generation-transition harness

**Backlog shape:** tests-only task; target 70–90 minutes.

- Add two deterministic scenarios using barriers/explicit state steps, never wall-clock sleeps:
  1. publish a new binding while the old generation owns the indexing lock, queue before generation advance, run the old-generation clear, and assert the new-binding request survives;
  2. arrange owner G+1 routine bits, then resume a stale G revalidation publish and assert no heavy companion leaks into G+1.
- Prefer private/unit access or existing public state hooks. A `cfg(test)`-only seam is acceptable; a new public test API is not.
- Demonstrate both scenarios RED against the pre-fix implementation before Unit 2 changes production behavior.

**Files cap:** at most two test surfaces; no production behavior change.  
**Function/scenario cap:** at most three helpers and two scenarios.  
**Stop trigger:** return blocked if deterministic control requires sleeps, process-global mutation, or a public hook.

### Unit 2 — GREEN binding/generation/publish linearization

**Backlog shape:** source-only task; target 80–110 minutes; depends on Unit 1.

- Add one crate-private AppState transition seam that replaces the lifecycle pair set_workspace_and_config(...) then begin_scan_generation(). It acquires the workspace/config write guards and the scan-cancel guard in that order, performs the capacity check, then briefly locks pending_sync to advance sync_generation; it installs the binding/config and fresh cancel channel before releasing the write guards. A sync snapshot therefore cannot observe the new binding until the new generation is published.
- In pending publish/arm entry points, acquire pending_sync before reading sync_generation, matching the transition seam. Tighten PendingSyncState::publish: newer generation replaces, equal generation OR-coalesces, older generation is ignored and cannot contaminate the owner.
- Preserve generation-scoped clear, sticky same-generation coalescing, newer-generation replacement, and companion+pending atomic publication from 105-F. Refactor the existing begin-generation helper only as needed to avoid duplicate channel/cancel logic.
- Required lock order is workspace -> config -> scan_cancel -> pending_sync. All awaits finish before pending_sync is acquired; never hold a std::sync::Mutex guard across .await.
- Correct the two comments from 1E70A289: state the actual generation capture/validation order and restate the R2 proof through pending-sync mutex critical-section order, with SeqCst only as a sufficient indexing-flag ordering.
- Do not change dispatch results, serialized models, queue bit meanings, retry bounds, or hydration readiness.

**Files cap:** `src/server/state.rs` and `src/tools/lifecycle.rs` only.  
**Function cap:** at most four production functions plus nearby comments.  
**Stop trigger:** return blocked if the invariant needs a third production file, public contract/model change, a second queue, unsafe code, or more than two hours.

### Unit 3 — RED startup final-peek harness

**Backlog shape:** tests-only task; target 60–80 minutes; depends on Unit 2.

- Add one deterministic startup-path scenario that pauses the producer after its initial indexing-lock failure, lets the holder release and complete its final empty-queue peek, then resumes publication.
- Assert that the startup path itself guarantees a finisher without an external watcher/index/sync tick.
- Keep the harness in the private `ipc_server` test surface. A narrow `cfg(test)` pause seam is allowed; timing sleeps and daemon-process orchestration are not.
- Demonstrate RED under the current `set_pending_sync` startup path.

**Files cap:** one test surface in `src/daemon/ipc_server.rs` or one focused test file, not both.  
**Scenario cap:** one race plus one no-contention control.  
**Stop trigger:** return blocked if the test needs a real daemon, IPC timing, or a public seam.

### Unit 4 — GREEN startup producer backstop

**Backlog shape:** source-only task; target 60–90 minutes; depends on Unit 3.

- Route `try_start_startup_sync` through the existing `publish_pending_sync_and_try_reacquire(false, false)` R2 primitive after the initial lock attempt fails.
- Distinguish initial ownership, queued-under-holder, and producer-reacquired outcomes only as much as needed. If the producer reacquires, it must become the guaranteed finisher: release the indexing flag, drain the queued startup request to completion, and avoid running the normal startup body twice.
- Preserve the bounded 64-pass drain, startup retry/backoff, registry ingestion, embedding backfill, flush behavior, and no-contention fast path.
- Do not add another pending flag, queue, scheduler, background task, or response contract.

**Files cap:** `src/daemon/ipc_server.rs` only.  
**Function cap:** at most three private functions.  
**Stop trigger:** return blocked on recursion/double-drain, a second state machine, changes outside startup coordination, or more than two hours.

## Dependency Graph

```text
Unit 1 RED generation harness
  -> Unit 2 GREEN generation linearization
     -> Unit 3 RED startup harness
        -> Unit 4 GREEN startup backstop
```

The Unit 3 dependency on Unit 2 is semantic, not merely serial: the startup producer must adopt the corrected generation-aware publish primitive. There is no implementation dependency on queued shipments 102-S or 103-S; they precede this unit only by operator dark-mode priority.

## Decisions and Rationale

1. **New feature, not reopened 105-F.** 105-F and its children are archived predecessor evidence and remain immutable.
2. **Include 1E70A289.** Its comments describe the exact lock/generation proof changed by Unit 2, so folding them adds no file or concern width.
3. **Separate RED and GREEN tasks.** This preserves test-first evidence and keeps every execution card below two hours.
4. **Reuse R2.** Startup must use the existing publish/reacquire/drain primitive rather than creating another wakeup mechanism.
5. **Do not fold 015-D.** Its non-persist cause remains unpinned and its IPC hang is a different architectural width.

## Risks and Caveats

- **Lock-order/deadlock:** binding uses async RwLocks while the queue uses a synchronous mutex. Mitigation: no synchronous guard across await; document a single order and keep critical sections bounded.
- **Reverse stale leak:** merely changing comparison direction could trade dropped intent for stale heavy work. Mitigation: both G/G+1 orderings are mandatory RED cases.
- **Startup double execution:** a reacquiring producer could both drain and continue the normal startup body. Mitigation: explicit outcome and exactly-one-finisher assertion.
- **Test flakiness:** scheduler timing is not evidence. Mitigation: barriers/explicit steps only; any sleep-based harness is a stop trigger.
- **Scope expansion:** an apparently simple atomicity fix could turn into dispatch-model refactoring. Mitigation: hard file/function/time caps and blocked return.

## Plan Hardening Signals

| Signal | Present | Reason |
|---|---|---|
| Public API, schema, or contract change | no | All seams remain private; serialized models and CLI/MCP responses are frozen |
| Security/auth/permission/compliance | no | No trust-boundary or credential behavior |
| Migration/backfill/destructive/irreversible | no | In-memory coordination only; no persisted data shape |
| External integration/operator checkpoint/dependency | no | No external service or release dependency |
| High runtime, rollout, or rollback risk | yes | Concurrency ordering can silently drop or duplicate work despite a small diff |

**Requires plan hardening: yes.**

## Plan Hardening

Hardening is required because both fixes alter daemon concurrency linearization and startup ownership. Reinforcing context: archived reviewed plan `docs/exec-plans/2026-07-30-daemon-sync-index-reconciliation-plan.md`, `concurrency.instructions.md`, `strict-safety.instructions.md`, `release-observability.instructions.md`, and Engram map/impact results for `publish_pending_sync` and `try_start_startup_sync`. No directly applicable compound record overrode the 105-F invariants.

### Protected invariants

- One pending request and both companion bits are observed atomically under one owner generation.
- Older-generation clear cannot erase newer intent; older publish cannot contaminate newer intent.
- Every failed producer/holder handoff has exactly one guaranteed finisher.
- Drain remains bounded; normal startup executes once; no public behavior or persistence changes.

### Risk actions

**ProposedAction PA-1**  
summary: linearize workspace binding/generation transition with pending publication  
targets: `src/server/state.rs`, `src/tools/lifecycle.rs`  
change_kind: shared runtime coordination edit  
ActionRisk: moderate  
rollback: revert the feature commit; restart daemon; no data repair or reindex required  
approval_required: no additional approval; operator approved moderate non-destructive staging  
ActionResult: planned

**ProposedAction PA-2**  
summary: route startup failed-lock producer through existing R2 backstop and exactly-one drain  
targets: `src/daemon/ipc_server.rs`  
change_kind: daemon startup coordination edit  
ActionRisk: moderate  
rollback: revert the feature commit; restart daemon; prior self-healing startup behavior returns  
approval_required: no additional approval  
ActionResult: planned

### Dark-mode execution order and stop triggers

1. Ship 102-S first (existing high-priority Python attribution).
2. Ship 103-S second (existing ordinary-index fail-closed pair).
3. Claim this shipment only after 102-S and 103-S are no longer active/queued by operator order. There is no technical cross-shipment dependency.
4. Execute Units 1 -> 2 -> 3 -> 4 without parallel edits.

Immediately return the current task blocked on any of these measurable conditions:

- task elapsed effort reaches 110 minutes, more than two production files, more than four production functions, or more than three test scenarios;
- any RED test cannot fail deterministically without sleeps or a public test seam;
- implementation requires a schema/public model/CLI/MCP response change, second queue, unbounded retry, unsafe code, or 015-D IPC work;
- a synchronous mutex guard would cross `.await`, lock order cannot be stated, or exactly-one-finisher cannot be proven;
- existing generation-scoped clear, sticky same-generation companions, no-contention startup, or bounded-drain control regresses.

### Monitoring, rollback, and observation

No production dashboard exists for these internal bits, so monitoring is a manual structured closure checklist using deterministic tests, daemon logs, and workspace status.

| SLI | Baseline/healthy | Alert or rollback trigger | Owner/window |
|---|---|---|---|
| generation race fixture | 0 dropped new-generation requests; 0 stale heavy-bit leaks | any failure in any run | Ship; targeted gate and 15-minute post-merge window |
| startup handoff fixture | 1 guaranteed finisher; 0 duplicate startup bodies | any stranded request or duplicate body | Ship; targeted gate and three controlled daemon restarts |
| drain-bound warnings | 0 `reached iteration bound` warnings in controlled runs | any warning attributable to this change | Ship; 15 minutes |
| startup completion | scan leaves running state within existing 30-second debug budget | one controlled restart exceeds budget or needs an external tick | Ship; three restarts |

Pre-merge audit: confirm no schema/contract change, rollback commit is identified, targeted RED evidence exists, targeted GREEN/control tests pass, normal startup path remains single-run, and monitoring checklist is attached to closure. Roll back before release on any wrong-generation clear/coalesce, startup double-run, stranded queue, deterministic flake, or drain-bound warning. Rollback is commit revert plus daemon restart; no migration reversal, cache repair, or full reindex is expected.

## Runtime Verification and Closure

- **Unit 1:** no runtime behavior change; retain RED evidence in task/PR trace.
- **Unit 2:** run deterministic generation interleavings and a normal same-generation coalescing control; inspect no stale companion and no dropped pending bit.
- **Unit 3:** no runtime behavior change; retain the final-peek RED trace.
- **Unit 4:** run startup race plus no-contention control, then three disposable-workspace daemon restarts; verify one startup body, no external tick, no drain-bound warning, and scan completion under the existing debug budget.
- Operational closure must carry PA-1/PA-2 results, the SLI table, observation outcome (`healthy`, `degraded`, or `rolled-back`), rollback commit/procedure, and owner.

## Harvest Shape

Create one medium-priority feature related to 105-F and four queued tasks matching Units 1–4. Parent first; then wire strict dependencies 2 on 1, 3 on 2, and 4 on 3. Add the feature to one queued shipment before its children. Preserve stash provenance for FF55E51A, 88EB5FB1, and 1E70A289; archive those three only after hierarchy and shipment verification. Leave every other stash entry active.

## Plan Review — GATE: PASS

**Review mode:** same-model fallback using the configured Stage model; no reviewer-subagent surface was available. Always-on Constitution, Rust, Scope Boundary, Learnings, and Architecture personas were applied. Agent-native parity and Security lenses were not triggered because the plan changes no MCP/CLI contract, trust boundary, credential, or sensitive store.

**Hardening gate:** required (high runtime/rollout risk) and satisfied. The plan contains protected invariants, moderate ProposedAction records, deterministic verification, explicit stop/return-blocked thresholds, manual SLIs, rollback triggers, owner, and observation window.

### Findings

- **P0:** none.
- **P1:** none.
- **P2:** none.
- **P3-1 — manual-only telemetry:** queue generation and finisher ownership have no production counter. The structured test/log/status checklist is sufficient for this bounded release; adding a new metric would widen the task. Carry the limitation into closure.
- **P3-2 — helper naming:** the named crate-private transition seam is a design direction, not a required public symbol. Ship may adjust the private name while preserving the exact lock order, atomic visibility invariant, and caps.

### Persona decisions

- **Constitution Reviewer:** PASS — test-first split, no unsafe/error-handling exception, each card is under two hours, and Stage performs no implementation.
- **Rust Reviewer:** PASS — one private state seam, explicit lock order, no synchronous guard across await, no serialized model change, and stale-generation semantics are specified.
- **Scope Boundary Auditor:** PASS — only daemon lifecycle/pending-sync files; 015-D and unrelated stash are explicitly excluded.
- **Learnings Researcher:** PASS — the reviewed 105-F plan is the controlling prior design; no retrieved compound record contradicted it.
- **Architecture Strategist:** PASS — startup reuses R2 rather than introducing a second queue; dependency order is semantic and acyclic.

### Runtime and closure decision

Runtime verification is adequate: two deterministic generation scenarios, startup final-peek plus no-contention control, three disposable daemon restarts, zero drain-bound warnings, and the existing 30-second debug startup budget. Any wrong-generation clear/coalesce, stranded request, duplicate startup body, deterministic flake, or attributable drain-bound warning is a rollback trigger.

**Decision:** PASS. Cleared for single-width harvest.
