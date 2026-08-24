---
title: "Retain one .engram capability through daemon-key selection"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: blocked-review
source: docs/decisions/2026-08-24-workspace-authority-followups-deliberation.md
source_stash_id: "7B15B447"
backlog_deliberation: "022-D"
---

# Retain one `.engram` capability through daemon-key selection

## Problem Frame

`daemon_key_for_workspace` retains one workspace-root proof but opens `.engram` separately in `workspace_id_present_via`, `workspace_id_from_metadata`, and `read_pid_file_via`. A rename/substitution between those calls can mix the probed directory, UUID source, live-PID source, and publish destination.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| One `.engram` authority for the decision | U3 opens or creates `.engram` once and passes the retained `CapRoot` through every branch. |
| Cover persisted UUID path | U1 RED swaps `.engram` after the presence decision and before UUID read. |
| Cover legacy live-PID path | U2 RED swaps `.engram` before PID selection and proves attacker PID cannot force fallback. |
| Cover cold-start publish | U3 reuses the same retained child root for read/create/publish. |
| Preserve public API | All new helpers remain private to `src/db/workspace.rs`. |

## Implementation Units

### U1 — RED: persisted-identity child substitution

Add one colocated deterministic hook-driven test in `src/db/workspace.rs`. The hook must rename/replace `.engram` after the presence probe but before the UUID read. Assert the checkpoint fires, the attacker UUID is never returned, and attacker state is not written. On current code the test must fail for the expected mixed-directory result. One file, one scenario, target 90 minutes.

### U2 — RED: legacy-PID child substitution

Add one colocated deterministic test in `src/db/workspace.rs` for the absent-ID/live-PID branch. The original `.engram/run/engram.pid` is absent; the substituted directory names a known live process. Assert the checkpoint fires and the substitution cannot force the legacy path-hash key. Current code must fail for the expected fallback selection. One file, one scenario, target 90 minutes.

### U3 — GREEN: thread the retained child capability

Refactor private helpers in `src/db/workspace.rs` so `daemon_key_for_workspace` obtains `Option<CapRoot>` for `.engram` once. Existing directories are opened no-follow once; cold start creates through the retained workspace root and then opens once. UUID probe/read, PID read, and identity publish receive that same retained child root. Remove internal helper shapes that reopen `.engram`. U1 and U2 turn green without changing public signatures. One file, fewer than five functions, target 110 minutes.

### U4 — Verification and closure

Run targeted RED/GREEN evidence, the workspace identity unit/integration set, Windows and Linux CI, daemon-key stability across restart, and one ordinary cold start. Record the retained-child invariant, latency, rollback trigger, and any platform caveat in closure. Verification only, target 90 minutes.

## Dependency Graph

`U1 + U2 -> U3 -> U4`. `1CB366DB` depends on completion of this release unit.

## Decisions and Rationale

- Retain one child handle rather than revalidate: revalidation leaves a new check/use window.
- Keep this separate from lifecycle bind composition: both edit different composition surfaces and need independent security evidence.
- Preserve the legacy-live-PID compatibility branch; only bind its evidence to the retained child.
- A test that does not fire its named checkpoint fails.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Cold start creates `.engram` too early and changes legacy behavior | Preserve the existing absent/present decision and test both branches. |
| Ownership refactor accidentally reopens the child | Review every `.engram` open in `daemon_key_for_workspace`; exactly one retained child per branch. |
| PID fixture becomes timing-based | Use current-process liveness and deterministic checkpointing; no sleeps. |
| Public capability leakage | Private helpers only; public signatures unchanged. |

## Plan Hardening Signals

- Public API/schema/contract: absent; private ownership contract changes.
- Security-sensitive behavior: present; daemon IPC key trust boundary.
- Migration/destructive action: absent.
- External integration/checkpoint: absent.
- High runtime/rollback risk: present; daemon discovery can become unavailable.

Requires plan hardening: yes

## Runtime Verification and Closure

Verify unchanged workspace UUID and daemon key across restart, ordinary primary checkout/worktree admission, legacy live-daemon fallback, and cold-start publication. Roll back by reverting U3; no data migration is required. Observation owner: Ship; window: 48 hours. Immediate rollback triggers: legitimate `NotGitRoot`, key change for unchanged workspace, fallback failure against a confirmed live legacy daemon, or any checkpoint test passing without firing.

## Plan Hardening

Protected invariants: one retained workspace root, one retained `.engram` child per decision, no path reopen, no attacker read/write, stable public behavior.

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Change daemon-key child-capability ownership | `src/db/workspace.rs` | high | revert U3 | preferred | planned |
| Exercise deterministic directory substitution fixtures | test temp directories | moderate | remove fixture changes | no | planned |

Reinforced gate: standard review plus operator-requested adversarial multi-model review before harvest. No shipment is permitted while that review is unavailable.

## Plan Review

Gate: **PASS (standard review only)**. Hardening required and present. Personas applied: constitution, Rust/API, architecture, test strategy, security, operational readiness, and learnings.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| S1 | P1 | A single child opened only for the UUID branch would leave the PID branch mixed. | Resolved: U2/U3 require the same child for probe, UUID, PID, and publish. |
| T1 | P1 | A timing race would not prove the defect. | Resolved: named deterministic hooks; not-fired is failure. |
| A1 | P1 | Creating `.engram` before checking legacy state could alter compatibility. | Resolved: preserve absent/present decision and test both existing and cold-start branches. |
| O1 | P2 | Rollback and observation were underspecified. | Resolved in hardening. |

No unresolved standard-review P0/P1 finding remains. Review-fix cycles: 1 of 3.

## Adversarial Multi-Model Review

Gate: **BLOCKED**. This plan is security-sensitive and the operator requires genuine multi-model review. The current tool surface exposes no independent reviewer/subagent dispatch and no cross-model execution. Single-model persona simulation is not represented as multi-model consensus. Harvest, feature creation, and shipment assembly are prohibited until at least three independent cross-model reviewers return and HIGH-confidence P0/P1 findings are cleared.
