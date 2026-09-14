---
title: "Plan review — Package B (staging PR ownership) — FAIL"
doc_type: review
date: 2026-09-13
agent: stage
package: B
plan: docs/exec-plans/2026-09-13-package-b-staging-pr-ownership-plan.md
verdict: FAIL
attempt: 1
---

## Panel

| Persona | Model | Provider |
|---|---|---|
| Architecture Strategist | `gpt-5.6-sol` | openai |
| Constitution Reviewer | `claude-opus-4.8` | anthropic |
| Scope Boundary Auditor | `grok-4.6` | xai |
| Agent-Native Parity Reviewer | `claude-sonnet-5` | anthropic |

4 personas / 3 providers. Cross-model diversity satisfied.

## Verdict: FAIL

**4 P0, 11 P1, 9 P2, 4 P3.** Per `plan-review/SKILL.md:118-130`, any P0 or P1 finding is FAIL.

## Decisive findings (convergent across independent reviewers)

### F-B1 — P0 ×3 — The named owner cannot exist within the plan's own constraints

Found independently by **Scope Boundary Auditor**, **Agent-Native Parity Reviewer**, and (as P1)
**Architecture Strategist** and **Constitution Reviewer**. Unanimous across 4 models.

The elimination logic is closed:

* Stage is excluded by `B-R2` (P-010's prohibition must survive verbatim).
* Orchestrator is excluded by P-010 sentence 3 (must not perform Ship work directly).
* **Ship is therefore the only P-010 role holding PR authority** — but Ship's Step 0.5 intake is
  *shipment-scoped*: it requires a pre-existing `shipment_id`, is gated on
  `features.shipments: true`, and drives branch naming off `feat/{slug}`/`chore/{slug}`, never
  `chore/stage-{shipment_id}`. **No staging-artifact entry point exists anywhere in Ship's contract.**

And the plan's own invariant `I-B5` / scenario `S-B5` **forbids any `_ship.agent.md` diff**.

> *"The plan therefore requires the only legally eligible owner to gain a capability it does not
> have, while explicitly forbidding the only file edit that could grant it. This is a hard
> self-contradiction, not a deferred decision."* — Agent-Native Parity Reviewer

`OD-B1` presents this as an open operator choice "between the remaining candidates" (plural). There
is exactly **one** remaining candidate, and it is disabled by the plan's own invariant.

### F-B2 — P0 — `B-U2` has no routing mechanism to route to

Orchestrator Step 1.5 sits *between* Step 1 (Stage, already returned) and Step 2 (Ship, not yet
invoked). There is no subagent-invocation call, no tool target, and no re-entry contract available
at that point. Rewriting the prose from "perform" to "route" leaves an **orphaned instruction**.
`B-U2`'s milestone only requires removing first-person language — it does not require a callable
target to exist.

### F-B3 — P1 — Strict-safety gate defeated by a false non-destructive claim

The hardening record asserts *"No destructive action is proposed … no `git restore`/`git revert` in
this package"* and concludes `require_approval_for: [destructive]` *"is not reached"* — while the
plan's own **Rollback** and **Blocked-path handling** sections invoke both. Under
`strict_safety.enabled: true`, these must appear in the `ProposedAction`/`ActionRisk` table with
operator approval. Instructing the executor that they are non-destructive **actively bypasses a
NON-NEGOTIABLE gate**.

### F-B4 — P1 — Mandatory `Constitution Check` section missing

Constitution Governance requires every implementation plan to include a `Constitution Check` section
mapping the work against all principles. The plan has Requirements Trace, Plan Hardening, and
`ProposedAction`/`ActionRisk`, but no `Constitution Check`.

### F-B5 — P1 — Verification commands do not prove the invariants, and two cannot pass

Verified directly this session:

* `autoharness verify-workspace` **requires `--workspace <path>`** and refuses without it. Every
  invocation in the plan (`B-U2`, `B-U5`, precheck, releasability) omits it.
* `scripts/pre-commit-markdownlint.ps1` lints only `git diff --cached` files and
  `exit 0`s when the staged set is empty (confirmed at line 20: `if (-not $StagedMd) { exit 0 }`).
  `B-U5` does not stage first → **vacuous pass**.
* `Select-String` exits 0 regardless of match count, so `S-B1`..`S-B4`'s "≥ 1 hit" criteria are
  assertions, not gates. A diff that leaves the owner unnamed would pass every check.

### F-B6 — P1 — `B-U2`/`B-U3` leave an invalid intermediate contract

Split across the same NON-NEGOTIABLE Step 1.5: after `B-U2`, items 3a–3d route away from the
Orchestrator while item 3e still instructs it to attempt a direct `main` push.

### F-B7 — P1 — Readiness, pause, and closure ownership never assigned

Program Package B requires an owner for staging-PR **creation, readiness, pauses, and closure**.
`B-R1`/`B-U2` cover only create/push plus wait-for-merge.

### F-B8 — P2 — P-009 mis-cited; latent P-010 violation preserved

A direct push to `main` is not a merge, so P-009 does not govern it; the applicable rule is P-010's
*"Ship MUST NOT commit or push directly to `main`"* — which is NON-NEGOTIABLE and, per its Violation
Action, **cannot be waived by operator approval**. `B-U3` preserves an "operator-authorised direct
push" path that is therefore illegal for the only eligible owner.

### F-B9 — P2 — Rollback-by-revert reasoning unsound (Constitution XI)

*"P-009 preserves merge-commit history, so revert is clean"* is a non-sequitur and backwards for the
post-merge case: reverting a **merge** commit requires `-m` parent selection, is not clean, and
impedes clean re-merge. No post-merge rollback is supplied. The pre-merge revert also advances HEAD
after the gate arms — re-arming review, the exact defect this program exists to eliminate.

## Correction allowance: NOT MET

The operator's standing rule permits one bounded correction + confirmation **only when findings are
mechanical and do not change architecture**.

F-B1 requires adding a staging-scoped intake to `_ship.agent.md` — **new architecture**, and
explicitly forbidden by the plan's own `I-B5`. F-B2 requires inventing a routing mechanism that does
not exist. Neither is mechanical. **Correction not attempted. Package B is BLOCKED.**

## Durable conclusion for re-decomposition

**There is no third role.** The staging-PR owner is necessarily **Ship**, and Package B must be
re-scoped to *include* a bounded, staging-scoped Ship intake unit (an additive Step 0.4/0.5-adjacent
entry point with explicit inputs, allowed mutations, readiness/wait behaviour, output, and return to
Orchestrator). `I-B5` must be narrowed from "no `_ship.agent.md` diff" to "no diff in `_ship.agent.md`
Steps 5, 6, or Session end" so the checkpoint/freeze exclusion survives while the intake becomes
expressible.

**Prerequisite for Package B v2**: operator disposition on `OD-B1` reduced to a single go/no-go on
Ship, plus authorisation to add the Ship staging intake.
