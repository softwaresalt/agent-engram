---
title: "Reduced Stage integration for 102-S and 103-S"
type: integration-plan
status: "reviewed-pass"
date: 2026-08-01
source_branch: "107-stage-102-104-integration"
source_head_examined: "538e0ab95ce1ad2ecb77925950f89e63d6d74f58"
remote_head_examined: "6402b7b915f283cde334d0e804096ef9277f4add"
base: "origin/main"
shipments: ["102-S", "103-S"]
excluded_shipment: "104-S"
---

## Purpose

Create a clean Stage integration pull request from current `origin/main` containing only reviewed queued release units `102-S` and `103-S`, their durable Stage provenance, and the three explicitly approved stowaways. Preserve the existing `107-stage-102-104-integration` branch and all 109-F work. Do not rewrite history, delete the branch, claim a shipment, or infer that 104-S is ready.

This plan is a path manifest and review gate for Ship. Stage does not create the branch, commit, push, or pull request.

## Scope decision

- Include `102-S` and `103-S` only.
- Preserve their existing feature and task statuses as queued.
- Preserve 103-S ordering after 102-S. A future claim of 103-S still requires positive terminal evidence for exact predecessor `102-S`.
- Exclude 104-S and all 109-F planning, backlog, review, checkpoint, memory, and consumed-stash state.
- Exclude every unrelated review and checkpoint artifact from this immediate pull request. The 107 and 108 plan files already contain their accepted inline plan reviews; this reduced integration plan provides the package-level PASS.
- Include only the approved config, orchestrator-agent, and atomic 087-F archive stowaways.

## Exact include manifest

Apply these paths from the preserved Stage worktree to a new branch created from current `origin/main`. No other path is authorized.

### Reviewed release unit 102-S

1. `.backlogit/queue/102-S.md`
2. `.backlogit/queue/107-F.md`
3. `.backlogit/queue/107.001-T.md`
4. `.backlogit/queue/107.002-T.md`
5. `docs/decisions/2026-07-31-python-qualified-staging-caller-attribution-decision.md`
6. `docs/exec-plans/2026-07-31-python-qualified-staging-caller-attribution-plan.md`

### Reviewed release unit 103-S

7. `.backlogit/queue/103-S.md`
8. `.backlogit/queue/108-F.md`
9. `.backlogit/queue/108.001-T.md`
10. `.backlogit/queue/108.002-T.md`
11. `docs/decisions/2026-07-31-ordinary-index-fail-closed-followups-decision.md`
12. `docs/exec-plans/2026-07-31-ordinary-index-fail-closed-followups-plan.md`

### Reduced integration provenance

13. `docs/exec-plans/2026-08-01-102-103-reduced-stage-integration-plan.md`

### Explicitly approved stowaways

14. `.autoharness/config.yaml`
15. `.github/agents/orchestrator.agent.md`
16. Atomic 087-F move: delete `.backlogit/queue/087-F.md` and add `.backlogit/archive/087-F.md` as one rename-equivalent change.

## Exact exclude manifest

The following paths and classes must not enter the immediate pull request, even if they exist on the preserved branch or local worktree.

### 104-S and 109-F

- `.backlogit/queue/104-S.md`
- `.backlogit/queue/109-F.md`
- `.backlogit/queue/109.001-T.md` through `.backlogit/queue/109.013-T.md`
- `.backlogit/archive/109.001-R-plan-review-post-105-pending-sync-generation-and-startup-han.md`
- `.backlogit/archive/018-D.md`
- `docs/decisions/2026-07-31-post-105-pending-sync-residuals-deliberation.md`
- `docs/decisions/2026-08-01-post-105-sync-coordinator-redesign-decision.md`
- `docs/exec-plans/2026-07-31-post-105-pending-sync-residuals-plan.md`
- `docs/exec-plans/2026-08-01-post-105-sync-coordinator-spike-plan.md`

### Consumed stash and tool state

- `.backlogit/stash.jsonl`
- `.backlogit/archive/stash.jsonl`
- every `.backlogit/checkpoints/*.json` change
- `.backlogit/memories.json`
- every `.backlogit/archive/*-R*.md` review artifact, including 107.001-R and 108.001-R

### Session and review-cycle memory

- every new or modified `docs/memory/**` path from the preserved branch or this recovery session
- specifically all 109-F review-cycle, circuit-breaker, harvest-shape, and wrapper-floor memory files

### Unapproved agent or unrelated scope

- `.github/agents/ship.agent.md`
- all source, tests, schemas, build files, generated output, and any path not named in the include manifest
- blocked shipments `025-S` and `081-S`, unrelated stash entries, and queued deliberations

## Ship construction constraints

1. Start from current `origin/main`; do not branch from local Stage HEAD.
2. Preserve `107-stage-102-104-integration` and its commits. Do not reset, rebase, force-push, or delete it.
3. Apply only the include manifest by path. Do not cherry-pick the Stage commit range because those commits mix 109 and review/checkpoint state.
4. Treat the 087-F queue deletion and archive addition as one atomic move. If either side is absent, stop.
5. Before commit, compare the candidate diff path set to the include manifest. Any extra path is a gate failure.
6. Confirm no path or content reference from the excluded 109 decision, plan, task, review, checkpoint, memory, or stash mutations entered the candidate branch.
7. Shipment claims, implementation, builds, tests, commits, pushes, and pull requests remain Ship-owned.

## Fail-closed backlog checks

Before Ship treats the reduced Stage package as ready:

- backlog index sync must succeed;
- exact reads must show `102-S` and `103-S` exist and are queued;
- `102-S.custom_fields.operator_predecessors` must be exactly `[]`;
- `103-S.custom_fields.operator_predecessors` must be exactly `[102-S]`;
- manifests must contain only their covering feature and two child tasks;
- any missing, blocked, unknown, unqueryable, or mismatched value fails the check.

These checks approve the Stage artifact package, not execution order. Ship must still require positive terminal evidence for 102-S before claiming 103-S.

## Verification matrix

| Check | Expected result |
|---|---|
| Candidate path set | Exactly the 16 include entries, counting the 087 move as one atomic change |
| 102-S manifest | `107-F`, `107.001-T`, `107.002-T` only |
| 103-S manifest | `108-F`, `108.002-T`, `108.001-T` only, with task order preserved from the shipment |
| 104/109 paths | Zero |
| Stash JSONL changes | Zero |
| Separate review/checkpoint artifacts | Zero |
| Ship agent changes | Zero |
| Source/test/build changes | Zero |
| Existing Stage branch | Unchanged and preserved |

## Rollback

Because Ship builds a new branch from main, rollback is to abandon that new candidate branch or revert its eventual integration commit. The preserved Stage branch and all 109 provenance remain available. No history rewrite or source restoration is needed.

## Plan hardening

Hardening is required because selective integration can silently leak blocked backlog state or omit one half of an approved archive move.

**ProposedAction PA-1**

- Summary: create a new Ship-owned integration branch from current main and apply only the explicit path manifest.
- ActionRisk: moderate.
- Approval required: already authorized by the operator; execution remains Ship-owned.
- Rollback: abandon the candidate branch or revert its integration commit; preserve the Stage branch.
ActionResult: planned.

**ProposedAction PA-2**

- Summary: carry the atomic 087-F queue-to-archive move.
- ActionRisk: moderate because an incomplete move can duplicate or lose backlog visibility.
- Approval required: explicitly approved by the operator.
- Rollback: restore the queue file and remove only the candidate archive addition on the candidate branch.
ActionResult: planned.

Reinforcing guidance: strict-safety instructions, shipment over-scope learning, backlog sync cache learning, plan-review divergence learning, and the Stage scope guard. No destructive or history-rewriting action is authorized.

## Plan and integration review

**Review mode:** reduced path manifest at local HEAD evidence `538e0ab95ce1ad2ecb77925950f89e63d6d74f58`, configured `.Stage` frontmatter model (`claude-opus-4.8`), no override. Cross-model dispatch was unavailable, so all required persona lenses were applied with the caller model.

**Gate: PASS.** Open P0: zero. Open P1: zero. Open P2: zero. Open P3: zero.

### Persona results

- Constitution Reviewer: PASS. Scope contains only queued reviewed Stage units and approved stowaways; Stage performs no implementation or Ship lifecycle action.
- Rust Reviewer: PASS. No source or test change is in scope.
- Scope Boundary Auditor: PASS. The include set is closed and the excluded classes explicitly cover 104/109, stash, reviews, checkpoints, memory, Ship agent, and unknown paths.
- Learnings Researcher: PASS. Path-only construction avoids the known over-scoped shipment and mixed-history hazards.
- Architecture Strategist: PASS. Quarantining 104-S removes the independent architecture blocker without altering 102-S or 103-S.
- Agent-Native Parity Reviewer: PASS. No CLI, MCP, schema, response, or runtime contract changes.
- Security Lens Reviewer: not triggered.

### Gate rationale

The package has no dependency on 109-F and cannot make 104-S executable. The two release units retain their own accepted inline plan reviews. Exact path allowlisting plus a closed exclusion set prevents consumed stash, separate review/checkpoint, memory, or unapproved agent state from leaking into the immediate pull request.

**Final verdict: PASS for the reduced 102-S and 103-S Stage integration scope.**
