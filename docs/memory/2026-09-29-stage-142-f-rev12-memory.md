---
title: Stage 142-F Revision 12 memory
date: 2026-09-29
agent: stage
feature: 142-F
status: rev12-review-failed-awaiting-operator
review_attempt: 11
verdict: FAIL
---

# Stage 142-F: Revision 12 (attempt-10 P1 fixes)

## Decision

Operator, 2026-09-29 20:04 -07:00, verbatim: "Revision 12, then review".

* **Scope:** a narrow Revision 12 closing attempt-10 P1-1 and P1-2, plus
  P2-2 and P2-8, then one fresh-session scoped review (not run by
  Stage).
* **Still in force:** 18:14 "approve resets, route (a), delegate layout,
  Revision 11" and 18:47 "route (c), review Revision 11".

## Files touched

* `docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md`:
  * frontmatter `revision: 12`;
  * new `### Revision 12` (R12.1-R12.9, lines ~3452-3755);
  * "Revision 12 note" lines in R11.2 (reset comment, CG-M), R11.4 (LD4),
    R11.7 (claim gates), R11.9 (intro) and R11.10 (LD3 includes).
* `docs/operator-glossary.md`:
  * `last_updated` bumped;
  * the Revision 11, route-forms, L1-L4, LD1-LD4 and CG rows updated;
  * new rows: Route (c), the Revision 12 decision, Revision 12, and the
    R12 halt tokens.
* No backlog item, edge, shipment, stash entry, source, test, Cargo or
  config change. No git mutation.

## Dispositions

* **P1-1 (R12.2):** closed by rules O1-O4 plus assembly edits A1-A5.
  * `142.062-T` lines 23, 28 and 38; `142.068-T` lines 23 and 52.
  * The LD4 baseline is the Step 2 record, and the union is withdrawn.
  * Token: `R12-OWNERSHIP-HALT`.
* **P1-2 (R12.3):** mechanism (b), crate-local `engram-indexer` tests.
  * Why: the root has no `engram-indexer` dependency, while
    `engram-indexer` depends on `engram`. `142.071-T` sets the precedent.
    Option (a) would create a dev-dependency cycle.
  * Edits A6-A14: `142.069-T` lines 28, 33, 47 and 52; `142.070-T` lines
    29, 34 and 54; `142.072-T` lines 30, 35 and 56.
  * Token: `R12-TEST-PATH-HALT`.
  * F50's existing own-file include (`preflight_gate_test.rs` lines 5-6)
    is out of scope, with a halt if it's not lint-clean.
* **P2-1 (R12.4):** frozen signatures; token `R12-SIGNATURE-HALT`.
* **P2-2 (R12.5):**
  * A15 pointer line (cites R11.4, R11.10 and R12);
  * A16 STATUS-RESET comment;
  * A17 (`142.064.001-ST` line 13), and A18/A19 (`142.054.002-ST` line
    27, `142.054.003-ST` line 28).
* **P2-8 (R12.6):**
  * CG-M reworded: Stage commits on `chore/stage-142-f-<purpose>`; the
    operator or Orchestrator pushes, opens and merges the staging PR.
  * New CG-S: re-read statuses after any index sync.
  * Token: `R12-RESET-REVERTED`.
* **P2-3:** partly fixed (the `R12-ROUTE-C-HALT` token and options). The
  feasibility pass is deferred as Q1.
* **P2-4, P2-5, P2-6, P2-7, P2-9:** fixed in R12.8.
  * P2-4: L3 recipe.
  * P2-5: LD2 `pub mod`, plus `R12-LAYOUT-HALT`.
  * P2-6: the architect counts files.
  * P2-7: post-H2 exemption.
  * P2-9: manual subtask safe-close.
* **P3s:** R12.8 table.
  * Deferred to the S3 hold lift: the F51/F52 Guardrail-4 edits, and
    `142.060-T`'s stale text.
  * Deferred to the S5 claim: `142.059-T`'s docs paths.
  * Operator question: P-001 acknowledgment (Q2).

## Open operator questions

* **Q1:** accept a possible `R12-ROUTE-C-HALT` for PRE-4 (`142.066-T`)
  at the S1 claim, or authorize a read-only route (c) feasibility pass
  before assembly step 6?
* **Q2:** acknowledge the P-001 block of other release units while
  `142-F` stays `active` during the PA-6 hold.

## Next step

Superseded by Attempt 11 below: the review ran and failed. Next step is an
operator decision.

## Attempt 11

* **Verdict: FAIL** (2026-09-29). Record: the plan section
  `### Attempt 11: FAIL` at the end of the file, ending
  `<!-- plan-review-attempt: 11 -->`.
* **Personas.** All five returned `FAIL`: Rust, Scope, Architecture,
  Constitution, Learnings (medium confidence). After deduplication,
  there are 4 P1s, 7 P2s and 13 P3s. The Scope P1 on `142.063-T`'s
  ownership exception was downgraded to P2, because it fires only at the
  held S4 claim.
* **P1s (Stage verified each against the files):**
  1. **A11 can't prove stage-set equality, so `142.072-T` will certainly
     halt at the S2 claim.** The relay set isn't public, and for `Build`
     an accept looks the same as a reject (`142.072-T.md` 26, 30).
     - *Fix:* an item-text constant `pub const RELAY_STAGES`, plus a
       two-way equality test.
  2. **The crate-local tests are outside `cargo dev-test`, Ship Step 4.3
     and CI.** The root package has no `default-members`
     (`Cargo.toml` 1-3; `.cargo/config.toml` 20; `ci.yml` 81 and 91). The
     constitution's rule at line 22 is broken, and no deviation is
     recorded.
     - *Fix:* A20, which adds `-p engram-indexer` test, clippy and check
       commands to the later S2 gates and to the `142.073-T` final
       criteria, and records the deviation.
  3. **The P2-9 subtask close requires the parent's `PASS`, but
     `142.064-T` and `142.054-T` are expected to get
     `EXPECTED_PENDING_RED`** (walkthrough 3091, 3102).
     - *Fix:* accept a parent that is `done` with `PASS` or verified
       `EXPECTED_PENDING_RED`. Close the subtasks after the PR merges and
       before Step 6 safe-close, with `backlogit move` on a staging PR,
       plus an evidence note.
  4. **Step 10 and CG-S call for a plain sync over a stale cache**
     (`docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md`
     35-68).
     - *Fix:* stop the backlogit process by PID, delete the
       `backlogit.db*` files, then sync.
* **Completion edits:** none. No stale cross-reference was found.
* **Circuit breaker:** attempts 5-11 are 7 consecutive FAILs, so this
  returns to the operator. There will be no Revision 13 and no assembly.
* **Nothing changed** in the backlog, edges, shipments, stash, source,
  tests, config or git.
* **Recommended operator decision:** authorize a narrow Revision 13
  limited to the four P1 text fixes (with the P2s at Stage's discretion),
  then one scoped review. Or accept P1-2 as a recorded limitation and
  decide the others.
* **Q1 and Q2 (R12.9) are still open** and unchanged.
