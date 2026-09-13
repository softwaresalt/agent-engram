---
doc_type: memory
date: 2026-09-13
agent: stage
session: 143-defect-2-revision-9-and-round-9-review
pr: 396
branch: chore/143-s-stage-checkpoint-lifecycle-continuity
start_head: bbb52b65df1e63f9e8ebbd28b4ccd0fc61718cdd
fixing_commit: 9b15fd4347c49c8a5f6d2277a8eba5dcd4f07242
review_commit: b945ed6f87a956ec338221ce81f3a4983d557f8f
end_head: b945ed6f87a956ec338221ce81f3a4983d557f8f
review_verdict: FAIL
review_attempts: 4
harvest_authorized: false
outcome: BLOCKED
---

# Stage session — Defect-2 revision 9 and round-9 plan review

## Authorization boundary

Invoked by Orchestrator to produce an outcome for blocked staging PR #396. The
operator authorized exactly **one bounded planning revision** plus **one fresh
full independent plan-review gate**, and nothing further. Both are now spent.

Explicitly forbidden and **not done**: harvest, shipment claim, shipment
assembly, activation, implementation, merge, and revival of the abandoned
`143-F` / `143-S` / `143.001-T`…`143.014-T` artifacts. Defect 1 (dark-mode
continuation auto-routing) stayed in open deliberation and was not implemented.

## What was done

1. **Established ground truth from live files rather than prose.** Read
   `.github/agents/_ship.agent.md` Step 5 (lines 552–680) verbatim. Confirmed all
   escalation claims, and found one the escalation had not named: the item number
   `7` is **literally duplicated** in the live file (first `7` = `fix-ci`, second
   `7` = `runtime-verification`).
2. **Applied one coherent revision** — plan and hardening to revision 9, decision
   to revision 3 — remediating escalation findings A–E. Committed as `9b15fd43`
   and pushed.
3. **Ran one fresh full independent plan review** — seven personas, six models,
   four vendors. Verdict **FAIL**: 1 P0, 15 P1, 11 P2, 6 P3 after dedupe.
4. **Recorded the review honestly**: appended `## Plan Review — round 9` to the
   plan, re-opened RR-3, reverted hardening D15 to undischarged, re-opened the
   decision's `PRRT_kwDORJEduc6h6juv` thread, set `review_verdict: FAIL` /
   `review_verdict_revision: 9` / `review_attempts: 4` /
   `status: halted-review-circuit-open` across all three artifacts. Committed as
   `b945ed6f`, pushed, then refreshed the PR body Reviewed HEAD **after** the
   push.

## Key findings and rationale (for the next session)

* **The root cause of revisions 6–8 is fixed.** Those rounds planned against a
  *prose description* of Ship Step 5 and mismatched it every time. Revision 9
  embeds a **verbatim extract** in U4, and two independent personas re-derived it
  from the live file and confirmed it accurate. This is the one structural
  advance revision 9 genuinely delivers and it should be preserved.
* **D14's mechanical root cause, not diagnosed by the escalation**: in revision 8
  the working-tree-placement paragraph had **lost its bolded heading** and began
  mid-sentence with "is entered at session start". An unnamed fragment cannot be
  cited by an acceptance criterion — that is *why* no criterion carried it. Fixed
  by naming it `Working-tree placement — committing re-entry only`.
* **Finding B's remediation introduced two new defects.** Making `S5`–`S8`
  universal was correct, but the locator-dependent clauses were not made
  conditional at the same time, so S8's five-way HEAD equality (F-04) and the
  postcondition's `OPEN → CLOSED` transition (F-05) are now unsatisfiable for
  zero-checkpoint units. Any future revision must fix these together.
* **F-02 is the most instructive finding.** The Constitution Check P-018 row
  still asserted the withdrawn "item 15 retained unmodified" claim — a residual
  instance of the very escalation finding the revision was authorized to
  remediate, surviving in a section the sweep missed. **Lesson: remediate by
  grepping the whole document for every restatement of a corrected fact, not
  section by section.**
* **RR-3 could not be closed.** `docs/closure/` was the right venue (endorsed by
  the architecture review) because `.github/skills/operational-closure/SKILL.md`
  already owns that schema and already uses the placeholder→finalize convention
  (`compaction_status: pending` → `done`). But the mechanism built on it fails on
  three independent grounds: Channel B is not independent of the mutable PR body
  (F-13), its commands are not executable as written (F-14), and the "detectable,
  halting" residual claim is unsubstantiated because RQ-8's no-SHA rule leaves no
  reference point to distinguish "never existed" from "erased" (F-19).

## Blocker returned to the operator

**RR-3 requires an operator decision** between:

* **(a)** branch protection forbidding force-push once `S4` has run (for example
  through a required status check set at resolution time), or
* **(b)** an out-of-band corroboration signal outside the branch author's erasure
  surface — a CI check run or deployment status posted at `S4` is not deletable
  by an account that can force-push.

Both reach past the reduced Defect-2 boundary, so Stage does not select one.
**RQ-7 was not weakened.** The RR-3 decision must be made *before* another
revision is attempted, because findings F-13, F-14 and F-15 are all consequences
of whichever mechanism RR-3 selects.

## Circuit state

Fourth consecutive FAIL. The plan-review circuit is **open again** at attempt
counter 4. Stage does **not** self-authorize a fifth attempt; a remediation pass
needs fresh operator authorization in addition to the RR-3 decision.

## Deferred, not handled

**14 new Copilot review threads** arrived at HEAD `9b15fd43` on 2026-09-13 at
20:55–56Z, across the reduced plan (8 threads), the superseded continuity plan
(1), two Stage memory files (3), and the abandoned `143-F` / `143-S` cards (2).
Per the governing directive these were **not** handled in this cycle and are
deferred to a later bounded pass. They are additional to, and independent of, the
round-9 findings.

## Validation performed

* `markdownlint-cli2` — 0 issues across all three artifacts, run both before the
  revision commit and after the review append.
* Frontmatter YAML parses on all three artifacts.
* Cross-reference check PASS: 10 unit headings == `task_count: 10`; RQ-1…RQ-12
  present in both the plan traceability table and the decision; D1…D15 all
  present in the hardening coverage table.
* `backlogit sync` OK — 1381 artifacts indexed, 0 parse failures.
* All fourteen `143.*` artifacts confirmed still `status: abandoned` and
  unmodified.
* `git diff --stat` confirmed exactly three changed files per commit; no product
  code, no abandoned task cards touched.

## Artifact state at session end

| Artifact | Revision | Verdict |
|---|---|---|
| `docs/exec-plans/2026-09-13-checkpoint-resolution-durability-plan.md` | 9 | FAIL (round 9) |
| `docs/exec-plans/2026-09-13-checkpoint-resolution-durability-hardening.md` | 9 | D15 undischarged |
| `docs/decisions/2026-09-13-checkpoint-resolution-durability-decision.md` | 3 | RQ-12 unsatisfied |

PR #396 body: `BLOCKED`, Reviewed HEAD `b945ed6f`, `P0=1, P1=15`.
Local HEAD == remote headRefOid == PR-body Reviewed HEAD == `b945ed6f`.
