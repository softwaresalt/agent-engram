---
title: "Package A — Freeze exception and tracked-write taxonomy"
doc_type: exec-plan
date: 2026-09-13
agent: stage
package: A
program: docs/decisions/2026-09-13-finalization-write-boundary-program-deliberation.md
depends_on: [B]
status: draft
---

## Problem Frame

Three prior attempts established, and this session re-verified against live file text, that the
Ship lifecycle performs **mandatory tracked repository writes after the gates that arm on HEAD**.

Verified sites in `.github/agents/_ship.agent.md` (1109 lines):

| Site | Line / item | Tracked write | Position relative to gates |
|---|---|---|---|
| Session memory summary | Step 5 item 2 (`:561`) | `docs/memory/` | **before** PR creation — already legal |
| P-014 readiness gate | Step 5 item 7b | — | gate arms on `headRefOid` |
| P-018 Copilot gate | Step 5 item 7c | — | gate arms on `headRefOid` |
| runtime-verification evidence | Step 5 item **7** (duplicate number) | evidence artifact | **after** 7b/7c |
| operational-closure artifact | Step 5 item **8** | `docs/closure/` | **after** 7b/7c |
| P-021 C2 follow-up stash | Step 5 item **9** | `.backlogit/stash.jsonl` | **after** 7b/7c |
| Push branch | Step 5 item **10** | — | HEAD advances **after** gates |
| Backlog archival commit | `:836` | `git add .backlogit/` | post-merge branch — legal |
| compact-context (P-020) | `:859` | `docs/memory/`, `docs/exec-plans/`, `docs/closure/`, `docs/archive/` | post-merge branch — legal |
| Circuit-breaker record | `:931` | `docs/memory/` | **unbounded phase** |
| Mid-session checkpoint | `:1035`, `:1044` | `docs/memory/` + `.backlogit/checkpoints/` | **unbounded phase** |
| Session-end final memory | `:1057` | `docs/memory/` | **after** Step 6 item 10 (`checkout main`) |
| Session-end checkpoint resolve | `:1058` | `.backlogit/checkpoints/` | **after** Step 6 item 10 |

Stage's own `Session end` items 1–2 are textually identical → symmetric exposure.

Two failure shapes result:

* **Shape 1 — gate re-arm.** A post-gate write that is pushed advances `headRefOid`, invalidating
  the §1.9 and P-018 results. `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md`:
  every push re-arms Copilot. `docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md`
  Family 6 measured this: **4 of 11 review rounds on PR #385 were triggered by doc/PR-body-only pushes**,
  and *"Reviewed-HEAD/checkpoint metadata written as its own push is self-invalidating."*
* **Shape 2 — orphaned write.** A write performed after the carrying branch merged never reaches
  `main`. This is the originating defect (stash `4EF24729`): `checkpoint-20260913-034100.json` was
  resolved in commit `43e70430` **after** PR #394 merged at `56381226`, so `main` still reads
  `status: active`.

**The missing artifact is a classification.** Prior attempts tried to *design a new substrate* or
*declare freeze exemptions*; both failed review because neither produced a partition. What does not
yet exist anywhere in `.github/` is a written statement of **which lifecycle phase each mandatory
write belongs to and where it may legally land**.

## The available destination (prior art)

`docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md`
already sanctions a destination for this exact class:

> Three recognized carry-forward classes — `.backlogit/stash.jsonl` (append-only backlog intake),
> `docs/memory/**` (mandatory session continuity records), and intentional `.gitignore` updates —
> must be validated and landed through the **NEXT staging branch/staging PR**, merged, and main
> returned clean BEFORE Ship claims the release-unit shipment. Never resolve via `git stash`,
> deletion, checkout, or silent exclusion.
>
> *"Session-end memory writes and stash updates are **inputs to the next staging round**, not
> merge-gate-time commits on the implementation branch."*

Two of the three write classes in that doc are already routed this way. Package A's work is to
**extend the same routing to the remaining classes and write it down as contract**, not to invent a
mechanism. This is why A depends on B: the next staging round must have a legal owner before A can
route writes to it.

## Requirements Trace

| Req | Source | Statement |
|---|---|---|
| A-R1 | Program S1 | Every mandatory tracked write is classified; **no write left unclassified** |
| A-R2 | carry-forward compound doc | Each class names a **reachable** destination: pre-gate implementation branch, post-merge closure branch, or next staging round |
| A-R3 | P-005 `:96-112` | Violation telemetry (*"Record violation in memory checkpoints"*) is classified |
| A-R4 | P-021 C2 `:702-740` | Deferred-capture stash writes are classified; capture remains an unconditional precondition |
| A-R5 | P-007 `:137-159`, P-015 `:410-447` | `git restore` archive restoration is classified |
| A-R6 | circuit-breaker `:36-53` | `docs/memory/{date}/circuit-break-*.md` records are classified |
| A-R7 | P-020 `:643-700` | compact-context outputs are classified |
| A-R8 | Stage/Ship Session end | continuous-learning (`learn`/`evolve`) and mandatory session memory are classified |
| A-R9 | Program decision | Package A changes **policy/contract text only** — zero checkpoint-ordering implementation |
| A-R10 | Package B | The next-staging-round destination is cited only after B establishes its owner |

## Implementation Units

### A-U1 — Author the write-class taxonomy skeleton

* **Domain**: docs (instruction)
* **Files**: new `.github/instructions/tracked-write-boundary.instructions.md`
* **Change**: Define three destination classes and their membership rule:
  `PRE_GATE` (implementation branch, before the §1.9/P-018 arm), `CLOSURE_BRANCH`
  (`post-merge/*`, before its own PR's final HEAD), `NEXT_STAGING_ROUND` (deferred to the following
  `chore/stage-*` round per the carry-forward compound doc). Define `UNCLASSIFIED` as a **hard
  error**, not a fallback.
* **Atomic milestone**: the file declares exactly three destination classes plus `UNCLASSIFIED`, and
  states that `UNCLASSIFIED` halts.
* **Verification**: `Select-String -Path .github/instructions/tracked-write-boundary.instructions.md -Pattern "PRE_GATE|CLOSURE_BRANCH|NEXT_STAGING_ROUND|UNCLASSIFIED"`
  returns ≥ 4 hits.

### A-U2 — Classify the Ship Step 5 write sites

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/tracked-write-boundary.instructions.md`
* **Change**: Add a classification table covering `_ship.agent.md` Step 5 items 2, 7 (runtime
  verification), 8 (operational closure), 9 (P-021 C2 stash), and 10 (push), each with its line
  reference and assigned class. Record the **duplicate item number `7`** in Step 5 as a noted
  defect without renumbering it (renumbering is out of scope and belongs to E).
* **Atomic milestone**: all five Step 5 sites appear in the table with a class assigned and a line
  citation.
* **Verification**: the table contains 5 rows; `Select-String ... -Pattern "duplicate item number"`
  returns ≥ 1 hit.

### A-U3 — Classify the Ship Step 6 and session-end write sites

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/tracked-write-boundary.instructions.md`
* **Change**: Add rows for `:836` (backlog archival), `:859` (P-020 compact-context), `:931`
  (circuit breaker), `:1035`/`:1044` (mid-session checkpoints), `:1057` (session-end memory),
  `:1058` (session-end checkpoint resolve), and Ship Step 6 items 6 (closure follow-up stash), 7
  (source artifact cleanup) and 11 (`learn`/`evolve`).
* **Atomic milestone**: every line reference listed in this plan's Problem Frame table appears in
  the classification table exactly once.
* **Verification**: `Select-String ... -Pattern ":1057|:1058|:931|:859|:836"` returns ≥ 5 hits.

### A-U4 — Classify Stage's symmetric session-end exposure

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/tracked-write-boundary.instructions.md`
* **Change**: Add rows for `_stage.agent.md` `Session end` items 1–2 and the mid-session checkpoint
  milestones, noting the exposure is textually identical to Ship's and receives the same
  classification.
* **Atomic milestone**: the table contains a Stage section with ≥ 2 rows.
* **Verification**: `Select-String ... -Pattern "_stage.agent.md"` returns ≥ 2 hits.

### A-U5 — State the completeness gate

* **Domain**: docs (instruction)
* **Files**: `.github/instructions/tracked-write-boundary.instructions.md`
* **Change**: State the A-R1 completeness obligation as a reviewable invariant: any mandatory
  tracked write introduced by a future contract edit must be added to this table, and an
  unclassified write is a halt condition. Cross-reference P-005 for the violation record.
* **Atomic milestone**: the section states the invariant and names P-005 as the violation channel.
* **Verification**: `Select-String ... -Pattern "P-005"` returns ≥ 1 hit.

### A-U6 — Anchor the taxonomy in the policy catalog

* **Domain**: docs (policy)
* **Files**: `.github/policies/workflow-policies.md`
* **Change**: Add a cross-reference from P-005, P-020, and P-021 C2 to the new instruction file so
  the taxonomy is discoverable from the policy catalog. **Additive only** — no existing normative
  sentence is edited.
* **Atomic milestone**: `git diff --numstat .github/policies/workflow-policies.md` reports `0`
  deletions.
* **Verification**: `git diff --numstat .github/policies/workflow-policies.md` deletion column is `0`.

### A-U7 — Full-surface consistency verification

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
A-U1 ──> A-U2 ──> A-U3 ──> A-U4 ──> A-U5 ──> A-U6 ──> A-U7
```

Strictly linear (single growing file, then a policy anchor, then verification).

**Package-level**: `A depends on B`. A-U1's `NEXT_STAGING_ROUND` class cites the staging round
established by Package B. A must not be harvested into an executable shipment until B has shipped.

## Decisions and Rationale

* **D-A1 — Classification, not substrate.** Attempts 2 and 3 failed because they proposed a new
  persistence substrate or a freeze-exemption list. The carry-forward compound doc supplies a
  sanctioned destination that already handles two of three write classes, so A is a document-sized
  unit rather than an architecture change.
* **D-A2 — `UNCLASSIFIED` is a hard error, not a fallback.** A permissive fallback is what let
  attempts 1–3 each ship a "partition" that was not one. Making the gap loud is the whole value.
* **D-A3 — A changes no ordering.** Ship Step 5's duplicate `7` and the session-end ordering are
  **recorded as findings** and left in place. Fixing them is Package E and is explicitly excluded
  here, so a reviewer can confirm A's diff contains no reordering.
* **D-A4 — Docs-only.** No Rust source, test, or configuration file is modified.

## Risks and Caveats

| R | Risk | Mitigation |
|---|---|---|
| A-K1 | A write site is missed, so the taxonomy is incomplete | A-U3's milestone pins the classification table against this plan's verified Problem Frame table, one row per site |
| A-K2 | A reviewer reads A as implementing the ordering fix | D-A3 and A-U2's "record, do not renumber" milestone make the exclusion diff-checkable |
| A-K3 | A ships before B, leaving `NEXT_STAGING_ROUND` ownerless | Explicit `blocks` dependency edge B → A recorded at harvest; A not claimable until B ships |
| A-K4 | The taxonomy conflicts with P-021 C2's unconditional capture precondition | A classifies C2 capture's **destination**; it does not make capture conditional. A-R4 states this explicitly |
| A-K5 | Future contract edits silently bypass the table | A-U5 states the completeness invariant with P-005 as the violation channel |

## Plan Hardening Signals

| Signal | Present | Evidence |
|---|---|---|
| Modifies a workflow policy | **yes** | A-U6 touches `.github/policies/workflow-policies.md` |
| Changes role authority boundaries | no | classification only |
| Affects a safety/merge gate | **yes** | reclassifies writes relative to §1.9 / P-018 arming |
| Depends on another unshipped package | **yes** | depends on B |
| Touches security/authn/authz | no | — |
| Touches concurrency/parallelism | no | — |
| Migration or data change | no | — |
| Cross-cutting blast radius | **yes** | classifies writes across Ship, Stage, and 6+ policies |

**Requires plan hardening: yes**

## Runtime Verification and Closure

Documentation-only change set; full local build is **non-applicable** and must be recorded as such
in PR readiness evidence. Verification evidence = the four A-U7 commands.

## Plan Hardening

### Hardening required — why

`Requires plan hardening: yes`. Four signals present: (1) modifies a workflow policy (A-U6);
(2) **affects a safety/merge gate** — it reclassifies writes relative to the §1.9 and P-018 arming
points; (3) **depends on another unshipped package** (B); (4) **cross-cutting blast radius** — it
classifies writes spanning Ship, Stage, and six policies.

### Context consulted

* `.github/policies/workflow-policies.md` — P-005 `:96-112`, P-007 `:137-159`, P-015 `:410-447`,
  P-020 `:643-700`, P-021 C2/C3/C5/C6 `:702-740`
* `.github/agents/_ship.agent.md` Step 5, Step 6.0, Step 6, Session end (`:543`–`:1058`)
* `.github/agents/_stage.agent.md` Session end
* `.github/instructions/circuit-breaker.instructions.md:36-53`
* `.github/skills/compact-context/SKILL.md:70-90` (destination paths)
* `docs/compound/workflow-issues/carry-forward-pipeline-state-before-next-claim-2026-09-08.md`
  — **the load-bearing prior art**: sanctions the next staging round as the destination
* `docs/compound/copilot-review-merge-gate-wait-for-head-review-2026-07-11.md` — `commit_id == HEAD`
* `docs/compound/best-practices/proactive-copilot-review-pattern-checklist-2026-09-08.md` Family 6
* Three superseded decisions (`supersedes` list in the program deliberation) — consulted as
  **failed-approach evidence**, not as design input

### High-risk triggers and invariants to preserve

| Invariant | Statement | How A-U* preserves it |
|---|---|---|
| I-A1 | **Completeness** — every mandatory tracked write appears exactly once in the table | A-U3 milestone pins the table against the plan's verified Problem Frame table |
| I-A2 | `UNCLASSIFIED` is a **hard error**, never a permissive fallback | A-U1 milestone requires the halt statement; this is the specific defect that let attempts 1–3 ship non-partitions |
| I-A3 | P-021 C2 capture stays an **unconditional precondition**; only its destination is classified | A-R4, A-K4; the diff must not add any condition to C2 |
| I-A4 | **No ordering change** — Step 5's duplicate `7` and session-end ordering are recorded, not fixed | D-A3, A-U2 "record, do not renumber"; diff contains no `_ship.agent.md` edit at all |
| I-A5 | P-020's mandatory-invocation semantics are unchanged | A-U6 is additive cross-reference only; deletions must be `0` |
| I-A6 | `NEXT_STAGING_ROUND` is not cited as executable until B ships | A-K3; `blocks` edge B → A recorded at harvest |

### ProposedAction / ActionRisk entries (strict-safety: enabled)

| # | ProposedAction | ActionRisk | Approval needed | Rationale |
|---|---|---|---|---|
| PA-A1 | Create `.github/instructions/tracked-write-boundary.instructions.md` | **HIGH** — new normative instruction whose `UNCLASSIFIED` class is a halt condition | **Yes — operator** | A halt-capable classification applied across both agents' full lifecycles |
| PA-A2 | Add cross-references to P-005 / P-020 / P-021 C2 in `workflow-policies.md` | **MEDIUM** — policy-catalog mutation (additive) | **Yes — operator** | Policy edits change agent behaviour repo-wide |
| PA-A3 | Run markdownlint, `autoharness verify-workspace`, topology, `git diff --check` | **NONE** — read-only | No | Non-mutating |

No destructive action is proposed: no deletion, no history rewrite, no backlog mutation, no
`git restore`/`git revert`, and — critically — **no edit to any agent contract file**. Strict-safety's
`require_approval_for: [destructive]` trigger is not reached; PA-A1/PA-A2 carry operator approval
because of contract blast radius.

### Deepened verification

**Environment prechecks**:
1. `git status --porcelain` empty for target files.
2. `autoharness verify-workspace` **baseline** exit code captured before edits.
3. Confirm Package B has **shipped** (merged to `main`) before the `NEXT_STAGING_ROUND` class is
   cited as executable. If B has not shipped, A-U1 must mark that class `PENDING-B` and the package
   is `READY_WITH_CONDITIONS` at most.

**Target scenarios** (demonstrated, not asserted):
* S-A1: `Select-String ... -Pattern "PRE_GATE|CLOSURE_BRANCH|NEXT_STAGING_ROUND|UNCLASSIFIED"` → ≥ 4 hits.
* S-A2: **completeness audit** — for each of the 14 line references in this plan's Problem Frame
  table, `Select-String` the taxonomy file and confirm exactly one matching row. A count mismatch
  fails the unit (I-A1). This is the decisive test.
* S-A3: `Select-String ... -Pattern "UNCLASSIFIED"` context confirms a halt statement, not a
  fallback (I-A2).
* S-A4: `git diff --name-only` contains **no** `_ship.agent.md` and **no** `_stage.agent.md` (I-A4).
* S-A5: `git diff --numstat .github/policies/workflow-policies.md` deletions column `0` (I-A5).
* S-A6: `git diff` of `workflow-policies.md` contains no change to any P-021 C2 sentence (I-A3).

**Blocked-path handling**: if the S-A2 completeness audit finds an unmatched site, the unit is
**blocked**, not partially accepted. A taxonomy with a known gap is worse than none, because I-A2
makes the gap a halt condition at runtime rather than at review time.

### Rollback

* **Trigger**: S-A2 completeness audit fails, or `autoharness verify-workspace` regresses against baseline.
* **Procedure**: `git rm --cached .github/instructions/tracked-write-boundary.instructions.md` and
  `git restore -- .github/policies/workflow-policies.md` before commit; after commit but before
  merge, `git revert` the package commit.
* **Coupling**: **B → A**. If B is reverted after A merged, A's `NEXT_STAGING_ROUND` class loses its
  owner. Mitigation: A's rollback procedure must be executed if B is reverted; this coupling is
  recorded as a dependency edge at harvest and must be surfaced in A's closure artifact.

### Operational closure

* **Monitoring signal**: the next Ship session completes Step 5 through Step 6 with every tracked
  write attributable to a class in the table.
* **Failure signal**: a Ship or Stage session encounters `UNCLASSIFIED` for a write the harness itself
  mandates — i.e. the table is incomplete despite S-A2 having passed.
* **Rollback trigger**: two `UNCLASSIFIED` halts in one session, or any `UNCLASSIFIED` on a P-021 C2
  capture (which would breach I-A3 in practice).
* **Validation window**: the next two Ship sessions plus the next Stage session (to exercise the
  symmetric Stage rows from A-U4).
* **Owner**: the operator.
* **Releasability**: `READY` only when B has shipped and S-A1..S-A6 pass;
  `READY_WITH_CONDITIONS` when B has not shipped and `NEXT_STAGING_ROUND` is marked `PENDING-B`.

### Unresolved operator decisions blocking safe execution

* **OD-A1** — Whether `UNCLASSIFIED` halts the session immediately or defers to the end of the
  current phase. This plan specifies immediate halt (A-U1), consistent with I-A2's rationale, but
  an immediate halt mid-closure could strand a merged PR — the failure mode P-020 explicitly avoids.
  Operator must confirm before A-U1 executes.
* **OD-A2** — Confirmation that Package B has shipped, or explicit authorisation to proceed with
  `NEXT_STAGING_ROUND` marked `PENDING-B`.

<!-- plan-review-attempt: 0 -->
<!-- plan-review-verdict: NOT REVIEWED — deferred; prerequisite Package B FAILED -->
