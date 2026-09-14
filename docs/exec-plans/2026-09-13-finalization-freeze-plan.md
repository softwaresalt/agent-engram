---
title: "Finalization freeze + hybrid continuity — implementation plan"
doc_type: exec-plan
date: 2026-09-13
agent: stage
source: docs/decisions/2026-09-13-finalization-freeze-and-pr-backed-pause-deliberation.md
branch: chore/checkpoint-resolution-ordering-restage
status: reviewed-fail
review: docs/reviews/2026-09-13-finalization-freeze-plan-review-fail.md
---

## Source document

`docs/decisions/2026-09-13-finalization-freeze-and-pr-backed-pause-deliberation.md`
(Option F — Finalization Freeze + Hybrid Continuity). Intake stash `4EF24729`.

## Primary objective

Make it contractually impossible for a Git-tracked write to land after the final reviewed/approved
HEAD of a carrying PR, and replace the tracked terminal-pause record with an open-PR-backed
discovery surface plus an exhaustive open-PR scan that keeps P-001 enforceable without checkpoints.

## Scope

**Prompt / policy / documentation only.** Verified durable: `.github/agents` (23 tracked),
`.github/instructions` (34), `.github/policies` (2). No product code, no backlogit change, no
schema change, no new policy ID, no tag machinery.

**Duplication control**: the contract text exists **once**, in U1. Units U2–U7 insert short
pointers plus only the surface-specific ordering/transition each file actually needs. No unit
restates C-1…C-9.

## Constitution Check

| Principle | Assessment |
|---|---|
| Simplicity | One canonical contract file + six pointer edits. No new abstraction layer, no new tooling. |
| Anti-abstraction | Uses commands the agents already run (`git status --porcelain`, `git rev-parse`, `gh pr list/view`). No wrapper introduced. |
| Integration-first | Freeze is verified at the three existing gate points rather than at a new synthetic gate. |
| Test-first | Not applicable to prompt/policy artifacts; U8 substitutes a text-and-shape verification unit executed last. |
| Reversibility | Every unit is a documentation edit revertible by `git revert` of its own commit; no data migration, no gitignore change, no `git rm --cached`. |

## Hardening signals

* public API, schema, or **contract change** — **yes** (agent contracts, P-001/P-014 semantics)
* security, auth, permission, or compliance-sensitive behavior — no
* migration, backfill, destructive data/config action, or irreversible step — no
* external integration, **operator checkpoint**, or external dependency — **yes** (GitHub PR
  metadata; operator approval semantics; fail-closed handoffs)
* high runtime, rollout, or rollback risk — no (docs-only, per-unit revertible)

**Requires plan hardening: yes**

## Implementation units

Each unit is single-domain and sized under the 2-hour rule (<3 files, <5 sections, <4 verification
scenarios). `Size` / `Complexity` recorded as prose because the backlog registry declares **no
`features.sizing`** flag (degradation flagged; see Harvest notes).

---

### U1 — Canonical finalization-freeze contract (docs)

**File**: `.github/instructions/finalization-freeze.instructions.md` (new, 1 file)

Author the single normative contract, transcribing C-1…C-9 from the decision artifact verbatim in
intent: definitions; pre-freeze obligations and freeze-entry blockers; frozen-window prohibition and
the three enforcement gate points; unfreeze transition with mandatory approval invalidation; terminal
PR-backed pause with hints-only semantics and the live-revalidation resume requirement; open-PR
detection inputs/correlation/transitions; session-memory timing amendment; parity clause;
strict-safety classification table.

Every rule names an explicit **owner**, **inputs**, **source fields**, **transition**, and **failure
behavior**. No undefined policy ID, no workspace ID, no ruleset ID, no future PR number.

* **Acceptance**: file exists; contains all nine contract sections C-1…C-9; every normative rule
  names an owner and a failure behavior; contains zero occurrences of `P-022`.
* **Depends on**: none. **Size**: M. **Complexity**: medium. **Posture**: documentation-first.

---

### U2 — Policy amendments: P-001 and P-014 (policy)

**File**: `.github/policies/workflow-policies.md` (1 file)

1. **P-001** — extend Precondition: an **open release-unit / staging / closure PR** blocks claiming a
   second shipment **even when zero active checkpoints exist**; zero-candidate checkpoint state is
   explicitly **not** proof that no PR-backed pause exists. Add a pointer to U1's contract.
2. **P-014** — add the freeze re-verification requirement to the Precondition: clean tree and
   local==remote HEAD re-verified at (a) pre-local-review, (b) pre-approval-presentation, (c)
   last-mile; and record that approval is void across an unfreeze.
3. Append an entry to the **Amendment Log**.

No new policy ID is created.

* **Acceptance**: P-001 Precondition names the open-PR blocker and the zero-checkpoint
  non-sufficiency; P-014 Precondition names the three re-verification points and approval
  invalidation; Amendment Log has a new dated entry; catalog still ends at P-021.
* **Depends on**: U1. **Size**: S. **Complexity**: medium. **Posture**: documentation-first.

---

### U3 — Ship Step 5 ordering and freeze establishment (agent contract)

**File**: `.github/agents/_ship.agent.md`, Step 5 only (1 file)

Reorder so every tracked write precedes the readiness gates, then establish the freeze:

1. Move the three post-gate tracked-write items — `runtime-verification`, `operational-closure`, and
   follow-up stashing (currently the duplicate-numbered `7`, `8`, `9`) — to **before** item 7b.
2. Fix the **duplicate item `7`** numbering defect while in the file.
3. Insert the explicit **freeze point** immediately after the branch push (current item 10) and
   before item 7b, requiring the C-2.4 remote-HEAD proof and the C-2.5 unfiltered checkpoint
   re-enumeration with anomaly-first inspection.
4. Insert the C-3 clean-tree + HEAD-parity re-verification at the three gate points (pre-local-review,
   pre-approval at item 14, last-mile at item 15), each halting with a P-014 violation via P-005
   telemetry on failure.
5. Add a pointer to U1; do not restate the contract.

* **Acceptance**: no tracked-write item remains after item 7b; freeze point appears exactly once and
  precedes 7b; three re-verification checkpoints present; no duplicate item numbers in Step 5.
* **Depends on**: U1. **Size**: M. **Complexity**: high. **Posture**: characterization-first (record
  the current ordering before editing).

---

### U4 — Ship session-end carve-out and closure-PR parity (agent contract)

**File**: `.github/agents/_ship.agent.md`, `Session end` + Step 6.0 (1 file)

1. Amend `Session end` item 1: the mandatory `docs/memory/` artifact is written **pre-freeze**
   (C-2.3), states entry into the final merge gate, references the PR, and must not restate a
   HEAD-pinned gate verdict as current.
2. Amend `Session end` item 2: **remove** the post-freeze "leave at most one final best-effort
   checkpoint" allowance for terminal PR-backed pauses; checkpoint creation/resolution is a
   pre-freeze-only operation on a carrying branch. Pre-PR/offline crash checkpoints are explicitly
   preserved unchanged.
3. Apply the same freeze cycle to the Step 6.0 post-merge **closure PR** (its own C-2 → C-3 → C-5),
   noting the closure phase opens a new mutation phase after the feature merge.

* **Acceptance**: session-end memory write is pre-freeze; no contract path permits a tracked
  checkpoint or memory write during a post-freeze terminal pause on a carrying branch; pre-PR/offline
  checkpoint behavior explicitly retained; closure PR inherits the cycle.
* **Depends on**: U1, U3. **Size**: M. **Complexity**: high. **Posture**: characterization-first.

---

### U5 — Stage session-end parity and staging-PR pause (agent contract)

**File**: `.github/agents/_stage.agent.md`, `Session end` (+ Step 5.5 pointer) (1 file)

Mirror U4 symmetrically for Stage: pre-freeze final memory artifact; staging checkpoint resolved
before freeze; no post-freeze tracked write on `chore/stage-*`; the open **staging PR** is the pause
discovery surface. Record that no contract requires a future PR number at checkpoint creation.

* **Acceptance**: Stage `Session end` matches Ship's carve-out semantics; staging-PR pause names the
  open PR as the discovery surface; no future-PR-number requirement introduced.
* **Depends on**: U1, U4. **Size**: S. **Complexity**: medium. **Posture**: documentation-first.

---

### U6 — Orchestrator open-PR scan and P-001 detection (agent contract)

**File**: `.github/agents/_orchestrator.agent.md`, Step 0 + Step 0.0b note (1 file)

1. Add **Step 0.5 — Open Carrying-PR Enumeration**, running before queue selection: exhaustive
   paginated `gh pr list --state open --json number,title,headRefName,state,isDraft,updatedAt,labels,author`;
   correlation against `backlogit_list_shipments` (active/queued), `.backlogit/queue/{shipment_id}.md`
   manifests, harness branch conventions (`chore/stage-*`, `post-merge/*`, manifest-recorded feature
   branch), and body-cited backlog IDs; explicit provenance check.
2. Encode the C-6 transition table: one match → route exclusively to the owning agent;
   zero → proceed; multiple / ambiguous / malformed / closed-unmerged-with-active-work → **fail
   closed**; merged → confirm via `git merge-base --is-ancestor {merge_sha} origin/main` then ordinary
   post-merge closure gates; GitHub unavailable or pagination incomplete → **fail closed**.
3. Add a note at Step 0.0b: the zero-candidate continuation remains correct for **checkpoint
   recovery** but is **not** sufficient for P-001 — Step 0.5 is the P-001 authority.
4. Scan must not be narrowed by pause labels or body markers.

* **Acceptance**: Step 0.5 exists and precedes queue selection; all seven transitions present; scan
  is label-independent; Step 0.0b note added; Orchestrator never revalidates a carrying PR itself.
* **Depends on**: U1, U2. **Size**: M. **Complexity**: high. **Posture**: documentation-first.

---

### U7 — PR automation: readiness block and pause hints (instructions)

**File**: `.github/instructions/github-pr-automation.instructions.md` (1 file)

1. Extend the §1.9.2 readiness-block format with an optional `Pause phase:` line.
2. Add **§1.9.7 — PR-Backed Terminal Pause Hints**: enumerate the permitted non-Git surfaces (body
   block, one namespaced label `autoharness:paused-merge-approval`, one comment); state
   hints-only/never-authority/never-immutable/never-sufficient; state that PR metadata does not change
   HEAD; require live re-fetch + re-run of readiness/P-018/P-009 + HEAD-bound approval on resume;
   require fail-closed on incomplete thread pagination (consistent with §1.9.1).
3. Add the C-9 strict-safety classification table for PR-metadata mutations.

* **Acceptance**: §1.9.2 carries the pause-phase line; §1.9.7 exists with all four hint disclaimers
  and the three resume requirements; strict-safety table present and marks only removals/closes as
  `destructive`; no repository-settings mutation is implied anywhere.
* **Depends on**: U1. **Size**: S. **Complexity**: medium. **Posture**: documentation-first.

---

### U8 — Verification against live file text and CLI/API shapes (verification)

**Files**: `docs/verification/2026-09-13-finalization-freeze-verification.md` (new, 1 file)

Executed **last**, after U1–U7 are applied. Verifies against **exact current file text** and
**actual** CLI/API shapes — never against the plan's own prose.

**Shape verification** (must match live output, not assumption): `git status --porcelain`;
`git rev-parse HEAD` / `git rev-parse origin/<branch>`; `git merge-base --is-ancestor`;
`gh pr list --state open --json <fields>` (field names confirmed against `gh pr list --json` help);
`gh pr view --json headRefOid,body,state`; `gh pr edit --body-file`; `backlogit checkpoint list`
(no `status`/`agent` filter) and `backlogit checkpoint resolve <filename>` (confirmed: takes a
filename, no flags).

**Scenario matrix** — each row asserts the expected contract-defined outcome:

| # | Scenario | Expected outcome |
|---|---|---|
| 1 | Pre-freeze crash | Tracked checkpoint + memory path intact; ordinary owner-exclusive recovery |
| 2 | Zero active checkpoint + open carrying PR | Step 0.5 detects PR; P-001 blocks second claim |
| 3 | Multiple matching open carrying PRs | Fail closed → operator handoff |
| 4 | GitHub unavailable during pause discovery | Fail closed; no merge, no second claim |
| 5 | Tracked change attempted after freeze | Halt at re-verification; P-014 violation; unfreeze required |
| 6 | Review fix requiring unfreeze | Full C-4 sequence; prior approval void; fresh HEAD-bound approval |
| 7 | Closure PR pause | Same cycle applied to `post-merge/*` |
| 8 | Staging PR pause | Same cycle applied to `chore/stage-*`; Stage owns revalidation |
| 9 | Merged PR with pending closure | Confirmed against `main`; P-001 + P-020 closure gates govern |
| 10 | Closed-unmerged PR with active work | Fail closed → operator handoff |
| 11 | Clean terminal pause | No tracked write; PR body hint updated; HEAD unchanged |
| 12 | Stale or missing label/body hint | Discovery degraded only; live state authoritative; no unsafe progress |
| 13 | No PR yet (pre-PR work) | Existing tracked-checkpoint path unchanged |

* **Acceptance**: all 13 scenarios recorded with a live-verified expected outcome; every referenced
  CLI/API shape confirmed against actual tool output; any divergence recorded as a finding rather
  than silently reconciled.
* **Depends on**: U1–U7. **Size**: M. **Complexity**: medium. **Posture**: verification-last.

---

## Dependency graph

```text
U1 ──┬─> U2 ──┬─> U6 ──┐
     ├─> U3 ──> U4 ──> U5 ──┤
     └─> U7 ─────────────────┴─> U8
```

Suggested execution order: **U1 → U2 → U3 → U4 → U5 → U6 → U7 → U8**.

## Runtime verification and closure

No unit changes a runtime surface (CLI, API, UI, background jobs). Runtime verification is
**non-applicable**; U8 is the substituting contract-text verification. Operational closure records:
monitoring = the three C-3 re-verification points exercised on the next carrying PR; rollback trigger
= any halt at a re-verification point that cannot be explained by a real tracked write; owner = Ship
for feature/closure PRs, Stage for staging PRs, Orchestrator for C-6 detection; validation window =
the first full Stage→Ship cycle after merge.

## Rollback

Per-unit `git revert` of that unit's commit. No `.gitignore` change, no `git rm --cached`, no data or
schema migration, so no restore-net impact (the P-007/P-015 exposure that blocked Option D does not
arise here).

## Out of scope

Executable enforcement tooling; any backlogit change; gitignoring `.backlogit/checkpoints/`;
narrowing `git add .backlogit/`; PR #396 disposition; RR-3 upstream template divergence; any
repository-settings change. If review establishes prompt-level enforcement is insufficient, that is a
**separate release unit** — this plan does not claim mechanical enforcement.

## Plan Hardening

Invoked per P-006 because the plan declares `Requires plan hardening: yes`.

### H-1 Risk triggers and protected invariants

| Trigger | Protected invariant that must survive this change |
|---|---|
| Contract change across 4 agent files | **I1**: pre-PR / offline crash continuity keeps working exactly as today — the tracked-checkpoint path is narrowed only on a *carrying branch after freeze*, never generally. |
| P-001 semantics amended | **I2**: P-001 becomes strictly *more* conservative. No path may newly permit a second claim that is blocked today. |
| Operator-checkpoint semantics | **I3**: no path may ever *widen* merge authority. Approval remains explicit, HEAD-bound, and void across unfreeze. |
| External integration (GitHub) | **I4**: GitHub unavailability must fail closed. It may never cause a merge or a second claim. |
| Recovery protocol interaction | **I5**: the owner-exclusive, operator-confirmed, anomaly-first checkpoint recovery state machine in Ship/Stage/Orchestrator is **not** modified by any unit. |
| Removing a durable record | **I6**: every finalization retains a committed pre-freeze memory handoff **plus** live PR state. Persistence is never reduced to zero. |

### H-2 Learnings and instructions consulted

* `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md` — each push re-arms
  Copilot; the post-push/pre-review window transiently reports clean. Directly motivates C-3.
* `docs/compound/gh-reviews-endpoint-paginate-hides-head-review-2026-07-22.md` — unpaginated review
  queries hide the HEAD review. Reinforces the fail-closed pagination requirement in U6 and U7.
* `docs/compound/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md` —
  mandates the cross-model persona diversity used at the review gate.
* `docs/memory/2026-09-13/139-s-checkpoint-resolution-remediation.md` — Ship independently refused a
  HEAD-pinned verdict restatement; precedent for C-2.3.
* Instructions re-read: `github-pr-automation.instructions.md` §1.9.1–§1.9.4;
  `strict-safety.instructions.md`; `escalation-protocol.instructions.md`;
  `concurrency.instructions.md`.

### H-3 Risky actions (strict-safety vocabulary)

| ProposedAction | change_kind | ActionRisk | Approval | Expected ActionResult |
|---|---|---|---|---|
| Edit 4 agent contract files (U3–U6) | local edit | `moderate` | not required | contracts updated, recovery state machines untouched (I5) |
| Amend P-001 / P-014 (U2) | config change | `moderate` | not required | policies strictly more conservative (I2, I3) |
| Create 2 new docs (U1, U8) | local edit | `low` | not required | additive only |
| PR body / label / comment hint update (runtime behavior introduced by U7) | external call | `moderate` | not required | HEAD unchanged; hint non-authoritative |
| Label removal / comment deletion / PR close | external call | `destructive` | **operator approval required** | never performed by this plan |
| Repository settings mutation | — | — | — | **explicitly never performed** |

### H-4 Added verification depth

1. **Negative test in U8**: assert no *new* `.github/**` path instructs a tracked write after a
   freeze point. Implemented as an exact-text scan for the tracked-write verbs
   (`checkpoint create`, `Write a … memory`, `git add`, `git commit`) appearing *after* the freeze
   marker within Ship Step 5 / Session end and Stage Session end.
2. **Invariant-I1 test**: U8 scenario 13 (no PR yet) must show the pre-PR checkpoint path byte-
   unchanged in behavior.
3. **Invariant-I5 test**: U8 diffs the Crash-Resumption sections of all three agents pre/post and
   asserts zero semantic change.
4. **Shape-drift test**: every CLI/API shape in U6/U7/U8 is confirmed against live `--help` / `--json`
   output at verification time, not against this plan's prose. A divergence is recorded as a finding.
5. **Ordering test (U3)**: assert zero tracked-write items remain after the first readiness gate, and
   that Step 5 contains no duplicate item numbers.

### H-5 Rollback coupling and ordering

Units are independently revertible, but **U3 and U4 are coupled**: U3 moves tracked writes before the
gates and U4 removes the post-freeze session-end writes. Reverting U4 alone would restore a
post-freeze tracked write while U3's freeze point remains — an inconsistent contract.

**Rollback rule**: revert U4 ⇒ revert U3 in the same operation. U5 depends on U4 for its parity text,
so revert U5 first. Safe revert order: **U8 → U7 → U6 → U5 → U4+U3 (together) → U2 → U1**.

No unit performs an irreversible action; there is no `.gitignore` change, no `git rm --cached`, and no
archive mutation, so the P-007 / P-015 restore-net exposure that blocked Option D does not arise.

### H-6 Operator checkpoints

| Checkpoint | Who | When |
|---|---|---|
| Plan review verdict | Operator | before harvest (PASS required; P2-only = ADVISORY, no harvest without explicit policy) |
| Shipment claim | Operator via Orchestrator | Stage hands off `shipment_id` only |
| Any `destructive` PR-metadata action | Operator | per H-3 |
| Unfreeze approval refresh | Operator | every unfreeze, per C-4.6 |

### H-7 Residual risks accepted

1. **Prompt-level, not mechanical, enforcement.** The freeze is enforced by agent adherence to
   contract text plus three concrete command checks. It is not a Git hook. Explicitly bounded in
   *Out of scope*; if review judges this insufficient, it becomes a separate release unit rather than
   a silent overclaim.
2. **Freeze-window crash loses in-memory session state.** Bounded by the committed pre-freeze
   handoff and the open PR; resume re-derives everything live regardless.
3. **`gh pr list` throttling on very large open-PR sets.** Mitigated by fail-closed on incomplete
   pagination (I4); a partial scan is never treated as a complete one.
4. **Upstream autoharness template divergence (RR-3).** These edits live in this repository's
   `.github/`; the upstream templates are not in this repo. Recorded as a known residual, unchanged
   by this plan.

### H-8 Monitoring and validation window

* **Monitoring**: the three C-3 re-verification points on the next carrying PR; the Step 0.5 open-PR
  scan result recorded in each Orchestrator state summary.
* **Rollback trigger**: any C-3 halt not explained by a genuine tracked write, or any Step 0.5
  fail-closed on a correctly-formed single carrying PR.
* **Owner**: Ship (feature/closure PRs), Stage (staging PRs), Orchestrator (C-6 detection).
* **Validation window**: the first complete Stage → Ship cycle after merge.

<!-- plan-review-attempt: 1 -->
