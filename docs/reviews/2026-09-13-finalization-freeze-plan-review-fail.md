---
title: "Plan review — finalization freeze + hybrid continuity (FAIL)"
doc_type: review
date: 2026-09-13
agent: stage
plan: docs/exec-plans/2026-09-13-finalization-freeze-plan.md
decision: docs/decisions/2026-09-13-finalization-freeze-and-pr-backed-pause-deliberation.md
verdict: FAIL
attempt: 1
---

## Gate configuration

Full `plan-review` with cross-model persona diversity (4 personas, 4 providers, 4 model families):

| Persona | Model | Provider |
|---|---|---|
| Correctness Reviewer | `gpt-5.6-sol` | openai |
| Architecture Strategist | `claude-opus-4.8` | anthropic |
| Scope Boundary Auditor | `grok-4.6` | xai |
| Constitution Reviewer | `claude-opus-5` | anthropic |

Hardening was required (`Requires plan hardening: yes`) and satisfied before review (P-006).

## Finding tally

| Persona | P0 | P1 | P2 | P3 |
|---|---|---|---|---|
| Correctness | 0 | 9 | 1 | 0 |
| Architecture | 0 | 2 | 6 | 1 |
| Scope Boundary | 0 | 8 | 6 | 0 |
| Constitution | 2 | 13 | 6 | 5 |
| **Total** | **2** | **32** | **19** | **6** |

**Verdict: FAIL.** PASS requires zero P0/P1.

## Decisive findings (architecture-changing — correction allowance NOT met)

### P0-1 — P-010 forbids Stage from owning a staging PR (Constitution; corroborated by Correctness)

C-8 parity and U5 assign staging-PR ownership, freeze, readiness, and approval to Stage. P-010's
Stage **Forbidden** column states verbatim: *"Create, push, or merge pull requests."* Compounding it,
the live sequencing is that **Stage ends before the staging PR exists** — Orchestrator Step 1.5
creates `chore/stage-{shipment_id}` *after* Stage completes. So C-2.3 requires Stage's pre-freeze
memory to reference a PR that does not yet exist, and U5 assigns Stage a revalidation duty it is
barred from performing.

The **Stage-parity pillar of the authorized architecture is not executable** under the live role
contract. Fixing it requires either amending P-010 (new role-boundary architecture) or reassigning
staging-PR ownership (different architecture). Neither is a bounded correction.

### P0-2 — P-021 C2 stash capture has no legal disposition inside the frozen window (Constitution)

C-3 prohibits every tracked write and names "backlog write" explicitly; C-1 lists
`.backlogit/stash.jsonl` as tracked. P-021 C2 makes stash capture an **unconditional precondition**
for closing an out-of-scope finding. A finding discovered during the post-freeze local review or the
P-018 wait therefore has *no* legal disposition: capture is forbidden, skipping capture is a P-021
violation, and unfreezing to capture defeats the freeze.

### P1 — The freeze is not a partition on its own failure path (Correctness; Constitution P-005)

C-3 requires recording a P-014 violation via **P-005 telemetry** before unfreezing. P-005 records
violations in memory checkpoints — itself a tracked write. A dirty/diverged frozen state is therefore
simultaneously on the no-write side and a mandatory-write side.

**This is Attempt 1's exact failure mode recurring**: the proposed partition is not a partition.

### P1 — Post-freeze tracked-write paths remain wide open (Correctness; Constitution P-020, P-007/P-015)

U4 closes only Ship `Session end` items 1–2. Still-live post-freeze tracked-write paths:
`compact-context` (P-020, `target: all`, **mandatory per merge**), closure-artifact compaction status,
`compound` / `learn` / `evolve`, circuit-breaker checkpoints, context-overflow memory writes, the
escalation-protocol `docs/memory/` checkpoint, and P-007/P-015 `git restore` + `git add` archive
remediation. Stage retains the same set.

**This is Attempt 2's P1 #1 recurring**, merely relocated: the residual tracked-doc cycle survives.

### P1 — C-6 makes P-001 *less* conservative, breaching the plan's own invariant I2 (Constitution)

The transition row *"Zero matching open carrying PRs → proceed to ordinary queue selection"* is stated
unconditionally as the sole outcome of the P-001 authority check. A merged release unit whose
post-merge closure is incomplete (missing tag/publish, or unset P-020 compaction status) has **no
open PR**, so a zero-PR scan would *authorize* a second shipment that P-001's existing Precondition
blocks today. The scan must be an **additional** blocker and never a clearance. This is a safety
regression, not an omission.

### P1 — Simultaneous Stage + Ship carrying PRs are legitimate, yet C-6 fail-closes on them (Correctness; Architecture)

C-8 explicitly allows Stage and Ship to hold independent freeze cycles, and planning-overlap mode
permits Stage activity while Ship awaits merge. C-6 classifies *any* multiple match as ambiguous →
fail closed, potentially deadlocking both frozen agents. C-6 and C-8 contradict each other.

### P1 — Draft and superseded open PRs wedge the Orchestrator immediately (Architecture)

C-6 fetches `isDraft` and `updatedAt` but no transition consumes them, and there is no category for a
superseded-but-still-open PR. Applied to this repository's **current** state — **#390** (draft,
blocked) and **#396** (OPEN, superseded) — the Orchestrator either fail-closes on every session start
or perpetually routes revalidation of a blocked draft.

### P1 — C-6 correlation fails OPEN (Architecture; Correctness)

`gh pr list --limit <n>` provides no exhaustion proof; the field list omits `body` although body-cited
IDs are a declared correlation input; shipment manifests do not in fact carry the asserted feature
branch field. A harness-authored carrying PR with a drifted branch name and no body-cited ID
correlates to "zero matches → proceed". The one place the design removes the checkpoint is the one
place a missing hint now un-gates P-001.

### P1 — C-4 unfreeze loop is unbounded (Correctness; Constitution P-021)

No cycle bound and no terminal disposition. Reachable: moving follow-up stashing pre-freeze means the
final local review can discover a new follow-up, which the live contract says must be stashed —
forcing unfreeze → push → re-review, which may discover another, indefinitely. No P-021 C1 scope test
is applied to unfreeze triggers.

### P1 — Overclaimed enforcement (Scope)

"Make it contractually **impossible**" and "the freeze is **enforced by** commands the agents already
run" contradict H-7.1's own admission of prompt-level, not mechanical, enforcement. `git status` /
`git rev-parse` detect after the fact; they do not prevent.

### P1 — Unit sizing and duplication-control claims are false (Scope)

U1 authors nine contract sections (>5); U8 packs 13 scenarios plus five hardening tests (>4) and mixes
docs with live CLI execution; the "no unit restates C-1…C-9" claim is contradicted by U2, U3 items
3–4, U6 item 2 ("Encode the C-6 transition table"), and U7 item 3 ("Add the C-9 strict-safety
classification table" — literally duplicated from U1).

## Notable P2s (recorded, not gating)

* Freeze is defined as a **branch** property but has **no durable substrate**; after a crash "am I
  frozen?" is unanswerable. The plan is not consistent about branch vs. PR vs. session scope.
* U3/U4 decomposition smell: same file, one invariant, split by heading; forward application leaves
  `_ship.agent.md` self-contradictory between commits.
* Fail-closed GitHub dependency is scoped to *all* queue selection rather than to release-unit
  *claiming*, so a GitHub outage wedges even pre-PR planning (P-012 degradation undeclared).
* The §1.9.2 readiness block is overloaded as both authoritative gate evidence and non-authoritative
  pause hint — and since the freeze's defining property is that HEAD does *not* advance, a stale
  "hint" revalidates as current-HEAD authority.
* Constitution Check table uses a Simplicity/Anti-abstraction/Integration-first/Test-first rubric that
  is **not this workspace's constitution** (Principles I–XI + two overlays are unmapped).
* P-002/P-004 disposition for a docs-only chore is undeclared → Ship may halt at Step 2/3.

## Catalog integrity

**Confirmed clean.** No undefined policy reference is introduced. Both `P-022` occurrences are
negative assertions. The catalog still ends at P-021. No invented workspace IDs, ruleset IDs, or
future PR numbers were found. (One Scope finding flags the namespaced label as "tag machinery"; the
operator's authorized architecture §3 does permit "an optional namespaced label/comment", so this is
recorded as a partial false positive — though the specific label *name* was plan-invented.)

## Gate decision

The operator's standing rule permits **one** correction + confirmation review only when P0/P1 findings
are **bounded corrections that do not change architecture**.

Not met. At minimum six findings are architecture-level:

1. Stage staging-PR parity is unexecutable without amending P-010 or reassigning ownership.
2. P-021 C2 capture has no legal disposition in the frozen window.
3. The P-005 telemetry path breaks the partition — Attempt 1's failure mode, recurring.
4. The residual tracked-doc cycle survives via P-020/compound/learn/circuit-breaker/escalation —
   Attempt 2's failure mode, recurring.
5. C-6 makes P-001 strictly *less* conservative — a safety regression against the plan's own I2.
6. C-6 and C-8 contradict each other on simultaneous Stage + Ship carrying PRs.

**Verdict: FAIL. Harvest NOT performed. No backlog IDs, no shipment created.**

## Durable conclusion for the next attempt

The finalization-freeze *thesis* is correct and is materially stronger than Options A–E: it correctly
identifies the defect class as "any tracked write after final reviewed HEAD" rather than "checkpoints",
and it correctly identifies that removing the checkpoint requires replacing P-001 detection. Both
prior attempts' decisive P1s are genuinely addressed at the level of intent.

What it does **not** yet survive is the interaction surface. Three unresolved structural questions
must be settled *before* any further plan is written:

1. **The freeze needs an exempt class, or it is not a partition.** P-005 telemetry, P-021 C2 capture,
   and P-007 archive restoration are all mandatory-write obligations that can fire inside the window.
   Either they get a declared non-Git substrate, or they become declared unfreeze triggers with a
   bounded cycle count, or the freeze admits a narrow exempt set. Until this is decided, every freeze
   variant will keep failing the same partition test.
2. **Staging-PR parity must be reconciled with P-010 and with Orchestrator Step 1.5 sequencing.**
   Stage cannot own a PR it is forbidden to create and which does not exist while Stage is running.
   Either P-010 is amended, or the staging PR's freeze belongs to whoever creates it.
3. **The open-PR scan must be specified as an additional blocker with an explicit PR taxonomy**
   (draft / superseded / external / harness-authored-uncorrelated), never as a clearance, and
   reconciled with planning-overlap's legitimate two-PR state.

Recommended next step: a **narrow, targeted deliberation on the freeze exemption model** (question 1),
strictly upstream of any further freeze plan. Questions 2 and 3 are each independently plannable once
question 1 is settled.

<!-- plan-review-attempt: 1 — FAIL -->
