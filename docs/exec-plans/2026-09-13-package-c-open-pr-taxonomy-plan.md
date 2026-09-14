---
title: "Package C — Open PR taxonomy as additive P-001 blocker"
doc_type: exec-plan
date: 2026-09-13
agent: stage
package: C
program: docs/decisions/2026-09-13-finalization-write-boundary-program-deliberation.md
depends_on: []
status: draft
---

## Problem Frame

`.github/policies/workflow-policies.md:16-37` (P-001) gates the next release unit on backlog state:

> **Precondition**: No backlog tasks with status `Active` exist under any top-level work item other
> than the current feature or chore, and no previously merged top-level release unit is still
> awaiting required post-merge release closure.

Institutional evidence contradicts backlog-state-only detection.
`docs/compound/workflow-issues/ship-single-pr-serialization-and-stash-handoff-2026-05-14.md`:

> *"'No active shipment' was treated as sufficient to start the next shipment even though the prior
> shipment PR was still open at the PR level."* … *"Open-PR state — not just backlog status — is the
> authoritative single-active blocker signal."* (overlapping PRs #138/#140)

Verified live this session: **the Orchestrator performs no open-PR enumeration anywhere.** Step 0
(`_orchestrator.agent.md:220`) assesses shipments and stash only.

Two hazards constrain any remedy:

1. **Pagination.** `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md`:
   *"a page-1-only read is a silent correctness defect in gate logic."*
2. **Monotonicity.** A prior plan-review recorded a P1 against an unconditional
   *"zero open PRs → proceed"* rule: it would **clear** a P-001 blocker held for an unrelated reason
   (a merged unit whose operational-closure artifact still reads `compaction: pending`, per P-020
   `:643-700`). Discovery must be strictly **additive**.

Live repository state proves the hazard is not hypothetical: **#396** (open, superseded, 34
unresolved threads) and **#390** (draft) both exist right now. A naive "any open PR blocks" or
"multiple carrying PRs fail closed" rule would wedge the Orchestrator on every session start.

## Requirements Trace

| Req | Source | Statement |
|---|---|---|
| C-R1 | `ship-single-pr-serialization-2026-05-14` | Open-PR state is a recognised single-active signal |
| C-R2 | prior plan-review P1 | Open-PR discovery may **only add** blockers/routes; it may never clear an existing claim or closure blocker |
| C-R3 | `gh-reviews-endpoint-paginate-2026-07-22` | Enumeration is exhaustively paginated; incomplete pagination fails closed |
| C-R4 | live #390 / #396 | Draft and superseded-open PRs are classified without wedging the pipeline |
| C-R5 | P-016 `:452-488` | Legitimate Stage/Ship overlap (planning overlap, Stage spike/research worktree) remains permitted |
| C-R6 | Program decision | Classification authority derives from **provenance/correlation**, never from mutable labels or PR-body markers |
| C-R7 | Program decision | Package C introduces no checkpoint-ordering change and no freeze semantics |

## Implementation Units

### C-U1 — Define the PR class vocabulary

* **Domain**: docs (instruction)
* **Files**: new `.github/instructions/open-pr-taxonomy.instructions.md`
* **Change**: Define exactly four classes with disjoint, machine-checkable membership tests:
  `feature-implementation`, `post-merge-closure`, `staging-planning`, `unrelated-or-draft`.
  Define the correlation inputs used to assign a class: base ref, head-ref branch-name prefix
  (`post-merge/*`, `chore/stage-*`), `isDraft`, and the shipment/feature ID embedded in the branch
  slug. State explicitly that **labels and PR-body markers are NOT authority**.
* **Atomic milestone**: the file declares four classes, each with a stated membership test and a
  stated `UNCLASSIFIED` fallback.
* **Verification**: `Select-String -Path .github/instructions/open-pr-taxonomy.instructions.md -Pattern "UNCLASSIFIED"`
  returns ≥ 1 hit; `pwsh scripts/pre-commit-markdownlint.ps1` exits 0.

### C-U2 — Specify exhaustive enumeration and fail-closed pagination

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/open-pr-taxonomy.instructions.md`
* **Change**: Specify the enumeration contract: list open PRs with full pagination until
  `hasNextPage` is false (mirroring `pr-lifecycle/SKILL.md:197-228` item 1, which already fails
  closed on incomplete pagination). State that an incomplete or errored enumeration yields
  `ENUMERATION_INCOMPLETE`, which is itself an **added blocker**, never a clearance.
* **Atomic milestone**: the section names the `ENUMERATION_INCOMPLETE` token and states it is
  additive.
* **Verification**: `Select-String -Path .github/instructions/open-pr-taxonomy.instructions.md -Pattern "ENUMERATION_INCOMPLETE"`
  returns ≥ 1 hit.

### C-U3 — State the monotonicity rule and its proof obligation

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/open-pr-taxonomy.instructions.md`
* **Change**: State the rule as a one-way lattice: the open-PR scan's output is a **set union** with
  the existing blocker set. Include the explicit proof obligation — *"no evaluation path of this
  instruction may remove an element from, or short-circuit evaluation of, the P-001 / P-020 blocker
  set"* — and name the P-020 `compaction: pending` case as the worked counterexample that motivated
  it.
* **Atomic milestone**: the section contains the literal phrase `set union` and names
  `compaction: pending` as the worked counterexample.
* **Verification**: both literals present via `Select-String`.

### C-U4 — Specify ambiguity and overlap behaviour

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/open-pr-taxonomy.instructions.md`
* **Change**: Define behaviour for `UNCLASSIFIED` (surface as an operator-visible advisory blocker,
  never a silent pass and never a hard wedge), for draft PRs (classified `unrelated-or-draft`; not a
  blocker on their own), and for superseded-open PRs (classified by correlation; blocking only when
  they correlate to the candidate shipment). Explicitly permit the P-016 Stage/Ship overlap states.
* **Atomic milestone**: the section resolves the live #390 and #396 cases by name as worked
  examples and neither produces a wedge.
* **Verification**: `Select-String ... -Pattern "#390|#396"` returns ≥ 2 hits.

### C-U5 — Wire the taxonomy into Orchestrator Step 0 as an additive check

* **Domain**: docs (agent contract)
* **Files**: `.github/agents/_orchestrator.agent.md` (Step 0, `:220-251`)
* **Change**: Add one sub-step that invokes the taxonomy and **unions** its result into the existing
  blocker set. The existing shipment/stash assessment logic is not modified and no existing halt
  condition is made conditional on the new check.
* **Atomic milestone**: `git diff` shows additions only within Step 0; no existing line in Step 0 is
  deleted or made conditional.
* **Verification**: `git diff --numstat .github/agents/_orchestrator.agent.md` reports `0` deletions.

### C-U6 — Full-surface consistency verification

* **Domain**: verification
* **Files**: none modified
* **Atomic milestone**: all four commands exit 0.
* **Verification** (executable, run last):
  * `pwsh scripts/pre-commit-markdownlint.ps1`
  * `autoharness verify-workspace`
  * `pwsh scripts/pre-commit-pipeline-topology.ps1`
  * `git diff --check`

## Dependency Graph

```
C-U1 ──> C-U2 ──> C-U3 ──> C-U4 ──> C-U5 ──> C-U6
```

Strictly linear: each unit extends the same instruction file, and C-U5 cannot cite the taxonomy
before it exists.

## Decisions and Rationale

* **D-C1 — A new instruction file, not a policy edit.** P-001's normative text stays untouched; the
  taxonomy is a *detection mechanism* consumed by the Orchestrator, not a new precondition. This
  keeps C independent of A, B, D, and E.
* **D-C2 — Union, not replacement.** The monotonicity rule is expressed as a set operation
  specifically so a reviewer can check it mechanically rather than by reading prose intent.
* **D-C3 — Correlation over markers.** Branch-name prefixes (`post-merge/*`, `chore/stage-*`) are
  already load-bearing in `_ship.agent.md:733` and Orchestrator Step 1.5 item 3a, so correlation is
  reusing an existing convention rather than inventing one. Labels are operator-mutable and
  HEAD-uncorrelated, so they are excluded by C-R6.
* **D-C4 — Docs-only.** No Rust source, test, or configuration file is modified.

## Risks and Caveats

| R | Risk | Mitigation |
|---|---|---|
| C-K1 | The scan wedges the Orchestrator on the live #396/#390 | C-U4's milestone requires both to be resolved by name as non-wedging worked examples |
| C-K2 | A future edit turns the additive check into a clearance | C-U3's explicit proof obligation is stated as a reviewable invariant on the file itself |
| C-K3 | Branch-prefix correlation breaks if naming conventions change | C-U1 defines `UNCLASSIFIED` as the fallback, which is an advisory blocker, so convention drift degrades safely |
| C-K4 | Pagination cost on a repo with many open PRs | Enumeration is once per Orchestrator Step 0, not per gate re-check |

## Plan Hardening Signals

| Signal | Present | Evidence |
|---|---|---|
| Modifies a NON-NEGOTIABLE contract section | no | Step 0 is not marked NON-NEGOTIABLE; P-001 text is untouched |
| Modifies a workflow policy | no | new instruction file; P-001 unchanged |
| Changes role authority boundaries | no | detection only |
| Affects a single-active safety gate | **yes** | P-001 blocker computation |
| Touches security/authn/authz | no | — |
| Touches concurrency/parallelism | **yes** | interacts with P-016 overlap states |
| Migration or data change | no | — |

**Requires plan hardening: yes**

## Runtime Verification and Closure

Documentation-only change set; full local build is **non-applicable** and must be recorded as such
in PR readiness evidence. Verification evidence = the four C-U6 commands.

## Plan Hardening

### Hardening required — why

`Requires plan hardening: yes`. Two signals present: (1) the package **affects a single-active
safety gate** — its output unions into the P-001 blocker set, and P-001's violation action is
"Halt"; (2) it **touches concurrency/parallelism** by interacting with P-016 overlap states.

### Context consulted

* `.github/policies/workflow-policies.md:16-37` (P-001), `:452-488` (P-016), `:643-700` (P-020)
* `.github/agents/_orchestrator.agent.md:220-251` (Step 0)
* `.github/skills/pr-lifecycle/SKILL.md:197-228` — existing fail-closed pagination precedent
* `docs/compound/workflow-issues/ship-single-pr-serialization-and-stash-handoff-2026-05-14.md`
* `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md`
* `docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md`

### High-risk triggers and invariants to preserve

| Invariant | Statement | How C-U* preserves it |
|---|---|---|
| I-C1 | **Monotonicity** — the scan's result is unioned into the blocker set and can never remove an element or short-circuit evaluation | C-U3 states it as a set operation with an explicit proof obligation; C-U5's milestone requires `0` deletions in Step 0 |
| I-C2 | Enumeration is exhaustively paginated; incomplete pagination is itself an added blocker | C-U2 defines `ENUMERATION_INCOMPLETE` as additive |
| I-C3 | The live #390 (draft) and #396 (superseded-open) states do not wedge the Orchestrator | C-U4 resolves both by name as worked examples |
| I-C4 | P-016 legitimate Stage/Ship overlap remains permitted | C-R5; C-U4 explicitly enumerates the permitted overlap states |
| I-C5 | P-001's normative text is untouched | D-C1; the diff touches no line in `workflow-policies.md` |
| I-C6 | No checkpoint-ordering or freeze semantics enter this package | C-R7; the diff touches no `_ship.agent.md` line |

### ProposedAction / ActionRisk entries (strict-safety: enabled)

| # | ProposedAction | ActionRisk | Approval needed | Rationale |
|---|---|---|---|---|
| PA-C1 | Create `.github/instructions/open-pr-taxonomy.instructions.md` | **MEDIUM** — new normative instruction consumed by a halt-capable gate | **Yes — operator** | A new blocker source can stall the pipeline if mis-specified |
| PA-C2 | Add an additive sub-step to `_orchestrator.agent.md` Step 0 | **HIGH** — modifies the routing agent's state assessment | **Yes — operator** | Step 0 gates every session start; a wedge here halts all work |
| PA-C3 | Run markdownlint, `autoharness verify-workspace`, topology, `git diff --check` | **NONE** — read-only | No | Non-mutating |

No destructive action is proposed: no deletion, no history rewrite, no backlog mutation, no
`git restore`/`git revert`. Strict-safety's `require_approval_for: [destructive]` trigger is not
reached; PA-C1/PA-C2 carry operator approval because they can halt the pipeline.

### Deepened verification

**Environment prechecks**:
1. `git status --porcelain` empty for target files.
2. `autoharness verify-workspace` **baseline** exit code captured before edits.
3. `gh pr list --state open --json number,isDraft,headRefName,baseRefName` — capture the live open-PR
   set (currently #396 open/superseded and #390 draft) as the fixture for I-C3.

**Target scenarios** (demonstrated, not asserted):
* S-C1: the taxonomy file declares 4 classes + `UNCLASSIFIED` (C-U1 milestone).
* S-C2: `Select-String ... -Pattern "ENUMERATION_INCOMPLETE"` → ≥ 1 hit (I-C2).
* S-C3: `Select-String ... -Pattern "set union"` and `-Pattern "compaction: pending"` → ≥ 1 hit each (I-C1).
* S-C4: **dry-run classification** of the live open-PR set from the precheck fixture: #390 must
  classify `unrelated-or-draft` (non-blocking) and #396 must classify without producing a wedge
  (I-C3). This is the decisive test — it is executed against real repository state, not a
  hypothetical.
* S-C5: `git diff --numstat .github/agents/_orchestrator.agent.md` deletions column `0` (I-C1, I-C5).
* S-C6: `git diff --name-only` contains no `_ship.agent.md` and no `workflow-policies.md` (I-C5, I-C6).

**Blocked-path handling**: if `gh` is unavailable or unauthenticated during the S-C4 precheck, the
unit is **blocked**, not skipped — record it as a blocked prerequisite in runtime-verification
evidence. A taxonomy whose wedge-freedom was never demonstrated against live state must not be
presented as `READY`.

### Rollback

* **Trigger**: S-C4 produces a wedge on #390 or #396, or `autoharness verify-workspace` regresses.
* **Procedure**: `git restore -- .github/agents/_orchestrator.agent.md` and
  `git rm --cached .github/instructions/open-pr-taxonomy.instructions.md` before commit; after commit
  but before merge, `git revert` the package commit.
* **Coupling**: none — C is a program root with no predecessor.
* **Kill-switch**: because C-U5 is a single additive sub-step, reverting only that sub-step disables
  the check entirely while leaving the instruction file inert (it is consumed nowhere else).

### Operational closure

* **Monitoring signal**: Orchestrator Step 0 completes and reports a classified open-PR set on the
  next two session starts.
* **Failure signal**: Step 0 halts on an open PR that is not correlated to the candidate shipment,
  or reports `UNCLASSIFIED` for a PR created by the harness's own naming conventions.
* **Rollback trigger**: any Step 0 halt attributable to the new sub-step.
* **Validation window**: the next two Orchestrator session starts.
* **Owner**: the operator.
* **Releasability**: `READY` when S-C1..S-C6 pass, `READY_WITH_CONDITIONS` if S-C4 is blocked by `gh`
  unavailability.

### Unresolved operator decisions blocking safe execution

* **OD-C1** — Whether an `UNCLASSIFIED` PR should be an advisory blocker (surfaced, pipeline
  continues with operator acknowledgement) or a hard blocker (pipeline halts). This plan specifies
  advisory (C-U4) on the grounds that a hard blocker reintroduces the wedge risk C-K1 exists to
  prevent, but the choice is operator policy and must be confirmed before C-U4 executes.

<!-- plan-review-attempt: 1 -->
<!-- plan-review-verdict: FAIL — docs/reviews/2026-09-13-package-c-plan-review-fail.md -->
