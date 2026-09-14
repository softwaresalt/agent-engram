---
title: "Plan review — Package C (open PR taxonomy) — FAIL"
doc_type: review
date: 2026-09-13
agent: stage
package: C
plan: docs/exec-plans/2026-09-13-package-c-open-pr-taxonomy-plan.md
verdict: FAIL
attempt: 1
---

## Panel

| Persona | Model | Provider |
|---|---|---|
| Architecture Strategist | `gpt-5.6-sol` | openai |
| Scope Boundary Auditor | `grok-4.6` | xai |
| Correctness Reviewer | `claude-opus-5` | anthropic |

3 personas / 3 providers. Cross-model diversity satisfied. Reviewed **independently of Package B** —
no PASS/FAIL in one package was allowed to influence the other.

## Verdict: FAIL

**2 P0, 13 P1, 8 P2, 4 P3.** Per `plan-review/SKILL.md:118-130`, any P0 or P1 finding is FAIL.

## Decisive findings

### F-C1 — P0 — The monotonicity guarantee is false: draft status clears a correlated blocker

The four classes are **not disjoint**. `unrelated-or-draft` fuses an *orthogonal state predicate*
(`isDraft`) with a *correlation predicate* (`unrelated`). A PR on `post-merge/155-S`, or on a feature
branch correlated to the candidate shipment, that is **also a draft** matches two classes. `C-U4`
resolves the collision by declaring draft PRs `unrelated-or-draft` and *"not a blocker on their
own"* — so **draft status wins over correlation and demotes a would-be blocking PR to non-blocking.**

This is precisely the clearance channel `C-R2` exists to forbid. Worse, `isDraft` is
**operator-mutable** — a ready PR can be converted back to draft at any time — so an in-flight
release-unit PR can silently open the P-001 single-active gate. This directly contradicts `C-R6`
(*"classification authority derives from provenance/correlation, never from mutable labels or
markers"*), while `C-U1` nonetheless lists `isDraft` as a correlation input.

### F-C2 — P0 — The class-to-blocker-contribution mapping is never specified

`C-U1` defines a vocabulary. `C-U4` defines behaviour for only `unrelated-or-draft`,
superseded-open, and `UNCLASSIFIED`. **Nothing in the plan states whether `feature-implementation`,
`post-merge-closure`, or `staging-planning` contribute a blocker at all.**

P-001's precondition explicitly names *"an open post-merge closure branch/PR"* as an in-flight
closure signal, so `post-merge-closure` **must** block — but no unit, milestone, or invariant says
so. The entire deliverable (which PRs block) is left to downstream interpretation while every
verification step is a literal grep.

### F-C3 — P1 — The new blocker has no consumer; it is computed and discarded

`C-U5` unions the result into a blocker set at Orchestrator **Step 0** — but Step 0 only gathers and
summarises backlog state. The actual routing decision and P-001/P-020 gate occur at **Step 2**, and
P-001's declared Gate Point is **Ship Step 1**. No unit modifies any consumer, and `C-U5` explicitly
promises *"no existing halt condition is made conditional on the new check."* The plan's own
kill-switch note concedes the instruction file *"is consumed nowhere else."*

### F-C4 — P1 — Monotonicity is vacuous where it matters

Step 0 computes blockers from shipment status and stash only. P-020 states closure completeness
*"is NOT tracked by shipment active-state; it is tracked by the operational-closure artifact's
recorded compaction status"* — which Step 0 never reads. The `compaction: pending` blocker the plan
names as its **own worked counterexample** is absent from the set being unioned. Set-monotonicity is
therefore vacuously true while the operator-visible outcome still functions as a de facto proceed
signal. **Monotonicity of the set does not imply monotonicity of the decision.**

### F-C5 — P1 — `C-U5`'s zero-deletion milestone is both insufficient and unsatisfiable

*Insufficient*: a purely **additive** line such as *"if the open-PR scan returns empty, proceed"*
clears a blocker with zero deletions. The milestone prose says "not deleted **or made conditional**",
but the mechanical check covers only deletion — the "made conditional" half has no verification at
all. This is the exact vector by which the prior plan's P1 defect would pass review again.

*Unsatisfiable*: Step 0 is a numbered list whose final item emits the state summary. For the scan to
be operator-visible and to precede routing it must run before that item — requiring renumbering,
which produces deletions. The milestone and the behaviour it guards are mutually exclusive.

### F-C6 — P1 — `ENUMERATION_INCOMPLETE` creates an unrecoverable offline deadlock

Defined as an additive blocker that is *"never a clearance"* — wording that forecloses even P-001's
documented `skip_policy: P-001` override. No bounded retry, no degraded-mode continuation, no
operator override. Step 0 currently requires **zero network**; this change would halt **every
session start**, including purely local Stage work, whenever `gh` is offline or unauthenticated.
It also contradicts the plan's own `OD-C1` reasoning and the repo's established P-012 fail-safe
posture.

### F-C7 — P1 — Fork PRs defeat branch-prefix correlation (security)

Classification rests on head-ref prefixes with **no repository-ownership check**. `headRefName` for a
cross-repository PR is the fork's branch name, unqualified by owner — so any external contributor can
name a branch `post-merge/155-S` or `chore/stage-X` and have it classified as harness-owned work.
`D-C3` justifies prefix reuse as "already load-bearing", but existing uses operate on branches the
harness itself created, not on attacker-controlled input.

### F-C8 — P1 — `staging-planning` self-deadlock unanalysed

Step 1.5 creates `chore/stage-*` PRs and then *waits for them to merge*. If a session ends while one
is open, the next session's Step 0 scan sees it. If `staging-planning` blocks, Step 0 halts on the
very PR that Step 1.5 must reach in order to merge — a **permanent wedge with no self-clearing path**.

### F-C9 — P1 — Two overlapping fallbacks with no precedence rule

A PR with no branch-name correlation matches **both** `unrelated-or-draft` (unrelated → non-blocking)
**and** `UNCLASSIFIED` (no test succeeded → advisory blocker). No evaluation order is defined, so an
identical PR classifies either way depending on implementation order — **non-deterministic gate input**.

### F-C10 — P1 — Verification commands cannot pass, and prove nothing

Same class of defect as Package B, verified directly this session:

* `autoharness verify-workspace` requires `--workspace <path>`; every invocation omits it.
* `scripts/pre-commit-markdownlint.ps1` exits 0 on an empty staged set; `C-U1`'s artifact is a **new
  untracked file** → vacuous pass.
* `scripts/pre-commit-pipeline-topology.ps1` runs a backlogit shipment/worktree gate and **never
  enumerates GitHub PRs** — copied Package B boilerplate, irrelevant to C.
* `Select-String` exits 0 regardless of match count → `S-C1`..`S-C4` are assertions, not gates.
* The hardening precheck `gh pr list --state open --json …` has **no `--limit`** and defaults to 30
  results — a page-1-only read, in a plan whose stated hazard #1 is pagination.

### F-C11 — P1 — `S-C4`, the self-declared "decisive test", is not a test

Bound to mutable external state (#390, #396), non-reproducible, with no executable classifier
(the change set is docs-only). It also hardcodes specific PR numbers into a normative instruction —
the special-casing pattern P-015 explicitly prohibits.

### F-C12 — P2 — The zero-open-PR case is never explicitly handled

The prior plan's recorded P1 was exactly an unconditional *"zero open PRs → proceed"* rule, yet no
milestone, invariant, or scenario states the guard *"an empty enumeration contributes the empty set
and is never evidence of clearance."* The one sentence that would mechanically prevent recurrence is
absent.

## Correction allowance: NOT MET

F-C1 requires replacing the class model (separate `kind`, `isDraft`, `correlation`, and `supersession`
as independent fields) — **a redesign of the package's central abstraction**. F-C2 requires
authoring the missing deliverable. F-C3 requires relocating the integration point from Step 0 to
Step 2 / Ship Step 1. None is mechanical. **Correction not attempted. Package C is BLOCKED.**

## Durable conclusion for re-decomposition

1. **Split C.** `C1` = pure, owner-neutral classification (PR *kind*, draft, correlation, and
   supersession as **four independent fields**, not four fused classes) with an explicit precedence
   order and an explicit class→blocker contribution table including `post-merge-closure` and the
   zero-PR guard. `C2` = blocker integration at the **correct consumer** (Orchestrator Step 2 and/or
   Ship Step 1, not Step 0), with degraded-mode/offline semantics and an operator override path.
2. **Supersession needs an authoritative input.** Branch prefix and `isDraft` cannot establish it;
   `mergedAt`, closing-PR reference, or commit-ancestry against `main` is required.
3. **Correlation must be repo-ownership-qualified** to resist fork spoofing.
