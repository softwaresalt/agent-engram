---
title: "Post-105 pending-sync generation linearization and startup handoff"
type: impl-plan
date: 2026-07-31
updated: 2026-08-01
status: "reviewed (fresh 109-F alternate-decomposition PASS)"
source: "docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md"
source_deliberation: "018-D"
source_stash: ["FF55E51A", "88EB5FB1", "1E70A289"]
relates_to: ["105-F"]
width: "daemon lifecycle pending-sync state machine only"
tags: ["daemon", "lifecycle", "pending-sync", "concurrency", "startup"]
---

## Problem Frame

Archived feature `105-F` established a generation-tagged pending-sync queue and the R2 producer/finisher backstop. PR #302 found two residual producer windows, and the final prior Stage concurrency review proved that the lifecycle lost-lock re-arm is also generation-sensitive. The former four-task shape blocked because one GREEN needed `state.rs`, `write.rs`, and `lifecycle.rs` while its hard cap allowed at most two production files.

On 2026-08-01 the operator authorized the safer alternate decomposition rather than a cap increase. This plan preserves total feature intent but separates four responsibilities: state ownership, the `write.rs` producer, the lifecycle claim/re-arm consumer, and startup handoff. Scope excludes 015-D IPC response behavior and all Python, Spark, SQL, PowerBI, deletion, schema, CLI-response, persistence, and Cozo work. No public API, schema, model, CLI, MCP, or response semantic change is authorized.

## Provenance and Supersession

- Source deliberation: `018-D`.
- Source stash: `FF55E51A`, `88EB5FB1`, and `1E70A289`.
- Archived predecessor `105-F` remains immutable.
- The prior BLOCK and Harvest Shape repair remain historical evidence. This PASS supersedes their execution gate only because the operator authorized cap-compliant decomposition.
- Shipments `102-S` and `103-S` are outside this mutation scope and remain untouched.

## Requirements Trace

| Requirement | Implementation action | Verification |
|---|---|---|
| G1: binding, generation, cancellation, and queue floor transition coherently | `109.002-T` changes state-owned transition semantics | `109.001-T` G1a |
| G2: newer replaces, equal OR-coalesces, older is ignored | `109.002-T` provides private explicit-generation publication | `109.001-T` G1b/G1c |
| W1: binding-specific producer carries a validated token | `109.006-T` performs bounded before/after snapshot validation and explicit publication | `109.005-T` W1a/W1b/W1c |
| L1: lost-lock re-arm preserves exact generation and companions | `109.008-T` atomically claims and explicitly republishes the full request | `109.007-T` L1a/L1b/L1c |
| S1: startup has exactly one guaranteed finisher | `109.004-T` captures G before contention and uses explicit R2 handoff | `109.003-T` final-peek race/control |
| Lock safety | Finish awaited acquisitions before synchronous queue critical sections; no synchronous guard crosses `.await` | Targeted review plus deterministic fixtures |
| Contract safety | Private helpers only; public and serialized shapes frozen | Structural review |

## Task Table

Execution order is dependency-driven and alternates RED and GREEN for each responsibility.

| Order | ID | Responsibility | Production files | Test scenarios | Function/helper cap | Target |
|---:|---|---|---|---:|---|---|
| 1 | `109.001-T` | RED state transition and queue publication | none; one `state.rs` test surface | 3 | <=3 test helpers | 70-90 min |
| 2 | `109.002-T` | GREEN state-owned transition and explicit publisher | `src/server/state.rs` | 0 | <=4 production functions | 80-105 min |
| 3 | `109.005-T` | RED write producer snapshot/token | none; one `write.rs` test surface | 3 | <=3 test helpers | 60-85 min |
| 4 | `109.006-T` | GREEN write producer token adoption | `src/tools/write.rs` | 0 | <=2 production functions | 60-90 min |
| 5 | `109.007-T` | RED lifecycle generation claim/re-arm | none; one `lifecycle.rs` test surface | 3 | <=3 test helpers | 60-85 min |
| 6 | `109.008-T` | GREEN atomic claim and generation-qualified re-arm | `src/server/state.rs`, `src/tools/lifecycle.rs` | 0 | <=4 production functions | 75-105 min |
| 7 | `109.003-T` | RED startup final-peek handoff | none; one `ipc_server.rs` test surface | 2 | <=2 test helpers | 60-80 min |
| 8 | `109.004-T` | GREEN startup explicit-token backstop | `src/daemon/ipc_server.rs` | 0 | <=3 private production functions | 60-90 min |

Every task has at most two production files, at most three scenarios, and a hard stop at 110 minutes. The `state.rs` plus `lifecycle.rs` pair in `109.008-T` is the only two-file GREEN: the queue owns the opaque generation claim while the consumer must atomically claim and republish it. Splitting that private seam would leave an unused or non-buildable intermediate API, so two files are the smallest coherent width and do not widen any cap.

## Implementation Units

### Unit 1 - RED state transition and publication (`109.001-T`)

Add exactly three deterministic state-level scenarios with explicit steps and no sleeps:
1. G1a: a new binding becomes visible while old G still owns the queue; old-generation clear loses new-binding intent before the fix.
2. G1b: explicit G resumes after G+1 establishes the queue floor; pre-fix G is relabeled or coalesced, while post-fix G is ignored with no heavy leak.
3. G1c: same-G explicit requests preserve sticky OR-coalescing.

Use one `state.rs` test surface and at most three helpers. The harness must compile and fail only the target assertions. Stop for a second surface, fourth scenario, public hook, process-global mutation, daemon/IPC harness, sleep, or 100 minutes.

### Unit 2 - GREEN state ownership (`109.002-T`)

In `src/server/state.rs` only:
- make binding/config, next generation, cancellation ownership, and the pending queue generation floor one logical state transition;
- complete async lock acquisitions and capacity validation before any synchronous pending queue critical section;
- provide private explicit-generation publish/reacquire behavior where newer replaces, equal OR-coalesces the complete mask, and older is ignored; and
- correct proof comments so SeqCst is not presented as a mutex happens-before proof.

G1a/G1b/G1c must pass and capacity failure must publish nothing. Maximum four production functions. Stop for a second production file, fifth function, public contract, unresolved lock order, guard across `.await`, second queue/state machine, unsafe code, or 110 minutes.

### Unit 3 - RED write producer (`109.005-T`)

At the real queued-sync seam in `write.rs`, add exactly three deterministic scenarios:
1. W1a: one mismatch around `snapshot_graph_handler_context` followed by a stable retry yields one accepted G.
2. W1b: repeated mismatch exhausts a fixed budget before indexing lock acquisition or publication and returns an existing internal error.
3. W1c: accepted G pauses before failed-lock publication, G+1 owns the queue, and resumed G is not relabeled and leaks no heavy bit.

Use one test surface and at most three helpers. No daemon, IPC timing, public seam, or sleep. Stop if the harness cannot compile while failing only target assertions or at 100 minutes.

### Unit 4 - GREEN write producer (`109.006-T`)

In `src/tools/write.rs` only, read G, await `snapshot_graph_handler_context`, then read G again. Accept a match, retry mismatch within a fixed private budget, and fail closed before `try_start_indexing` or publication on exhaustion. Carry accepted G through contention to the explicit-generation publish/reacquire path and never reread current G to relabel. Preserve queued response and gate flags. Maximum two production functions. Stop for a second production file, unbounded retry, later-generation fallback, lock before certification, contract change, or 110 minutes.

### Unit 5 - RED lifecycle claim/re-arm (`109.007-T`)

Add exactly three deterministic scenarios to the private `lifecycle.rs` test surface using a private or `cfg(test)` pause seam:
1. L1a: a bare G request is consumed, G+1 advances the queue floor, then lost-lock re-arm resumes; current unqualified re-arm relabels old intent, while the fix ignores G.
2. L1b: a G request with heavy companions is claimed, G+1 publishes a routine owner, then old re-arm resumes; no G companion may survive or leak into G+1.
3. L1c: same-G lost-lock re-arm republishes pending plus companions and remains drainable.

One surface, at most three helpers, no sleep, real daemon, or public seam. Stop for a second test file, fourth scenario, or 100 minutes.

### Unit 6 - GREEN lifecycle claim/re-arm (`109.008-T`)

In `src/server/state.rs` and `src/tools/lifecycle.rs` only:
- atomically claim the complete pending request as an opaque private token containing generation and pending/revalidate/backfill bits, leaving no heavy companion behind;
- on lost indexing lock, republish exactly that token through explicit-generation semantics: older ignored, equal full-mask coalesced, and newer ownership never overwritten;
- on acquired lock, drain claimed companions exactly once; and
- preserve the bounded 64-pass drain and every existing `finish_indexing` drain call site.

Maximum four production functions across both files. No synchronous guard crosses `.await`. Stop for a third production file, fifth function, public token, recursion, unbounded drain, second queue, or 110 minutes.

### Unit 7 - RED startup handoff (`109.003-T`)

In one private `ipc_server.rs` test surface, add one final-peek race and one no-contention control. Startup captures G before the initial lock attempt, loses the attempt, pauses publication, lets the holder release and finish its final empty peek, then resumes with the same G. The race fails before the fix and passes with one guaranteed finisher. Stop for a third scenario, second test surface, real daemon, IPC timing, sleep, public hook, or 100 minutes.

### Unit 8 - GREEN startup handoff (`109.004-T`)

In `src/daemon/ipc_server.rs` only, capture G before initial `try_start_indexing`. On failure, pass exact G to explicit-generation publish/reacquire. Distinguish initial owner, queued-under-holder, and producer-reacquired outcomes privately. A reacquirer releases and drains exactly once, then returns without running the normal startup body twice. Preserve retry/backoff, registry ingestion, embedding backfill, flush, bounded drain, and responses. Stop for a second production file, fourth function, post-failure capture, unqualified publisher, recursion/double-drain, guard across `.await`, or 110 minutes.

## Production Publication Inventory

| Site | Disposition |
|---|---|
| `src/tools/write.rs::sync_workspace` failed-lock branch | `109.005-T`/`109.006-T`: validate snapshot G, then explicit publish/reacquire. |
| `src/tools/lifecycle.rs::drain_pending_sync` lost-lock branch | `109.007-T`/`109.008-T`: full-mask generation-owned claim and exact republish. |
| `src/daemon/ipc_server.rs::try_start_startup_sync` failed-lock branch | `109.003-T`/`109.004-T`: pre-attempt G and explicit R2 handoff. |
| Unqualified state helpers with no remaining production caller | Deferred cleanup; do not widen this release unit. |

Consumers such as `clear_pending_sync_for_generation` and `has_pending_sync` are not extra producers. No lifecycle site is classified as generation-neutral.

## Dependency Graph

```text
109.001-T RED state
  -> 109.002-T GREEN state
     -> 109.005-T RED write producer
        -> 109.006-T GREEN write producer
           -> 109.007-T RED lifecycle claim/re-arm
              -> 109.008-T GREEN lifecycle claim/re-arm
                 -> 109.003-T RED startup
                    -> 109.004-T GREEN startup
```

The explicit graph is authoritative even if the shipment manifest stores appended members in another order. Shipments `102-S` and `103-S` precede `104-S` by operator order only, not technical dependency.

## Verification Plan

Stage validates plan/backlog structure only. Ship records compiling-but-failing RED evidence before each GREEN, then runs:
1. deterministic G1a/G1b/G1c;
2. deterministic W1a/W1b/W1c;
3. deterministic L1a/L1b/L1c;
4. startup final-peek race and no-contention control;
5. publication and every-`finish_indexing` drain-site inventory;
6. `cargo fmt --all -- --check`;
7. `cargo clippy --all-targets -- -D warnings -D clippy::pedantic`;
8. `cargo dev-test`; and
9. `cargo audit`.

Any flaky timing harness, cap breach, unclassified publication site, public seam, second queue, unbounded retry/drain, synchronous guard across `.await`, or task over 110 minutes returns the affected task and `104-S` blocked.

## Risks, Rollback, and Monitoring

### Risks and mitigations

- Torn binding/snapshot: state owns the queue floor; write validates around the snapshot.
- Stale relabel or heavy leak: explicit tokens only; older ignored, equal full-mask coalesced, newer replaces.
- Claim/re-arm tear: atomically move the complete mask into an opaque claim and republish the same claim.
- Deadlock: finish async acquisitions first; no synchronous guard crosses `.await`.
- Duplicate startup body/drain: explicit outcome plus exactly-one-finisher assertion.
- Scope creep: per-card file/function/scenario caps and hard 110-minute stops.

### Rollback

Rollback is release-unit commit revert plus daemon restart. No migration reversal, cache repair, full reindex, or user-workspace mutation is expected. Do not partially roll back one state/write/lifecycle unit while leaving dependent units active.

### Monitoring

| SLI | Healthy | Block/rollback trigger | Owner/window |
|---|---|---|---|
| State generation fixtures | G1a/G1b/G1c deterministic pass | drop, relabel, heavy leak, same-G regression | Ship; targeted gate |
| Producer snapshot fixtures | bounded retry; no publish on exhaustion | later-G fallback or lock/publish after exhaustion | Ship; targeted gate |
| Lifecycle claim/re-arm | L1a/L1b/L1c pass with no stale companions | old intent runs as G+1, heavy leak, or same-G loss | Ship; targeted gate |
| Startup finisher | exactly one finisher and no duplicate body | stranded request or duplicate body | Ship; targeted gate |
| Runtime drain | zero attributable bound warnings; within existing 30-second debug budget | warning or controlled restart over budget | Ship; three restarts plus 15 min |
| Publication inventory | every site matches this plan | any unclassified producer/re-arm | Ship; pre-merge |

## Plan Hardening

Hardening is required because a partial concurrency fix can silently drop, relabel, or over-upgrade work. Consulted strict-safety and concurrency instructions plus:
- `docs/compound/best-practices/packed-atomic-clear-requires-atomic-publish-2026-07-29.md`;
- `docs/compound/concurrency-issues/pending-sync-drain-must-cover-all-finish-indexing-sites-2026-05-09.md`; and
- `docs/compound/concurrency-issues/atomicbool-drain-race-take-before-lock-2026-05-09.md`.

Protected invariants are coherent generation floor ownership, explicit producer tokens, atomic full-mask claim/republish, complete drain coverage, exactly one startup finisher, bounded retry/drain, no guard across `.await`, and frozen contracts.

**ProposedAction PA-1A**
- summary: state-owned binding/generation floor and explicit publisher
- targets: `src/server/state.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert release-unit commit and restart daemon
- approval_required: no additional Stage approval; operator authorized this decomposition
- ActionResult: approved, not executed

**ProposedAction PA-1B**
- summary: validated write producer token adoption
- targets: `src/tools/write.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert release-unit commit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-1C**
- summary: generation-owned lifecycle claim and lost-lock re-arm
- targets: `src/server/state.rs`, `src/tools/lifecycle.rs`
- change_kind: shared runtime coordination edit
- ActionRisk: moderate
- rollback: revert release-unit commit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

**ProposedAction PA-2**
- summary: pre-lock startup generation capture and explicit-token R2 handoff
- targets: `src/daemon/ipc_server.rs`
- change_kind: daemon startup coordination edit
- ActionRisk: moderate
- rollback: revert release-unit commit and restart daemon
- approval_required: no additional Stage approval
- ActionResult: approved, not executed

## Runtime Verification and Closure

Stage performed no implementation or runtime verification. Ship must execute the Verification Plan, retain monitoring/rollback triggers through operational closure, and return blocked on any stop condition.

## Harvest Shape

Existing shipment `104-S` remains the only release unit and existing feature `109-F` remains the covering feature. No replacement feature or shipment is created.

- Existing tasks: `109.001-T`, `109.002-T`, `109.003-T`, `109.004-T`.
- New tasks: `109.005-T`, `109.006-T`, `109.007-T`, `109.008-T`.
- Order: `109.001-T` -> `109.002-T` -> `109.005-T` -> `109.006-T` -> `109.007-T` -> `109.008-T` -> `109.003-T` -> `109.004-T`.

The shipment manifest may append new members after existing ones; dependency order is authoritative. Parent feature is already in `104-S`. Keep `operator_order = 3` and do not mutate `102-S` or `103-S`.

## Plan Review - Fresh 109-F Alternate-Decomposition Cycle: PASS

**Review mode:** complete Stage plan review under the configured `.Stage` frontmatter model. No model override or cross-model dispatch occurred; persona lenses ran under the caller model.

**Gate decision: PASS.** Plan hardening is required and satisfied. No P0, P1, P2, or P3 finding remains. The real three-file cap conflict is resolved by coherent state, write, lifecycle, and startup widths with explicit RED-before-GREEN pairs. No task exceeds two production files, three scenarios, four production functions, 105 planned minutes, or the 110-minute hard stop.

### Findings

- P0: none.
- P1 fixed - production-file cap conflict: replaced the three-file GREEN with one-file state/write tasks and a cohesive two-file state+lifecycle claim/re-arm task.
- P1 fixed - missing lifecycle RED: added `109.007-T` before `109.008-T`.
- P1 fixed - inconsistent caps: task table, units, graph, stop triggers, verification, and Harvest Shape now agree.
- P2/P3: none.

### Persona decisions

- Constitution Reviewer: PASS. Test-first pairs, one responsibility per card, traceability, and under-two-hour stops are explicit.
- Rust/Concurrency Reviewer: PASS. Queue floor, opaque full-mask claim, explicit republish, and no-guard-across-await constraints directly cover the identified race without a second queue.
- Scope Boundary Auditor: PASS. Only `109.008-T` has two production files, justified as the smallest buildable private seam; all other GREEN tasks are one file.
- Learnings Researcher: PASS. Atomic full-mask publish, explicit re-arm after take-before-lock, and all-finish-site drain coverage are preserved.
- Architecture Strategist: PASS. The graph establishes primitives before producer and consumer migration, then startup handoff.
- Agent-Native Parity Reviewer: PASS. Queued response, CLI/MCP fields, and user/agent behavior remain frozen.
- Security Lens Reviewer: not triggered; no auth, credential, sensitive-data, or external trust-boundary change.

**Decision:** PASS. Harvest the four new tasks, wire the full dependency chain, add them to `104-S`, then return `104-S`, `109-F`, and all eight tasks to queued.
