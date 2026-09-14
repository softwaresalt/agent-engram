---
title: "Checkpoint-resolution publication ordering — implementation plan (restage)"
doc_type: plan
date: 2026-09-13
status: blocked
source: docs/decisions/2026-09-13-checkpoint-resolution-ordering-restage-decision.md
plan_review_attempt: 2
review_verdict: FAIL
requires_plan_hardening: yes
---

**Source decision**: `docs/decisions/2026-09-13-checkpoint-resolution-ordering-restage-decision.md`
**Base**: `origin/main` @ `9ab53499`
**Scope**: documentation / agent-contract only. No source code, no tests, no
config, no repository settings, no tags.

This plan is a **fresh restage**. It does not inherit PR #396's architecture.
`143-*` IDs are abandoned and are not referenced as authority.

## Revision note (round 1 → round 2)

Round-1 independent review returned FAIL. This revision **narrows** scope; it
introduces no new architecture. Changes:

- **RQ-4 / U4 retired.** The Stage-symmetry unit rested on an ungrounded "Stage
  staging PR" premise. Stage-side exposure is now disclosed as RR-4, deferred.
- **RQ-5 rewritten.** "Zero active on `main`" contradicted the RR-2 residual it
  sanctioned. It is now a positive, checkable assertion evaluated against
  `origin/main`.
- **RQ-2 restated positively**, closing the letter-satisfiable loophole.
- **Terminal pause class explicitly removed from scope** (decision D1) rather
  than half-handled.
- **RQ-9 added** for the `null`-not-`[]` zero-checkpoint path.
- **Verification anchors corrected** — document order ≠ execution order in
  Step 6 (see U1 V1.2); greps made discriminating via a unique sentinel.

## Objective

Close gaps G1 (ordering), G2 (prohibition) and G3 (verification) for the
**committed non-terminal checkpoint class** defined in decision D1, eliminating
the stranded-commit mechanism that required PR #395.

This plan does **not** claim to fix the terminal pause class. That is decision
D3's separate deliberation.

## Critical ordering fact (do not misread)

In `_ship.agent.md`, **document order and execution order differ**:

- Step 6.0 item 4 — *"**After all closure work is committed**, push the branch
  and create a PR"* — appears at roughly line 756.
- The Step 6 closure-work items (the `1.`…`10.` list containing shipment
  archival, operational-closure, compact-context) appear **later**, around line
  820+.

So item 4 sits **earlier in the document** but **executes later**. Resolution
must be inserted into the closure-work item list (higher line number), and its
execution-order precedence over the push is inherited from item 4's own
"After all closure work is committed" wording. A verification asserting
"resolve line number < item-4 line number" would be **semantically inverted** and
is not used.

## Sentinel

Every edit made by this plan carries the literal sentinel string
`CHECKPOINT-PUBLICATION INVARIANT`. The file currently contains **zero**
occurrences (`git grep -c "CHECKPOINT-PUBLICATION INVARIANT" .github/agents/_ship.agent.md`
→ no match), so sentinel-based greps are discriminating rather than matching
pre-existing prose.

## Files in scope

| File | Tracked | Generated | Units |
|---|---|---|---|
| `.github/agents/_ship.agent.md` | yes | yes (`ship.agent.md.tmpl`) | U1, U2, U3 |

Generated artifact whose template is gitignored and lives outside this
repository (decision F8). Every unit therefore also satisfies RQ-8.

**On the three-units-one-file decomposition**: round-1 review flagged this as
possibly artificial (P2, advisory). It is retained because each unit is an
independently verifiable milestone with a distinct failure mode — U1 adds a
capability, U2 adds a prohibition, U3 adds a verification — and each can be
reverted alone. They are strictly sequential and carry explicit dependency edges.

## Units

### U1 — Add the checkpoint-resolution site to Ship Step 6 closure work

- **File**: `.github/agents/_ship.agent.md`
- **Domain**: docs (agent contract)
- **Size**: `S`   **Complexity**: `medium`
- **Requirements**: RQ-1, RQ-3, RQ-7, RQ-8, RQ-9
- **Depends on**: none (root)

**Change**: insert a numbered substep into the Step 6 **closure-work item list**
(the `1.`…`10.` sequence), positioned before the `backlogit_sync_index` item so
the committed resolution is indexed. The substep must:

1. enumerate active checkpoints owned by the current session;
2. **if none (`{"checkpoints": null, "total": 0}`), skip as a clean no-op** —
   explicitly not a halt (RQ-9);
3. otherwise resolve each via `backlogit checkpoint resolve <filename>` (RQ-7 —
   official command only; manual JSON edit and cherry-pick forbidden);
4. commit the resulting `.backlogit/checkpoints/` mutation onto the
   `post-merge/{feature_slug}` branch as part of closure work, so it is included
   in the push performed by Step 6.0 item 4;
5. carry the sentinel and the RQ-8 generated-file divergence note.

**Acceptance criteria**

- AC1.1 The substep appears **within** the Step 6 closure-work numbered list, at
  a line number **greater** than the `#### Step 6.0` heading, and **before** the
  `backlogit_sync_index` item.
- AC1.2 It names `backlogit checkpoint resolve` verbatim and forbids manual JSON
  edit and cherry-pick.
- AC1.3 It contains an explicit null/zero no-op branch (RQ-9).
- AC1.4 It carries the sentinel and the RQ-8 divergence note.

**Verification**

- V1.1 `git grep -n "backlogit checkpoint resolve" .github/agents/_ship.agent.md`
  → ≥1 hit.
- V1.2 **Execution-order anchor.** Capture three line numbers and assert
  `L(6.0 heading) < L(resolve substep) < L(Step 6 sync_index item)`:
  `git grep -n "#### Step 6.0" .github/agents/_ship.agent.md` (line 724 on
  `origin/main`),
  `git grep -n "CHECKPOINT-PUBLICATION INVARIANT" .github/agents/_ship.agent.md`,
  `git grep -n "Backlog index resync" .github/agents/_ship.agent.md` (line 860 on
  `origin/main`). **Use `"Backlog index resync"`, not a bare
  `backlogit_sync_index` grep** — the latter matches twice (Step 0.1 at line 122
  and the Step 6 closure item at line 860) and would be ambiguous.
  (Deliberately **not** compared against the item-4 text — see "Critical
  ordering fact".) The satisfiable insertion window is therefore lines 725–859.
- V1.3 `git grep -n "total\": 0\|no active checkpoints\|clean no-op" .github/agents/_ship.agent.md`
  → ≥1 hit inside the new substep, proving AC1.3.
- V1.4 `npx --no-install markdownlint-cli2 ".github/agents/_ship.agent.md"` → exit 0.

### U2 — Restate Session-end item 2 as a positive publication invariant

- **File**: `.github/agents/_ship.agent.md`
- **Domain**: docs (agent contract)
- **Size**: `S`   **Complexity**: `medium`
- **Requirements**: RQ-2, RQ-8
- **Depends on**: U1 (same file; U2 defers to the site U1 creates)

**Change**: rewrite Session-end item 2 so that it:

1. states resolution normally already happened in Step 6 closure work (U1), and
   that Session end is a **backstop**, not the primary site;
2. states the invariant **positively** (RQ-2): a resolution MUST land on `main`
   via the same carrying PR that is subsequently merged;
3. derives the prohibition from that invariant — committing a resolution to an
   already-merged branch, **or to any branch with no open carrying PR**, is
   forbidden because neither can deliver it to `main`;
4. names the PR #395 failure mode as the worked example;
5. carries the sentinel and the RQ-8 divergence note.

**Acceptance criteria**

- AC2.1 The invariant is stated positively (land on `main` via the carrying PR),
  not only as a negative prohibition.
- AC2.2 Both disqualifying cases are named: already-merged branch, and branch
  with no open PR.
- AC2.3 The unqualified instruction "resolve any still-active checkpoints" no
  longer stands without an ordering qualifier.
- AC2.4 Sentinel and RQ-8 note present.

**Verification**

- V2.1 `git grep -n "CHECKPOINT-PUBLICATION INVARIANT" .github/agents/_ship.agent.md`
  → ≥1 hit within the Session-end section (discriminating: sentinel count in the
  file is 0 before this plan).
- V2.2 `git grep -n "no open" .github/agents/_ship.agent.md` → ≥1 hit, proving
  AC2.2's second disqualifying case is present.
- V2.3 Read-back of the Session-end section confirming AC2.3 — the phrase
  "resolve any still-active checkpoints" is either removed or immediately
  qualified. Located via
  `git grep -n "resolve any still-active checkpoints" .github/agents/_ship.agent.md`.
- V2.4 `npx --no-install markdownlint-cli2 ".github/agents/_ship.agent.md"` → exit 0.

### U3 — Add post-merge landing verification against origin/main

- **File**: `.github/agents/_ship.agent.md`
- **Domain**: docs (agent contract)
- **Size**: `S`   **Complexity**: `medium`
- **Requirements**: RQ-5, RQ-6, RQ-8, RQ-9
- **Depends on**: U2

**Change**: add a verification step, executed after the carrying PR merges, that:

1. runs `git fetch origin main`, then evaluates checkpoint state **against
   `origin/main` explicitly** — not against the working tree. Round-1 review
   established that `backlogit checkpoint list` reads the checked-out branch, so
   a branch-local read can report success while the commit never landed. The
   contract must therefore direct an `origin/main`-scoped check, for example by
   inspecting the resolved file at that ref:
   `git show origin/main:.backlogit/checkpoints/<filename>` and confirming the
   resolved status, for each checkpoint the session resolved;
2. degrades to a clean no-op when the session resolved no checkpoints (RQ-9);
3. requires any **terminal pause class** checkpoint (decision D1, out of scope)
   to be recorded as an explicit operator-visible residual, stating plainly that
   this unit does not handle it (RQ-6);
4. reports and halts to the operator on mismatch — it does **not** attempt
   auto-recovery;
5. carries the sentinel and the RQ-8 divergence note.

**Acceptance criteria**

- AC3.1 The verification is scoped to `origin/main`, not the working tree, and
  names an executable command that reads that ref.
- AC3.2 The RQ-6 residual-recording obligation is explicit and states that the
  terminal pause class is **out of scope for this unit**.
- AC3.3 It reports and halts; it makes no auto-recovery claim.
- AC3.4 It degrades to a no-op when nothing was resolved (RQ-9).
- AC3.5 Sentinel and RQ-8 note present.

**Verification**

- V3.1 `git show origin/main:.backlogit/checkpoints/checkpoint-20260913-034100.json`
  executes and returns file content — proving the `origin/main`-scoped read
  named in the contract is a real, executable operation. (Confirmed available:
  this file is tracked and present on `main`.)
- V3.2 `git grep -n "origin/main" .github/agents/_ship.agent.md` → ≥1 hit inside
  the new verification step, proving AC3.1.
- V3.3 `git grep -n "CHECKPOINT-PUBLICATION INVARIANT" .github/agents/_ship.agent.md`
  → hit within the new verification step (discriminating via sentinel, since the
  word "residual" already occurs elsewhere in the file and would not be).
- V3.4 `npx --no-install markdownlint-cli2 ".github/agents/_ship.agent.md"` → exit 0.

## Dependency graph

```text
U1 ──▶ U2 ──▶ U3
```

- Root: **U1** (no predecessor). Terminal: **U3**.
- Edges: U1→U2, U2→U3. Two edges, three nodes, acyclic.
- All three units touch `_ship.agent.md` and are strictly sequential; every
  same-file adjacent pair carries a direct edge, and U1→U3 is transitively
  implied.

## Requirements coverage

| Requirement | Units |
|---|---|
| RQ-1 | U1 |
| RQ-2 | U2 |
| RQ-3 | U1 |
| RQ-5 | U3 |
| RQ-6 | U3 |
| RQ-7 | U1 |
| RQ-8 | U1, U2, U3 |
| RQ-9 | U1, U3 |

RQ-4 is retired (decision). No remaining requirement is uncovered; no unit is
requirement-free.

## Out of scope (explicit)

Tags, protected markers, rulesets, repository settings, `workspace_id`,
dark-mode auto-routing (Defect 1), lineage, cursors, locking, CAS, terminal
pause-class handling and crash-window auto-recovery (decision D1/D3), Stage-side
symmetry (RR-4), upstream template propagation (decision D6), vendoring
autoharness templates, minting any new policy ID.

## Plan hardening

`requires_plan_hardening: yes`. Hardening signals: the change modifies
**NON-NEGOTIABLE** agent-contract steps that gate merges, and it edits generated
artifacts (F8).

### H1 — Editing a NON-NEGOTIABLE gate

Step 6.0 is labelled NON-NEGOTIABLE. U1 inserts into Step 6's closure-work list.
**Mitigation**: U1 *adds* a substep and changes no existing gate's verdict
semantics. No existing NON-NEGOTIABLE check is removed, reordered relative to
another gate, or weakened. AC1.1 pins placement by **relative** position between
two stable anchors, so it is robust to upstream line drift.

### H2 — Generated-artifact divergence

Covered by decision F8/D6/RR-3 and RQ-8. Every unit carries the sentinel and the
divergence note. **Residual accepted and disclosed.**

### H3 — Extra review round on closure PRs

Resolving before final HEAD advances HEAD, re-arming §1.9 and P-018.
**Mitigation**: this is the existing intended behaviour of those gates (Ship
Step 5 item 15 already mandates an unconditional last-mile re-check), not new
machinery. No gate is bypassed.

### H4 — Terminal pause class

Out of scope per decision D1. **The plan makes no claim to fix it.** U3/RQ-6
require it to be surfaced as an explicit residual so it is not mistaken for
handled. Round-1 review established that forcing it into this unit produces
either a permanent operator halt or an unbounded PR recursion; both are avoided
by explicit exclusion.

### H5 — Stage-side exposure remains open

RR-4. Disclosed in the decision, deferred to separate scope. No silent gap.

### H6 — Index consistency

U1 places resolution **before** the `backlogit_sync_index` closure item so the
committed resolution is reflected in the index (round-1 P3).

### H7 — Strict-safety classification

`strict_safety.require_approval_for: [destructive]`. This plan performs **no**
destructive action: no settings mutation, no ruleset change, no tag creation, no
history rewrite, no file deletion. All edits are additive or in-place text
changes to one tracked Markdown file. **Classification: non-destructive — no
operator approval prerequisite beyond normal merge approval.**

## Verification summary

All verification commands are executable from the repository root with tooling
confirmed present this session: `git`, `backlogit` v1.10.1 (`checkpoint list`
confirmed, returning `{"checkpoints": null, "total": 0}`), and
`markdownlint-cli2` v0.23.2 via `npx --no-install` (confirmed exit 0).
No verification depends on an undefined input, a future PR number, a
`workspace_id`, a ruleset ID, or a non-existent merge flag.

## Effort

3 units × ≤2 h = **≤6 h**, single domain (docs), single file.


## Plan Review — round 1 (FAIL)

Two independent reviewers, cross-model (Scope Boundary Auditor / `gpt-5.6-sol`
xhigh; Correctness Reviewer / `claude-opus-4.8` high). Both returned **FAIL**.

Convergent P1 (found independently by both): `RQ-5` ("verify zero session-owned
checkpoints remain active on `main`") contradicted `RQ-6`/`RR-2`, which sanction
leaving a terminal merge-gate-pause checkpoint active. Additional P1s: `V1.2`
ordering anchor semantically inverted (document order != execution order in Step
6); `U3` read the working tree rather than `origin/main`, permitting false
success; `U4`/`RQ-4` rested on an ungrounded Stage "staging PR" premise.

## Plan Review — round 2, confirmation (FAIL) — TERMINAL

One bounded, scope-narrowing revision was applied (no new architecture), then a
single confirmation review by a fresh model (Correctness Reviewer /
`gpt-5.6-sol` xhigh).

The reviewer confirmed round-1 items 2-6 CLOSED: anchors corrected and verified
live (Step 6.0 at line 724, "Backlog index resync" at line 860, insertion window
725-859 real), `U3` now `origin/main`-scoped with a verified executable command,
`RQ-4`/`U4` retired with `RR-4` disclosed, `RQ-2` restated positively with both
disqualifying cases, `RQ-9` wired into `U1`/`U3`, sentinel confirmed absent
(count 0) hence discriminating, coverage intact, graph acyclic.

**But it returned a new, decisive P1 that terminates the cycle**: the `D1`
scope partition is not a partition, and the narrowed plan therefore does not fix
the reported incident.

**Independently verified by this session:**

- `git log origin/main -- .backlogit/checkpoints/checkpoint-20260913-034100.json`
  shows the file was committed via `513ec98a`, part of **PR #394**.
- `git show 5638122691796b1502bd55768ea41ac7adc60143:.backlogit/checkpoints/checkpoint-20260913-034100.json`
  → **`"status":"active"`** at the #394 merge commit.
- That same checkpoint's `phase` is `closure-pr-merge-gate-halt-operator-pause`.

So the incident checkpoint is simultaneously **in** the "committed non-terminal"
class (its file rode the carrying PR) and **in** the excluded "terminal pause"
class (created during the closure-PR approval pause). `D1` places it in scope and
out of scope at once. Excluding the terminal class excludes **the very instance
the unit exists to fix**, making the Objective's claim to eliminate the PR #395
mechanism an overclaim.

### Disposition: BLOCKED

Per the governing bounded-outcome rule — one revision permitted only if
corrections are bounded and introduce no new architecture, then one confirmation
review — the budget is exhausted at 2 attempts / 2 consecutive FAILs.

Closing this P1 is **not bounded**: it requires deciding whether intra-session
pause checkpoints should be committed to a tracked path at all, which is the
substrate/durability question `D3` deliberately deferred. That is new
architecture and is out of bounds for a revision.

**No harvest was performed. No backlog IDs were allocated. No shipment was
assembled. No replacement PR was opened.**

### What this result establishes (the durable finding)

This is the sixth-through-thirteenth consecutive independent failure of a
Defect-2 plan across PR #396 and this restage. The convergent evidence now
supports a structural conclusion rather than another revision attempt:

**Defect 2 cannot be closed by ordering discipline alone.** Every attempt to fix
it without a durability decision produces the same contradiction, because the
defect's own instance is a checkpoint that is created after its carrying PR's
final HEAD yet must be published by that PR. The genuine decision to make first
is: *should intra-session pause checkpoints be committed to a git-tracked path
at all?* Until that is deliberated and decided, no ordering plan can be
non-contradictory.

Recommended next step: a **fresh deliberation on checkpoint persistence
substrate** (tracked-vs-untracked pause checkpoints), explicitly upstream of any
ordering plan. The ordering fix should be re-planned only after that decision.
