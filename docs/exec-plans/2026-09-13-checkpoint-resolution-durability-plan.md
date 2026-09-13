---
doc_type: exec-plan
date: 2026-09-13
revision: 9
scope: defect-2-only
status: awaiting-independent-review
review_verdict: pending
review_verdict_revision: 8
review_attempts: 3
escalation: P-013.6 fired at revision 8; route gpt-5.6-sol/openai/xhigh; operator authorized ONE bounded remediation revision and ONE fresh full independent review
harvest_authorized: false
source_document: docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md
supersedes: docs/exec-plans/2026-09-13-checkpoint-lifecycle-continuity-plan.md
stash_ids: [4EF24729]
policies: [P-001, P-003, P-005, P-006, P-008, P-009, P-010, P-012, P-014, P-015, P-016, P-017, P-018, P-020, P-022]
requires_plan_hardening: yes
hardening_document: docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md
task_count: 10
dependency_edge_count: 16
sub_epic_count: 2
---

# Checkpoint Resolution Durability — Implementation Plan (revision 9, Defect 2 only)

**Source document**: `docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md`
**Requires plan hardening**: **yes** — this plan changes merge-adjacent ordering
governed by P-014/P-018 and adds a startup route that could, if mis-specified,
become an unsupervised merge path.

**Machine-readable status.** The frontmatter is authoritative. `revision: 9`,
`scope: defect-2-only`, `harvest_authorized: false`,
`review_verdict: pending`, `review_verdict_revision: 8`, `review_attempts: 3`,
`status: awaiting-independent-review`.

**What this revision is, and what it is not.** Revision 8 was independently
reviewed and returned **FAIL** — the third consecutive FAIL (revisions 6, 7, 8) —
the plan-review circuit opened at attempt counter 3, and the P-013.6 escalation
fired and ran against HEAD `bbb52b65`. Revision 9 is **one bounded remediation
revision explicitly authorized by the operator** after that circuit opened. It
remediates the escalation's findings. It is **not** a clean bill of health:
`review_verdict` is `pending` because revision 9 has **not** been reviewed, and
`review_verdict_revision` still reads `8` because the last *completed* verdict
belongs to revision 8. `harvest_authorized` stays **false** and moves only on a
fresh independent full-plan review of **revision 9** returning PASS. No earlier
verdict, and no part of the retained history, may be cited as a harvest gate.

**What changed in revision 9.** Five corrections, all from the P-013.6
escalation:

1. **RQ-6 gained an executable enforcement path against the *real* Step 5.**
   Every prior round described Step 5 in prose and mismatched it. U4 now carries
   a **verbatim extract of the live item list** — including the fact that the
   item number `7` is **duplicated** in the file, that readiness items 7b/7c
   currently precede the mutating items 7(second)/8/9/10 and the push, and that
   item 15 re-fetches only the P-018 verdict and `headRefOid`. The canonical
   `RESOLUTION_PREFIX` is restructured into eight named segments S1…S8 that
   define the exact safe order; items 7b/7c **move** after the push, item 14
   records an `approved_head`, item 15 is **amended** (revision 8's "item 15
   unmodified" criterion forbade the unit from implementing the requirement it
   was credited with, and is withdrawn), and a **six-part merge bar** plus
   explicit no-stale-approval refresh rules are stated.
2. **The zero-checkpoint bypass claim is removed as false.** The checkpoint count
   now selects **only** segment S4. Every unit runs the reordered common
   finalization tail; a complete zero enumeration omits exactly four things
   (locator publication, resolution, resolution commit/push, phase-2 locator) and
   nothing else. Enumeration failure, malformed or quarantined records, and
   ambiguity are **not zero** and halt; a checkpoint appearing after enumeration
   forces re-evaluation before merge; no empty locator is ever published; Stage's
   startup recovery stays a separate protocol.
3. **D14 is given real executable coverage.** The working-tree placement
   paragraph — which revision 8 left as an **unnamed fragment with its heading
   lost**, so no criterion could cite it — is now the named procedure
   `Working-tree placement — committing re-entry only` with steps WP-1…WP-7,
   defined in **U3** (AC12), executed by name in **U5** (AC9) before the only
   committing recovery branch, and verified by **V12/V12a–V12f**. The existing
   U3→U5 dependency is unchanged and **no new unit was needed** for it.
4. **RR-3 is closed rather than weakened or silently deferred.** New unit **U9**
   installs `RESOLUTION_OBLIGATION_RECORD`: a SHA-free, Git-tracked record
   written into the unit's existing `docs/closure/` pre-merge closure artifact,
   in the **same commit** as the resolutions, discovered from exhaustive trusted
   PR/commit/tree history independently of the mutable PR body, with deletion
   detected from commit history. New unit **U10** declares the field in the
   owning `operational-closure` skill so the record is not an unowned squatter —
   an openly recorded scope addition of one file. RQ-7 is made true; **RQ-8 is
   not traded**, because the record carries no SHA.
5. **Consistency cleanup.** V2's unsatisfiable verb prohibition is replaced with
   a sequence-restatement check that permits ordinary prose verbs; V8's reference
   command gains the `--paginate` it was missing (without it the comparison could
   only ever prove the two commands disagreed); the abandoned `143.*` IDs are
   retained as historical evidence only and no longer appear as implementation
   targets; and the revision counter is incremented with
   `harvest_authorized: false` held.

**Abandoned-ID notice.** `143-F`, `143-S` and `143.001-T` … `143.014-T` are
machine-state **abandoned**. They appear in this document **only** as historical
evidence of what was previously attempted. They are **never** to be revived,
re-parented, or reused, and no future-tense statement in this plan depends on
them. Replacement IDs stay **unassigned** until a later authorized harvest.
Where a section below still needs to name the release-unit shape, it does so
structurally (top-level release unit → two sub-epics → ten tasks) rather than by
citing an abandoned ID as a live target.

**What changed in revision 8.** Revision 7 was reviewed by the same independent
four-persona panel and returned **FAIL** again — Scope, Correctness and Parity
FAIL; Constitution ADVISORY. The panel confirmed that every revision-6 finding
was genuinely closed, and that the design still holds; the new findings were
deeper specification defects that only became visible once the earlier layer was
fixed. Revision 8 remediates them, and they converge on five root corrections:

1. **`RESOLUTION_ORDER` is split into `RESOLUTION_PREFIX` (pre-merge) and
   `RESOLUTION_POSTCONDITION` (a Step 6 metadata write).** A single blob spanning
   both sides of the merge could not be invoked from Step 5 without either
   re-merging or stopping mid-sequence at an undefined boundary. The existing
   approval/re-fetch/merge items are now explicitly *not* moved or duplicated.
2. **The circular entry condition is gone.** The canonical block said
   `work complete AND PR merge-ready`, but readiness is established *by* this
   sequence. Revision 7 fixed this only in U4's prose and left the canonical text
   — which U2 installs verbatim — still circular. A zero-checkpoint bypass is now
   stated too.
3. **Orchestrator discovery is owner-scoped.** Revision 7 keyed it on *global*
   zero-candidate, so a legitimately-active **Stage** checkpoint would mask the
   whole obligation, Ship would never be routed, and the shipment would then be
   skipped at Step 2 for being `active` — nobody discharging it. It is now keyed
   on the absence of a **ship-owned** checkpoint, matching Ship's own scoping.
4. **Recovery is executable from a fresh checkout.** Re-entry has to commit, but
   startup is normally on `main`, where committing is forbidden. A mandatory
   working-tree placement step (fetch `refs/pull/<n>/head`, check out, halt on
   failure) is now specified.
5. **The Stage side is completed.** U8 now covers *both* Stage resolve sites,
   gives an executable merged-PR predicate instead of a prose condition, and
   retires the undischargeable best-effort checkpoint.

Also: the **P-003 sub-epic tier is restored** (revision 7 justified a flat
decomposition by precedent, but P-003 item 4 is explicit and its violation action
is Halt); the `pr_role: closure` ghost specification is **removed** (no unit ever
produced a closure-PR locator); U1 gains the amendment-log and gate-point
mechanics every other policy carries; U2's self-contradictory heading locus is
fixed; U4 stops restating the sequence it is supposed to reference; and the
Constitution Check gains P-008, P-012 and P-017. The full disposition table is in
`## Review record`.

**What changed in revision 7.** Revision 6 was reviewed by an independent
four-persona panel (Scope Boundary Auditor `gpt-5.6-sol`, Constitution Reviewer
`claude-opus-4.8`, Correctness Reviewer `gemini-3.8-flash`, Agent-Native Parity
Reviewer `grok-4.6`) and returned **FAIL** — three FAIL, one ADVISORY. Every
finding was a specification defect, not a falsified design. Revision 7 remediates
all of them; the disposition table is in `## Review record` below. The
substantive changes: `RESOLUTION_ORDER` now has one named canonical owner
(the instructions file) that every other surface references rather than restates;
U4 is retargeted at Ship **Step 5**, which is where the executable merge path
actually lives, and at Session end item 2, which is where the only happy-path
resolution actually is; the residual-window checkpoint-creation directive is
explicitly retired; locator discovery is now a real executable command with a
completeness test; `LAST_MILE_RECOVERY` gains the missing `RESOLUTION_PENDING`
rows and the missing ancestry assertion on the advanced-HEAD row; a new unit U8
closes the Stage-side procedural gap that revision 6 recorded only as a residual
risk; and `RECONCILED` is tied to the full P-001 closure set.

**What changed from revision 5.** Revision 6 was a **scope reduction**, not a
remediation round. Revision 5 planned Defect 1 and Defect 2 together across 14
tasks. Defect 1 has been removed from implementation and returned to an open
deliberation
(`docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md`)
because its central assumption was falsified: safe auto-routing needs durable,
concurrency-safe executable persistence that prose cannot supply. Removed with
it: the continuation predicate and its precedence table, the P-017 amendment, the
Orchestrator auto-route branch, the Stage/Ship owner-side continuation paths, the
activation record store, cursor typing, handoff evidence, owner-side
revalidation, mis-evaluation directionality, the drift-checker script pair with
its fixture corpus and hook shim, and the task↔plan parity gate. Eight tasks
remain, all Defect 2 (seven at revision 6, plus U8 added in revision 7).

**Prior review history is retained as evidence**, not erased — see
`## Retained review history` at the end of this document. Revision 5's FAIL
stands on the record against revision 5's scope.

## Primary objective

Make a unit of work's checkpoint resolution reach `main` in the **same merge**
that carries the work, and make any residual outstanding closure obligation
discoverable and safely recoverable without conferring merge authority.

## Constraints

* Documentation-only. No `src/`, no `crates/`, no scripts, no build system.
* Installed harness surfaces only: `.github/policies/`, `.github/instructions/`,
  `.github/agents/`, **one field declaration in
  `.github/skills/operational-closure/SKILL.md`** (added in revision 9 by U10, so
  the RR-3 obligation record lives on an *owned* schema rather than squatting on
  someone else's artifact), plus one `docs/compound/` learning.
* No backlogit tool change. No shipment status outside `queued`/`active`.
* Every unit is single-domain and under two hours of human-equivalent effort.
* **No implementation unit introduces or depends on a Defect-1 construct.**
  Defect-1 names appear in this document only in the revision summary, the
  out-of-scope list, and the retained review history — as documentary exclusions,
  never as implementation targets. V7 enforces the distinction by scanning the
  *changed files*, not this plan.

## Constitution Check

| Principle | Check |
|---|---|
| P-001 Single-release-unit completion | P-001 requires that no previously merged release unit is still awaiting required post-merge closure. `CLOSURE_LOCATOR` discovery at zero-candidate startup (U6, U5) is the mechanism that makes that precondition *checkable* rather than assumed. `RECONCILED` is defined to require the full P-001 closure set. Composed with, not weakened. |
| P-003 Decomposition chain | P-003 precondition item 4 requires that **every task references its parent sub-epic**. Revision 7 asserted a flat feature-direct decomposition on the strength of workspace precedent; precedent is not policy text, and the Violation Action is Halt at pre-harvest. Revision 8 restored the tier and revision 9 keeps it, stated **structurally** because the former IDs are abandoned: source document → this plan → **one top-level release unit** → **two sub-epics** → **ten tasks**. Sub-epic **E1 — Ordering contract** covers the prevention half (U1, U2, U4, U8); sub-epic **E2 — Discovery and recovery** covers the recovery half (U3, U5, U6, U9, U10, U7). The split is the plan's own prevention/recovery structure, not an artificial tier. Each sub-epic references this plan and the top-level release unit; each task references its sub-epic and carries acceptance criteria. **No ID is assigned here** — `143-F`, `143-S` and `143.001-T` … `143.014-T` are abandoned and must not be revived or reused; replacement IDs are allocated only at a later authorized harvest. |
| P-005 Policy telemetry | No gate is bypassed; the review gate is explicitly open. U1 requires P-022's Violation Action to record P-005 telemetry, matching neighbouring policies. |
| P-006 Plan hardening | `requires_plan_hardening: yes`; hardening document exists and is reduced in lockstep. |
| P-008 Markdown conformance | Engaged by every unit: each carries a `markdownlint passes` acceptance criterion and V1 runs it across all changed files. Satisfied by construction. |
| P-009 Merge-commit-only | Untouched. U4 does not alter Step 5's P-009 guardrail (item 16); U3/U6 confer no merge authority and therefore cannot select a merge strategy. |
| P-010 Role boundary | Stage plans; Ship executes. No source mutation planned by Stage. **U8 adds only a prohibition to Stage** — do not resolve into a merged staging PR, do not create an undischargeable checkpoint — and confers no merge authority, no locator obligation, and no `RESOLUTION_PREFIX` execution. U6 has the Orchestrator *route* to Ship rather than perform recovery. |
| P-012 Tool availability | Locator discovery adds a required `gh api` capability to the startup critical path. It is probed per P-012, and its failure mode is the fail-closed halt already required by RQ-11 — never a silent fall-through to ad hoc filesystem scanning. |
| P-014 Local review readiness | **This plan introduces a §1.9 hazard and then closes it.** Today §1.9 is trivially satisfiable because resolution happens post-merge and the reviewed HEAD is stable. Moving resolution pre-merge advances HEAD past the reviewed HEAD, which — left unmitigated — would defeat §1.9 while appearing to satisfy it (hardening D4) or cause the gate to be quietly skipped (D5). The mitigation is specific and mandatory: re-run the actual review at the final pushed HEAD, write the PR-body record, *then* gate. P-014 is **preserved by construction**, not strengthened. |
| P-015 Single-artifact shipment closure | Untouched. The locator records shipment identity but does not alter shipment-closure sequencing and introduces no cascade-close behaviour. |
| P-016 No parallel branches | The recovery path routes one owner to one PR; it never opens a second branch or worktree. |
| P-017 Dark factory | `LAST_MILE_RECOVERY` is auto-entered at startup and walks to a merge bar, so dark mode could otherwise supply approval for it. A recovered obligation belongs to a **prior** unit and is outside the current run's declared dark scope: U6 AC7 states that a dark approval for the current scope does **not** satisfy the merge bar for a prior unit's recovered obligation. |
| P-018 Copilot review gate | Re-run at the final HEAD is explicit in `RESOLUTION_PREFIX`. The resolution push naturally re-arms P-018, and Step 5's existing unconditional last-mile re-check (item 15) is retained unmodified. |
| P-020 Post-merge context compaction | U4 edits Ship Step 5 and Session end — **not** Step 6's `compact-context` invocation. U4 carries an explicit acceptance criterion that the P-020 invocation remains present and unmodified, and `RECONCILED` is defined to require the P-020 compaction record, so the locator cannot discharge an obligation P-020 has not yet met. |

## Canonical definitions

These five definitions are the single source of truth. Task cards quote them;
they are not restated differently anywhere.

**Canonical ownership — one installed surface per definition.** Each definition
has exactly **one** installed home that carries it verbatim. Every other surface
**references** it by name and must not restate the sequence, so the surfaces
cannot drift into two different orders.

| Definition | Canonical installed home | Referencing surfaces |
|---|---|---|
| `HEAD_EVIDENCE_RULE` | `github-pr-automation.instructions.md` (U2) | — |
| `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION` | `github-pr-automation.instructions.md` (U2) | `workflow-policies.md` P-022 (U1); `_ship.agent.md` Step 5 and Step 6 (U4); `_stage.agent.md` (U8) |
| `CLOSURE_LOCATOR` | `github-pr-automation.instructions.md` (U2, U3) | `_ship.agent.md` (U4, U5); `_orchestrator.agent.md` (U6) |
| `RESOLUTION_OBLIGATION_RECORD` | `github-pr-automation.instructions.md` (U9) | `_ship.agent.md` (U4, U5); `_orchestrator.agent.md` (U6); `operational-closure/SKILL.md` (U10, schema field only) |
| `LAST_MILE_RECOVERY` | `github-pr-automation.instructions.md` (U3) | `_ship.agent.md` (U5); `_orchestrator.agent.md` (U6) |

### `HEAD_EVIDENCE_RULE`

Any artifact whose own commit advances HEAD must not restate a HEAD-pinned
verdict. HEAD-pinned evidence belongs in PR metadata (PR body `Reviewed HEAD`);
committed documents use point-in-time wording or defer to the PR body.

Observed on PR #395, threads `PRRT_kwDORJEduc6h2uOQ` and `PRRT_kwDORJEduc6h2viu`.

### `RESOLUTION_PREFIX` and `RESOLUTION_POSTCONDITION` *(RQ-1 … RQ-6)*

The sequence is defined in **two named parts** with the existing merge step
between them. This split is deliberate: a single blob spanning both sides of the
merge cannot be invoked from one step without either re-merging or stopping
mid-sequence with no defined boundary.

**Entry condition.** `RESOLUTION_PREFIX` is the **whole finalization tail** of
Ship Step 5 and it runs for **every** unit. It is explicitly **not** gated on "PR
merge-ready" — readiness is established *by* this sequence, so gating entry on it
would be circular.

**The checkpoint count selects one conditional segment, not the whole
sequence** *(corrected in revision 9; see B below)*. The unit's active-checkpoint
count selects **only** segment **S4** — locator publication, checkpoint
resolution, the resolution commit, its push, and the phase-2 locator
publication. **Every other segment — S1, S2, S3, S5, S6, S7, S8 — runs
identically for every unit, zero-checkpoint units included.** A unit with a
**complete** zero enumeration omits S4 and nothing else. There is no
"pre-existing path unchanged" bypass and no unit escapes the reordered common
finalization tail; the revision-8 claim to the contrary was false and is
withdrawn.

**Zero is a proven result, never a default.** S3 must *prove* the enumeration
complete. An enumeration that fails, returns a malformed or quarantined record,
or is ambiguous in any way is **not zero** — it **halts**. A checkpoint that
appears after S3 completed forces re-evaluation from S3 before merge. No empty
locator is ever published: when S4 is omitted there is no locator, not a locator
with an empty checkpoint list.

This is scoped to the **Ship unit's own** pre-merge obligation. Stage's
crash-resumption startup recovery is a **separate** protocol and is not altered,
subsumed, or gated by anything in this definition.

```text
RESOLUTION_PREFIX  (the finalization tail of Ship Step 5)
  entry: all in-scope task work complete
         (NOT gated on "PR merge-ready" — readiness is produced here)

  S1  complete the existing CI / review fix loop
        real Step 5 items 7 (fix-ci) and 7a (shadow review).
        These MAY commit and push. They run to completion FIRST.

  S2  complete every remaining branch-mutating item
        real Step 5 second item 7 (runtime verification),
        item 8 (operational closure), item 9 (follow-up stash writes),
        item 10 (the ordinary push).
        After S2 no ordinary work remains that can mutate the branch.

  S3  prove current-unit checkpoint enumeration COMPLETE
        enumerate every checkpoint owned by THIS unit.
        complete + count == 0        → skip S4 only; continue at S5
        complete + count >= 1        → run S4
        failed / malformed / quarantined / ambiguous
                                     → HALT (this is NOT zero)

  S4  CONDITIONAL — runs only on a complete NONZERO enumeration
      → publish CLOSURE_LOCATOR to the PR body, status RESOLUTION_PENDING
            (metadata write; MUST precede the first resolution commit, so
             the checkpoint-free window is never uncovered)
      → resolve every checkpoint owned by this unit, and write the
            RESOLUTION_OBLIGATION_RECORD, in ONE commit
            (the Git-tracked durable obligation record; see its canonical
             definition below — it rides the SAME commit, carries NO SHA,
             and is what survives deletion of the PR body)
      → PUSH that commit
      → prove the remote PR head now equals the local HEAD
            (nothing downstream may derive evidence from an unpushed commit)
      → update CLOSURE_LOCATOR in the PR body: status RESOLUTION_PUBLISHED,
            resolution commit SHAs, final_head

  S5  at the resulting pushed HEAD — RUNS FOR EVERY UNIT
      → RE-RUN THE ACTUAL LOCAL REVIEW at that pushed HEAD
            (a real review pass over the final diff — never a restatement
             of an earlier verdict)
      → publish the final locator resolution state, if a locator exists
      → record Reviewed HEAD in the PR BODY at that HEAD
            (metadata write; does NOT advance headRefOid)
      → run, in this order:
            P-014 §1.9 local readiness   (real Step 5 item 7b, MOVED here)
            EXPLICIT required-check evaluation — each required check
              enumerated and evaluated green, or explicitly PROVEN
              non-applicable; "no red" is not an evaluation
            P-018 copilot-review gate    (real Step 5 item 7c, MOVED here)

  S6  record approval, pinned  (real Step 5 item 14)
      → obtain explicit operator approval and record `approved_head`
            equal to the S5 HEAD. An approval without a recorded
            approved_head is not an approval.

  S7  strengthened last-mile re-check  (real Step 5 item 15, AMENDED)
      → re-fetch and re-evaluate, all of them, unconditionally:
            headRefOid; the PR body; reviewDecision;
            review requests and reviews; EVERY review-thread page to
            exhaustion; required checks; and the ancestry of every
            recorded resolution commit.

  S8  merge bar — merge ONLY when ALL hold
      1. live headRefOid == local HEAD == locator final_head
             == PR-body Reviewed HEAD == approved_head  (all five agree)
      2. every recorded resolution commit is an ancestor of that HEAD
      3. review-thread pagination COMPLETED to exhaustion
      4. no blocking review thread and no blocking review exists
      5. required checks pass, or are explicitly PROVEN non-applicable
      6. P-018 passes
      Any doubt at any of the six — HALT. Then real Step 5 item 16 (P-009)
      and item 17 (P-017) apply unchanged.

  REFRESH RULES (no stale approval may ever be reused)
    * Any HEAD change at any point requires a FRESH cycle:
      push → re-run the actual review → PR-body metadata → S5 gates →
      S6 approval. The prior approval is void.
    * A thread or check change WITHOUT a HEAD change requires the
      affected gates to be re-run and the S6 approval to be refreshed.

RESOLUTION_POSTCONDITION  (a Step 6 closure postcondition)
  → after the FULL required post-merge closure set is complete AND verified
  → set CLOSURE_LOCATOR to RECONCILED and close the
    RESOLUTION_OBLIGATION_RECORD in the same closure commit
```

**No step of either part may be performed on a merged branch.** The prefix ends
before merge; the postcondition is a PR-body metadata write only and commits
nothing.

Eight ordering invariants are load-bearing:

1. **No checkpoint resolution may occur after merge** *(RQ-1)*. Checkpoint JSONs
   are Git-tracked, so a post-merge resolution commit sits on an already-merged
   branch with no unmerged PR able to carry it to `main` — precisely the 139-S
   defect (commit `43e70430`, repaired only by the extra PR #395). A "resolve
   after merge" step is never correct for Git-tracked state.
2. **Push precedes evidence** *(RQ-3)*. The re-run review, the locator SHAs and
   the PR-body record all describe a remote state. Deriving any of them from an
   unpushed commit publishes a claim about SHAs no remote has, and a crash before
   push silently loses the resolution.
3. **The PR-body `Reviewed HEAD` record precedes the §1.9 gate** *(RQ-5)*. §1.9
   reads the PR body and requires `Reviewed HEAD == headRefOid`. Running the gate
   first is unsatisfiable, because the resolution commits already advanced HEAD
   past whatever the body recorded. Recording the body first is safe and
   terminating precisely because a PR-body edit is metadata and does not advance
   `headRefOid` (`HEAD_EVIDENCE_RULE`).
4. **The recorded evidence must be produced, not relabelled** *(RQ-4)*. Moving
   the SHA without re-running the review lets a stale verdict be re-pointed at a
   HEAD it never examined — a HEAD that by construction contains commits the
   earlier review never saw. Updating only the SHA is explicitly insufficient.
5. **Every mutating step precedes S3** *(RQ-6)*. S1 and S2 exhaust the branch's
   ordinary mutation — CI/review fixes, runtime verification, closure-artifact
   generation, follow-up stash writes, and the ordinary push. The readiness
   gates (§1.9, required-check evaluation, P-018) **move** from their current
   real-Step-5 position at items 7b/7c to **after** that push, in S5. This is
   the reorder, and it applies to **every** unit, not only checkpoint-owning
   ones.
6. **Approval is pinned to a HEAD** *(RQ-6)*. S6 records `approved_head`. An
   approval with no recorded `approved_head` cannot be checked against anything
   at S8 and is therefore not an approval. **No stale approval is ever reused**:
   any HEAD change voids it, and a thread or check change without a HEAD change
   requires the affected gates plus a refreshed approval.
7. **Required checks are evaluated, never assumed** *(RQ-6)*. S5 and S8 require
   each required check to be enumerated and evaluated green or **proven**
   non-applicable. The real Step 5 item 15 re-runs P-018 and re-queries
   `headRefOid` only; "no red observed" is not an evaluation, and S7 is the
   amendment that closes that gap.
8. **The durable obligation record rides the resolution commit** *(RQ-12)*. The
   SHA-free `RESOLUTION_OBLIGATION_RECORD` is written in the **same** commit as
   the checkpoint resolutions and pushed with them, so the obligation is
   recorded in Git history — not only in mutable PR-body metadata. See its
   canonical definition below.

**Gate-failure remediation loop.** "These are the LAST commits on the branch"
describes the *intended* terminal state, not a prohibition on remediation. If a
downstream gate fails after resolution — CI red on the resolution commit, a new
Copilot thread, a §1.9 finding — the correct response is:

* push a remediation commit **on top of** the resolution commits, which remain
  ancestors of the new HEAD;
* do **not** re-touch, re-create, or re-resolve any checkpoint — they are already
  resolved and their resolution already rides this branch;
* re-run the actual local review at the new HEAD, then update the locator
  `final_head` and the PR-body `Reviewed HEAD` to that new HEAD;
* do **not** re-write, duplicate, or re-open the `RESOLUTION_OBLIGATION_RECORD`
  — it is already in history on this branch and a remediation commit does not
  create a second obligation;
* re-enter the gate set from the top **at S5**, and obtain a **fresh** S6
  approval with the new `approved_head`. The prior approval is void.

The loop terminates because each iteration ends in either a merge or a halt, and
because remediation never creates new checkpoint state to resolve.

**Residual-window checkpoint prohibition.** Once a unit enters
`RESOLUTION_PREFIX`, no new Git-tracked checkpoint may be created for that unit —
not on yield, not for merge approval, not for closure work. Such a checkpoint
could only be resolved by a further commit, which needs a further PR once this
one merges: the original defect, one level down, recursively. The window is
covered by `CLOSURE_LOCATOR`, the `RESOLUTION_OBLIGATION_RECORD`, and
`LAST_MILE_RECOVERY` instead.

### `RESOLUTION_OBLIGATION_RECORD` *(RQ-12)*

**This definition is revision 9's answer to RR-3.** It is the narrowest
correction that makes RQ-7 true rather than weakening it. The problem it solves
is stated exactly: because `RESOLUTION_PREFIX` resolves **every** checkpoint
before merge, the PR body becomes the **sole** record of an outstanding closure
obligation, and a PR body is mutable — a human or bot can delete it and no
mechanism in revisions 6–8 would notice. Startup would then see zero
checkpoints **and** zero locators and conclude "clean".

**Surface: the unit's existing pre-merge operational-closure artifact in
`docs/closure/`.** This is an **existing owned repo-local state surface**, not a
new ad-hoc tracker. `docs/closure/` is created and owned by the
`operational-closure` skill, which Ship already invokes at real Step 5 item 8 —
inside **S2**, therefore already committed on the PR branch before S4 runs. The
surface already carries exactly this placeholder→finalize lifecycle shape for
two other fields (`compaction_status`, initialized `pending` by the skill and
finalized by Ship's post-merge closure; and the source-artifact cleanup
placeholder). The obligation record is a third field of the same shape, so it
inherits an owner, a schema, a path convention and a lifecycle rather than
inventing any of them.

**Why this and not the alternatives.** A record in its own new file under
`docs/closure/` would be an unowned ad-hoc tracker. A record in the checkpoint
store is self-defeating: resolving the checkpoints is precisely what removes the
signal. A record in a commit **message** is not addressable by path and is lost
to any history rewrite that preserves trees. A new executable persistence
substrate is Defect-1 scope and is forbidden here.

```text
# canonical schema — a frontmatter field of the pre-merge closure artifact
# canonical path     docs/closure/{pre-merge closure artifact for this shipment}
# canonical identity (shipment_id, pr) — exactly one OPEN record per shipment

resolution_obligation:
  status: OPEN | CLOSED      # 'none' when the unit had zero checkpoints
  shipment: <id>
  feature: <id>
  pr: <number>
  branch: <branch>
  checkpoints: [<filename>, ...]
  opened_at: <RFC3339 UTC>
  closed_at: <RFC3339 UTC>   # present only when status is CLOSED
```

**It carries NO SHA.** `resolution_commits` and `final_head` stay in the
`CLOSURE_LOCATOR` and appear nowhere in this record. The record is written
*inside* the resolution commit, so a SHA field would be self-referential — RQ-8
is preserved intact, not traded away. Recovery re-derives the SHAs from
checkpoint state at the fetched PR head using the existing *resolution-state
classification*, which is exactly the enumeration that already exists for the
`RESOLUTION_PENDING` path.

**Lifecycle.**

| Transition | When | Commit that carries it |
|---|---|---|
| absent → `none` | S2, when `operational-closure` creates the artifact | the ordinary S2 closure commit |
| `none` → `OPEN` | S4, in the **same commit** as the checkpoint resolutions | the resolution commit, pushed in S4 |
| `OPEN` → `CLOSED` | `RESOLUTION_POSTCONDITION`, after the full verified P-001 closure set | the post-merge closure commit, merged by the closure PR |

`OPEN` → `CLOSED` is the **only** permitted way to discharge the record, it
happens in a **later** commit whose ancestry stays auditable, and that commit
must itself be **merged**. Deleting the record is never a discharge.

**Two-channel discovery — the Git channel does not read the PR body.**
Discovery runs **both** channels and takes the union; neither may be skipped
because the other returned nothing:

* **Channel A (mutable, fast)** — the `CLOSURE_LOCATOR` in the PR body, per the
  read protocol below.
* **Channel B (durable, history-backed)** — for every PR admitted by
  **PV-1…PV-3** of the same exhaustive paginated enumeration, plus the default
  branch, fetch the head and read `docs/closure/` from the **tree**:

```text
# post-merge / default-branch tree
git ls-tree -r --name-only origin/main -- docs/closure/
  → read resolution_obligation from each artifact; keep status OPEN

# still-open PRs (the pre-merge window, where the record is not yet on main)
git fetch origin refs/pull/<pr>/head        # for each PV-1..PV-3-admitted PR
git ls-tree -r --name-only FETCH_HEAD -- docs/closure/
  → read resolution_obligation from each artifact; keep status OPEN

# deletion detection — absence from the tree is NEVER "no obligation"
git log --follow --diff-filter=D --format=%H -- <artifact path>
git log --format=%H -S'resolution_obligation' -- docs/closure/
  → any record that ever appeared in history and is now absent from the
    tree WITHOUT a recorded OPEN → CLOSED transition is a DELETION → HALT
```

Channel B's provenance is established **entirely** from API response fields and
Git ancestry — **PV-1** (same repository), **PV-2** (base is the default
branch), **PV-3** (trusted `author_association`) — and **never** from PR-body
text. That is what makes it independent of the surface under suspicion. PV-4 and
PV-5 (which compare against locator body fields) are **not** applicable to
Channel B; the Channel-B analogues are **PV-B4** (the record's embedded `pr`
equals the PR it was read from) and **PV-B5** (the record's `branch` equals that
PR's head ref), both evaluated against API fields. PV-6 and PV-7 apply
unchanged.

**Fail-closed rules.** Each of these **halts to the operator**:

| # | Condition | Why it cannot be tolerated |
|---|---|---|
| OB-1 | An artifact that carried a record in history is absent from the tree with no `OPEN` → `CLOSED` transition | This is the deletion attack. Absence is never evidence of discharge. |
| OB-2 | Force-push, rebase, or shallow/grafted history makes the record's introducing commit unreachable or its ancestry unverifiable | An unverifiable record cannot be trusted either way; guessing re-opens the orphan. |
| OB-3 | Two or more **conflicting** `OPEN` records for the same shipment | No field can rank them. Identical duplicates collapse to one and are logged; **any** field difference halts. `opened_at` MUST NOT be a tie-breaker. |
| OB-4 | `OPEN` records for more than one distinct shipment | A P-001 violation, exactly as for the locator. |
| OB-5 | A record is unparseable, or is missing a field its declared status requires | Partial evidence is not evidence. |
| OB-6 | Channel A and Channel B **disagree** — a locator says `RECONCILED` while an `OPEN` record exists, or a locator is absent while an `OPEN` record exists | The disagreement is the signal. Never prefer the mutable channel, and never let a missing locator discharge a live record. |
| OB-7 | Channel B enumeration is incomplete — a fetch, `ls-tree`, or `log` exits non-zero, or PR pagination is truncated | Identical to RQ-9's rule for Channel A. |

**What this deliberately does not do.** It adds **no** executable persistence
substrate, **no** lock or compare-and-swap, and **no** cross-run cursor. It is a
frontmatter field on a file the workspace already writes, read with `git
ls-tree` and `git log`. It implements **no** part of Defect 1 — no continuation
predicate, no activation record store, no auto-routing. Its only authority is to
**halt**; it confers no merge authority (RQ-10 is untouched).

### `CLOSURE_LOCATOR` *(RQ-7 … RQ-9)*

**Surface: the pull-request body**, in an HTML-comment-fenced block. The PR body
is correct on four independent counts: it is not on any branch, so a fresh
checkout of `main` can read it; it is not a commit, so recording a SHA in it is
not self-referential and does not advance `headRefOid`; it survives branch
deletion after merge; and it is queryable by `gh`, which the protocols already
use.

```text
<!-- autoharness:closure-locator v1
shipment: <id>
feature: <id>
pr: <number>
branch: <branch>
status: RESOLUTION_PENDING | RESOLUTION_PUBLISHED | RECONCILED
checkpoints: <filename>[, <filename>...]
resolution_commits: <sha>[, <sha>...]   # empty while RESOLUTION_PENDING
final_head: <sha>                       # empty while RESOLUTION_PENDING
updated_at: <RFC3339 UTC>
-->
```

**Three-phase publication protocol.** The protocol has exactly three phases and
is referred to as three-phase everywhere.

| Phase | When | Content | Why |
|---|---|---|---|
| 1 — `RESOLUTION_PENDING` | **Before** the first resolution commit | shipment, feature, PR number, branch, the checkpoint filenames about to be resolved | A locator appearing only *after* the resolutions would leave the checkpoint-free window uncovered for the interval it exists to cover. Phase 1 requires no SHA, so it has no self-reference problem. |
| 2 — `RESOLUTION_PUBLISHED` | After the resolution commits exist **and are pushed**, before the §1.9 gate | adds `resolution_commits` and `final_head` | The SHAs now exist on the remote and are recorded in **metadata**, never inside a commit *(RQ-8)*. |
| 3 — `RECONCILED` | After merge **and** the **full P-001 post-merge closure set** has completed and been verified — including the P-020 compaction record | terminal | Prevents a completed unit from being rediscovered forever. Setting it at merge, or after partial closure, would discharge the obligation while closure work is still outstanding — the exact failure the locator exists to prevent, one door over *(RQ-7)*. |

**Read protocol — exhaustive and trusted** *(RQ-9)*. Discovery is over pull
requests, not branches and not shipment status. The command is given exactly,
because the obvious formulation is silently wrong:

```text
gh api --paginate \
   -H "Accept: application/vnd.github+json" \
   "repos/{owner}/{repo}/pulls?state=all&per_page=100" \
   --jq '.[] | {number, state, body, author_association,
                head_repo: .head.repo.full_name, head_ref: .head.ref,
                base_repo: .base.repo.full_name, base_ref: .base.ref}'
  → select entries whose body contains `autoharness:closure-locator`
  → apply PV-1 … PV-7 provenance validation (below); discard untrusted
  → parse each TRUSTED block; keep those whose status is NOT RECONCILED
```

**Provenance validation — required before a locator participates in recovery.**
Enumeration is exhaustive over `state=all`, which necessarily includes **fork
PRs opened by untrusted authors**. A PR body is attacker-controlled text, so
"contains the marker" is an insufficient admission test: without the checks
below, any account able to open a PR can forge a locator and either halt every
startup (denial of service) or steer Ship's recovery onto an attacker-chosen
branch, PR number, and checkpoint set. Marker presence establishes only that a
*candidate* block exists — never that it is authentic.

A parsed candidate is **TRUSTED** only when **all** of the following hold. They
are evaluated on API response fields, never on the body text, because the body
is the very thing under suspicion:

| # | Check | Field | Fail action |
|---|---|---|---|
| PV-1 | The PR is **same-repository**: `head.repo.full_name == base.repo.full_name == {owner}/{repo}`. Any fork-originated PR is rejected outright. | `head.repo.full_name`, `base.repo.full_name` | **Discard** as untrusted |
| PV-2 | The PR **base** is the repository's default branch. | `base.ref` | **Discard** as untrusted |
| PV-3 | The PR author is trusted: `author_association` is `OWNER`, `MEMBER`, or `COLLABORATOR`. | `author_association` | **Discard** as untrusted |
| PV-4 | The locator's embedded `pr` equals the PR number the block was read from. | block `pr` vs. API `number` | **Halt to operator** — a self-inconsistent locator on a trusted PR is corruption, not noise |
| PV-5 | The locator's embedded `branch` equals the PR's own head ref. | block `branch` vs. `head.ref` | **Halt to operator** |
| PV-6 | The locator's `shipment` and `feature` exist in this workspace's backlog and the shipment owns that feature. | backlog lookup | **Halt to operator** |
| PV-7 | Every filename in the locator's checkpoint list is a checkpoint this workspace could own (resolvable path shape, `agent` field `ship` or `stage`). | checkpoint lookup | **Halt to operator** |

**Discard versus halt is deliberate and must not be collapsed.** PV-1…PV-3 are
*authenticity* filters: a failing candidate is simply **not ours**, is discarded
**silently**, and MUST NOT halt the session — otherwise any outsider could
permanently deny startup by opening a fork PR containing the marker. PV-4…PV-7
run only on candidates that already passed PV-1…PV-3, so a failure there means a
**trusted** locator is internally inconsistent, which is exactly the corruption
the halt-on-incomplete rule exists to catch.

Discarded (untrusted) candidates MUST NOT be counted toward the
multiple-locator P-001 check below, MUST NOT be acted on, and MUST NOT be
treated as evidence of an outstanding obligation. They SHOULD be logged with
their PR number and the failing check so a genuine misconfiguration is
diagnosable. Every subsequent rule in this section — the P-001 multiplicity
check, `LAST_MILE_RECOVERY` entry, and every action table row — operates
**exclusively over the TRUSTED set**.

**Why not `gh pr list`.** `gh pr list --state all` does **not** return PR bodies
unless `--json ... ,body` is supplied, and it has **no** `--paginate` flag — it
takes a bounded `--limit` that defaults to **30**. The naive formulation
therefore returns thirty bodiless records, finds zero locators, and reports a
clean startup. That is a fail-*open* discovery path masquerading as a fail-closed
one, and it is precisely the failure this requirement exists to prevent. `gh api
--paginate` follows `Link` headers to exhaustion and is the only form permitted.

* Enumeration MUST be **complete**. If the command exits non-zero, or any page
  is truncated, rate-limited, or unparseable, that is an **error**, and the
  session **halts**. An incomplete scan is never evidence of "nothing
  outstanding".
* A shipment-status filter MUST NOT be applied. 139-S's shipment was **archived**
  while its closure obligation was outstanding, so status-keyed discovery would
  have missed exactly the case the mechanism exists for.
* Only the **implementation PR** carries a locator. The post-merge closure PR does
  not publish one: the implementation locator already survives its own merge and
  branch deletion (the PR body is immutable to branch state), and it stays
  discoverable until phase 3 sets `RECONCILED`. A second locator would add a
  reconciliation case with no producer.
* A locator missing any field required for its declared status, or whose block is
  unparseable, is **incomplete** — a **halt-to-operator** signal, never a licence
  to proceed on partial evidence.
* **More than one distinct shipment carrying a non-`RECONCILED` locator is a
  P-001 violation and halts immediately.** P-001 permits exactly one release unit
  in flight; two outstanding closure obligations mean an earlier unit was
  abandoned mid-closure. Recovery must not silently pick one and proceed. This
  check is evaluated over the **TRUSTED** set only (see *Provenance validation*),
  so a discarded fork-authored forgery can neither manufacture a P-001 halt nor
  mask a genuine one.
* **Two or more TRUSTED non-`RECONCILED` locators naming the *same* shipment
  halt to the operator unless they are byte-identical.** The previous wording —
  "handled normally" — was undefined and unsafe. Same-shipment duplicates are
  *not* a P-001 violation, but they are not automatically benign either: they may
  disagree on `branch`, `pr`, the checkpoint filename set, `status`, or
  `final_head`, and no rule can rank them, because both are equally authentic
  under PV-1…PV-7 and the locator carries no revision, sequence, or signature
  field to canonicalize on. `updated_at` MUST NOT be used as a tie-breaker: it is
  body text, so a stale duplicate can carry the later timestamp. The rule is
  therefore mechanical and fail-closed:

  1. Normalize each block (strip trailing whitespace; compare fields, not layout).
  2. If every duplicate is **field-for-field identical**, they are one logical
     locator recorded twice. Proceed on that single record; log the duplication.
  3. If any field differs — *including* `status`, and *including* the case where
     one is `RESOLUTION_PENDING` and another `RESOLUTION_PUBLISHED` — **halt to
     the operator** and present every conflicting copy. Do not merge, do not
     prefer the more advanced status, and do not re-resolve.

  Rationale: an ambiguous same-shipment pair means an earlier session published
  a locator the current one cannot account for. Guessing between them risks
  merging a branch whose resolution commits were never enumerated — the exact
  orphaning failure the locator exists to prevent *(RQ-7, RQ-11)*.

### `LAST_MILE_RECOVERY` *(RQ-10, RQ-11)*

Resolving every checkpoint before merge necessarily leaves a **checkpoint-free
residual window** between the last resolution and verified closure. This window
is unavoidable: any Git-tracked checkpoint intended to cover a post-merge window
is provably unresolvable without a further PR, which is the orphaning defect
itself, one level down. The window is therefore covered by **live state
reconstruction anchored on the `CLOSURE_LOCATOR`**, not by a checkpoint.

Recovery in this window MUST NOT rely on checkpoint enumeration, which will
correctly report zero active candidates. **Entry is wired into the zero-candidate
branch itself**: a session finding zero active checkpoints MUST run
**both discovery channels** — the `CLOSURE_LOCATOR` read protocol (Channel A)
**and** the `RESOLUTION_OBLIGATION_RECORD` Git-history scan (Channel B) —
**before** concluding "clean startup" and before selecting new queue work.
Neither channel may be skipped because the other returned nothing, and a
Channel-A/Channel-B disagreement halts (OB-6). When **both** channels complete
and neither yields a non-`RECONCILED` locator nor an `OPEN` obligation record,
normal startup continues unchanged, byte for byte.

**Step 1 — classify the locator status, then the live PR state; the ancestry
target follows from both.** Asserting `origin/main` ancestry unconditionally is
wrong: while the PR is open its resolution commits are *correctly* not on `main`,
so the assertion would fire a false alarm on the normal path and train the
operator to ignore it.

**Step 1a — locator status gate (runs first).** A `RESOLUTION_PENDING` locator
means the crash happened *between* locator publication and the resolution
commits, so `resolution_commits` and `final_head` are empty **by design**.
Feeding that state into the live-PR table below would compare against an empty
`final_head`, mis-classify it as "HEAD advanced", and — following that row — mark
a branch with **no resolution commits at all** as `RESOLUTION_PUBLISHED` and
present it for merge. Treat it separately and first:

| Locator status | Live PR state | Required action |
|---|---|---|
| `RESOLUTION_PENDING` | **Open** | Resolution has not been published. Do **not** update the locator, do **not** re-establish readiness, do **not** approach the merge bar. Place the working tree on the PR branch (see *Working-tree placement* below), then run the **resolution-state classification** (below) over the locator's checkpoint list at the fetched PR head. **`NONE`** → re-enter `RESOLUTION_PREFIX` at the resolve step. **`PARTIAL`, `ALL`, or `INDETERMINATE`** → do **not** re-resolve; **halt to operator**, because the locator cannot be trusted to enumerate them. If the working tree cannot be placed on the branch, **halt**. |
| `RESOLUTION_PENDING` | **Merged** | **Unrecoverable orphan — halt to operator immediately.** The PR merged carrying unresolved checkpoints. Never run closure, never mark `RECONCILED`. An empty `resolution_commits` list makes every ancestry assertion vacuously pass, so the merged row below must never be reached in this state. |
| `RESOLUTION_PENDING` | any other | **Halt to operator**, per the rows below. |
| `RESOLUTION_PUBLISHED` | — | Proceed to Step 1b. |
| `RECONCILED` | — | Not discovered; terminal. |

**Resolution-state classification (executable; required by the
`RESOLUTION_PENDING` / Open row).** A `RESOLUTION_PENDING` locator deliberately
carries **no** `resolution_commits` and an empty `final_head` *(RQ-8 — the
locator is never self-referential)*, so "have the resolution commits been made?"
cannot be answered from the locator. It MUST be answered from **checkpoint
state at the fetched PR head**, over the locator's own `checkpoints` list, which
is the only enumeration that exists in this phase.

Run after working-tree placement, at the fetched `refs/pull/<pr>/head`, for
**every** filename in the locator's `checkpoints` list:

```text
git fetch origin refs/pull/<pr>/head          # already done by placement
for each <file> in locator.checkpoints:
    git cat-file -e FETCH_HEAD:<file> 2>/dev/null   # does it exist at PR head?
      → absent            ⇒ state(<file>) = MISSING
      → present: read it; parse CheckpointV1
          → parse failure / schema-invalid    ⇒ state(<file>) = UNPARSEABLE
          → status == "resolved"              ⇒ state(<file>) = RESOLVED
          → status == "active"                ⇒ state(<file>) = UNRESOLVED
          → any other status value            ⇒ state(<file>) = UNPARSEABLE
```

Classify the **whole list**, never a sample, then reduce:

| Reduced state | Condition | Required action |
|---|---|---|
| `NONE` | **every** listed checkpoint is `UNRESOLVED` | Resolution never started. **Re-enter `RESOLUTION_PREFIX` at the resolve step.** This is the only branch that resumes. |
| `PARTIAL` | at least one `RESOLVED` **and** at least one `UNRESOLVED` | Crash mid-resolution. **Halt to operator.** Re-resolving would re-commit already-resolved records and the locator cannot enumerate what was done. |
| `ALL` | **every** listed checkpoint is `RESOLVED` | Crash after resolution, before the phase-2 locator update. **Halt to operator** — the locator under-reports the branch's true state and must be reconciled by hand. |
| `INDETERMINATE` | any `MISSING` or `UNPARSEABLE`, **or** the `checkpoints` list is empty, **or** any `git`/read command exits non-zero | **Halt to operator.** Missing or unreadable evidence is never "nothing was done". |

**Precedence is strict**: evaluate `INDETERMINATE` **first**, then `PARTIAL`,
then `ALL`, then `NONE`. A single unreadable file therefore halts rather than
being silently skipped into a `NONE` verdict — the failure mode this
classification exists to close, in which a partially-resolved branch is
re-classified as untouched and resolved a second time.

**Working-tree placement — committing re-entry only.** *(The canonical name;
referenced by that exact phrase from U5 and V12. Revision 8 left this paragraph
with its heading lost, so it was an unnamed fragment that no acceptance
criterion could cite — the mechanical half of open finding 3.)*

`LAST_MILE_RECOVERY` is entered at session start, when the working tree is
normally on `main` and may be a fresh checkout. Committing a resolution on
`main` is forbidden (P-010), and staying on `main` makes the re-entry
undischargeable. This procedure is **required before any committing re-entry**
and is **not** required — and must not be performed — for read-only steps: every
ancestry assertion in Step 1b needs the fetch but **no** switch.

| # | Step | Fail action |
|---|---|---|
| WP-1 | Assert the working tree is **clean** (no staged, unstaged, or untracked changes that a switch would carry or clobber) | **Halt.** Never stash, never discard. |
| WP-2 | Read `branch` and `pr` from a locator that already passed PV-1…PV-7, or from an obligation record that already passed PV-1…PV-3 + PV-B4/PV-B5. An untrusted or unvalidated source is never used | **Halt.** |
| WP-3 | `git fetch origin refs/pull/<pr>/head` | **Halt** on non-zero exit. |
| WP-4 | Place the tree on that tip: if no local branch of that name exists, create it at `FETCH_HEAD` and switch; if one exists, switch and **fast-forward only** to `FETCH_HEAD` | **Halt** on switch failure, and **halt** on any non-fast-forward/divergence. Never `git reset --hard`, never force checkout, never rebase. |
| WP-5 | Assert the **current branch name** equals the locator/record `branch` | **Halt** on mismatch. |
| WP-6 | Assert `HEAD == FETCH_HEAD` | **Halt** on mismatch. |
| WP-7 | Assert HEAD is **not** the default branch and **not** detached | **Halt.** |

**After any WP failure the session performs no resolution and no commit.** It
does not retry with a reset, a force checkout, or a discard; it halts to the
operator with the failing step named. Proceeding on the wrong branch, on a
diverged branch, or on `main` is precisely the hazard this procedure exists to
close, and a "best effort" placement is worse than none because the subsequent
commit would look legitimate.

**Step 1b — live PR state classification** (reached only for a complete
`RESOLUTION_PUBLISHED` locator):

| Live PR state | Ancestry target | Required action |
|---|---|---|
| **Open**, HEAD == locator `final_head` | fetched `refs/pull/<n>/head` | Assert each resolution commit is an ancestor of the fetched PR head; **halt to operator on any failure**. Then re-run the local review, §1.9, CI and P-018 at that HEAD; then the merge-authority bar below. `origin/main` ancestry is **not** expected and its absence is **not** an error. |
| **Open**, HEAD ≠ locator `final_head` | fetched `refs/pull/<n>/head` | **Assert first that each recorded resolution commit is still an ancestor of the fetched PR head** (`git merge-base --is-ancestor <sha> FETCH_HEAD`). A force-push, rebase, or branch reset can drop the resolution commits from the new HEAD; merging that HEAD would re-orphan them. If any assertion fails, **halt to operator**. Only if all pass: treat HEAD-pinned evidence as stale, re-establish readiness at the live HEAD, update the locator `final_head`, then the merge-authority bar. |
| **Merged** | `origin/main` | Assert each resolution commit is an ancestor of `origin/main` (`git merge-base --is-ancestor <sha> origin/main`). A non-ancestor here **is** the 139-S orphan and halts. Otherwise continue the full P-001 closure set; set `RECONCILED` only once that whole set is verified. |
| **Closed, not merged** | — | **Halt to operator.** Never report completion, never re-open, never merge. |
| **PR state unavailable / lookup failed** (including a 404 that cannot be distinguished from a deleted PR) | — | **Halt to operator.** Merge status is never inferred. This row absorbs every failed or ambiguous lookup, so no response code is unhandled. |
| **Merge requested, response lost** | — | Not a GitHub-reported state; it is a client-side execution ambiguity. Re-fetch the PR and `origin/main`, then **re-enter this table** with the freshly observed state. **Never issue a blind second merge.** |
| **Locator incomplete / unparseable** | — | **Halt to operator** with the locator contents surfaced. |

**Boundary with the Merge Confirmation Gate.** The "open PR whose resolution
commits are absent from `origin/main` is not an error" rule belongs to **this
recovery protocol only**. It does **not** relax Ship's Merge Confirmation Gate,
which remains NON-NEGOTIABLE: post-merge closure may not begin while the PR
state is anything other than `MERGED`. Recovery may *reconstruct readiness* for
an open PR; it may never *enter closure* for one.

**Step 2 — merge authority is NOT conferred by this path** *(RQ-10)*. This
protocol restores *readiness evidence*; it is not an alternate merge route and
must never become one. A merge may proceed from this path only when **all** hold,
and otherwise the session **halts**:

1. A **complete** locator (status `RESOLUTION_PUBLISHED`, all resolution SHAs
   present, `final_head` present, block parseable) was the trigger.
2. Merge approval is **proven at the live HEAD** — a fresh explicit operator
   approval at that HEAD. An approval read from a memory file, a checkpoint, or
   the locator itself does **not** count.
3. The full current-HEAD gate set (re-run local review, P-014 §1.9, required CI,
   P-018 when engaged) passes at that live HEAD.
4. Resumption authority never implies merge, admin-fallback, or destructive
   approval.

Any doubt at any of the four — halt. The correct failure mode for the last mile
is an unmerged PR awaiting an operator, never an unsupervised merge.

## Implementation units

Ten units. Each edits **one file**, is **documentation-domain only**, and is
sized for well under two hours. Acceptance criteria below are the *exact* text
carried into the task cards.

### U1 — State the resolution-durability policy *(root)*

**File**: `.github/policies/workflow-policies.md` — one new policy section,
`P-022: Checkpoint Resolution Durability`, placed after P-021.

Add an agent-agnostic policy in the existing policy table format (Policy ID,
Applies To, Gate Point, Statement, Precondition, Postcondition, Violation
Action). It states RQ-1 and RQ-2 normatively and names `RESOLUTION_PREFIX` as the
procedure that satisfies them. It does **not** restate the full ordering — the
instruction file owns that — so the two surfaces cannot drift into two different
orders.

**Acceptance criteria**

1. A new `## P-022: Checkpoint Resolution Durability` section exists, using the
   same field table shape as the surrounding policies.
2. `Applies To` names every agent that resolves Git-tracked checkpoints
   (`ship`, `stage`), not Ship alone.
3. `Gate Point` is stated concretely as *Ship Step 5 pre-merge; Stage and Ship
   session end and crash-resumption resolution*, matching the convention that
   every existing policy names a gate point.
4. The Statement prohibits resolving a Git-tracked checkpoint after its carrying
   PR has merged, and requires that resolution state reach the default branch in
   the same merge as the work it belongs to.
5. The Statement names the 139-S incident as the evidence: commit `43e70430`,
   PR #394, repaired by PR #395.
6. Violation Action is halt plus a P-005 telemetry record, matching the
   surrounding policies' convention.
7. The section **cross-references** `RESOLUTION_PREFIX` in
   `github-pr-automation.instructions.md` by name and does **not** restate the
   sequence, so the two surfaces cannot drift into two different orders.
8. The file's **Amendment Log** gains a row for this addition
   (`1.25.0 — Added P-022: Checkpoint Resolution Durability`), matching the
   convention that every prior policy addition carries one, and the header
   `**Version**` field is reconciled to the log's latest entry. Adding a policy
   without an amendment row would break the file's own governance convention.
9. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

### U2 — Record the HEAD-evidence rule, the resolution order, and the closure locator *(root)*

**File**: `.github/instructions/github-pr-automation.instructions.md` — one new
subsection at **heading level 3**, inserted **after the end of `### 1.9`** (that
is, after its last `####` child) and before the next `###` section. It is a
sibling of `### 1.9`, never spliced inside it — a level-3 heading placed among
1.9's children would silently terminate the readiness-gate section and orphan
1.9.3 onward. No existing section is renumbered.

This file is the **canonical home** for `HEAD_EVIDENCE_RULE`, `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION`
and `CLOSURE_LOCATOR`. It is the correct home because the locator is PR-body
metadata — the surface this file already governs — and because siting the text
here keeps it out of any commit, the property that makes it non-self-referential.

**Acceptance criteria**

1. States that PR-body updates do not advance `headRefOid`, and gives
   point-in-time wording as the alternative for committed documents.
2. Cites the observed instances: PR #395 threads `PRRT_kwDORJEduc6h2uOQ` and
   `PRRT_kwDORJEduc6h2viu`.
3. **`RESOLUTION_PREFIX` and `RESOLUTION_POSTCONDITION` appear here verbatim**, exactly as given in this plan's
   canonical definition, including its four ordering invariants, the
   gate-failure remediation loop, and the residual-window checkpoint
   prohibition. This is the single canonical copy.
4. The `CLOSURE_LOCATOR` block is given verbatim with every field named.
5. The publication protocol is described as **three-phase** and its three phases
   are `RESOLUTION_PENDING`, `RESOLUTION_PUBLISHED`, `RECONCILED`. No other
   phase count appears anywhere in the file.
6. Phase 1 is stated to be published **before the first resolution commit** and
   to require no SHA.
7. Phase 2 is stated to follow the **push** of the resolution commits, and to
   carry the SHAs and `final_head`.
8. Phase 3 is stated to require merge **plus the full P-001 post-merge closure
   set, including the P-020 compaction record** — not merge alone, and not
   partial closure.
9. States explicitly that no commit is ever required to record its own SHA, and
   that a branch-only artifact is not fresh-checkout discoverable.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U3 — Record exhaustive discovery and last-mile recovery

**Depends on**: U2 (extends the subsection U2 creates).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

Add the locator read protocol and `LAST_MILE_RECOVERY` immediately after U2's
subsection.

**Acceptance criteria**

1. The read protocol gives the **exact executable command**, whose `--jq`
   projection carries **every field the provenance checks are evaluated on** —
   never the pre-provenance `{number, state, body}` form:
   `gh api --paginate -H "Accept: application/vnd.github+json"
   "repos/{owner}/{repo}/pulls?state=all&per_page=100" --jq '.[] | {number,
   state, body, author_association, head_repo: .head.repo.full_name, head_ref:
   .head.ref, base_repo: .base.repo.full_name, base_ref: .base.ref}'`. It
   explicitly records **why `gh pr list` is forbidden**: it returns no body
   without `--json ...,body` and has no `--paginate` flag, only a bounded
   `--limit` defaulting to 30, so the naive form silently reports a clean
   startup. It further requires that **`PV-1`…`PV-7` provenance validation runs
   on every marker-bearing candidate before that candidate participates in
   recovery**, evaluated on API response fields rather than body text, and
   reproduces the **discard-versus-halt split without collapsing it**: `PV-1`
   (same-repository), `PV-2` (base is the default branch) and `PV-3` (author
   `OWNER`/`MEMBER`/`COLLABORATOR`) are authenticity filters whose failures are
   **discarded silently and MUST NOT halt**, or an outsider could permanently
   deny startup with a fork PR carrying the marker; `PV-4`…`PV-7` (locator `pr`
   matches, `branch` matches head ref, shipment/feature resolve, checkpoint
   filenames are ownable) run only on candidates that already passed
   `PV-1`…`PV-3` and **halt to the operator**, because a trusted-but-inconsistent
   locator is corruption. Discarded candidates MUST NOT be counted toward the
   P-001 multiplicity check, acted on, or treated as evidence of an outstanding
   obligation, and every downstream rule operates over the **TRUSTED set only**.
2. A non-zero exit, a truncated or rate-limited page, or an unparseable response
   is stated to be an **error that halts**, never evidence that nothing is
   outstanding.
3. Filtering discovery by shipment status is **explicitly prohibited**, with the
   139-S archived-shipment case given as the reason.
4. It is stated that only the implementation PR publishes a locator, and why: the
   PR body survives merge and branch deletion, so no closure-PR locator is needed.
5. Non-`RECONCILED` locators for **more than one distinct shipment** are stated
   to be a P-001 violation that halts immediately.
6. An incomplete or unparseable locator is stated to be a halt-to-operator
   signal.
7. The **Step 1a locator-status gate** appears with all five rows, and states
   that a `RESOLUTION_PENDING` locator is evaluated **before** any live-PR
   classification — including that a merged PR with a `RESOLUTION_PENDING`
   locator is an unrecoverable orphan that halts, and must never reach an
   ancestry assertion that an empty commit list would vacuously pass.
8. The **Step 1b live-PR classification** table appears with all seven rows and
   their stated ancestry targets and actions.
9. The `Open, HEAD ≠ final_head` row requires the ancestry assertion against the
   fetched PR head **before** re-establishing readiness, and states that a
   force-push or rebase that dropped the resolution commits halts.
10. It is stated that an open PR whose resolution commits are absent from
    `origin/main` is **not** an error **for this recovery protocol**, and that
    this does **not** relax Ship's Merge Confirmation Gate.
11. The four-part merge-authority bar appears in full, with halt as the default
    for any doubt, and it is stated that this path confers **no** merge authority
    and cannot supply the approval signal P-014 requires.
12. A procedure titled **exactly** `Working-tree placement — committing re-entry
    only` appears, and is required **before any committing re-entry** and
    explicitly **not** required for read-only ancestry assertions (fetch, no
    switch). It carries all seven steps with their halt actions: **WP-1** clean
    working tree; **WP-2** branch/PR read only from a provenance-validated
    locator or obligation record; **WP-3** `git fetch origin
    refs/pull/<pr>/head`; **WP-4** safe create-and-switch for a new local branch
    and **fast-forward-only** switch for an existing one; **WP-5** current branch
    name equals the recorded `branch`; **WP-6** `HEAD == FETCH_HEAD`; **WP-7**
    HEAD is neither the default branch nor detached. It states that a dirty tree,
    a failed fetch, a failed switch, a divergence, or any assertion mismatch
    **fails closed**, and that after any failure the session performs **no**
    `git reset`, **no** force checkout, **no** resolution and **no** commit.
13. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U4 — Re-order Ship Step 5 into the `RESOLUTION_PREFIX` finalization tail

**Depends on**: U1, U2, U9.
**File**: `.github/agents/_ship.agent.md` — **Step 5 (PR Lifecycle)** and
**Session end item 2**.

**The real Step 5 item list, extracted verbatim rather than described.** The
escalation's assessment was that every prior round mismatched what this plan
*asserted* Step 5 contains against what it *actually* contains. The extraction
below is the authority for this unit; the task card carries it verbatim.

| Real item | Content | Mutates branch? |
|---|---|---|
| 1, 1a | full quality gates; `pipeline-topology` lifecycle gate | no |
| 2 | session memory summary to `docs/memory/` | yes (commit) |
| 3 | full local build | no |
| 4, 5, 5a | confirm readiness covers HEAD; prepare §1.9 body block; topology gate | no |
| 6 | `pr-lifecycle` — create/update the PR | no |
| **7** *(first)* | `fix-ci` loop | **yes — may commit and push** |
| 7a | optional shadow-review loop | **yes — may commit and push** |
| 7b | **P-014 §1.9 readiness gate** | no |
| 7c | **P-018 copilot-review gate** | no |
| **7** *(second — the item number is duplicated in the live file)* | `runtime-verification` | yes |
| 8 | `operational-closure` (writes `docs/closure/`) | yes |
| 9 | follow-up stash writes | yes |
| 10 | push the branch | yes |
| 11, 12, 13 | broadcast; present PR; branch retention | no |
| 14 | P-014 operator approval gate | no |
| 15 | last-mile re-check — **P-018 re-run + `headRefOid` re-query only** | no |
| 16 | P-009 merge-commit guardrail | no |
| 17 | P-017 dark-mode fallback state machine | no |

Two facts follow directly and are the substance of this unit. First, **items 7b
and 7c currently run before items 7(second)/8/9/10**, so readiness is gated
*before* four mutating items and the push — which is why moving resolution into
Step 5 changes the common path for **every** unit, not only checkpoint-owning
ones. Second, **item 15 re-fetches only the P-018 verdict and `headRefOid`** — it
never evaluates required checks and never re-paginates review threads — so RQ-6
has no executable enforcement path in the live file. Revision 8's assertion that
the approval/re-fetch/merge items "run unchanged" was false; it is withdrawn.

Wire `RESOLUTION_PREFIX` in as the **finalization tail** of Step 5 by name, and
amend items 7b, 7c and 15 to their new positions and strengthened content. Then
amend Session end item 2 so it no longer resolves after merge and no longer
creates a residual-window checkpoint.

**Acceptance criteria**

1. Step 5 invokes **`RESOLUTION_PREFIX`** *by name*, referencing
   `github-pr-automation.instructions.md` as its canonical definition, and does
   **not** restate any of its steps, orderings or rationale. A reader must follow
   the reference to learn the sequence.
2. The resulting Step 5 order matches the canonical segments exactly, and the
   task card records the **old-to-new item mapping** using the verbatim extract
   above so the reorder is auditable rather than implicit:
   **S1** = items 7 (first) and 7a, run to completion first, permitted to commit
   and push; **S2** = item 7 (second), item 8, item 9, item 10 (the ordinary
   push); **S3** = the checkpoint-enumeration proof; **S4** = the conditional
   locator/resolution/push segment; **S5** = the re-run local review, the final
   locator resolution state, the PR-body `Reviewed HEAD` write, then item **7b
   (MOVED here)**, the explicit required-check evaluation, and item **7c (MOVED
   here)**; **S6** = item 14 with a recorded `approved_head`; **S7** = item 15,
   **amended**; **S8** = the six-part merge bar, after which items 16 and 17
   apply unchanged.
3. **The checkpoint count selects segment S4 only.** The criterion states that a
   unit whose enumeration proves **zero** checkpoints omits **exactly four
   things** — locator publication, checkpoint resolution, the resolution
   commit/push, and the phase-2 locator publication — and **runs every other
   segment identically**, including the whole reordered common finalization tail.
   The criterion MUST NOT claim that any unit runs "the pre-existing path
   unchanged"; that claim is false and is expressly prohibited here. It further
   states that a failed, malformed, quarantined, or ambiguous enumeration is
   **not zero** and **halts**, that a checkpoint appearing after S3 forces
   re-evaluation from S3 before merge, that **no empty locator is ever
   published**, and that Stage's crash-resumption startup recovery is a separate
   protocol this does not alter. The circular "PR merge-ready" wording is **not**
   used.
4. **No branch-mutating step remains after S4.** Every mutating item in the
   verbatim extract — item 2, item 7 (first), 7a, item 7 (second), 8, 9, 10 —
   sits in S1 or S2, before the enumeration proof. Nothing between S5 and merge
   mutates the branch.
5. Session end item 2 no longer resolves checkpoints after merge. No
   checkpoint-resolution step remains anywhere after merge in this file.
6. Session end item 2's directive to *"leave at most one final best-effort
   checkpoint"* is **retired for units inside `RESOLUTION_PREFIX`**, with the
   recursion reason stated: such a checkpoint could only be resolved by a further
   commit needing a further PR. The window is covered by `CLOSURE_LOCATOR`, the
   `RESOLUTION_OBLIGATION_RECORD` and `LAST_MILE_RECOVERY` instead.
7. Step 6 gains **`RESOLUTION_POSTCONDITION`** by name — set the locator to
   `RECONCILED` **and** transition the `RESOLUTION_OBLIGATION_RECORD` from
   `OPEN` to `CLOSED` in the same post-merge closure commit, after the full
   required closure set is verified.
8. Step 6.0's post-merge branch rule is unchanged for every other closure
   artifact, and **Step 6's P-020 `compact-context` invocation is present and
   unmodified**.
9. Item **15 is amended**, not retained unmodified. *(Revision 8's "item 15
   retained unmodified" criterion is withdrawn: it forbade the unit from
   implementing the very requirement it was credited with.)* The amended item
   re-fetches and re-evaluates, unconditionally and fail-closed, **all** of:
   `headRefOid`; the PR body; `reviewDecision`; review requests and reviews;
   **every review-thread page to exhaustion**; required checks; and the
   **ancestry of every recorded resolution commit**. Item **16 (P-009) is
   retained unmodified**.
10. The merge step permits merge **only** when all six merge-bar conditions hold:
    live `headRefOid` == local HEAD == locator `final_head` == PR-body
    `Reviewed HEAD` == `approved_head`; every recorded resolution commit is an
    ancestor of that HEAD; review-thread pagination completed to exhaustion; no
    blocking thread or review exists; required checks pass or are explicitly
    **proven** non-applicable; and P-018 passes. It further states the refresh
    rules: any HEAD change voids the approval and requires a fresh
    push/review/metadata/gates/approval cycle, and a thread or check change
    without a HEAD change requires the affected gates plus a refreshed approval.
    **No stale approval may be reused.**
11. S4's resolution commit also writes the `RESOLUTION_OBLIGATION_RECORD` at
    `OPEN` **in the same commit**, and that commit is pushed before the S5 review
    re-run.
12. The section cites P-022.
13. markdownlint passes.

**Posture**: documentation-first. **Size**: M. **Complexity**: high.
*(Revision 9 raises this from S/medium honestly: the unit now carries a verbatim
item extract, a segment mapping, two moved items and one amended item. It remains
a single-file documentation change and stays inside two hours because the
extract removes the re-derivation work every prior round repeated.)*

### U5 — Ship orphan detection, startup discovery, and locator reconciliation

**Depends on**: U3, U4, U9.
**File**: `.github/agents/_ship.agent.md` — Merge Confirmation Gate and the
`ZERO-CANDIDATE NORMAL STARTUP` block.

Two assertions plus a startup entry point. The durable input is the
`CLOSURE_LOCATOR` in the PR body — not a commit, not a branch-only artifact.

**Acceptance criteria**

1. The locator status is classified **before** the live PR state, per Step 1a, and
   the live PR state is classified **before** any ancestry assertion, per
   Step 1b.
2. The exact commands are given:
   `gh pr view <n> --json state,mergedAt,headRefOid`,
   `git fetch origin refs/pull/<n>/head`, and
   `git merge-base --is-ancestor <sha> <target>`.
3. An open PR whose resolution commits are not on `origin/main` is stated **not**
   to be an error **for the recovery path**, together with an explicit statement
   that this does **not** relax the Merge Confirmation Gate: post-merge closure
   still may not begin while the PR state is anything other than `MERGED`.
4. A merged PR with a non-ancestor resolution commit is stated to be the 139-S
   orphan and **halts**. A merged PR whose locator is still `RESOLUTION_PENDING`
   is stated to be an unrecoverable orphan and **halts** without any ancestry
   assertion.
5. Closed-unmerged, missing PR, failed lookup, and incomplete locator all halt to
   the operator; failure is surfaced, never logged silently.
6. Ship's `ZERO-CANDIDATE NORMAL STARTUP` item 4 runs **both** discovery
   channels — the `CLOSURE_LOCATOR` read protocol and the
   `RESOLUTION_OBLIGATION_RECORD` Git-history scan — **before** continuing to
   normal shipment validation, with the same halt-on-incomplete-enumeration rule
   for each, and halts on a Channel-A/Channel-B disagreement (OB-6). Ship is
   directly invokable, so relying on the Orchestrator's route alone would leave
   direct-Ship startup uncovered.
7. The locator is set to `RECONCILED` **and** the `RESOLUTION_OBLIGATION_RECORD`
   is transitioned `OPEN` → `CLOSED` in the same post-merge closure commit,
   **only after the full P-001 post-merge closure set is complete and verified,
   including the P-020 compaction record**, and it is stated that setting either
   at merge or after partial closure would lose the obligation, and that deleting
   the record is never a discharge.
8. No-op when no locator was published **and** no obligation record is `OPEN`;
   every recorded resolution commit is asserted when several were recorded.
9. **Before any committing re-entry**, the `Working-tree placement — committing
   re-entry only` procedure is executed **by that exact name**, and all seven of
   its steps are named with their halt actions: **WP-1** clean working tree;
   **WP-2** branch/PR read only from a provenance-validated locator or obligation
   record; **WP-3** `git fetch origin refs/pull/<pr>/head`; **WP-4** safe
   create-and-switch for a new local branch, **fast-forward-only** for an
   existing one; **WP-5** current branch name equals the recorded `branch`;
   **WP-6** `HEAD == FETCH_HEAD`; **WP-7** HEAD is neither the default branch nor
   detached. A dirty tree, failed fetch, failed switch, divergence, or any
   assertion mismatch **fails closed**, and after any failure the session
   performs **no** `git reset`, **no** force checkout, **no** resolution and
   **no** commit. It is stated that read-only ancestry assertions fetch but do
   **not** switch, and therefore do not run this procedure.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U6 — Orchestrator locator reconciliation route

**Depends on**: U3, U5, U9.
**File**: `.github/agents/_orchestrator.agent.md` — Step 0.0b, applied to the
whole step rather than only its zero-candidate arm.

Wire `LAST_MILE_RECOVERY` routing into Step 0.0b. Without this the protocol has
no Orchestrator entry point: an Orchestrator finding no checkpoints proceeds to
state assessment and then to queue selection, starting new work while an unmerged
PR carrying resolution commits sits open.

**Scoping correction (revision 8).** Revision 7 wired discovery only into the
**global** zero-candidate arm — "no checkpoints at all". That leaves the exact
window this plan exists to close still open: Orchestrator permits planning
overlap, so a **Stage-owned** checkpoint can legitimately be active while Ship's
last-mile obligation is outstanding. Under the revision-7 wiring the Orchestrator
would see "a checkpoint exists", skip discovery entirely, never route Ship, and
then skip the shipment at Step 2 because it is still `active` — so nobody
discharges the obligation. Discovery must therefore be scoped the same way Ship's
is (U5 AC6): keyed on the absence of a **ship-owned** active checkpoint, not on
global emptiness.

**Acceptance criteria**

1. Step 0.0b runs **both** discovery channels — the `CLOSURE_LOCATOR` read
   protocol and the `RESOLUTION_OBLIGATION_RECORD` Git-history scan — whenever
   there is **no `ship`-owned active checkpoint**, regardless of whether
   `stage`-owned checkpoints exist, and always **before** `Continue directly to
   Step 0 State Assessment` — therefore before any queue selection.
2. When a `ship`-owned active checkpoint **does** exist, the pre-existing
   owner-routing path is unchanged and discovery is skipped, because that
   checkpoint already carries the obligation.
3. A discovered non-`RECONCILED` locator **or** an `OPEN` obligation record is
   routed to **Ship** for `LAST_MILE_RECOVERY` **before** any Stage routing and
   before queue selection. If a `stage`-owned checkpoint is also present, both
   are presented to the operator and no new queue work is auto-selected.
4. Discovery is not filtered by shipment status, so an archived shipment with an
   outstanding non-`RECONCILED` locator or `OPEN` record is found (the 139-S
   shape).
5. Incomplete enumeration in **either** channel halts rather than falling
   through to state assessment, and a Channel-A/Channel-B disagreement halts
   (OB-6). A missing locator never discharges an `OPEN` record.
6. Non-`RECONCILED` locators for more than one distinct shipment halt as a P-001
   violation.
7. Routing into `LAST_MILE_RECOVERY` conveys **no merge authority** and no
   implicit approval; the four-part merge bar applies unchanged and any doubt
   halts. In dark-factory mode (P-017), a dark approval for the *current* declared
   scope does **not** satisfy the merge bar for a *prior* unit's recovered
   obligation.
8. Owner exclusivity is preserved: the Orchestrator **routes**; Ship **performs**
   the reconciliation. The Orchestrator never performs recovery itself.
9. When **neither** channel finds anything, the pre-existing fall-through is
   unchanged: the same `Continue directly to Step 0 State Assessment`
   instruction, the same non-failure classification, and no new operator
   interaction.
10. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U8 — Close the Stage-side resolution gap

**Depends on**: U1.
**File**: `.github/agents/_stage.agent.md` — **both** resolve sites: Session end
item 2 **and** the Crash-Resumption `OWNER-SCOPED RESOLUTION` block.

Stage's Session end item 2 carries the same *"resolve any still-active
checkpoints"* and *"leave at most one final best-effort checkpoint"* sentences as
Ship's, and its `OWNER-SCOPED RESOLUTION` block resolves after a confirmed
resume. P-022 (U1) is agent-agnostic, so after U1 lands, Stage would hold a
procedure that unconditionally resolves against a policy that forbids resolving
after the carrying PR merged. Revision 6 recorded this as residual risk RR-2; the
independent review correctly held that a universal requirement with a procedural
gap in one of its two named agents is not realized. This unit closes it with a
narrow qualifier rather than by narrowing the policy.

Stage cannot open, approve, or merge pull requests (P-010), so Stage does **not**
execute `RESOLUTION_PREFIX` and is given no locator obligation. The qualifier is
only: do not resolve into an already-merged staging PR.

**Acceptance criteria**

1. **Both** resolve sites — Session end item 2 and `OWNER-SCOPED RESOLUTION` —
   state that a checkpoint MUST NOT be resolved once the staging PR carrying its
   resolution has merged, citing P-022.
2. An **executable predicate** is given for that test, not a prose condition:
   determine the current branch, then
   `gh pr list --state merged --head <branch> --json number,mergedAt`; a non-empty
   result means the carrying PR has merged. A lookup failure **halts** rather than
   assuming "not merged".
3. The correct action in the merged case is stated: leave the checkpoint active,
   surface it, and hand off to the operator — never resolve into a merged branch.
4. The *"leave at most one final best-effort checkpoint"* directive is qualified:
   a Git-tracked checkpoint MUST NOT be created when no open staging PR can carry
   its eventual resolution, because Stage cannot open a PR (P-010) and so could
   never discharge it. Hand off to the operator instead.
5. It is stated that Stage does **not** execute `RESOLUTION_PREFIX` and publishes
   no locator, because Stage holds no merge authority (P-010); the reference is to
   P-022's prohibition only.
6. No other Stage behaviour is modified; the existing checkpoint payload contract
   and normal (unmerged-PR) resolution semantics are unchanged.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U9 — Record the durable resolution-obligation record *(closes RR-3)*

**Depends on**: U3 (extends the same subsection; same file, strictly sequential).
**File**: `.github/instructions/github-pr-automation.instructions.md`.

This unit is revision 9's answer to **RR-3**, the blocking design gap the round-8
review left open. It installs the canonical `RESOLUTION_OBLIGATION_RECORD`
definition — the Git-tracked, history-immutable second channel that makes RQ-7
true when the mutable PR body is deleted. It does **not** weaken RQ-7, and it
does **not** touch RQ-8: the record carries no SHA.

**Acceptance criteria**

1. The `RESOLUTION_OBLIGATION_RECORD` schema block appears verbatim as given in
   this plan's canonical definition, with its canonical **path** (the unit's
   pre-merge operational-closure artifact under `docs/closure/`) and canonical
   **identity** (`shipment_id`, `pr`; exactly one `OPEN` record per shipment).
2. It is stated that the record **carries no SHA** and why: it is written inside
   the resolution commit, so any SHA field would be self-referential — RQ-8 is
   preserved, not traded. Recovery re-derives SHAs from the existing
   *resolution-state classification* at the fetched PR head.
3. The three-transition lifecycle table appears: absent → `none` (S2, the
   `operational-closure` artifact write); `none` → `OPEN` (S4, **in the same
   commit as the checkpoint resolutions**, pushed before the S5 review re-run);
   `OPEN` → `CLOSED` (`RESOLUTION_POSTCONDITION`, in the post-merge closure
   commit, after the full verified P-001 closure set). It is stated that
   `OPEN` → `CLOSED` is the **only** discharge, that it happens in a **later**
   commit whose ancestry stays auditable, that the closure commit must itself be
   **merged**, and that **deletion is never a discharge**.
4. It is stated that `docs/closure/` is chosen because it is an **existing owned
   repo-local state surface** — created by the `operational-closure` skill,
   already written at real Step 5 item 8, and already carrying the same
   placeholder→finalize field convention as `compaction_status` — and that a
   new standalone tracker file, a checkpoint-store record, and a commit-message
   record were each rejected, with the stated reason for each.
5. **Two-channel discovery** is specified: Channel A (the PR-body
   `CLOSURE_LOCATOR`) and Channel B (the Git-history scan). Both run, the union
   is taken, and **neither may be skipped because the other returned nothing**.
6. Channel B's commands are given exactly — `git ls-tree -r --name-only
   origin/main -- docs/closure/`; `git fetch origin refs/pull/<pr>/head` plus
   `git ls-tree -r --name-only FETCH_HEAD -- docs/closure/` for every
   PV-1…PV-3-admitted PR; and the deletion probes `git log --follow
   --diff-filter=D --format=%H -- <artifact path>` and `git log --format=%H
   -S'resolution_obligation' -- docs/closure/`.
7. It is stated that Channel B's provenance is established **only** from API
   response fields and Git ancestry — PV-1, PV-2, PV-3, plus **PV-B4** (record
   `pr` equals the PR read from) and **PV-B5** (record `branch` equals that PR's
   head ref), with PV-6 and PV-7 unchanged — and **never** from PR-body text,
   because the body is the surface under suspicion. PV-4 and PV-5 are stated to
   be inapplicable to Channel B.
8. **Deletion is detected via commit history, never inferred from absence.** The
   criterion states that a record which ever appeared in history and is now
   absent from the tree **without** a recorded `OPEN` → `CLOSED` transition is a
   deletion that **halts**, and that absence from the current tree is never
   evidence that no obligation exists.
9. All seven fail-closed rules appear with their halt actions: **OB-1** deletion;
   **OB-2** force-push/rebase/shallow history making ancestry unverifiable;
   **OB-3** conflicting `OPEN` records for one shipment (byte-identical
   duplicates collapse and are logged; **any** field difference halts;
   `opened_at` is barred as a tie-breaker); **OB-4** `OPEN` records for more than
   one shipment (P-001); **OB-5** unparseable or field-incomplete record;
   **OB-6** Channel A/Channel B disagreement, with the explicit rule that the
   mutable channel is **never** preferred and a missing locator **never**
   discharges an `OPEN` record; **OB-7** incomplete Channel-B enumeration.
10. It is stated explicitly that this mechanism introduces **no** executable
    persistence substrate, **no** lock or compare-and-swap, **no** cross-run
    cursor persistence, and **no** part of Defect 1, and that it confers **no**
    merge authority — its only authority is to halt (RQ-10 untouched).
11. markdownlint passes.

**Posture**: documentation-first. **Size**: S. **Complexity**: medium.

### U10 — Declare the obligation field in the closure-artifact schema

**Depends on**: U9.
**File**: `.github/skills/operational-closure/SKILL.md` — the pre-merge closure
artifact's field list only.

**Why this unit exists, and why it is a scope addition made openly.** The
`operational-closure` skill **owns** the `docs/closure/` artifact schema. Adding
a field that the skill does not declare would leave the record an unowned
squatter on someone else's artifact — precisely the "ad-hoc tracker" the RR-3
correction is required to avoid — and would drift the moment the skill's field
list changed. One field declaration keeps the surface owned. This adds
`.github/skills/operational-closure/SKILL.md` to the plan's in-scope file set;
the decision's scope boundary is updated to match rather than the addition being
made silently.

**Acceptance criteria**

1. The pre-merge closure artifact's field list gains `resolution_obligation`,
   documented in the same style as the existing `compaction_status` and
   source-artifact-cleanup fields.
2. The skill is stated to **initialize** the field to `none` when it creates the
   pre-merge artifact, exactly as it initializes `compaction_status` to
   `pending`.
3. The allowed transitions are stated as `none` → `OPEN` (written by Ship in the
   resolution commit) and `OPEN` → `CLOSED` (written by Ship's post-merge
   closure). No other transition is permitted, and the skill does **not** itself
   write `OPEN` or `CLOSED`.
4. The field's canonical definition is **referenced by name** in
   `github-pr-automation.instructions.md`; the skill does **not** restate the
   schema, the discovery protocol, or the fail-closed rules.
5. No other skill behaviour is modified — `compaction_status` handling, the
   source-artifact-cleanup placeholder, and the releasability verdict set are
   untouched.
6. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

### U7 — Capture the compound learning *(closure deliverable)*

**Depends on**: U5, U6.
**File**: `docs/compound/workflow-issues/` — one new learning.

U7 is a **closure deliverable**, not a requirement-realizing unit. It implements
none of RQ-1 … RQ-11 and is deliberately absent from the requirement trace table
below. It is retained because capturing hard-won solutions is standing workspace
practice, and this defect class cost two extra pull requests to discover.

**Acceptance criteria**

1. The structural post-merge ordering root cause is documented with its observed
   evidence: 139-S, PR #394, commit `43e70430`, repair PR #395.
2. `HEAD_EVIDENCE_RULE` is stated with the PR #395 thread citations
   (`PRRT_kwDORJEduc6h2uOQ`, `PRRT_kwDORJEduc6h2viu`).
3. The **self-referential locator trap** is recorded: a commit cannot contain its
   own SHA, and a branch-only record is invisible to the fresh checkout that must
   perform the recovery.
4. The **status-filtered discovery trap** is recorded: 139-S's shipment was
   archived while its obligation was outstanding.
5. The **sole-mutable-record trap** is recorded: resolving every checkpoint
   pre-merge makes the PR body the only obligation record unless a Git-tracked
   record rides the resolution commit, and absence of a record must be
   distinguished from deletion of one via commit history.
6. Frontmatter matches the prevailing convention of
   `docs/compound/workflow-issues/` and is internally consistent.
7. markdownlint passes.

**Posture**: documentation-first. **Size**: XS. **Complexity**: low.

## Requirement → unit traceability

Every requirement has exactly **one owning unit** — the unit that installs the
normative text — plus zero or more **enforcing units** that wire that text into
an execution path. U7 is a closure deliverable and realizes no requirement; it is
therefore absent by design.

| Requirement | Owning unit | Enforcing units |
|---|---|---|
| RQ-1 no post-merge resolution | U1 (policy) | U4 (Ship), U8 (Stage) |
| RQ-2 resolution rides the same merge | U2 (`RESOLUTION_PREFIX`) | U4 (Ship). **Not U8** — U8 enforces the RQ-1 prohibition on the Stage path; Stage's resolution reaches `main` through the ordinary staging-PR merge, which Stage does not control, so U8 cannot *guarantee* RQ-2 and is no longer credited with it. |
| RQ-3 push before evidence | U2 | U4 |
| RQ-4 re-run the actual review | U2 | U4 |
| RQ-5 PR-body record precedes §1.9 | U2 | U4 |
| RQ-6 approval after gate, live re-fetch before merge | U2 (`RESOLUTION_PREFIX` S5–S8) | U4 (Ship — moves items 7b/7c, **amends item 15**, installs the six-part merge bar and the refresh rules) |
| RQ-7 locator discoverable through closure | U2 (`CLOSURE_LOCATOR`) | U5, **U9** (the durable Git channel that makes RQ-7 hold when the PR body is deleted) |
| RQ-8 non-self-referential locator | U2 | U9 (the obligation record carries no SHA, by construction) |
| RQ-9 exhaustive, trusted, status-independent discovery | U3 (read protocol) | U5, U6, U9 (Channel B) |
| RQ-10 no merge authority conferred | U3 (`LAST_MILE_RECOVERY` Step 2) | U5, U6, U9 |
| RQ-11 every failure halts | U3 | U5, U6, U9 (OB-1 … OB-7) |
| RQ-12 durable Git-tracked obligation record | **U9** (`RESOLUTION_OBLIGATION_RECORD`) | U4 (writes it in the resolution commit), U5 (discovers and closes it), U6 (routes on it), U10 (schema ownership) |

## Dependencies

```text
U2 ──→ U1 ──┬──→ U4 ──→ U5 ──→ U6 ──→ U7
  │         └──→ U8            ↑
  └──→ U3 ──→ U9 ──┬──→ U4     │
                   ├──→ U5 ────┤
                   ├──→ U6 ────┘
                   └──→ U10
```

Sixteen edges: **U2→U1**, U1→U4, U1→U8, U2→U4, U2→U3, U3→U5, U3→U6, U4→U5,
U5→U6, U5→U7, U6→U7, **U3→U9**, **U9→U4**, **U9→U5**, **U9→U6**, **U9→U10**.
The graph is acyclic.

The single **root is U2**. Revision 7 listed U1 and U2 as co-roots, but U1
installs a by-name reference to `RESOLUTION_PREFIX` whose definition only U2
creates; running U1 first would leave a dangling reference that cannot be
verified. **U2→U1** is therefore a real edge, and it transitively orders U8 after
the canonical definition too. **U9→U4** is the same kind of edge: U4's AC11
requires the resolution commit to write a record whose definition only U9
creates.

The three units that share `github-pr-automation.instructions.md` (U2, U3, U9)
are strictly sequential, as are the two that share `_ship.agent.md` (U4, U5). U8
is the only unit touching `_stage.agent.md`, U6 the only unit touching
`_orchestrator.agent.md`, U1 the only unit touching `workflow-policies.md`, and
U10 the only unit touching `operational-closure/SKILL.md`. No two
concurrently-eligible units edit the same file.

## Verification

| # | Check | Expected result |
|---|---|---|
| V1 | `markdownlint` over every changed file | passes |
| V2 | **Canonical-copy check**, in two parts. **(a)** The `RESOLUTION_PREFIX` code block appears verbatim exactly once, in `github-pr-automation.instructions.md`. **(b)** *Sequence-restatement check (rewritten in revision 9; the revision-8 form was unsatisfiable — it banned the ordinary verbs `resolve`, `record` and `push`, which U1's policy Statement and U8's Stage prohibition are **required** to use).* In each referencing file's changed region — `workflow-policies.md`, `_ship.agent.md`, `_stage.agent.md`, `_orchestrator.agent.md`, `operational-closure/SKILL.md` — assert that **no ordered enumeration of three or more canonical segment steps appears**: that is, no list or arrow-chain reproducing three or more of the S1…S8 segment contents in their canonical order. Ordinary prose use of any individual verb is **permitted and expected**. | (a) exactly one verbatim copy; (b) no referencing file contains a three-or-more-step ordered restatement; each contains the bare name `RESOLUTION_PREFIX` / `RESOLUTION_POSTCONDITION` / `RESOLUTION_OBLIGATION_RECORD` plus a file reference |
| V3 | Grep every changed file for `two-phase` and for `pr_role` | zero occurrences of each; only "three-phase" appears |
| V4 | **Ordered-step inspection** of `_ship.agent.md` (not a grep), recording the item numbers against U4's verbatim extract: read Step 5 top-to-bottom and confirm (a) items 7 (first) and 7a complete first; (b) item 7 (second), 8, 9 and 10 (the push) all precede the checkpoint-enumeration proof; (c) items **7b and 7c now appear after that push**, together with the re-run review, the PR-body `Reviewed HEAD` write and an explicit required-check evaluation; (d) item 14 records `approved_head`; (e) item 15 is **amended** to re-fetch headRefOid, PR body, reviewDecision, review requests/reviews, every review-thread page, required checks and resolution-commit ancestry; (f) item 16 (P-009) is byte-identical to its pre-change text; (g) **no branch-mutating item appears after the enumeration proof**; then read Step 6 and Session end and confirm neither contains a resolution or checkpoint-creation step for the merged unit | (a)–(f) each hold as stated; (g) the set is empty of mutations; no post-merge resolution or creation |
| V5 | Backlog structure equals **one top-level release unit → 2 sub-epics → 10 tasks**, and the shipment manifest membership equals those **13** IDs. *(IDs are allocated at harvest; the abandoned `143.*` IDs must not be reused.)* | exact **membership** match; order is not asserted, because Ship treats manifest order as non-executable and sorts by unfinished dependencies |
| V6 | Every task card's acceptance criteria are textually identical to its unit's criteria in this plan | identical |
| V7 | Grep the **changed files** (not this plan) for this closed list of Defect-1 construct names: `DARK_CONTINUATION_PREDICATE`, `ACTIVATION_RECORD_STORE`, `CURSOR_TYPING_RULES`, `CONTINUATION_HANDOFF_EVIDENCE`, `OWNER_SIDE_REVALIDATION`, `PREDICATE_PRECEDENCE`, `MIS_EVALUATION_DIRECTIONALITY`, `SCOPE_MATCH_RULES`, `check-continuation-predicate-drift`, `canonical-phrases.json`, `parity-gate` | zero occurrences of all eleven |
| V8 | Execute the U3 discovery command against this repository with `per_page=2`, count returned records; compare against the **paginated** reference `gh api --paginate -H "Accept: application/vnd.github+json" "repos/{owner}/{repo}/pulls?state=all&per_page=100" --jq '.[].number' \| Measure-Object -Line`. *(Revision 9 fix: the revision-8 reference command omitted `--paginate`, so it was itself capped at one page and the comparison was invalid — it could only ever prove the two commands disagreed.)* | both counts are equal **and** exceed 2, proving `Link`-header pagination actually followed; every record carries a non-empty `body` field; exit code 0 on both |
| V9 | Confirm Step 6's P-020 `compact-context` invocation in `_ship.agent.md` is byte-identical to its pre-change text | identical |
| V10 | Read `_stage.agent.md` Session end item 2 **and** the `OWNER-SCOPED RESOLUTION` block; confirm both carry the P-022 merged-PR prohibition and the executable `gh pr list --state merged --head <branch>` predicate, and that the best-effort-checkpoint directive is qualified | both sites carry both; directive qualified |
| V11 | Read `_orchestrator.agent.md` Step 0.0b; confirm discovery is keyed on the absence of a **ship-owned** active checkpoint, not on global zero-candidate, and that it runs **both** channels | keyed on ship-owned absence; both channels present |
| V12 | **Working-tree placement coverage** *(the verification half of D14)*. Read U3's criteria and confirm the procedure named **exactly** `Working-tree placement — committing re-entry only` is defined with WP-1…WP-7 and their halt actions; read U5's criteria and confirm it is **executed by that exact name before the only committing recovery branch**. Then run the positive and fail-closed cases below. | the name matches exactly in both units; every case below behaves as stated |
| V12a | *Positive*: clean tree on `main`, valid locator, PR head fetchable. Run the placement | WP-1…WP-7 all pass; HEAD == FETCH_HEAD; current branch == locator `branch`; not `main`, not detached |
| V12b | *Fail-closed, dirty tree*: uncommitted change present | halts at WP-1; **no** stash, **no** discard, **no** switch, **no** commit |
| V12c | *Fail-closed, fetch failure*: unreachable `refs/pull/<pr>/head` | halts at WP-3; no switch, no resolution, no commit |
| V12d | *Fail-closed, divergence*: local branch of that name exists and is not a fast-forward of `FETCH_HEAD` | halts at WP-4; **no** `git reset --hard`, **no** force checkout, **no** rebase |
| V12e | *Fail-closed, mismatch*: switch succeeds but `HEAD != FETCH_HEAD`, or branch name differs, or HEAD is detached | halts at WP-5/WP-6/WP-7 respectively; no resolution, no commit |
| V12f | *Read-only path*: a Step 1b ancestry assertion | fetch performed; **no** switch performed; the placement procedure is **not** run |
| V13 | **Zero-checkpoint case**: a unit whose enumeration proves zero. Trace Step 5 | S4 omitted in full — no locator published, no resolution commit, no phase-2 update, **no empty locator**; S1, S2, S3, S5, S6, S7, S8 all execute; item 7b/7c run **after** the push |
| V14 | **Nonzero-checkpoint case**: a unit owning ≥1 checkpoint. Trace Step 5 | S4 executes; the locator, the resolutions and the `OPEN` obligation record ride **one** commit that is pushed before the S5 review re-run; remote head proven equal to local head |
| V15 | **Enumeration-failure case**: enumeration errors, or returns a malformed or quarantined record, or is ambiguous | **halts**; it is **not** classified as zero; no locator, no resolution, no merge |
| V16 | **Race case**: a checkpoint appears after S3 completed but before merge | re-evaluation from S3 is forced before merge; the prior approval is void and a fresh S6 approval with a new `approved_head` is required |
| V17 | **Obligation-record durability**: publish a record, then delete the PR body. Run both discovery channels | Channel A finds nothing; **Channel B still finds the `OPEN` record**; OB-6 (A/B disagreement) halts; the missing locator does **not** discharge the record |
| V18 | **Obligation-record deletion**: delete the closure artifact from the tree with no `OPEN` → `CLOSED` transition | `git log --diff-filter=D` detects it; **OB-1 halts**; absence is **not** read as "no obligation" |
| V19 | Grep the changed files for `resolution_commits` and `final_head` within the `RESOLUTION_OBLIGATION_RECORD` schema and its lifecycle text | zero occurrences — the record carries no SHA, so RQ-8 is preserved |

## Residual risks

| Ref | Risk | Disposition |
|---|---|---|
| RR-1 | These are prose protocols executed by an LLM. Correct wording does not prove correct execution. | **Accepted and recorded.** The prior plan's answer — a static wording checker — could only prove the words had not changed, not that the protocol ran. U5's ancestry assertion is the real defence: a concrete command with a pass/fail outcome that makes a miss loud. V8 additionally proves the one discovery command that was silently wrong in revision 6. |
| RR-2 | *(Closed in revision 7.)* Stage's session-end resolution was previously bound by P-022 with no procedural rewiring. | **Closed by U8.** The independent review held that a universal requirement with a procedural gap in one of its two named agents is not realized. U8 adds the narrow Stage qualifier without granting Stage merge authority. |
| RR-3 | The locator lives in the PR body, which a human or bot can edit or delete. | **CLOSED in revision 9 by unit U9** (`RESOLUTION_OBLIGATION_RECORD`) **plus U10** (schema ownership), and by U4 AC11 / U5 AC6, AC7, AC8 / U6 AC1, AC3, AC5 wiring it into every discovery and discharge path. The gap was real and is restated honestly: because `RESOLUTION_PREFIX` resolves *every* checkpoint before merge, the PR body was the **sole** obligation record, deletion was undetectable, and startup would conclude "clean" — strictly worse than the pre-change still-active checkpoint, and a direct contradiction of RQ-7. The fix adds a **second, Git-tracked, history-immutable channel**: a SHA-free record written into the unit's existing `docs/closure/` pre-merge closure artifact **in the same commit as the resolutions**, pushed before the final review. It is discovered from exhaustive trusted PR/commit/tree history with provenance taken only from API fields and Git ancestry, never from the body; deletion is detected from commit history (OB-1) rather than inferred from absence; force-push/history gaps (OB-2), conflicting records (OB-3, OB-4), unparseable records (OB-5), channel disagreement (OB-6) and incomplete enumeration (OB-7) all fail closed; and discharge is only an `OPEN` → `CLOSED` transition in a later, merged, ancestry-auditable commit. **RQ-7 is not weakened, and RQ-8 is not traded** — the record carries no SHA (V19). Verified by V17, V18, V19. **No** executable persistence substrate, lock/CAS, or cross-run cursor is introduced, and no Defect-1 construct is touched. *Residual after the fix (accepted):* an actor able to force-push the branch **and** rewrite history **and** edit the PR body can still destroy both channels — but that is now a detectable, halting condition (OB-2) rather than a silent one. |
| RR-4 | `gh api --paginate` over all PRs grows with repository history. | **Accepted.** Cost is bounded by PR count and runs once per zero-candidate startup. Correctness was chosen over speed deliberately (RQ-9). |
| RR-5 | A crash between locator phase 1 and the resolution commits leaves a `RESOLUTION_PENDING` locator with no commits. | **Handled, not merely accepted** — `LAST_MILE_RECOVERY` Step 1a runs the *resolution-state classification* over the locator's checkpoint list at the fetched PR head and re-enters **`RESOLUTION_PREFIX`** (at the resolve step) **only** on a reduced state of `NONE`; `PARTIAL`, `ALL` and `INDETERMINATE` all halt, as does the merged case. `RESOLUTION_POSTCONDITION` is never re-entered here — it is a Step 6 metadata write. Listed here because the handling is a recovery path, not a prevention. |

## Out of scope

* All Defect-1 work — see
  `docs/decisions/2026-09-13-dark-mode-continuation-auto-routing-deliberation.md`
  (status `open`). Harvest of Defect 1 is **not** authorized.
* The drift-checker script pair, fixture corpus, parity runner, hook shim, and
  the `.gitignore` entry they required.
* The task↔plan acceptance parity gate.
* backlogit tool changes; `src/`; `crates/`.
* Shipments 140-S, 141-S, 142-S; feature 142-F.
* Upstream autoharness template propagation.

## Retained review history

This section is **evidence, not authority**. It records what previous revisions
were reviewed against, and it is **appended to, never rewritten**. Revision 8's
own Round 8 review genuinely returned **FAIL** with three open P1s and left the
circuit **OPEN** at attempt counter 3; that record stands unaltered below.
Revision 9 remediates those findings under explicit operator authorization, but
**no row below, and no earlier revision's verdict, may be cited as a harvest
gate for revision 9**. Revision 9's own gate is a fresh independent full-plan
review that has not yet returned.

| Round | Reviewers | Verdict | Scope reviewed |
|---|---|---|---|
| 1 | Scope Boundary Auditor (`gpt-5.6-sol`), Constitution Reviewer (`claude-opus-4.8`) | FAIL (6×P1, 4×P2) / ADVISORY | revision 1 (both defects) |
| 2 | Scope Boundary Auditor | FAIL (5 blocking, 1 new P2) | revision 2 (both defects) |
| 3 | Scope Boundary Auditor | FAIL (5 blocking, mechanical cross-reference contradictions) | revision 3 (both defects) |
| 3-confirm | Scope Boundary Auditor | PASS — **superseded and withdrawn** | Same-reviewer, scoped to its own five findings, obtained without the mandatory escalation. Never a valid harvest gate. |
| 4 — P-013.6 escalation | Independent escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh`; same-route guard NOT triggered | **ESCALATION_BLOCKS** | 8 blocking corrections. **Seven were incorporated into revision 4; blocker 8 — the requirement for a fresh independent full-plan review — remained outstanding at that point.** |
| 5 | Independent seven-persona panel: Constitution, Rust/feasibility, Scope Boundary Auditor, Learnings, Architecture, Agent-Native Parity, Security | **FAIL** (13 blocking P1 plus P2/advisory) | revision 4 (both defects). Blocker 8 discharged **as process** — the review was performed — but its verdict was FAIL, so the gate it guarded stayed closed. |
| 6 | Independent four-persona panel: Scope Boundary Auditor (`gpt-5.6-sol`), Constitution (`claude-opus-4.8`), Correctness (`gemini-3.8-flash`), Agent-Native Parity (`grok-4.6`) | **FAIL** (3 FAIL, 1 ADVISORY) | revision 6 (Defect 2 only). All findings were specification defects; none falsified the design. Remediated into revision 7 — see the disposition table below. |
| 7 | Independent four-persona panel, same personas and models as round 6 | **FAIL** (Scope, Correctness, Parity FAIL; Constitution ADVISORY) | revision 7. Panel confirmed **every** revision-6 finding genuinely closed and the design still sound; new findings were deeper specification defects exposed by the earlier fixes. Remediated into revision 8 — see below. |
| 8 | Independent Scope Boundary Auditor (`gpt-5.6-sol`, xhigh) | **FAIL** (3 P1, 5 P2) | revision 8. **Third consecutive FAIL. Review circuit OPEN — attempt counter 3.** P-013.6 escalation fired; Stage halted without harvesting. |
| 8-escalation | P-013.6 escalation reviewer, route `gpt-5.6-sol` / `openai` / `xhigh`, against HEAD `bbb52b65` | **ESCALATION_BLOCKS** | revision 8. Reasoning-only. Produced findings A–E, remediated into revision 9 under explicit operator authorization for ONE bounded revision plus ONE fresh full review. |
| 9 | *(pending)* | *(not yet returned)* | revision 9. The **only** review that can gate revision 9's harvest. |

### Round 8 — outstanding findings (SUPERSEDED by revision 9; recorded as they stood)

**Status of this subsection.** The three P1s below were genuinely open when
revision 8 was reviewed, and the text is preserved as it stood rather than
rewritten. Revision 9 remediates each; the remediation column names where.
Marking them remediated here is **not** a verdict — only the pending revision-9
review can confirm the remediations are adequate.

Revision 8 closed every round-7 P1 the auditor could verify: the P-003 sub-epic
tier is restored, U4 AC1 references without restating, the RQ-2 trace explicitly
excludes U8, U4 AC4 exposes the Step-5 reorder, hardening D6 defers to D10, U2's
locus is fixed, `pr_role` is gone, and U6 is owner-scoped. No Defect-1 construct
is reintroduced. All eight units remain single-file documentation units.
*(Historical: revision 9 raised the count to ten; every unit is still a
single-file documentation unit.)*

These three P1s were open at revision 8 and were carried to escalation. The
revision-9 remediation for each is recorded in the fourth column:

| # | Finding | Required fix | Revision-9 remediation |
|---|---|---|---|
| 1 | **RQ-6 has no executable enforcement path.** The canonical prefix asserts that a live re-fetch of HEAD, threads and CI already exists in Ship Step 5 and is unchanged. It does not: real item 15 re-runs the P-018 gate and re-queries `headRefOid` only — it never re-fetches required CI, and it does not refresh all review threads when P-018 is disabled. U4 AC9 then *requires* item 15 to stay unmodified, so RQ-6 is credited to a unit that is forbidden from implementing it. | Expand U4 to amend the last-mile item with an explicit fail-closed post-approval query of live HEAD, full thread state and required checks; drop the "item 15 unmodified" criterion. | **Applied.** `RESOLUTION_PREFIX` restructured into S1…S8 with the exact safe order; U4 carries a verbatim Step-5 extract; the "item 15 unmodified" criterion is **withdrawn** and U4 AC9 now **amends** item 15 to re-fetch headRefOid, PR body, reviewDecision, review requests/reviews, every review-thread page, required checks and resolution-commit ancestry; U4 AC10 adds the six-part merge bar and the no-stale-approval refresh rules; item 16 (P-009) stays unmodified. Verified by V4. |
| 2 | **The zero-checkpoint bypass claim is false.** U4 AC3 promises a zero-checkpoint unit runs "the pre-existing path unchanged", while AC2/AC4 require all mutating items to move before the prefix and the readiness gate to move after it. Real Ship Step 5 has readiness items 7b/7c *before* runtime verification, closure-artifact generation, follow-up writes and the push (items 7–10), so the reorder changes the common path for every unit, including zero-checkpoint ones. | State that zero-checkpoint units skip locator publication and resolution but use the newly ordered common readiness path. Do not claim their step order is unchanged. | **Applied.** The checkpoint count now selects segment **S4 only**; the "pre-existing path unchanged" claim is removed everywhere and expressly prohibited by U4 AC3. Enumeration failure/malformed/quarantine/ambiguity is **not zero** and halts; a late-appearing checkpoint forces re-evaluation; no empty locator; Stage startup recovery preserved as separate. Verified by V13–V16. |
| 3 | **D14 is not actually folded into a task criterion.** D14 requires reading `branch`/`pr`, fetching the PR head, checking out a local branch, confirming the checkout and halting on failure. Its cited U5 AC2 lists only `gh pr view`, `git fetch` and `git merge-base` — no checkout, no verification — and no V-check covers it. Because task-card criteria are declared exact, U5 could pass while recovery is still sitting on `main`. This re-opens the very hazard D14 was written to close. | Add a U5 acceptance criterion requiring the by-name working-tree placement, checkout and verification before any committing re-entry, plus a matching verification check; then repoint D14's fold reference. | **Applied using existing units — no new unit.** The placement paragraph (whose heading revision 8 had lost, leaving it uncitable) is named `Working-tree placement — committing re-entry only` and defined in **U3 AC12** (WP-1…WP-7); **U5 AC9** executes it by that exact name before the only committing recovery branch; the U3→U5 dependency is unchanged. Verified by **V12** and cases **V12a–V12f**. D14's fold reference is repointed. |

Open P2s at revision 8, both now closed: V2 was unsatisfiable as written (it
forbade verbs U1/U8 are required to use) — **replaced in revision 9** by a
sequence-restatement check that permits ordinary prose verbs; and V8's reference
command lacked `--paginate` so its comparison was invalid — **fixed in revision
9** by paginating the reference command.

**Closed on 2026-09-13** (PR #396 Copilot review remediation pass, commit
recorded in the PR): the decision document's in-scope list now carries
`_stage.agent.md`, states the unit count explicitly instead of "7-task plan", and
no longer names the retired `RESOLUTION_ORDER`; hardening D14's stale
`Folds into` reference is withdrawn along with its incorrect "applied" status.
*(The unit count that sentence recorded was **eight (U1–U8)** at revision 8; it
is **ten (U1–U10)** at revision 9 following the RR-3 closure.)*

### Additional findings from the PR #396 Copilot review (remediated in the revision-8 pass)

These were raised on the published PR rather than by the four-persona panel.
They are **specification hardenings and honesty corrections**. The dispositions
below are recorded **as they stood at revision 8** and are not rewritten; at that
point none of them closed any of the three P1s and none changed the circuit
state. Where revision 9 has since advanced a disposition, that is noted inline.

| Thread | Finding | Disposition |
|---|---|---|
| `PRRT_kwDORJEduc6h6juE` | The "exhaustive and trusted" read protocol validated no provenance, so a fork PR could forge a locator and halt startup or steer recovery. | **Fixed.** New *Provenance validation* block: PV-1…PV-7, with untrusted candidates **discarded silently** (so an outsider cannot deny startup) and trusted-but-inconsistent ones **halting**. All downstream rules operate over the TRUSTED set only. |
| `PRRT_kwDORJEduc6h6juv` | RR-3's "no worse than today" disposition contradicts RQ-7. | **Fixed by honest reclassification** *(at revision 8)*. RR-3 was reclassified **OPEN**, not accepted: resolving every checkpoint pre-merge makes the deletable PR-body marker the *sole* obligation record, which is strictly worse than the pre-change still-active checkpoint. The durable publication record was **not yet designed** at that point. **Superseded at revision 9:** RQ-12, units U9/U10 and hardening D15 design and install it; RR-3 is now **CLOSED**, subject to the pending revision-9 review. |
| `PRRT_kwDORJEduc6h6jv7` | Body status said `review_verdict: none` while frontmatter said FAIL. | **Fixed.** The status paragraph now states the round-8 FAIL, the three open P1s, attempt counter 3 and the open circuit. |
| `PRRT_kwDORJEduc6h6jv-` | Retained-history note still said "revision 7". | **Fixed.** Now names revision 8 and its FAIL verdict. |
| `PRRT_kwDORJEduc6h6ldd` | "Handled normally" undefined for two locators naming the same shipment. | **Fixed.** Byte-identical duplicates collapse to one record; **any** field difference halts. `updated_at` explicitly barred as a tie-breaker (it is body text). |
| `PRRT_kwDORJEduc6h6ldr` | "Resolution commits exist" had no executable definition on the `RESOLUTION_PENDING` path. | **Fixed.** New *Resolution-state classification*: per-checkpoint state at the fetched PR head, reduced to `NONE` / `PARTIAL` / `ALL` / `INDETERMINATE`, with strict precedence. Only `NONE` resumes; the other three halt. |

## Escalation record (P-013.6)

**Trigger**: plan-review attempt counter reached **3** with three consecutive FAIL
verdicts (revisions 6, 7, 8).

**Resolved escalation route**: `gpt-5.6-sol` / `openai` / `xhigh`, read fresh from
`.autoharness/config.yaml` `model_routing.stage.escalation` at session start. The
legacy flat `model_routing.escalation` key is empty, so there is no both-present
ambiguity.

**Same-route guard**: NOT triggered. Stage's own role route is
`claude-opus-5` / `anthropic` / `high`, which differs from the escalation route in
all three fields. `ESCALATION_DEGRADED` therefore does **not** apply and the
escalation is live rather than a no-op.

**Disposition**: the failing operation was **not** re-executed at revision 8.
Stage halted: no harvest, no shipment assembly, no successor ID allocation. The
escalation is a reasoning escalation only and confers no authority to promote
this plan.

**Escalation execution and operator re-authorization (revision 9).** The
escalation ran under route `gpt-5.6-sol` / `openai` / `xhigh` against HEAD
`bbb52b65df1e63f9e8ebbd28b4ccd0fc61718cdd` and returned `ESCALATION_BLOCKS` with
findings A–E. The operator then explicitly authorized **one bounded planning
revision plus one fresh full independent plan-review gate** — and nothing
further. Revision 9 is that revision. It remains within the reasoning-escalation
boundary: **no** harvest, **no** shipment claim or assembly, **no** activation,
**no** implementation, **no** merge, and **no** revival of the abandoned `143.*`
artifacts. `harvest_authorized` stays `false` and moves only if the fresh review
of revision 9 returns PASS **and** the Orchestrator separately routes the next
step.

**Answer to the escalation question.** The escalation asked whether U4 should be
decomposed against a **verbatim extract of the real Step 5 item list** rather
than against a prose description of it. **Yes — and revision 9 does exactly
that.** U4 now carries the extract inline, including the two facts every prior
prose round missed: the item number `7` is **duplicated** in the live file, and
readiness items 7b/7c sit **before** the mutating items 7(second)/8/9/10 and the
push. Those two facts are the direct cause of open findings 1 and 2, which is
strong evidence the escalation's diagnosis was correct.

**Assessment carried to escalation**: the architecture has not been falsified.
Three independent panels have each confirmed the design sound and each closed
finding has stayed closed. The failure mode is that this plan specifies *edits to
agent prompt files* against a target (`_ship.agent.md` Step 5) whose real item
ordering is more entangled than a documentation-domain unit can restate safely —
every round has surfaced a further mismatch between what the plan asserts Step 5
contains and what it actually contains. The escalation question is therefore
whether U4 should be decomposed against a **verbatim extract of the real Step 5
item list** rather than against a prose description of it.

### Round 7 — disposition of the independent revision-7 review

| Finding | Source | Revision-8 disposition |
|---|---|---|
| Circular precondition `work complete AND PR merge-ready` still in the **canonical block** (line 135) and U4, despite the disposition table claiming it fixed; U2 installs that block verbatim, so the circularity would ship | Correctness P1 | **Fixed.** Canonical block rewritten with an explicit `entry:` clause — at least one active checkpoint plus all branch-mutating work complete except the resolution-dependent gates. The phrase "PR merge-ready" is gone from the definition and from U4. |
| `RESOLUTION_ORDER` spans both sides of the merge, so U4's "invoke by name, do not restate" is unsatisfiable — an agent either re-merges or stops at an undefined boundary | Parity P2, Scope P1 | **Fixed.** Split into `RESOLUTION_PREFIX` (ends at readiness, before approval/merge) and `RESOLUTION_POSTCONDITION` (a Step 6 metadata write). The existing approval/re-fetch/merge items are explicitly not moved or duplicated. |
| U4 AC6–AC9 restated ordering the canonical owner owns, so the surfaces could drift while V2 still passed | Scope P1 | **Fixed.** Those ACs removed; U4 now carries only invocation, placement and non-restatement criteria. V2 gained part (b): grep for the sequence's step verbs in the referencing files and require **zero**. |
| U4 hid a materially larger Step-5 reorder than its S/<2h estimate admitted — AC2 and AC3 together require moving several existing mutating items | Scope P1 | **Fixed.** New U4 AC4 requires all branch-mutating items to sit before the invocation and the old-to-new order to be recorded in the task card. V4 gained part (b): assert the set of items between invocation and merge contains no mutation. |
| Orchestrator discovery wired only into the **global** zero-candidate arm — a legitimately active Stage checkpoint masks the obligation, Ship is never routed, and Step 2 then skips the shipment for being `active` | Parity P1 | **Fixed.** U6 rescoped to fire whenever there is no **ship-owned** active checkpoint, matching Ship's own scoping (U5 AC6), with explicit precedence over Stage routing and queue selection. |
| Recovery re-entry is not executable from a fresh checkout — it must commit, but startup is on `main`, where committing is P-010-forbidden | Parity P2 | **Fixed.** New **Working-tree placement** rule: read `branch`/`pr` from the locator, fetch `refs/pull/<pr>/head`, check out, halt on failure. Read-only ancestry assertions need a fetch but no checkout. |
| U8 patched only Session end, gave no executable predicate, and left the undischargeable best-effort checkpoint in place | Parity P2, Scope P1 | **Fixed.** U8 now covers **both** Stage resolve sites, supplies `gh pr list --state merged --head <branch>` as the test (halt on lookup failure), and qualifies the checkpoint-creation directive. New V10. |
| U8 credited with enforcing RQ-2 although it only prohibits; Stage cannot guarantee its resolution reaches `main` | Scope P1 | **Fixed.** Trace now credits U8 with RQ-1 only, and states why it is not credited with RQ-2. |
| P-003 item 4 requires every task to reference a parent **sub-epic**; the flat decomposition was justified by precedent, not policy text, and P-003's violation action is Halt | Scope P1, Constitution P2 | **Fixed.** Two sub-epics restored — `143-E1` (ordering contract: U1, U2, U4, U8) and `143-E2` (discovery and recovery: U3, U5, U6, U7) — mirroring the plan's own prevention/recovery split. V5 updated. |
| `pr_role: implementation \| closure` is a ghost specification — no unit ever publishes a closure-PR locator, and recovery never branches on it | Correctness P2, Scope P2 | **Fixed.** Field removed everywhere. The locator is stated to be implementation-PR-only, with the reason: the PR body survives merge and branch deletion. |
| No bypass specified for a unit owning zero checkpoints — the prefix would publish an empty locator | Correctness P2 | **Fixed.** Entry condition requires ≥1 active checkpoint; zero-checkpoint units skip the sequence and run the pre-existing path unchanged. |
| U1 lacked the Amendment Log row and version bump every prior policy addition carries; `Gate Point` value never specified | Constitution P2, P3 | **Fixed.** New U1 AC3 (concrete gate point) and AC8 (amendment row `1.25.0` plus header version reconciliation). |
| U2's locus was self-contradictory — "sibling of `### 1.9`" but "placed after `#### 1.9.2`", which would orphan 1.9.3 onward | Scope P2, Correctness P3 | **Fixed.** Locus is now a level-3 section after the **end** of `### 1.9`, with the orphaning hazard stated as the reason. |
| Constitution Check omitted P-017 (recovery auto-enters and walks to a merge bar; dark mode could supply approval for a prior unit's obligation), P-012 and P-008; P-010 row not updated for U8 | Constitution P2, P3 ×3 | **Fixed.** All three rows added, P-010 extended, and U6 AC7 states that a dark approval for the current scope does not satisfy the merge bar for a prior unit's recovered obligation. |
| U1 and U2 listed as co-roots although U1 references a definition only U2 creates | Scope P2 | **Fixed.** Edge **U2→U1** added; U2 is the single root; edge count 10 → 11. |
| Step 1a's "resolution never happened" prose ignored the crash-after-push-before-phase-2 window | Correctness P3 | **Fixed.** The row now branches: no commits → re-enter; commits present → halt, because the locator cannot be trusted to enumerate them. |
| V2/V3/V4/V7/V8 not uniformly falsifiable | Scope P2 | **Fixed.** V2 split into two parts, V3 given concrete tokens, V4 given an explicit mutation check, V7 closed to eleven literal names, V8 rewritten to force a page boundary with `per_page=2`. V10 and V11 added. |
| Hardening D6 ("classify live PR state first") contradicted D10 (locator-status gate first) | Scope P1 | **Fixed** in the hardening document: D6 now applies only after Step 1a admits a complete `RESOLUTION_PUBLISHED` locator. Stale `Folds into` AC references refreshed throughout. |

### Round 6 — disposition of the independent revision-6 review

| Finding | Source | Revision-7 disposition |
|---|---|---|
| `RESOLUTION_ORDER` ownership contradiction — U1 said the instructions file owned it, but U2/U3 never installed it there while U4 required it verbatim in Ship | Scope P1 | **Fixed.** New `Canonical ownership` table names one installed home per definition. U2 AC3 installs `RESOLUTION_ORDER` verbatim; U1, U4 and U8 reference it by name only. V2 rewritten to assert exactly one verbatim copy. |
| `gh pr list --state all` is not an exhaustive-discovery command — no `--paginate`, bounded `--limit 30`, and returns no body | Scope P1, Correctness P3, Parity P2 | **Fixed.** Read protocol now gives the exact `gh api --paginate … /pulls?state=all&per_page=100 --jq …` command, with an explicit note on *why* `gh pr list` is forbidden. New V8 executes it. |
| U4 targeted the wrong loci — resolution is only at Session end item 2; Step 6.0 has no resolve step; the executable merge path is Step 5 | Scope P1, Correctness P2, Parity P1 | **Fixed.** U4 retargeted at **Step 5** and Session end item 2, with an AC requiring the invocation to precede the readiness gate, approval, last-mile re-check and merge. Circular "PR merge-ready" precondition replaced. |
| Session end still directs creating a best-effort checkpoint on yield — hardening D1 recursion, one level down | Correctness P1, Parity P2 | **Fixed.** New `Residual-window checkpoint prohibition` in `RESOLUTION_ORDER`; U4 AC5 retires the directive explicitly; V4 inspects for it. |
| `RESOLUTION_PENDING` locator unhandled — an open PR would be marked `RESOLUTION_PUBLISHED` with no resolution commits; a merged one would vacuously pass ancestry and be marked `RECONCILED`, orphaning the checkpoints | Correctness P1 | **Fixed.** New `Step 1a` locator-status gate runs before any live-PR classification, with explicit open (re-enter `RESOLUTION_ORDER`) and merged (unrecoverable orphan, halt) rows. |
| `Open, HEAD ≠ final_head` row omitted the ancestry assertion — a force-push could drop the resolution commits and the row would merge anyway | Correctness P1 | **Fixed.** Row now asserts ancestry against the fetched PR head **first** and halts on failure. |
| RQ-1/RQ-2 universal but only Ship procedurally rewired; RR-2 does not realize a requirement | Scope P1, Parity P3 | **Fixed.** New unit **U8** adds the narrow Stage qualifier. RR-2 closed rather than carried. |
| Decision DoD required "exactly one implementation unit" per RQ while the trace was many-to-many | Scope P1 | **Fixed.** Trace table now names one **owning** unit plus **enforcing** units per RQ; the decision's DoD wording is corrected to match. |
| U6 routed to an undefined "owning agent"; Ship's own zero-candidate path had no discovery | Scope P2, Parity P2 | **Fixed.** U6 AC6 routes explicitly to **Ship**; U5→U6 edge added; U5 AC6 adds discovery to Ship's `ZERO-CANDIDATE NORMAL STARTUP`. |
| U5's "open PR not an error" could be read as weakening the NON-NEGOTIABLE Merge Confirmation Gate | Parity P2 | **Fixed.** New `Boundary with the Merge Confirmation Gate` paragraph; U3 AC10 and U5 AC3 both state the boundary. |
| No remediation loop specified for a gate failure after resolution | Correctness P2 | **Fixed.** New `Gate-failure remediation loop` in `RESOLUTION_ORDER`; U4 AC9. |
| Constitution Check omitted P-001, P-020, P-015; P-014 row overclaimed "strengthened"; P-003 sub-epic tier unaddressed | Constitution P2/P3 ×4 | **Fixed.** All added; the P-014 row now names the hazard this plan *introduces* and the specific mitigation; P-003 states the flat feature-direct shape explicitly. |
| `RECONCILED` could be set before the P-020 compaction record completes | Constitution P2 | **Fixed.** Phase 3, U2 AC8 and U5 AC7 all require the full P-001 closure set including the P-020 record. U4 AC10 and V9 protect the P-020 invocation itself. |
| U7 was listed as realizing a requirement but implements none | Scope P2 | **Fixed.** U7 reclassified as a closure deliverable and deliberately excluded from the trace table. |
| Verification was weak — V2 token-presence only, V4 asked grep to infer ordering, V7 list incomplete, "byte-for-byte" unfalsifiable | Scope P2 | **Fixed.** V2 is a canonical-copy check; V4 is an ordered-step inspection; V7 expanded to eleven names; U6 AC7 replaced with a concrete fall-through criterion; V8 and V9 added. |
| Multiple non-`RECONCILED` locators across different shipments undefined | Correctness P3 | **Fixed.** Read protocol states this is a P-001 violation that halts; U3 AC5, U6 AC4. |
| Constraint "nothing in this plan references Defect 1" factually false | Scope P3 | **Fixed.** Narrowed to "no implementation unit introduces or depends on a Defect-1 construct"; V7 scans changed files, not this plan. |

**Why revision 6 is a reduction rather than a revision-5 remediation.** The
revision-5 findings R3, R4, R5, R11 and R12 were each attempts to specify
executable persistence in prose. They were remediated by *adding more prose*. The
halted revision-6 attempt recognized that this could not converge and classified
Defect-1 safety as requiring a new executable component or upstream support — a
conclusion the operator has approved. Revision 6 therefore removes Defect 1
rather than attempting an eighth specification of it. The revision-5 findings
that apply to Defect 2 — R7 (self-referential locator), R8 (live-PR-state-first),
R9 (status-independent discovery), R10 (re-run the review), R13 (merge-authority
bar) — are all carried forward and are realized by U2–U6.

<!-- plan-review-attempt: 3 -->
