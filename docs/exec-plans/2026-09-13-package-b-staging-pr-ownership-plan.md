---
title: "Package B — Staging PR ownership and lifecycle"
doc_type: exec-plan
date: 2026-09-13
agent: stage
package: B
program: docs/decisions/2026-09-13-finalization-write-boundary-program-deliberation.md
depends_on: []
status: draft
---

## Problem Frame

`.github/policies/workflow-policies.md:211-249` (P-010) declares three mutually exclusive
authorities:

1. **Stage MUST NOT** *"Create, push, or merge pull requests"*.
2. **Ship MAY** *"Create, update, and merge pull requests (with operator approval)"*.
3. *"The Orchestrator agent must not perform Stage or Ship work directly — it routes to them as
   subagents."*

`.github/agents/_orchestrator.agent.md:270-297` (Step 1.5, "Staging Artifact Merge Gate
(NON-NEGOTIABLE)") instructs the Orchestrator itself to:

* item 3a — *"Commit any uncommitted backlog files to a staging branch: `chore/stage-{shipment_id}`"*
* item 3b — *"Push the staging branch and create a PR to `main`"*
* item 3c — *"Wait for the staging PR to merge (operator approval required)"*
* item 3e — *"Attempt a direct push to `main` first. If the push is rejected … fall back to creating
  a staging PR"*

PR creation is Ship-classed work under P-010's own table, so Step 1.5 has the Orchestrator
performing Ship-classed work directly — which sentence 3 forbids. Stage cannot do it (sentence 1).
Ship is never invoked for it (Step 1.5 runs *between* Stage and Ship, and Ship's Step 0.5 intake is
shipment-scoped, not staging-scoped).

**Result: the staging PR has no legal author.** Every staging round in this workspace has been
executed under an unresolved contract contradiction.

Item 3e compounds this: *"Attempt a direct push to `main` first"* conflicts with the
branch-per-release-unit principle asserted in `_ship.agent.md:724-751` (*"These commits MUST NOT
land directly on `main`"*) and, when it succeeds, bypasses review entirely.

## Requirements Trace

| Req | Source | Statement |
|---|---|---|
| B-R1 | P-010 `:211-249` | Exactly one agent role is authorised to create/push the staging PR |
| B-R2 | P-010 `:216-221` | Stage's PR prohibition survives **verbatim** — no new authority granted to Stage |
| B-R3 | Orchestrator `:270-297` | Step 1.5 text names that owner explicitly and stops instructing the Orchestrator to do it itself |
| B-R4 | `_ship.agent.md:724-751`, P-009 | The `main` direct-push fallback (3e) is classified against branch protection and merge-commit policy |
| B-R5 | P-010 `:216-221` | Stage's source/template mutation prohibition is restated unchanged where staging authority is described |
| B-R6 | Program decision | Package B introduces **no** checkpoint-ordering change and **no** freeze semantics |

## Implementation Units

### B-U1 — Record the staging-PR authority gap as a policy defect note

* **Domain**: docs (policy)
* **Files**: `.github/policies/workflow-policies.md` (P-010 section, `:211-249`)
* **Change**: Add a `**Staging-artifact PR authority**` sub-clause under P-010 that states the
  owner explicitly and cites Orchestrator Step 1.5 as the consuming site.
* **Atomic milestone**: `Select-String -Path .github/policies/workflow-policies.md -Pattern "Staging-artifact PR authority"`
  returns exactly one hit, and the three original P-010 authority sentences are byte-unchanged.
* **Verification**: `git diff --stat .github/policies/workflow-policies.md` shows additions only,
  zero deletions in lines `211-249`.

### B-U2 — Name the owner in Orchestrator Step 1.5

* **Domain**: docs (agent contract)
* **Files**: `.github/agents/_orchestrator.agent.md` (Step 1.5, `:270-297`)
* **Change**: Rewrite items 3a–3d so the Orchestrator **routes** the staging-artifact commit and PR
  creation to the owning role rather than performing it, preserving the existing gate semantics
  (wait for merge, then verify manifest on `origin/main`).
* **Atomic milestone**: Step 1.5 contains no first-person instruction to create or push a PR; item 4's
  `git show origin/main:.backlogit/queue/{shipment_id}.md` verification and its
  `STAGING_GATE_FAIL` halt token are unchanged.
* **Verification**: `Select-String -Path .github/agents/_orchestrator.agent.md -Pattern "STAGING_GATE_FAIL"`
  still returns its original hit; `autoharness verify-workspace` exits 0.

### B-U3 — Classify the direct-push-to-`main` fallback

* **Domain**: docs (agent contract)
* **Files**: `.github/agents/_orchestrator.agent.md` (Step 1.5 item 3e)
* **Change**: Replace *"Attempt a direct push to `main` first"* with an explicit classification:
  direct push is permitted only when branch protection is absent **and** the operator has authorised
  it; otherwise the staging PR is the sole path. Cite P-009 (merge-commit-only) and
  `_ship.agent.md:726-727` (*"These commits MUST NOT land directly on `main`"*).
* **Atomic milestone**: item 3e no longer instructs an unconditional `main` push; the fallback
  branch name `chore/stage-{shipment_id}` is preserved.
* **Verification**: `Select-String -Path .github/agents/_orchestrator.agent.md -Pattern "chore/stage-"`
  returns its original occurrences.

### B-U4 — Restate Stage's unchanged prohibitions at the staging boundary

* **Domain**: docs (agent contract)
* **Files**: `.github/agents/_stage.agent.md` (Role Boundary table + the "Return the branch to
  Orchestrator" language in its harvest/packaging section)
* **Change**: Add one clarifying sentence that Stage hands the planning branch off and never creates
  the staging PR, cross-referencing P-010's new sub-clause. **No row of the Role Boundary table is
  edited.**
* **Atomic milestone**: `git diff .github/agents/_stage.agent.md` shows zero modifications inside the
  Role Boundary table block; the `Forbidden` column still contains
  `Create, push, or merge pull requests`.
* **Verification**: `Select-String -Path .github/agents/_stage.agent.md -Pattern "Create, push, or merge pull requests"`
  returns its original hit unchanged.

### B-U5 — Full-surface consistency verification

* **Domain**: verification
* **Files**: none modified
* **Change**: none.
* **Atomic milestone**: all four commands below exit 0.
* **Verification** (executable, run last):
  * `pwsh scripts/pre-commit-markdownlint.ps1`
  * `autoharness verify-workspace`
  * `pwsh scripts/pre-commit-pipeline-topology.ps1`
  * `git diff --check`

## Dependency Graph

```
B-U1 ──> B-U2 ──> B-U3 ──> B-U5
           └────> B-U4 ────┘
```

B-U1 establishes the policy anchor that B-U2/B-U3/B-U4 cite. B-U5 is terminal and runs last.

## Decisions and Rationale

* **D-B1 — Package B does not choose Stage as the owner.** Granting Stage PR authority would require
  editing P-010's Stage prohibition, which B-R2 forbids and which the operator's role-boundary
  instruction treats as non-negotiable. The owner must be a role that already holds PR authority
  under P-010's existing table.
* **D-B2 — Gate semantics are preserved, not redesigned.** Step 1.5's halt token, its
  `git show origin/main:...` manifest verification, and its operator-approval requirement are
  load-bearing and untouched. Only the *actor* changes.
* **D-B3 — Docs-only.** No Rust source, test, or configuration file is modified. Full local build is
  therefore non-applicable; `autoharness verify-workspace` is the substituting evidence.

## Risks and Caveats

| R | Risk | Mitigation |
|---|---|---|
| B-K1 | Renaming the actor silently breaks the Orchestrator→Ship handoff | B-U2's milestone pins the unchanged `STAGING_GATE_FAIL` token and manifest check |
| B-K2 | Editing P-010 destabilises other policies that cite it | B-U1 is additive; B-U1's milestone requires zero deletions in `:211-249` |
| B-K3 | A reviewer reads B as granting Stage new authority | B-U4 explicitly re-asserts the prohibition and forbids table edits |
| B-K4 | Direct-push classification conflicts with an operator's existing workflow | B-U3 preserves direct push as an operator-authorised path, not a removal |

## Plan Hardening Signals

| Signal | Present | Evidence |
|---|---|---|
| Modifies a NON-NEGOTIABLE contract section | **yes** | Orchestrator Step 1.5 is titled "(NON-NEGOTIABLE)" |
| Modifies a workflow policy | **yes** | P-010 in `.github/policies/workflow-policies.md` |
| Changes role authority boundaries | **yes** | staging-PR ownership is a role-authority question |
| Touches security/authn/authz | no | — |
| Touches concurrency/parallelism | no | — |
| Migration or data change | no | — |
| Cross-cutting blast radius | no | 3 files, all documentation |

**Requires plan hardening: yes**

## Runtime Verification and Closure

Documentation-only change set; full local build is **non-applicable** and must be recorded as such
in PR readiness evidence per `pr-lifecycle/SKILL.md:53-89` item 6.

Verification evidence = the four B-U5 commands. Closure requires no runtime validator manifest entry
because no runtime surface changes.

## Plan Hardening

### Hardening required — why

`Requires plan hardening: yes`. Three signals present: (1) the plan edits a section explicitly
titled **NON-NEGOTIABLE** (`_orchestrator.agent.md:270` "Staging Artifact Merge Gate
(NON-NEGOTIABLE)"); (2) it edits a workflow policy (`P-010`); (3) it changes a **role authority
boundary**, which P-010 itself classifies as halt-on-violation territory.

### Context consulted

* `.github/policies/workflow-policies.md:211-249` (P-010), `:186-209` (P-009), `:452-488` (P-016)
* `.github/agents/_orchestrator.agent.md:270-297` (Step 1.5), `_ship.agent.md:724-751` (Step 6.0)
* `.github/agents/_stage.agent.md` Role Boundary table
* `docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md`
  — establishes the staging PR as the sanctioned vehicle for pipeline/bookkeeping state
* `docs/compound/workflow-issues/ship-single-pr-serialization-and-stash-handoff-2026-05-14.md`
  — establishes that staging/implementation PR lanes must not blur

### High-risk triggers and invariants to preserve

| Invariant | Statement | How B-U* preserves it |
|---|---|---|
| I-B1 | Stage's `Create, push, or merge pull requests` prohibition is byte-unchanged | B-U4 milestone forbids Role Boundary table edits and pins the literal string |
| I-B2 | Step 1.5's `STAGING_GATE_FAIL` halt token and its `git show origin/main:.backlogit/queue/{id}.md` check survive | B-U2 milestone pins both |
| I-B3 | P-010's three authority sentences at `:211-249` are byte-unchanged | B-U1 milestone requires zero deletions in that range |
| I-B4 | `chore/stage-{shipment_id}` branch naming survives (Package C correlates on it) | B-U3 milestone pins the prefix |
| I-B5 | No checkpoint-ordering or freeze semantics enter this package | B-R6; diff contains no `_ship.agent.md` Step 5/6 or session-end edit |

### ProposedAction / ActionRisk entries (strict-safety: enabled)

| # | ProposedAction | ActionRisk | Approval needed | Rationale |
|---|---|---|---|---|
| PA-B1 | Edit `.github/policies/workflow-policies.md` P-010 section (additive sub-clause) | **HIGH** — contract mutation of a halt-on-violation policy | **Yes — operator** | A policy edit changes agent behaviour repo-wide and is not covered by any automated test |
| PA-B2 | Rewrite `_orchestrator.agent.md` Step 1.5 items 3a–3d (actor change) | **HIGH** — NON-NEGOTIABLE gate section | **Yes — operator** | Mis-stating the actor could strand every future staging round |
| PA-B3 | Rewrite `_orchestrator.agent.md` Step 1.5 item 3e (direct-push classification) | **MEDIUM** — removes an unconditional `main` push instruction | **Yes — operator** | Narrows an existing capability; operator may rely on direct push locally |
| PA-B4 | Add one clarifying sentence to `_stage.agent.md` outside the Role Boundary table | **LOW** — additive, non-normative restatement | No | Restates an existing prohibition; cannot widen authority |
| PA-B5 | Run `autoharness verify-workspace`, markdownlint, topology, `git diff --check` | **NONE** — read-only verification | No | Non-mutating |

No destructive action is proposed: there is no file deletion, no history rewrite, no backlog
mutation, and no `git restore`/`git revert` in this package. The strict-safety
`require_approval_for: [destructive]` trigger is therefore not reached; PA-B1..PA-B3 nonetheless
carry explicit operator approval because of contract blast radius.

### Deepened verification

**Environment prechecks** (before any edit):
1. `git status --porcelain` — must be empty for the four target files.
2. `git rev-parse --abbrev-ref HEAD` — confirm the execution branch.
3. `autoharness verify-workspace` — capture a **baseline** exit code; if baseline is already
   non-zero, halt and report rather than attributing a pre-existing failure to this package.

**Target scenarios** (each must be demonstrated, not asserted):
* S-B1: `Select-String -Path .github/agents/_stage.agent.md -Pattern "Create, push, or merge pull requests"` → ≥ 1 hit (I-B1).
* S-B2: `Select-String -Path .github/agents/_orchestrator.agent.md -Pattern "STAGING_GATE_FAIL"` → ≥ 1 hit (I-B2).
* S-B3: `git diff --numstat .github/policies/workflow-policies.md` → deletions column `0` (I-B3).
* S-B4: `Select-String -Path .github/agents/_orchestrator.agent.md -Pattern "chore/stage-"` → ≥ 1 hit (I-B4).
* S-B5: `git diff --name-only` contains no `_ship.agent.md` (I-B5).

**Blocked-path handling**: if `autoharness verify-workspace` fails *after* the edits but the
baseline passed, revert the working tree with `git restore --` limited to the four target files and
report the failing check. Do not attempt a second contract edit to satisfy the validator.

### Rollback

* **Trigger**: any of S-B1..S-B5 fails, or `autoharness verify-workspace` regresses against baseline.
* **Procedure**: `git restore -- .github/policies/workflow-policies.md .github/agents/_orchestrator.agent.md .github/agents/_stage.agent.md`
  before commit; after commit but before merge, `git revert` the package commit (P-009 preserves
  merge-commit history, so revert is clean).
* **Coupling**: none — no other package is merged at the time B lands (B is a program root).

### Operational closure

* **Monitoring signal**: the next Orchestrator Step 1.5 execution completes without an ownership
  ambiguity halt.
* **Failure signal**: a staging round halts with an actor-resolution error, or `STAGING_GATE_FAIL`
  fires on a manifest that does exist on `origin/main`.
* **Validation window**: the next two staging rounds.
* **Owner**: the operator (no session-scoped agent can own a multi-round validation window — this
  was a recorded review finding on closure PR #394, stash `3F1AEFE1`).
* **Releasability**: `READY` when S-B1..S-B5 pass and `autoharness verify-workspace` matches baseline.

### Unresolved operator decisions blocking safe execution

* **OD-B1** — Which role is named as the staging-PR owner. This plan deliberately does **not**
  pre-decide it (D-B1 only excludes Stage). The implementing session must obtain an explicit
  operator choice between the remaining candidates before executing B-U2, because the choice is a
  role-authority decision and P-010 requires operator disposition for boundary changes.

<!-- plan-review-attempt: 1 -->
<!-- plan-review-verdict: FAIL — docs/reviews/2026-09-13-package-b-plan-review-fail.md -->
