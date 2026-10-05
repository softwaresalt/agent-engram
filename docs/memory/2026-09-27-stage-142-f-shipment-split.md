---
type: memory
date: "2026-09-27"
agent: stage
feature: 142-F
title: "142-F launcher-preflight: granted edges applied, shipment split proposed"
---

## Operator decisions applied (2026-09-27 21:18 and 21:21 -07:00)

The Orchestrator routed these operator decisions:

* "1. Yes, but 24 tasks is much too large for a single shipment; this
  should be decomposed into at least 3 shipments of 8 tasks each." (PA-1)
* "2. Yes" (the supplemental NEW-1/NEW-3 edge phrase)
* "approve 1 and 2, start planning 4, hold 3" (PA-4 is held. PA-5 goes to
  a later Stage session.)

**Applied** (verified with backlogit afterwards):

* **Seven `blocks` edges.**
  * `142.054-T` → `142.068-T`
  * `142.058-T` → `142.066-T` and `142.065-T`
  * `142.055-T` and `142.057-T` → `142.073-T`
  * `142.069-T` and `142.071-T` → `142.054-T`

  Every call returned `added`. A cycle check over the 32
  launcher-preflight edges found the graph acyclic.
* **PA-2.** The launcher contract text was appended to the
  implementation notes of `142.055-T` and `142.057-T`.
* **PA-2b.**
  * The "F50 completion after PRE-6" guidance was appended to the
    implementation notes of `142.054.002-ST` and `142.054.003-ST`.
  * Two acceptance criteria were added to `142.054-T`: the real-chain
    `ProductionVerifier` case, and the indexer pedantic clippy check.
* **Deliberation.** An "Operator Approval (2026-09-27)" section was
  added. It records PA-3/D1-A as ratified, confirms D2-A, D4-A and D5-A,
  and describes the modified PA-1.
* **Plan.** A note on the 2026-09-27 operator decisions was added to the
  Harvest Record.
* **Comments.** `142.063-T` got a comment saying the invariant-6
  exception is granted but 142-S placement is not. `142.069-T` and
  `142.071-T` got comments saying their edges are now recorded.

**Not changed:**

* Task statuses.
* The 142-S manifest and status.
* Stash.
* `.engram/config.toml`.
* Source, tests and config.
* Git. There was no commit, stage, checkout or history change.

The `pending-pa-1` labels on `142.061-T`..`142.074-T` were left in
place.

## Key finding: why 3 × 8 cannot all merge green as the plan stands

Every later unit depends on the F54 test file,
`tests/contract/read_server_cli_mcp_parity_test.rs`:

* PRE-3F (`142.063-T`) edits it.
* PRE-3, PRE-4b and PRE-4 (`142.064-T`, `142.064.003-ST`, `142.065-T`,
  `142.066-T`) use its per-case maps and three-run results as their
  proof.
* The whole chain runs PRE-3F → PRE-3 → PRE-4 → PRE-5 → PRE-6 → F50 →
  NEW → launchers.

That file exists only on the unpushed 142-S branch, in commits
`7bd9e504` and `6d216d19`. There it has 4 failing tests, and at least
one of them (the equivalence case) stays failing until the PA-5 work
(stash `86F93068`) lands. So any shipment that includes PRE-3F or the
F54 proof also includes known-failing tests and cannot merge green.
The reviewed plan assumed a single final merge ("the window closes at
the final 142-S readiness run").

The other failing tests on the branch are all test-only commits for
later tasks:

| Test harness | Failing tests | Commits |
|---|---|---|
| F51 | 3 | `1dfc1b5b`, `4995d681` |
| F52 | 1 | `5760b948` |
| F53 | 3 | `e24f5ae2` |
| archive | 2 | `a47b8aff` |

The F50 scaffold (`c269fa79` and `41dd5081`) passes. Commit `a0ccdc27`
holds administrative operator edits and checkpoint repairs.

## Proposed split (Option A, recommended): each test harness ships with its task

Shipment IDs are assigned by backlogit at creation. S1–S4 are provisional
labels.

| Part | Items | Tasks | Branch content (cherry-picked from the parked branch) |
|---|---|---|---|
| S1: read-server plumbing | `142.061-T` PRE-1, `142.062-T` PRE-2, `142.064-T` PRE-3 (+`.001/.002/.003-ST`), `142.065-T` PRE-4b, `142.066-T` PRE-4, `142.067-T` PRE-5, `142.068-T` PRE-6 | 7 (+3 ST) | `a0ccdc27` and the planning-state commit |
| S2: self-check and the `preflight` command | `142.054-T` F50 (+`.001/.002/.003-ST`), `142.069-T`..`142.074-T` NEW-1..NEW-6 | 7 (+3 ST) | `c269fa79`, `41dd5081` |
| S3: launch scripts and archive fix | `142.055-T` F51, `142.056-T` F52, `142.057-T` F53, `142.060-T` archive verifier | 4 | `1dfc1b5b`, `4995d681`, `5760b948`, `e24f5ae2`, `a47b8aff` |
| S4: parity test and docs | `142.063-T` PRE-3F, `142.058-T` F54 (+`.002/.003-ST`; `.001-ST` done), `142.059-T` F55 docs, future PA-5 tasks, and `142-F` added last so this shipment closes the feature | 3 + PA-5 | `7bd9e504`, `6d216d19` |

The ordering between shipments is S2 after S1, S3 after S2, and S4 after
S3.

**Why not 8/8/8.** There are 21 tasks. The dependency chain and the
"merge only when green" rule leave two choices:

* Hold the launchers until PA-5, which means merging S3 into S4.
* Merge with failing tests.

The 3-shipment variant (S3 and S4 combined: 7 tasks + PA-5) is valid,
but then the launchers wait for PA-5.

**Plan amendment Option A needs** (revision 6, one bounded confirmation
review):

1. **Move PRE-3F into S4.**
   * Remove the edge `142.064-T` → `142.063-T`.
   * Add `142.063-T` → `142.066-T` and `142.058-T` → `142.063-T`.
   * PRE-3F's "before" baseline becomes Ship's recorded `6d216d19`
     per-case map (in the `142.058-T` comments), plus the changes
     already enumerated for PRE-3 and PRE-4.
   * PRE-3F's positive test becomes a regression test that is GREEN
     when it lands, not a RED-first witness. This change of test-first
     posture must be reviewed.
2. **Move the F54 proof criteria into S4.** These are the per-case
   maps, the three identical runs, and the `F54 settle: Active` lines,
   taken out of `142.064-T`, `142.064.003-ST`, `142.065-T` and
   `142.066-T`. S1 keeps every other consumer-baseline criterion.
3. **Remove the edge `142.060-T` → `142.058-T`.** The archive fix does
   not need F54.
4. **Guard against L3-1.** In S4, the F54 test-file cherry-pick and
   PRE-3F land together, before any test gate runs.

## Proposed handling of 142-S and the branch

Facts checked:

* The installed backlogit reports `1.10.1-0.20260823032255-b07729386a31+dirty`,
  not 1.8.0. The source at `b0772938` was read.
* The only shipment transitions are queued→active, active→shipped and
  active→abandoned.
* `add_to_shipment` accepts queued or active shipments. It rejects any
  item still assigned to another queued, active or blocked shipment.
  Items in an abandoned or shipped shipment are free.
* `return_blocked` DOES exist. It removes an item from an active
  manifest (journaled), but sets the item's status to `blocked`.
* The ~107 uncommitted files are almost all backlog, memory, plan and
  decision state, plus a few harness files. They include the harvested
  tasks `142.061-T`..`142.074-T`, which have never been committed.

| Step | ActionRisk | Reversible |
|---|---|---|
| H1: commit the uncommitted planning and backlog files as one planning-only commit on the current branch (operator-approved; harness files included only if the operator agrees) | moderate | yes (revert) |
| H2: abandon 142-S with a safe-close record: nothing shipped, 142-F stays active, every item and commit mapped to its new part | high | no (terminal status; nothing is deleted) |
| H3: set 142-S's active items back to `queued`: `142.054-T` (+3 ST), `142.055-T`, `142.056-T`, `142.057-T`, `142.058-T` (+`.002/.003-ST`), `142.059-T` (`142.058.001-ST` stays done, `142.060-T` is already queued) | moderate | yes |
| H4: create S1–S4 as queued shipments with the ordering edges; `142-F` only in S4 | low | yes (a queued shipment can be claimed and then abandoned) |
| H5: Ship makes the S1 branch from `origin/main` (worktree clean after H1) and cherry-picks `a0ccdc27` and H1. The old branch is kept as a parked source and is never rewritten or deleted until S4 ships. | moderate | yes (branches are refs) |
| H6: Ship claims S1 | low | Ship-owned |

**Rejected alternatives:**

* **Re-scope 142-S in place with `return_blocked`.** It would flip 12
  items to `blocked`, it cannot move the done `142.058.001-ST` cleanly,
  and the 142-S name and branch would no longer match their content.
* **Edit the manifest directly.** This bypasses the tool's journal.
* **Keep 142-S active as a later part.** This breaks P-001, which allows
  one active release.

**Created in this session:** no shipments. Items in 142-S cannot be
added elsewhere until H2, and S1 depends on the Option A choice.

## Open operator questions

1. Is the 4-part split approved, or the 3-part variant?
2. May Stage write plan revision 6 (Option A) and run ONE bounded review
   of it?
3. May 142-S be abandoned (H2) and its active items returned to queued
   (H3)?
4. May Stage create the queued shipments (H4) after the review passes?
5. Is the git handling approved: H1 commit, the branch per part, the
   parked branch, cherry-picks only?

## Checkpoints

No structured checkpoint was created. The startup scan found 0 active
Stage checkpoints and 0 quarantined.
