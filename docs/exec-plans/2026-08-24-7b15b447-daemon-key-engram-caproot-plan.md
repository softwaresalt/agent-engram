---
title: "Retain one .engram capability through daemon-key selection"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: blocked
adversarial_review: failed-unverified
standard_review: failed-blocked-cold-start-primitive
source: docs/decisions/2026-08-24-workspace-authority-followups-deliberation.md
source_stash_id: "7B15B447"
backlog_deliberation: "022-D"
---

# Retain one `.engram` capability through daemon-key selection

## Problem Frame

`daemon_key_for_workspace` can mix `.engram` directory objects because helpers reopen the child between presence, UUID, PID, and publication operations. The prior blocked plan also said cold start could create `.engram` and then open it by name. That create/open pair itself leaves a substitution window before the first retained child capability exists.

This security plan remains blocked. It may not claim a safe cold-start protocol until a pinned, safe API or separately reviewed platform protocol can return or proof-preservingly retain the exact directory object created. Plain create-then-open, ambient paths, reopen-by-name, and post-hoc checking of an attacker-substitutable object are prohibited.

## Protected Authority Model

Carry one private `EngramAuthority` state machine through the entire decision:

* `Existing(CapRoot)`: one no-follow open occurs before any presence/UUID/PID probe; that exact child capability is retained through publication/read-back and the final key decision.
* `Vacant(VacantEngramSlot)`: the retained workspace-root capability and verified absent child slot remain owned by the same state machine. It may transition only through a safe create-and-retain primitive/protocol that returns `Created(CapRoot)` for the exact object created.
* `Created(CapRoot)`: the exact first-created child capability remains live through UUID/PID reads, staging, publication, winner read, and the final decision.

There is no transition that drops the child and reopens `.engram` by name. The vacancy/existence decision and created capability cannot be represented as unrelated booleans or paths.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| One authority through existing-child decision | U1 tests UUID and live-PID substitution after the initial child open. |
| One authority through cold-start existence/create/open | U2 tests substitution exactly after creation and before any possible named open. |
| Exact created object retained | U3 requires an API/protocol that returns or proves and retains the exact created object; create-then-open is forbidden. |
| UUID/PID/publication bound to same object | U3 passes `&CapRoot`/owned authority through all helpers and decision branches. |
| No ambient or reopen escape | U3/U4 audit every child interaction and reject path-derived helpers. |
| Fail closed while primitive is unproven | Feature 132-F, review 132.001-R, shipment 126-S, and all tasks stay blocked. |

## Implementation Units

### U1 / 132.001-T — RED: existing-child UUID and PID substitution

In one colocated test module, add two deterministic checkpoints after the one existing-child open: persisted UUID read and absent-ID/live-PID selection. Rename/replace the directory at each checkpoint. Assert attacker UUID/PID state is never consumed or written and each checkpoint must fire. Current code fails because helpers reopen by name. One file, two scenarios, target 105 minutes.

### U2 / 132.002-T — RED: cold-start first-create/open substitution

Add one deterministic cold-start checkpoint immediately after `.engram` creation and before any first named open or publication. Replace the directory at that checkpoint. The test passes only if code continues through a capability retained for the exact created object or fails closed before any UUID/PID read, staging write, publication, or key decision. Reading/writing the replacement is failure. A test that cannot fire this exact checkpoint is invalid. One file, one scenario, target 90 minutes.

### U3 / 132.003-T — GREEN: authority state machine and safe create-and-retain

Refactor private helpers so `daemon_key_for_workspace` owns one `EngramAuthority` from existence decision through return. Existing-child code opens no-follow exactly once. Cold start must use a safe pinned primitive/protocol that returns or proof-preservingly retains the exact newly created directory object in the same transition from `Vacant` to `Created`.

**Blocking prerequisite:** no such portable pinned primitive is proven in this plan. `create_dir` followed by `open_dir`, ambient canonicalization, reopen-by-name, retry-until-stable, or checking metadata only after opening a potentially substituted directory is not acceptable. If the primitive/protocol cannot be demonstrated on Windows and Unix without unsafe or ambient authority, U3 remains blocked and no implementation begins.

Once that prerequisite is met, presence, UUID read, PID read, staging creation, no-clobber publication, winner read, and final key decision all consume the same authority object/child capability. No helper may accept a workspace path and reopen `.engram`. One production file, fewer than five functions, target 115 minutes.

### U4 / 132.004-T — Verification and closure

Run both deterministic existing-child checkpoints and the cold-start post-create checkpoint, workspace identity unit/integration coverage, Windows and Linux CI, daemon-key restart stability, and one ordinary cold start. Review every `.engram` interaction and prove no ambient/reopen path. Record the exact safe primitive/protocol and pinned source evidence, rollback trigger, platform caveats, observation query, and baseline. Verification only, target 90 minutes.

## Dependency Graph

`U1 + U2 -> U3 -> U4`. Four tasks, exactly three prerequisite edges inside the fan-in chain: `U1 -> U3`, `U2 -> U3`, and `U3 -> U4`. `1CB366DB`/133-F remains dependent on terminal completion of this release unit.

## Decisions and Rationale

- Retain authority rather than revalidate: revalidation creates another check/use window.
- Treat cold-start vacancy as owned state, not a boolean followed by a named open.
- Require the exact created object, not merely whichever object occupies `.engram` when later opened.
- Preserve legacy live-PID compatibility only when its evidence comes from the retained child.
- Fail closed while a safe portable create-and-retain transition is unproven.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Cold-start protocol silently substitutes the attacker object | Named post-create/pre-open checkpoint; exact created-object capability or fail closed. |
| Helper reopens child later | Authority-typed private APIs and complete interaction audit. |
| PID fixture becomes timing-based | Current-process liveness plus deterministic checkpoints; no sleep. |
| Platform API differs | Require pinned Windows and Unix evidence before U3 can unblock. |
| Public capability leakage | Private state machine/helpers; public signatures unchanged. |

## Runtime Verification and Closure

Verify unchanged UUID/key across restart, primary checkout/worktree admission, retained legacy PID fallback, and cold-start publication. Observation owner: Ship; window: 48 hours after any future valid implementation. Immediate rollback triggers: legitimate `NotGitRoot`, key change for unchanged workspace, fallback failure against a confirmed live daemon, attacker object read/write, or any checkpoint test passing without firing. No migration is required; rollback reverts U3.

## Plan Hardening — Exact-Head Rerun

Hardening remains **required but blocked** because this changes the daemon IPC key trust boundary.

| ProposedAction | Targets | ActionRisk | Rollback | Approval required | ActionResult |
|---|---|---|---|---|---|
| Introduce private `EngramAuthority` state machine | `src/db/workspace.rs` | high | Revert U3 | preferred | blocked |
| Use a safe exact-create-and-retain primitive/protocol | Pinned platform/capability APIs | high | No implementation until proven | required before execution | blocked |
| Exercise deterministic substitution fixtures | Test temp directories | moderate | Remove fixture changes | no | planned |

Protected invariant: from the first existing-child open or vacancy decision through create/acquire, UUID/PID reads, publication, winner read, and final decision, one authority state is retained; no ambient/reopen substitution window exists. The earlier create-then-open instruction is withdrawn.

## Standard Plan Review — Exact-Head Rerun

Gate: **FAIL / BLOCKED**. Local constitution, Rust/API, architecture, scope, test, security, operations, and learnings lenses were rerun. Intercom/cross-model dispatch remains unavailable.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| COLD-1 | P0 | Create-by-name then open-by-name cannot preserve the exact first-created directory object across the intervening substitution window. | Previous instruction withdrawn; U2 now proves the gap. |
| API-1 | P1 | No pinned safe portable create-and-retain primitive/protocol is demonstrated. | Open blocker on U3; implementation/harvest remains forbidden. |
| TEST-1 | P1 | Existing tests did not checkpoint immediately after create and before first open. | U2 now requires the exact deterministic checkpoint. |
| SCOPE-1 | P2 | Platform-specific proof may exceed one Rust file. | Resolve in a new spike/review before changing this blocked unit; do not hide dependency/API work in U3. |
| EDGE-1 | P2 | Review `5015710467` found that the fan-in chain was mislabeled as four prerequisite edges. | Corrected to the exact three-edge backlog graph; blocked status and task dependencies are unchanged. |

The standard gate cannot pass until `API-1` is resolved with pinned source evidence and the plan is reviewed again. The adversarial multi-model gate also remains failed/unverified. Status stays `blocked`; this plan is not harvest authorization.

## References

- PR 363 reviews `5015447062` and `5015710467`; thread `PRRT_kwDORJEduc6b8_I0`; suppressed `EDGE-1` finding
- Stash `7B15B447`; blocked feature `132-F`; blocked review `132.001-R`; blocked shipment `126-S`; replacement stash `172AE8CE`
- `src/db/workspace.rs`
- `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-final.md`
