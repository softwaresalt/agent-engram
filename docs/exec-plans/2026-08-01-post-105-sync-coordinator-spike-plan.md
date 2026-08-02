---
title: "Bounded proof for post-105 sync coordinator"
type: impl-plan
execution_posture: spike
status: "reviewed-pass-for-spike-only"
date: 2026-08-01
source: "docs/decisions/2026-08-01-post-105-sync-coordinator-redesign-decision.md"
feature: "109-F"
task: "109.013-T"
time_box: "110 minutes"
tags: ["spike", "concurrency", "pending-sync", "compatibility", "red-proof"]
---

# Bounded proof for post-105 sync coordinator

## Problem frame

The current 109-F implementation plan failed three adversarial review cycles because it never established one authority for generation, indexing ownership, pending-mask ownership, and hydration handoff. Source evidence supports a mutex-protected `SyncCoordinator`, but two source-coupled questions remain unsafe to settle through more prose:

1. Can the currently public `AppState` pending and indexing methods map to one permit model without a second ownership channel or a public signature break?
2. Can the required races be demonstrated by tests that compile before implementation and fail deterministically without sleeps, real DB timing, or a public test-only seam?

This spike answers only those questions. It does not implement 109-F and does not authorize shipment `104-S`.

## Goal and success criteria

### Goal question

Can Option A from the redesign decision preserve current public compatibility while proving stale-generation rejection, whole-mask ownership, hydration pre-DB exclusion, and startup handoff with compiling deterministic RED tests?

### Success criteria

- Produce an exact public-method compatibility table for `is_indexing`, `try_start_indexing`, `finish_indexing`, all pending publish/set methods, and all split take methods.
- Produce a minimal coordinator state diagram and exact internal caller migration inventory.
- Demonstrate at most four co-located private RED scenarios that pass `cargo test --no-run` and then fail on the intended current-behavior assertion.
- Demonstrate that the proposed GREEN needs no second queue, producer reacquire, double drain, mutex across await, public test-only item, wire change, or persistence change.
- Record a proceed, pivot, or abandon recommendation in `docs/decisions/2026-08-01-post-105-sync-coordinator-spike-findings.md`.

## Scope constraints

- One task: `109.013-T`, at most 110 minutes.
- Temporary proof edits may touch at most two production files: `src/server/state.rs` and `src/tools/lifecycle.rs`.
- At most four deterministic scenarios.
- Co-located `#[cfg(test)]` modules may access private state. Nothing test-only becomes `pub`.
- Synchronization uses barriers, oneshot channels, or `Notify`; no sleeps, permission races, wall-clock assertions, live daemon, or operator workspace.
- Temporary proof code is not the implementation deliverable. The durable output is the findings artifact and the exact revised-plan constraints.
- No changes to CLI, MCP, schema, persistence, config, startup caller signatures, or queued response semantics.
- Stage does not execute this spike. A build-capable executor performs it under the Ship boundary without claiming or closing `104-S`.

## Investigation approach

1. Inventory every production and external-test caller of the public pending/indexing methods and classify the observable contract.
2. Write a minimal test-local coordinator reference model and compatibility adapter table; reject any mapping that creates a second owner or split mask.
3. Add the four compiling RED scenarios below in co-located private tests. First run only the compile proof, then run exact tests and capture assertion failures.
4. Map the smallest implementation slices and issue a proceed, pivot, or abandon finding. Remove or quarantine temporary proof edits according to the executor workflow; preserve evidence in the findings artifact.

## Implementation unit: 109.013-T

### Scenario S1: stale public wrapper below an advanced floor

Drive an older captured operation after a newer floor and mask are installed. Assert that the newer generation and complete mask remain unchanged and no owner is acquired. The harness must control ordering with a barrier or direct private transition, not scheduling luck.

### Scenario S2: full-mask exactly-once handoff

Queue routine, revalidation, and backfill bits while one owner is active. Assert that release transfers one complete `WorkMask` to exactly one successor and leaves no companion-only state. No sequence of three public take calls is accepted as the authoritative proof.

### Scenario S3: hydration performs zero work before ownership

Hold an owner, start hydration with a private test collaborator that signals before DB access, and assert the signal cannot fire until a permit is transferred. Cancel-before-acquire must exit without the signal. Use channels or barriers only.

### Scenario S4: startup after release has one executor

Arrange owner release and startup request on opposite sides of the coordinator linearization point. Assert either the releasing driver receives the complete pending request or startup directly acquires, never both and never neither. No producer reacquire or second drain is allowed.

### Compiling RED evidence contract

For every scenario:

1. Run the narrow target with `--no-run` and record successful compilation.
2. Run the exact test and record an assertion failure that names the violated invariant.
3. Reject missing symbols, visibility errors, compile failures, timeouts, sleeps, and flakiness as RED evidence.
4. Do not add a public hook solely for the test.

## Compatibility inventory required in findings

The findings artifact must classify every current public method as one of:

- stable adapter over the coordinator;
- stable observer with no ownership mutation;
- stable legacy claim mapped to one coordinator owner kind; or
- incompatible and requiring an explicit semver decision.

It must also list every production internal caller that must migrate to token-qualified permits. Zero internal legacy pending or split-take callers is a final implementation proof obligation.

## Dependency and backlog contract

- `109.013-T` is the only queued research item for this redesign and is not added to blocked shipment `104-S`.
- Existing implementation task `109.001-T` depends on `109.013-T`; the current dependency chain keeps every later implementation task behind it.
- Existing implementation tasks remain blocked even if the spike succeeds.
- After findings, Stage must create or revise the implementation plan, run hardening and review, and only then harvest or re-queue implementation work.

## Runtime verification and closure

The spike has no runtime rollout. It operates only in disposable test state. The durable closure is the findings artifact with:

- compile command and result for each RED;
- exact failing assertion and why it represents current behavior;
- public compatibility table;
- state diagram and caller inventory;
- recommended task boundaries;
- remaining unknowns;
- proceed, pivot, or abandon conclusion with confidence.

## Plan hardening

Hardening is required because the spike probes public synchronization contracts and can accidentally normalize an unsafe implementation shape.

Reinforcing sources: strict-safety instructions; concurrency instructions; circuit breaker instructions; the packed-mask, take-before-lock, finish-and-drain, public-visibility, and review-divergence compound learnings; and the current source contracts listed in the redesign decision.

**ProposedAction PA-1**  
Summary: create temporary co-located private RED proof in at most `state.rs` and `lifecycle.rs`.  
ActionRisk: moderate.  
Approval required: future build-capable executor follows normal Ship safeguards; Stage does not execute.  
Rollback: discard the temporary proof changes after evidence capture unless a later reviewed implementation plan explicitly retains them.  
ActionResult: planned.

**ProposedAction PA-2**  
Summary: classify public compatibility without widening public API.  
ActionRisk: moderate.  
Approval required: any semver-visible change requires a new operator decision and cannot be inferred from spike success.  
Rollback: retain existing public signatures and report pivot if no safe adapter exists.  
ActionResult: planned.

Stop and report pivot if the proof requires a public test seam, a third production file, a fifth scenario, sleeps, a second queue, a double drain, a mutex across await, or a public contract change.

## Plan review

**Review mode:** configured `.Stage` frontmatter model (`claude-opus-4.8`), no override. Cross-model dispatch was unavailable, so all required persona lenses were applied with the caller model. This verdict covers the bounded spike only.

**Gate: PASS FOR SPIKE ONLY.** Open P0: zero. Open P1: zero. Open P2: zero. Open P3: zero.

### Persona findings

- Constitution Reviewer: PASS. One task, 110 minutes, two temporary production files, and four scenarios satisfy the declared caps.
- Rust Reviewer: PASS. Co-located unit tests can inspect private state, avoiding the known external-test visibility trap. The design forbids a mutex guard across await.
- Scope Boundary Auditor: PASS. The durable output is findings; no 109 implementation or 104 shipment execution is authorized.
- Learnings Researcher: PASS. The plan applies whole-mask publish, ownership-before-consume, co-located release/handoff, private test visibility, and review circuit-breaker guidance.
- Architecture Strategist: PASS. The proof directly tests the two unknowns that prevent review of Option A and has explicit pivot conditions.
- Agent-Native Parity Reviewer: PASS. CLI and MCP contracts remain unchanged during the spike.
- Security Lens Reviewer: not triggered.

### Gate rationale

The plan is executable as a bounded investigation and cannot leak into implementation by status or shipment membership. A successful spike is necessary but not sufficient to re-queue `104-S`. The implementation architecture still requires a revised hardened plan and a fresh zero-P0/P1 PASS.
