---
title: "Compose lifecycle bind values from one retained workspace proof"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed-ready
source: docs/decisions/2026-08-24-workspace-authority-followups-deliberation.md
source_stash_id: "1CB366DB"
backlog_deliberation: "021-D"
depends_on_stash_id: "7B15B447"
---

# Compose lifecycle bind values from one retained workspace proof

## Problem Frame

`set_workspace_with_probe` independently calls `canonicalize_workspace`, `load_or_create_workspace_id`, and `resolve_git_branch`. Each call resolves and drops a `GitMetadata`/`CapRoot`, so canonical path, UUID, and branch can describe different objects after an ancestor substitution.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| One proof for canonical path, UUID, branch | U2 adds a crate-private combined result derived from one `GitMetadata`. |
| Main bind consumes the combined result | U3 replaces the three lifecycle calls atomically. |
| Deterministic regression | U1 swaps between old value derivations and verifies no mixed tuple or attacker write. |
| Width isolation | Workspace API and lifecycle integration are separate GREEN units. |

## Implementation Units

### U1 — RED: mixed bind tuple

In colocated `src/tools/lifecycle.rs` tests, add one deterministic probe at bind-value composition. Replace the workspace ancestor after canonical-path derivation and before UUID/branch derivation. Assert the probe fires and bind either rejects or returns canonical path, UUID, and branch from one fixture; attacker `.workspace-id` must not be created. Current code must fail with an expected mixed tuple/write. One file, one scenario, target 100 minutes.

### U2 — GREEN: crate-private combined proof result

In `src/db/workspace.rs`, introduce a crate-private value (canonical path, UUID, branch) and resolver that calls `resolve_git_metadata` once, derives branch from validated `head_content`, and consumes that same metadata/root for identity load or creation. Preserve the existing `default` branch fallback and keep `CapRoot` private. One file, fewer than five functions, target 100 minutes.

### U3 — GREEN: lifecycle bind consumption

In `src/tools/lifecycle.rs`, replace the three independent calls with U2. Keep downstream `workspace_hash`, hydration, config, metrics, and ambiguity behavior unchanged. U1 turns green. One production file plus colocated test, target 90 minutes.

### U4 — Verification and closure

Run targeted RED/GREEN evidence, lifecycle contract/integration coverage, ordinary checkout/worktree binds, unchanged-workspace rebind, Windows/Linux CI, and adversarial review. Record key/path/branch coherence and rollback. Verification only, target 90 minutes.

## Dependency Graph

`7B15B447 -> U1 -> U2 -> U3 -> U4`. The prerequisite ensures the combined bind proof does not call an identity routine that still reopens `.engram`.

## Decisions and Rationale

- A new crate-private operation is safer than changing public wrappers or passing capability types into tools.
- Branch is derived before `GitMetadata` is consumed for identity persistence.
- Only the all-three-values lifecycle bind is migrated; canonical-plus-branch callers without UUID are out of scope unless impact review proves the same exploit.
- No security claim relies on timing.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Public API churn | Crate-private result and function only. |
| Changed default-branch semantics | Characterize and preserve the current fallback. |
| Test writes attacker identity | Assert no external write and confine fixtures to temp roots. |
| Scope expands to unrelated CLI callers | Freeze to multi-value bind composition. |

## Plan Hardening Signals

- Public API/schema/contract: absent; crate-private composition contract changes.
- Security-sensitive behavior: present; workspace bind trust boundary.
- Migration/destructive action: absent.
- External integration/checkpoint: absent.
- High runtime/rollback risk: present; every MCP bind uses this path.

Requires plan hardening: yes

## Runtime Verification and Closure

Verify one primary checkout and one native worktree on Windows/Linux; compare canonical path, UUID, branch, workspace hash, and daemon key across repeated binds. Observation window: 48 hours owned by Ship. Roll back U2/U3 together. Triggers: legitimate bind rejection, UUID/key change without workspace change, branch mismatch, attacker fixture write, or latency above the 121-S absolute budget.

## Plan Hardening

Protected invariants: one `GitMetadata` proof per bind tuple; no capability leak; no second resolver call; no attacker read/write; existing public wrapper behavior preserved.

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Add combined trust-boundary resolver | `src/db/workspace.rs` | high | revert U2 | preferred | planned |
| Replace lifecycle value composition | `src/tools/lifecycle.rs` | high | revert U3 with U2 | preferred | planned |

Hard gate: 7B15B447 shipped first; standard and adversarial multi-model review both clear before harvest.

## Plan Review

Gate: **PASS (standard review only)**. Hardening required and present. Personas applied: constitution, Rust/API, architecture, scope, tests, security, operational readiness, and learnings.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| S1 | P1 | A combined API that calls existing public wrappers internally would still re-resolve. | Resolved: U2 starts with one private `resolve_git_metadata` and never re-enters wrappers. |
| S2 | P1 | The identity helper can still reopen `.engram`. | Resolved by hard dependency on 7B15B447. |
| T1 | P1 | The RED must detect external writes, not only tuple mismatch. | Resolved in U1. |
| A1 | P2 | Migrating every canonical/branch caller would widen scope. | Resolved by explicit all-three-values boundary. |

No unresolved standard-review P0/P1 finding remains. Review-fix cycles: 1 of 3.

## Adversarial Multi-Model Review — Cycle 5 Final

Gate: **PASS**. The valid three-model four-plan review returned no consensus finding for this plan. The final bounded rerun reconfirmed its unchanged pass and hard prerequisite. No HIGH, MEDIUM, P0, or P1 finding remains.

Execution remains blocked by dependency—not review—until the complete `7B15B447` release unit, including verification/closure, finishes. Review-fix cycles: 0 of 3.

Evidence: `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-rerun.md` and `docs/closure/2026-08-24-dark-factory-cycle5-four-plan-adversarial-review-final.md`.
