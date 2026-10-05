---
title: Stage 142-F Revision 13 memory
date: 2026-09-29
agent: stage
feature: 142-F
status: rev13-reviewed-fail-awaiting-operator
review_attempt: 12
verdict: FAIL
---

# Stage 142-F: Revision 13 (attempt-11 P1 fixes)

## Decision

Operator, 2026-09-29 21:00 -07:00, verbatim: "Revision 13 (P1 fixes only),
approve the cache rebuild, then review. Q1: I think Stage should check
first with read-only pass. Q2: Yes. Also, I don't understand why a
subtask cannot be marked as done as a constituent of a parent task. Why
would that not be allowed?"

## Files touched

* The plan
  (`docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md`):
  * frontmatter `revision: 13`;
  * `### Revision 13` (R13.1-R13.9) inserted before
    `## Requirements Trace`;
  * "Revision 13 note" lines in R11.7 (claim-gate note), R12.6 (CG-S),
    R12.7 (step 10), and the R12.8 P2-9 row.
* `docs/operator-glossary.md`: the Revision 12 status updated; new rows
  for the Revision 13 decisions, Revision 13, the cache rebuild and the
  feasibility pass; `last_updated` bumped.
* No backlog, edge, shipment, stash, source, test, Cargo or config
  change. No build, no cache rebuild, no git mutation.

## Dispositions

* **P1-1 (R13.2):**
  * `pub const RELAY_STAGES: [&str; 7]` becomes item text.
    `normalize_verdict` uses it.
  * The test checks two-way equality against `stage_name`, guarded by a
    wildcard-free `match`. The constant is frozen under R12.4.
  * Edits: A11 revised (072 line 30), A21 (line 23), A22 (a new line
    after 26).
* **P1-2 (R13.3):** A20 adds the `engram-indexer` test line (and clippy
  where missing) to the acceptance criteria.
  * Insert points: `142.054-T` before 47, `142.069-T` before 39,
    `142.070-T` before 41, `142.071-T` before 38, `142.072-T` before 43,
    and `142.073-T` before 39 (final: all GREEN).
  * The Step 2 record also runs `cargo check -p engram-indexer
    --all-targets`.
  * A15 now cites R13.
  * The Constitution Check deviation is recorded. Residual: CI doesn't
    run these tests, a follow-up stash after assembly.
* **P1-3 (R13.4):** Ship's Step 4.5 grant isn't clear (line 38 says
  "move tasks"; no "subtask" anywhere in `_ship.agent.md`), so the
  fallback applies.
  * Stage moves each subtask to `done` once its parent is `done` (`PASS`
    or a verified `EXPECTED_PENDING_RED`), after the shipment PR merges
    and before Ship Step 6.
  * The moves land through `chore/stage-142-f-subtask-close-<shipment>`.
  * The PR is recorded as a release-closure item on the S1, S2 and S5
    shipments.
  * The operator's question is answered in R13.4.
* **P1-4 (R13.5):** the cache rebuild procedure (stop by PID, delete
  `backlogit.db*`, sync, re-read under CG-S) is recorded.
  * ActionRisk: destructive. ActionResult: `approved`, not `applied`.
  * Rollback: the Markdown files are the source of truth.
* **Q1 (R13.6):** PRE-2, PRE-3, PRE-5 sc. 1-2 and PRE-6 sc. 1, 3 are F1.
  PRE-4b, NEW-1, NEW-2 and NEW-4 are feasible. PRE-4 (`142.066-T`) is
  **F3**.
* **Q2 (R13.7):** acknowledged and closed.
* **Attempt-11 P2s and P3s:** deferred (R13.8).

## Open operator questions

* **Q3:** PRE-4's F3. The options are (i) a halt at the S1 claim,
  (ii) (recommended) an (a-1) variant where the const lands as `&[]` at
  Step 2 and each test first asserts its final value, or (iii) a separate
  shipment.
* **Q4:** should Ship move subtasks at Step 4.5? That needs a Ship
  role-boundary clarification, a harness-rule change.

## Next step (superseded by Attempt 12)

One fresh-session scoped plan-review of Revision 13. Assembly happens only
after `PASS`, or `ADVISORY` with operator confirmation.

## Attempt 12 (2026-09-29): FAIL

* **Personas:**
  * Rust Reviewer: ADVISORY (0/0/5/3).
  * Scope Boundary Auditor: ADVISORY (0/0/1/4).
  * Architecture Strategist: FAIL (0/1/4/2).
  * Constitution Reviewer: FAIL (0/1/3/3).
  * Learnings Researcher: ADVISORY (0/0/4/3).
  * Merged: 1 P1, 12 P2, 10 P3. The Constitution P1 (no crate-local
    tests after S2) was downgraded to P2-1, because R13.3 applied
    attempt 11's fix as proposed and recorded a deviation that meets
    the Governance section.
* **P1-1 (verified):** R13.4's "after merge, before Ship Step 6"
  ordering can't be carried out.
  * The Merge Confirmation Gate goes straight to Step 6.0
    (`_ship.agent.md` ~758, ~778-786).
  * The Release Closure Completion Gate (~761-767) is a check at the end
    and reads no recorded item.
  * P-016 allows only one worktree.
  * Result: S5's safe-close would archive `142-F` over active
    `142.058.002-ST` and `.003-ST`.
  * Fix: (a) Q4 "yes" as a separate harness chore merged before the S1
    claim, or (b) run the subtask-close PR after Ship's Step 6, enforced
    by a Stage claim-gate item on the next shipment, with an operator
    checkpoint after S5.
* **Q3:**
  * (i) is sound but halts for certain.
  * (ii) is sound with conditions: an explicit marker message in
    `assert_eq!`, freeze the type only, record the R11.10 departure, and
    fix P2-9 (the `142.067-T` line 42 test).
  * (iii) is unsound as written. It is workable only as S1b = [066, 067,
    068].
  * Consensus: (ii).
* **Q4:** "yes" is the cleaner fix for P1-1. It is a harness-rule change
  outside 142-F.
* **Completion edits:** none.
* **Circuit breaker:** attempts 5-12 are eight consecutive FAILs. This
  goes back to the operator.
* **What didn't happen:** no Revision 14, no assembly, no cache rebuild,
  and no backlog, source or git change.
* **Next:** an operator decision on Q4 (which chooses P1-1 fix (a) or
  (b)) and on Q3, then whether to authorize Revision 14 (P1-1 plus the
  chosen P2s) and one more review.
