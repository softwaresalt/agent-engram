---
title: "142-F decomposition plan (Revision 18): one task per shipment, sized slots, blocks-edge DAG, gated landing"
description: "Current, single statement of the 142-F decomposition: one-task shipments (Slot-01 to Slot-21, split slots and PA-5 slots) ordered by task and shipment blocks edges plus queue_position; a per-slot sizing gate; finished-slot definition by manual safe-close; harness, planning and 142-S disposition land on main through gated PRs; per-task final content requirements"
date: 2026-09-30
revised: 2026-10-04
feature: "142-F"
revision: 18
status: frozen-rev18
supersedes: "Revision 16 of this file (OD-1 to OD-7 as open decisions, the W step and W3/W4 classes, PA-6 holds, X1) and, in the old plan, S1-S5 membership and order, R10.1 cross-task EXPECTED_PENDING_RED mappings, R12.2/R12.4 ownership and freeze rules, the A/M/O delta edit catalogs as assembly instructions, R14.4 run point 3"
old_plan: "docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md (Revisions 6-14, Attempts 5-13)"
source: "docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md"
related_decisions:
  - "docs/decisions/2026-09-28-142-f-escalation-review.md"
  - "docs/decisions/2026-09-27-pa5-read-handler-conversion-deliberation.md"
  - "docs/decisions/2026-09-25-archive-verifier-read-before-close-deliberation.md"
review_findings: "docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md (attempt 15); docs/scratch/2026-09-30-142-f-plan-review-attempt-14-findings.md (attempt 14)"
stash_source: "03AA00A8, 49809128, 9B7EC1E4, 6C5DF765"
follow_up_stash: "86F93068 (PA-5), 5AF5CD66, 23E287C6, F99C705E, 7BF90213"
---

## 1. What this document is

The single current statement of how 142-F is decomposed, sized, ordered, landed and closed. Revision 17 rewrites Revision
16 in place to record and apply the operator's 2026-10-04 answers to OD-1 to OD-7 and the operator's sizing requirement.
Retired rules are listed once in 1.3 and appear nowhere else. Revision 18 (2026-10-04) is a
narrow fix of the attempt-15 findings (18); the plan is frozen at Revision 18 (17).

### 1.1 Decision record

**Operator, 2026-09-30 19:09 -07:00 (verbatim):** "As recommended, let's split the work into one task per shipment with DAG
ordering; optionally, run the three extra rebuilds now if that will actually help narrow the concerns. Use your best
judgement."

**Operator, 2026-10-04 12:32 -07:00** (full verbatim text in section 3), applied as follows:

| OD | Answer (verbatim) | Effect in this revision |
|---|---|---|
| OD-7 | "OD-7: Option A" | One multi-model adversarial review of Revision 17, then freeze; residual P2/P3 to the backlog (17) |
| OD-2 | "OD-2: I manually committed the W3 related changes since Stage is not allowed to.  W4 related changes can be discarded." | W3 = commit `f6f3171f`, landed by H0 (7.1); W4 deleted 2026-10-04 (PS-10 `applied`) |
| OD-4 | "OD-4: Yes, mark 142-S as abandoned and then archive." | PS-5 `approved`, conditional on a passing probe (7.5) |
| OD-1 | "OD-1: Yes, 142.060-T can start now." | Slot-01 created at assembly and claimed first (R-3) |
| OD-3 | "OD-3: Option B" | `142.056-T` is route (c) against its own placeholder marker (9.2) |
| OD-6 | "OD-6: Sure, lift the rest of the PA-6 hold; however, use your best judgement depending on circumstances." | PA-6 lifted for every held slot; the Orchestrator keeps a re-hold reserve (5.4); PA-4 stays held |
| OD-5 | "OD-5: Q1: Yes; Q2: Yes; Q3: Yes, although that would seem to contradict the decision on Q1; PA-5 tasks can go under 142-F." | PA5-C; real indexed F54 fixture; pinned `get_workspace_status` reads only `code_graph` from the generation; PA-5 tasks are Slot-19.k under 142-F (R-1, 7.6) |
| sizing | "Note that I want to ensure that the work is sufficiently decomposed such that no single shipment is excessively large or overly complex …" | Binding sizing gate SG-1 and session budget SB-1 (4.1-4.3) |

**Orchestrator judgements (2026-09-30):** rebuilds declined (11); finished slot by manual safe-close, `shipment ship`
forbidden (6); gated landing PRs (7); shipment edges and `queue_position` (5.2); `142.060-T` Slot-01 (C6); E10; note on
`142.059-T`.

**Orchestrator judgements (2026-10-04), applied here:** (A) H0 before H1, `f6f3171f` wins every conflict (7.1, 7.3); (B)
PS-5 probe mandatory, approval covers only the probed steps (7.5); (C) PA-6 lifted, DAG still orders, PA-4 held, re-hold
reserve (5.4); (D) R-1; (E) PA5-P (7.6); (F) SG-1 (4.1); (G) OD-1 to OD-7 resolved (14); (H) next step (17).

**Readings to confirm in the Orchestrator's report (not open decisions):**

* **R-1 (OD-5 Q1/Q3).** Consistent: Q1 covers reads that never touch the generation database (`get_daemon_status`, the
  report and metrics reads, `get_retrieval_eval_report`, `lint_dax`); they carry no provenance. Q3 covers pinned
  `get_workspace_status`, which is converted to read its `code_graph` counts from the generation and so carries provenance
  for those; `path`, `branch`, `db_path`, `stale_files` and `scan_status` describe the managed binding and stay as they are,
  so F54's binding fingerprint stays stable.
* **R-2 (PA-7 under the split).** "Only `142.058-T` edits the F54 file, after PA-5" is read as: `142.058-T` and the tasks
  split from it (Slot-20a-20d) are the F54 file's only editors, all after the PA-5 sinks. The deliberation's PA5-T7 (real
  fixture) and the F54 half of PA5-T6 (`DaemonScopedRead`) therefore land in Slot-20c and Slot-20d, not in PA-5 slots.
* **R-3 (OD-1 "start now").** `142.060-T` has no hold and no predecessor; its earliest claim is right after assembly,
  because P-001 blocks any claim while 142-S is `active` and CG-M needs its slot on `main`.

**Stage additions (not operator decisions):** ProposedActions, slot labels, H0/C1, SG-1/SB-1, section 9, HALT tokens, glossary.

### 1.2 Precedence

| Topic | Authority |
|---|---|
| Shipment shape, sizing, order, edges, landing, 142-S disposition, assembly, claim gates, closure, cache rebuild, per-task final content, HALT tokens | **This document** |
| Unit technical design (PRE-1 to PRE-6, PRE-3F, PRE-4b, NEW-1 to NEW-6, F50-F55, `142.060-T`) and route (c) mechanics (R11.10, R12.3, R12.5, R12.8, R13.2-R13.3, R13.6, R14.3, R14.5-R14.7) | **The old plan**, read through section 9 here; where section 9 states a different final content, section 9 wins |
| PA-5 unit design | `docs/decisions/2026-09-27-pa5-read-handler-conversion-deliberation.md` (PA5-C), read through R-1, R-2 and 7.6 |
| Review and decision history | The old plan's `## Plan Review`, section 3 and section 18 here |

### 1.3 Changes in force (C) and retirements (X)

| # | Change |
|---|---|
| C1-C2 | One task per shipment: Slot-01 to Slot-21, split slots (letter suffix), Slot-19.k (4); order = task edges + mirrored shipment edges + `queue_position`, slot number a tie-break only (5) |
| C3-C5 | Finished slot = archived `done`/`shipped`, merge SHA, closure record, manual safe-close (6); H0, H1, H3 reach `main` through gated PRs before assembly (7); PS-5 probed, destructive, approved conditional on the probe (7.5) |
| C6-C8 | `142.060-T` Slot-01 with no task edges to F50-F54, edge E10 (5.1); rollup guard (10.3); final per-task content replaces the delta catalogs (9) |
| C9 | Rev 17: PA-6 lifted; held slots are created at assembly, or after the PA-5 harvest for Slot-19.k to Slot-21 (5.4) |
| C10 | Rev 17: sizing gate SG-1, session budget SB-1, four split families (Slot-02, 04, 09, 20; eight new tasks) (4.1-4.4) |
| C11 | Rev 17: H0 lands `f6f3171f`; C1 carries the one colliding W2 path; W3/W4 retired as open classes (7.1, 7.2) |
| C12 | Rev 17: OD-3 option B for `142.056-T`; PA5-P planning step (7.6) |
| C13 | Rev 18: PS-5 = `shipment block`, then `archive` (no `move`), probe cleanup in scope (7.4, 7.5); CG-D archived predecessors (5.3); Slot-19.k positions 191-199 (4); frozen (17) |
| X1 | Revision 16's X1 ("subtasks meet the de-risking bound"): replaced by SG-1; promoted subtasks become their own slots |
| X2-X5 | R12.2 (O1-O4, A1-A5), R12.4 freeze, A15, `R12-OWNERSHIP-HALT`, `R12-SIGNATURE-HALT` (X2); CG-T and `R14-SUBTASK-OPEN` as a claim gate (X3); D-NOTE, `R15-EDIT-ANCHOR`, "From/To" edits, step 5b, `R15-SIZE-SPLIT` (X4); PS-7, PS-8 (X5) |
| X6 | Rev 17: the W step, W3/W4 as open classes, `PA-6-HOLD` as an assembly comment, OD-1 to OD-7 as open decisions |

## 2. Current state (read-only, 2026-10-04 12:40 -07:00)

* **Git.** Checked out: the parked branch
  `feat/142-s-preflight-state-machine-launchers-cross-surface-parity-matrix-and-operator-docs`, tip `f6f3171f` (the
  operator's W3 commit, parent `41dd5081`), 11 commits ahead of `main` (`32f6f9c7`), no upstream, no `parked/*` tag, one
  worktree. Map in 7.3. Orchestrator re-check (2026-10-04, read-only): `git branch --contains f6f3171f` lists only
  this parked branch.
* **Worktree.** 116 dirty paths: 33 `.backlogit` 142-* files, 16 other `.backlogit` files, 67 docs. No harness or junk
  path (W3 committed, W4 deleted). `142.060-T`-`142.074-T`, `142.064.00x-ST`, both 142-F plans and the 142-F decisions
  exist **only in the worktree**. One dirty path differs between `HEAD` and `main`:
  `.backlogit/checkpoints/checkpoint-20260924-010034.json` (W2; changed in `a0ccdc27`), handled by C1 (7.1).
* **H0 dry check.** `git merge-tree --merge-base 41dd5081 main f6f3171f` (no ref written) reports a content conflict in
  `.autoharness/config.yaml` only; `.github/agents/_ship.agent.md` auto-merges. Both files were also changed by `a0ccdc27`.
* **142-S and statuses.** 142-S on `main`: `queued`, 12 members (no `142.060-T`), dependencies 137-S, 140-S, 141-S
  (archived `done`); in the worktree: `active` since 2026-09-23 22:48, plus `142.060-T`. `142-F` `active`.
  `142.054-T`-`142.059-T` and subtasks: `queued` on `main`, `active` in the worktree; `142.058.001-ST` archived `done`.
  `142.060-T`-`142.074-T`, `142.064.00x-ST`: `queued` (worktree only).
* **Edges (index).** `142.060-T` → 054-058; `142.065-T` → 062; `142.063-T` → 062; `142.064-T` → 063; `142.059-T` → 025,
  026, 055, 057, 058; `142.074-T` → 073; archived dependencies are `done`. Sizing is prose (no `features.sizing`). The F54
  file is a stub on `main`; parked `7bd9e504`, `6d216d19` add about 1,630 lines.
* **Tooling (backlogit 1.11.0).** No shipment-abandon operation; `shipment block`, `return-blocked`,
  `normalize_blocked_shipment`, `unblock` (to `queued`/`active`), `ship` (cascading); `dep add` with two shipment IDs makes
  a shipment `blocks` edge; no write path for `custom_fields.queue_position`.
* **Main baseline.** `integration_release_archive_smoke_workflow` recorded failing on `main` (Windows 8192-char cut); not
  re-run (Stage runs no tests). Owner: `142.060-T`.
* **PA-5** (stash `86F93068`): OD-5 answered (PA5-C); no task yet (PA5-P, 7.6).

## 3. Decisions in force

| When (-07:00) | Decision | Effect now |
|---|---|---|
| 2026-09-26 | Option A harvest PRE-1-PRE-6, PRE-3F, PRE-4b, NEW-1-NEW-6 | `142.061-T`-`142.074-T` |
| 2026-09-27 | PA-1 granted except "add everything to 142-S"; split instead. PA-2, PA-2b, PA-3 granted | split 142-S; PS-5 approved 2026-10-04 |
| 2026-09-28 16:48 | PA-6 = Hold S3 (R8.6), later S4, S5 | **lifted 2026-10-04** (OD-6), except the re-hold reserve (5.4) |
| 2026-09-29 14:55 | **PA-7**: T0 pushed under a parked name at H2; PRE-3F owns its own test target; only `142.058-T` edits the F54 file, after PA-5 | PS-3, PS-4; R-2 |
| 2026-09-29 17:38-18:47 | Option A (Step 4.3 unchanged); six resets, route (a), layout delegation; route (c) | every slot PASS-only; PS-6; per task |
| 2026-09-29 21:00 | cache rebuild approved (assembly step 11, one CG-S re-apply); Q2 P-001 acknowledged; subtasks close with their parent | run points 1, 2 |
| 2026-09-30 17:35 | Q4 no (fix b); Q3 (ii) | Stage closes subtasks after the carrying slots (10.1); PRE-4 placeholder |
| 2026-09-30 19:09 | section 1.1 (verbatim) and Orchestrator judgements | one task per shipment |
| 2026-10-04 12:32 | OD-1 to OD-7 answered and the sizing requirement (verbatim below) | this revision |
| 2026-10-04 | W4 (`scripts/__pycache__/`, repo-root `checkpoint.md`, both untracked) deleted by the Orchestrator under OD-2 | PS-10 `applied` |

**Operator, 2026-10-04 12:32 -07:00 (verbatim):**

> OD-7: Option A
> OD-2: I manually committed the W3 related changes since Stage is not allowed to.  W4 related changes can be discarded.
> OD-4: Yes, mark 142-S as abandoned and then archive.
> OD-1: Yes, 142.060-T can start now.
> OD-3: Option B
> OD-6: Sure, lift the rest of the PA-6 hold; however, use your best judgement depending on circumstances.
> OD-5: Q1: Yes; Q2: Yes; Q3: Yes, although that would seem to contradict the decision on Q1; PA-5 tasks can go under 142-F.
> Note that I want to ensure that the work is sufficiently decomposed such that no single shipment is excessively large or overly complex to ensure we don't run excessively long individual sessions that run up transcript lengths that cost AIC more than necessary.  Use best judgement in this regard.

Context: OD-7 A, OD-3 B and OD-5 Q1-Q3 as defined in 1.1 (PA-5 deliberation lines 123-170 and 224-250).

## 4. Work breakdown: one task per shipment, sized

Each slot is one shipment whose manifest is exactly `[<task>]`. Subtasks are never members; they stay as records and close
through 10.1. `queue_position`: Slot-NN → NN × 10; a letter suffix adds 0, 1, 2, 3 for a, b, c, d; Slot-19.k → 190 + k.
IDs `142.0NNa`/`b`/`c` are placeholders for tasks created at assembly step 4 (4.3); backlogit assigns the real IDs and
Stage records the map in its memory.

| Slot | Task | Unit | Route | Task predecessors after assembly | Created at |
|---|---|---|---|---|---|
| 01 | `142.060-T` | archive verifier | (a-1) | none (E11) | assembly |
| 02a | `142.061a` | PRE-1a layout, identity, minting | (a-1) | none | assembly |
| 02b | `142.061-T` | PRE-1b build and seal | (a-1) | 061a (E12) | assembly |
| 03 | `142.062-T` | PRE-2 | (c) | 061 | assembly |
| 04a | `142.064a` | PRE-3a identity REPLACE, RED baseline | (c) | 062 (E5) | assembly |
| 04b | `142.064b` | PRE-3b gate slot, install before ready, driver | (c) | 064a (E13) | assembly |
| 04c | `142.064-T` | PRE-3c transient retry, PRE-3 close | (c) | 064b (E13) | assembly |
| 05 | `142.065-T` | PRE-4b | (c) | 062, 064 (E10) | assembly |
| 06 | `142.066-T` | PRE-4 | (c) | 064, 065 | assembly |
| 07 | `142.067-T` | PRE-5 | (c) | 066 | assembly |
| 08 | `142.068-T` | PRE-6 | (c) | 067 | assembly |
| 09a | `142.054a` | F50a types, transitions, mock harness | (a-1) | 068 (E14) | assembly |
| 09b | `142.054b` | F50b Build, Seal, Publish stages | (a-1) | 054a (E14) | assembly |
| 09c | `142.054-T` | F50c probe stages, real chain | (a-1) | 054b (E14), 068 (+ archived) | assembly |
| 10 | `142.069-T` | NEW-1 | (c) | 054 | assembly |
| 11 | `142.070-T` | NEW-2 | (c) | 069 | assembly |
| 12 | `142.071-T` | NEW-3 | (c) | 070, 054 | assembly |
| 13 | `142.072-T` | NEW-4 | (c) | 069 | assembly |
| 14 | `142.073-T` | NEW-5 | (c) | 072, 071 | assembly |
| 15 | `142.074-T` | NEW-6 docs | docs | 073 | assembly |
| 16 | `142.055-T` | F51 | (a-1) | 054, 073 | assembly |
| 17 | `142.056-T` | F52 | (c), own placeholder (OD-3 B) | 055 (+ archived) | assembly |
| 18 | `142.057-T` | F53 | (a-1) | 054, 073 (+ archived) | assembly |
| 19 | `142.063-T` | PRE-3F | (c) | 062, 066 (E2), 060 (E8) | assembly |
| 19.k | PA-5 tasks (k from PA5-P) | PA5-C | (c) | from the PA5-P harvest; each source → 063 (E7) | PA5-P harvest |
| 20a | `142.058a` | F54a harness core (FL) | (c) | 063 (E3), 066, 065 (E15), PA-5 sinks (E6) | PA5-P harvest |
| 20b | `142.058b` | F54b refusal classes | (c) | 058a (E15) | PA5-P harvest |
| 20c | `142.058c` | F54c real indexed fixture (OD-5 Q2) | (c) | 058b (E15) | PA5-P harvest |
| 20d | `142.058-T` | F54d read provenance, equivalence | (c) | 058c (E15) (+ archived) | PA5-P harvest |
| 21 | `142.059-T` | F55 docs | docs | 058, 055, 057 (+ archived) | PA5-P harvest |

Slot count: 29 outside PA-5, plus k PA-5 slots (29 + k; 36 for k = 7), where k is the number of PA-5 tasks after
PA5-P's SG-1 splits. Slot-19.k take `queue_position` 191 to 199, below Slot-20a (200); if PA5-P yields more than 9 PA-5
tasks, PA5-P re-spaces `queue_position` for Slot-19.k and Slot-20a-20d in its staging PR (no HALT). HC (the Step 2 harness of `142.063-T`) keeps its 2-hour,
400-line cap. Docs-route tasks are `harness-verification-gated`.

### 4.1 Sizing gate SG-1 (binding, operator 2026-10-04)

For every slot, Stage counts: **files** with substantive edits (registration-sized glue is not counted: one `pub mod` line,
one `[[test]]` stanza, one dispatch arm or list entry, one field with a trivial getter and setter); **functions** new or
changed (fn, method or const body); **test scenarios** (a table-driven case set counts once); **open subtasks** (they run in
the same Ship session). **OK** needs files < 3, functions < 5, scenarios < 4, no open subtask and an estimate ≤ 2 h (XS ≤
0.5 h, S ≤ 1.5 h, M ≤ 2 h); anything else is **SPLIT**. `Complexity: high` also needs a split or a named de-risking step.
SG-1 runs here, again on every task created at assembly step 4 and at PA5-P; a created task that fails it is HALT
`R17-SIZE-GATE <task> <metric>`. "(est)" marks a Stage estimate from the task text.

### 4.2 Sizing table

| Slot | Task | Files | Fns | Scen. | Open ST | Size, est. | Verdict |
|---|---|---|---|---|---|---|---|
| 01 | `142.060-T` | 2 | 1-2 (est) | 3 | 0 | S, 1.5 h | OK |
| 02 | `142.061-T` | 2 | 5 | 3 | 0 | S, 2 h | **SPLIT** S-02 |
| 03 | `142.062-T` | 2 | 3 (est) | 3 | 0 | S, 1.5 h | OK |
| 04 | `142.064-T` | 3 | 5 | 3 + 8-target baseline | 3 | M high, 4.5 h | **SPLIT** S-04 |
| 05-08 | `142.065-T`, `066`, `067`, `068` | 2 each | 2, 2 (est), 3 (est), 2 | 3 each | 0 | S, 1.5 h each | OK |
| 09 | `142.054-T` | 2 | ≥ 8 (est) | ≥ 5 | 3 | M high, 4.5 h | **SPLIT** S-09 |
| 10-14 | `142.069-T` to `142.073-T` | 2 each | 2, 3, 1, 4, 2 (est) | 3 each | 0 | S, 1.5 h (071 XS, 1 h) | OK (072 at the limit) |
| 15 | `142.074-T` | 1 | 0 | 0 | 0 | XS, 0.5 h | OK |
| 16, 18 | `142.055-T`, `142.057-T` | 2 each | 2 (est) | 3 (est) | 0 | S, 1.5 h each | OK |
| 17 | `142.056-T` | 2 | 1 (est, placeholder) | 3 | 0 | M, 2 h | OK (at the limit) |
| 19 | `142.063-T` | 1 + stanza | 4 | 2 (HC RED, IC GREEN × 3) | 0 | S, 2 h | OK (HC cap) |
| 19.k | PA5-T1 to PA5-T7 | 2 + glue | ≤ 4 | 3 | 0 | S; T4 M | one slot each; T4 **SPLIT** at PA5-P |
| 20 | `142.058-T` | 1 (≈ 1,630 lines) | > 5 (est) | > 4 | 2 | M, > 4 h (est) | **SPLIT** S-20 |
| 21 | `142.059-T` | 2 docs | 0 | 0 | 0 | XS, 0.5 h | OK |
| 02a / 02b | `142.061a` / `142.061-T` | 2 / 1 | 3 / 2 | 1 / 2 | 0 | S, 1 h each | OK |
| 04a / 04b / 04c | `142.064a` / `142.064b` / `142.064-T` | 2 / 2 + glue / 2 | 1-2 / 3 / 1 | 2 / 3 / 2 | 0 | S, 1.5 h each | OK (04b high, de-risked: careful mode, own RED, recorded HALT) |
| 09a / 09b / 09c | `142.054a` / `142.054b` / `142.054-T` | 2 each | 3 / 4 / 4 (est) | 3 each | 0 | S, 1.5 h each | OK |
| 20a-20d | `142.058a`-`c`, `142.058-T` | 1 each | ≤ 4 (est) | ≤ 3 each | 0 | S, 1.5 h each | OK if each FL share ≤ 400 lines (4.3) |

### 4.3 Splits (performed at assembly step 4, not now)

Common rules: the parent is the **last** slot of its family, so its `done` means the family is done; each new task's parent
is `142-F`; every acceptance criterion and verification item of the parent and its subtasks maps to exactly one family task
and the map is written into each task (unmapped or double-mapped: HALT `R17-SPLIT-AC-UNMAPPED <task> <item>`); inbound edges
stay on the parent, the first family task copies the parent's open predecessor edges, and chain edges order the family
(E12-E15); each family task writes its own RED cases at its own Step 2 in the shared harness file, under the parent's route;
subtasks are not deleted or re-typed and close through 10.1; new tasks carry prose `Size | Complexity`.

* **S-02 `142.061-T`.** `142.061a` (Slot-02a): `ReadServerLayout`, `READ_SERVER_ACTIVATION_DEADLINE`,
  `expected_identity()`, `read_server_layout`, `mint_generation_id`; scenario (1); the layout, identity-spelling, minting and
  attempt-4 P2-4 criteria; it creates the harness file and stanza. Parent (Slot-02b): `build_candidate_generation`,
  `seal_candidate_inventory`; scenarios (2)-(3) and the remaining criteria.
* **S-04 `142.064-T`.** `142.064a` (Slot-04a, from `.001-ST`): the harness file and stanza with scenario 1's `workspace_id`
  equality and scenario 3; the consumer baseline maps, the pinned-read sender grep and HEAD's non-git refusal, recorded once
  for the family in 064a's task record; identity REPLACE and its evidence. `142.064b` (Slot-04b, from `.002-ST`): the gate
  slot with `pub` `set_read_server_gate`/`read_server_gate`, `install_generation_gate`, `drive_generation_activation`;
  scenario 1 activation and scenario 2 except the transient step; install failure final; publication-last; invariant 7.
  Parent (Slot-04c, from `.003-ST`): the transient same-revision retry; scenario 2's transient step; every consumer target
  matched against 064a's baseline; root pedantic clippy. No F54 evidence in any of them (9.2). Careful mode for all three.
* **S-09 `142.054-T`.** `142.054a` (Slot-09a, from `.001-ST`): the typed stage enum, transitions, typed `Failed`, one
  deadline and one expected generation; mock-driven cases for success, failure at each of the seven stages (one table) and
  the single deadline. `142.054b` (Slot-09b, from `.002-ST`): Build, Seal and Publish in `ProductionVerifier`; typed expiry;
  a failed stage leaves the published generation serving. Parent (Slot-09c, from `.003-ST`): DaemonVerified,
  HealthVerified, CliProbeVerified, McpProbeVerified (observational only, R48); the PA-2b real-chain cases; the A20
  engram-indexer all-targets test and pedantic clippy gates.
* **S-20 `142.058-T`.** `142.058a` (Slot-20a): FL lands from `6d216d19` the descriptor-driven case generation, the coverage
  assertion, the declared-surface rule (no hard-coded CLI absence) and `_health` on direct IPC only. `142.058b` (Slot-20b,
  from `.003-ST`): every refusal class with no side effects, and unknown methods. `142.058c` (Slot-20c; deliberation PA5-T7,
  OD-5 Q2): the fixture publishes a real indexed generation of its `src/lib.rs`, replacing `probe_row`. Parent (Slot-20d,
  from `.002-ST`): read success and provenance for generation-backed reads, the `DaemonScopedRead` expectation for
  non-generation reads (OD-5 Q1; F54 half of PA5-T6), CLI/MCP equivalence, the whole matrix GREEN. At the split, Stage
  measures each slot's share of the `6d216d19` body; a share over 400 lines is split again by case class before creation.

### 4.4 Session budget SB-1

A slot session stops, without extending itself, when it (a) passes 1.5 × its 4.2 estimate, (b) reaches a third failing
build-fix loop at Step 4, or (c) its Step 2 harness diff passes 400 lines. Ship writes a checkpoint and raises HALT
`R17-SLOT-BUDGET <slot> <a|b|c>` to Stage; Stage proposes the split in a staging PR before the slot is claimed again.

## 5. Ordering

### 5.1 Task edges

All `blocks`. Existing edges stay except as below.

| # | Order | Operation | Item (depends on) | Target |
|---|---|---|---|---|
| E5 | 1 | add | `142.064a` | `142.062-T` |
| E1 | 1 (same step) | remove | `142.064-T` | `142.063-T` |
| E2 | 2 | add | `142.063-T` | `142.066-T` |
| E3 | 3 | add | `142.058a` | `142.063-T` |
| E11 | 4 | remove ×5 | `142.060-T` | 054, 055, 056, 057, 058 |
| E8 | 5 | add | `142.063-T` | `142.060-T` |
| E10 | 6 | add | `142.065-T` | `142.064-T` (PRE-3 RED baseline, `142.065-T` line 40) |
| E12-E15 | 7 | add | 061-T → 061a (E12); 064b → 064a, 064-T → 064b (E13); 054a → 068, 054b → 054a, 054-T → 054b (E14); 058a → 066, 065, 058b → 058a, 058c → 058b, 058-T → 058c (E15) | |
| E6, E7, E9 | PA5-P harvest | add | each PA-5 source → `142.063-T` (E7); `142.058a` → each PA-5 sink (E6); internal edges (E9) | |

E5 and E3 need the split tasks, so assembly step 4 (splits) runs before the edge step. E1 precedes E2 (else 063 → 066 →
064 → 063). E11 precedes E8 (else 063 → 060 → 058 → 063). If PA5-P yields zero tasks, E6, E7 and E9 are not added and E3
alone orders Slot-20a after 063. Stage runs `backlogit_get_dependencies` after each edge and a full cycle check after the
last; a cycle is HALT `R15-DAG-CYCLE <edge>`. E2 and E8 are real code dependencies under PA-7; no order-only edge exists.

### 5.2 Shipment edges and queue_position

* **Mirrored edges.** For every task edge X → Y where both tasks have a created slot shipment, Stage runs `backlogit dep add
  <ship(X)> <ship(Y)> --type blocks` (routed to AddShipmentBlock). At assembly that covers every edge among Slot-01 to
  Slot-19 (count recorded in the assembly record); edges to Slot-19.k to Slot-21 are added when those slots are created.
* **queue_position.** No write path exists in 1.11.0, so Stage writes `custom_fields.queue_position` (section 4 values) into
  each created shipment's Markdown on the staging branch. Run point 1 indexes the assembly slots; the PA5-P slots are
  indexed by the plain sync after their landing (a further rebuild needs a new approval). After each, Stage checks that
  `backlogit queue view --type shipment --status queued` lists the slots in that order; otherwise HALT `R16-QUEUE-POSITION
  <found>`.
* A shipment predecessor counts as finished when it meets the section 6 definition (precedent: 142-S was claimed on
  2026-09-23 with predecessors archived `done`).

### 5.3 Claim gate CG-D

A slot may be claimed only when: (1) every task predecessor is `done` (archived) and every shipment predecessor is finished
(section 6), checked with `backlogit_get_dependencies` and `backlogit get`; (2) among eligible slots it has the lowest
`queue_position` (tie-break only); (3) its manifest still holds exactly its one task (CG-Q). Failure: HALT `R15-DAG-ORDER
<slot> <predecessor>`. A predecessor slot is finished when its shipment is archived with `archived_status` `done` or
`shipped` (the 141-S manual safe-close); the pipeline-topology `pre_claim` gate (`autoharness gate pipeline-topology
--phase pre_claim`) accepts archived predecessors as satisfying a `blocks` edge (`.backlogit/archive/141-S.md`,
"Successor eligibility clarification"). The plan intentionally uses archived `done` for these task-only, partial-feature
slots, because P-015 forbids `shipment ship` (6).

### 5.4 PA-6 lift and the re-hold reserve

PA-6 is lifted (OD-6, 2026-10-04) for every formerly held slot: Slot-01, 16, 17, 18, 19, 19.k, 20a-20d and 21. The
blocks-edge DAG still orders them; PA-4 stays held. Slot-01 to Slot-19 are created at assembly; Slot-19.k to Slot-21 in the
PA5-P staging PR. **Re-hold reserve** (the operator's "use your best judgement"): if CG-B shows `main`'s full suite red, or a
slot halts `R15-SLOT-NOT-SELF-CONTAINED`, the Orchestrator may re-hold the downstream slots by not routing their claims;
Stage appends a `PA-6-REHOLD <reason>` comment to each affected task. A re-hold needs no new operator decision but is
reported to the operator in the same session; lifting it is also reported.

## 6. Finished slot and per-slot closure

**A slot is finished** when all of these hold: its implementation PR is merged to `main` with a merge commit (P-009) and the
merge SHA is recorded; the task is `done` (archived); the shipment is `status: archived` with `archived_status` in {`done`,
`shipped`}; the closure record `docs/closure/<shipment>-<date>-post-merge-closure.md` is on `main`.

**Step 6 of every slot is the 141-S manual safe-close** (P-015; `docs/closure/141-S-2026-09-23-post-merge-closure.md`
"Shipment Safe-Close"; `.backlogit/archive/141-S.md` audit rationale): hand-author `.backlogit/archive/<S>.md` with
`archived_status: done`, remove `.backlogit/queue/<S>.md`, run `backlogit sync`, verify `142-F` unchanged. `backlogit
shipment ship` is **forbidden** for every slot (it cascades: it force-releases `142-F` and expands children); `backlogit
move <S> --status shipped` is refused (exit 9, 141-S).

Per slot, Ship also: hashes `142-F`'s queue file before and after the safe-close (byte-identical, R7.6 Closure row); treats
the safe-close sync as a CG-S trigger (`docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md`); applies the
rollup guard (10.3); re-checks that `lifecycle_hooks.pre_task_completion` is still disabled. The closure record is one
canonical file per slot (140-S and 141-S needed repair).

## 7. Landing and 142-S disposition

### 7.1 Sequence

| Step | Owner | Action | Record |
|---|---|---|---|
| P0 | Stage | Classify the worktree (7.2); list paths per class in the planning PR description; read-only | — |
| A0 | Ship | On the parked branch, before any branch switch: capture B0 (R9.9); `git worktree list --porcelain` shows one worktree; record the `main` full-suite baseline (CG-B) | PS-2 |
| C1 | Ship | Carry the one colliding W2 path, `.backlogit/checkpoints/checkpoint-20260924-010034.json`: copy it to `tmp/w2-carry/`, record both SHA-256 values (equal), then `git restore --source=HEAD -- <path>`; `git status` shows the path clean. Any mismatch: HALT `R17-W2-CARRY-MISMATCH <stage>` | PS-13 |
| A0R | Ship | Commit `docs/memory/<A0 date>-ship-142-f-a0-b0.md` (B0 path, CG-B result, C1 hash) on the parked branch | PS-2 |
| T0 | Ship | Re-derive 7.3 from `git log --name-only main..A0R^` and halt on a difference; annotated tag `parked/142-s-split-<A0R sha8>` at A0R (a child of `f6f3171f`, so T0 reaches all eleven parked commits); `git cat-file -e <T0>:<B0 path>` exits 0 | PS-3 |
| H2a | Ship | No workflow trigger matches `parked/`; push the tag only; remote recovery check; record URL, ref, SHAs, B0 path | PS-4 |
| H0 | Ship | Land the operator's W3 commit on `chore/harness-142-f-w3` (mechanism below); P-014-gated PR; the operator merges with a merge commit | PS-12 |
| H1 | Stage | After H0 is merged: `git switch -c chore/stage-142-f-planning origin/main` (fetched); commit W1 only; the operator opens and merges (P-014, P-009) | PS-1 |
| H3 | Ship | On `chore/ship-142-s-disposition` from fetched `main`: copy the C1 file back (hash equals the C1 record, else `R17-W2-CARRY-MISMATCH restore`), commit W2, then PS-5; P-014-gated PR; merge commit | PS-11, PS-5, PS-13 |
| ASM | Stage | Assembly (section 8) on `chore/stage-142-f-assembly` from fetched `main` | PS-6 |
| PA5-P | Stage | PA-5 planning and harvest (7.6) | — |

After A0R the parked branch is untouched (no rebase, push, reset, deletion or commit); T0 is kept indefinitely. H0 precedes
H1 so every later step and slot runs under `f6f3171f`'s rules; H1 precedes H3 because the W2 manifest of 142-S names
`142.060-T`. Branch switches carry the other dirty W1/W2 paths untouched; commits stage by explicit pathspec only.

**H0 mechanism (PS-12).** `git fetch origin`; `git switch -c chore/harness-142-f-w3 origin/main`; `git cherry-pick -x
f6f3171ff4a5b1a76860942d99a2cc7d72e8d146`. The dry check in section 2 predicts a conflict in `.autoharness/config.yaml`.
For each of the nine paths take the operator's version: `git checkout f6f3171f -- <path>`; then `git diff --quiet f6f3171f
-- <nine paths>` exits 0, and `git cherry-pick --continue` keeps the operator's author and message. The commit must change
exactly the nine paths of 7.3; any other result: HALT `R17-H0-DIFF <paths>`. Version rule: the operator's latest
(`f6f3171f`) wins over `main` and over `a0ccdc27`; taking its blobs also lands `a0ccdc27`'s earlier edits to
`config.yaml` and `_ship.agent.md` (the operator's own). `a0ccdc27`'s `_orchestrator`/`_stage` agent edits and checkpoint
repairs stay reference-only on T0; the C1 checkpoint follows W2. Neither Stage nor Ship authors any of the nine files.

### 7.2 Worktree classes

* **W1 Stage planning** → H1: 142-F plans, decisions, memory, scratch, `docs/operator-glossary.md`; queue files
  `142.060-T`-`142.074-T`, `142.064.00x-ST`; decision items `035-D`-`038-D`; stash files.
* **W2 142-S execution record** → H3 (Ship): `142-F.md`, `142-S.md`, `142.054-T`-`142.059-T` and their subtask files
  (including the `142.058.001-ST` archive move), checkpoints (including the C1 path), `memories.json`, Ship memory files.

W3 (harness/config) is commit `f6f3171f`, landed by H0; W4 (`scripts/__pycache__/`, repo-root `checkpoint.md`) was deleted on
2026-10-04 under OD-2. Any path Stage can't place in W1 or W2: HALT `R16-WORKTREE-UNCLASSIFIED <path>`.

### 7.3 Parked commit → slot map

From `git log --name-only main..f6f3171f`. Ship re-derives it at T0 from `main..A0R^` and halts on a difference.

| Commit | Files | Slot |
|---|---|---|
| `f6f3171f` | `.autoharness/backlog-registry.yaml`, `.autoharness/config.yaml`, `.autoharness/continuous-learning/observations/2026-09-26.jsonl`, `.github/agents/_ship.agent.md`, `.github/copilot-instructions.md`, `.github/policies/workflow-policies.md`, `.github/skills/build-feature/SKILL.md`, `.github/skills/harness-architect/SKILL.md`, `AGENTS.md` | none (H0) |
| `a0ccdc27` | `.autoharness/config.yaml`, checkpoints, three `.github/agents` files, a memory file | none (reference only) |
| `c269fa79`, `41dd5081` | `crates/engram-indexer/src/{lib,preflight}.rs`, `preflight_gate_test.rs` | 09a-09c, reference only |
| `a47b8aff` | `release_archive_smoke_workflow_test.rs` | 01 (`142.060-T`) |
| `1dfc1b5b`, `4995d681` | `start_launcher_test.rs` | 16 (`142.055-T`) |
| `5760b948` | `start_launcher_failure_test.rs` | 17 (`142.056-T`) |
| `e24f5ae2` | `start_sh_launcher_test.rs` | 18 (`142.057-T`) |
| `7bd9e504`, `6d216d19` | `read_server_cli_mcp_parity_test.rs` | 20a-20d (FL share per slot, 4.3) |

`142.059-T` has no parked commit. A slot re-uses a parked commit only where its section 9 entry says so; otherwise Step 2
re-derives on `main`.

### 7.4 ProposedActions

| ID | Owner | Action and targets | change_kind | ActionRisk | approval_required | Approval / ActionResult | Rollback / containment |
|---|---|---|---|---|---|---|---|
| PS-1 | Stage (commit); operator (merge) | H1: W1 on `chore/stage-142-f-planning` | local commit, PR | moderate | merge only | split decision 2026-09-27; `approved` | revert PR |
| PS-2 | Ship | A0, A0R on the parked branch | read-only run, local commit | low | no | R9.9; `approved` | `git revert` (unpushed) |
| PS-3 | Ship | T0 local tag | local ref create | low | no (create) | PA-7; `approved` | deleting the tag is destructive and needs a new approval |
| PS-4 | Ship | H2a: push T0 only | external, additive | high | recorded | PA-7; `approved`; careful mode | none needed; a failed check stops before H0 |
| PS-5 | Ship | Abandon 142-S: `shipment block`, then `archive` (7.5) | backlog state, irreversible in-tool | **destructive** | **yes, verbatim** | OD-4 2026-10-04 12:32 verbatim; **`approved`, conditional on probe PASS** | 7.5 restore path |
| PS-5c | Ship | PS-5 probe cleanup: delete the probe-created, gitignored scratch copy `tmp/ps5-probe/` (7.5 step 2) | worktree scratch delete | low | no (inside the approved PS-5 probe scope) | covered by the PS-5 approval (OD-4) | none needed; scratch copy, recreated by re-running the probe |
| PS-6 | Stage | Six resets `142.054-T`-`142.059-T` `active` → `queued` | backlog state | moderate | recorded | 2026-09-29 18:14; `approved` | re-setting `active` needs a new approval; a revert by sync is CG-S |
| PS-9 | Ship | Slot branches start from `main` | branch policy | low | no | R11.2; `approved` | n/a |
| PS-10 | Orchestrator | W4: delete untracked `scripts/__pycache__/` and repo-root `checkpoint.md`; W3 handled by the operator's commit | **destructive** (untracked files) | moderate | yes | OD-2 verbatim; W4 **`applied`** 2026-10-04; W3 **`superseded`** by `f6f3171f` and PS-12 | none (untracked junk; W3 content is in `f6f3171f`) |
| PS-11 | Ship | H3: W2 commit and the disposition PR | commit, PR | moderate | merge (P-014) | `approved` (PS-5 approved, conditional) | revert PR |
| PS-12 | Ship (commit); operator (merge) | H0: cherry-pick `f6f3171f` onto `chore/harness-142-f-w3`, operator's version wins | commit, PR | moderate | merge (P-014) | OD-2 2026-10-04 and judgement A; `approved` | revert PR |
| PS-13 | Ship | C1: back up one W2 path to `tmp/w2-carry/`, restore it to `HEAD`; copy it back in H3 | worktree file | moderate | no (copy and hash first) | judgement A; `approved` | copy the backup back; the hash proves identity |

### 7.5 PS-5 mechanism (Ship, inside H3)

Checked against the backlogit tool descriptions: no abandon operation exists; `return_blocked` acts on one item;
`normalize_blocked_shipment` needs an out-of-band snapshot; `unblock` only targets `queued` or `active`; `ship` cascades; a
direct shipment status `move` is refused (exit 9, 141-S), so PS-5 uses no `move`. **Orchestrator verification (2026-10-04,
read-only):** `backlogit shipment --help` lists add, block, claim, create, get, list, reconcile-shipped, repair-evidence,
return-blocked, ship, unblock (no `abandon`); `backlogit shipment block <id> --reason <r> [--by] [--resume-checkpoint]`
exists ("Block an active shipment"); `backlogit archive <id>` exists. Precedent `081-S`: `.backlogit/logs/081-S.jsonl`
shows active → (comment: active -> blocked) → archived, and `.backlogit/archive/081-S.md` has `status: archived`,
`archived_status: abandoned`, with no move step. No precedent re-ships an abandoned shipment's members.

1. **Snapshot.** Copy `142-S.md`, every member file and `142-F.md` to `docs/memory/<date>-ship-142-s-ps5-snapshot/` and
   commit it on the H3 branch first.
2. **Probe** on a copy of `.backlogit/` under `tmp/ps5-probe/` (gitignored), with `--cwd tmp/ps5-probe`: (a) `backlogit
   shipment block 142-S --reason "<split per 2026-09-27>" --by ship`; (b) `backlogit archive 142-S` gives
   `archived_status: abandoned` (081-S); (c) reset one former member to `queued`, then `backlogit shipment create --title
   probe --items <member>` succeeds; (d) `142-F` unchanged. Then delete `tmp/ps5-probe/`: this cleanup is part of the
   approved PS-5 probe scope (a gitignored scratch copy the probe itself created inside the workspace), needs no separate
   approval and is recorded as ActionRisk low (PS-5c, 7.4).
3. **Approval.** Given in advance (OD-4, 2026-10-04 12:32, verbatim in section 3) for exactly steps (a)-(b). Block, then
   archive carries out OD-4 ("mark 142-S as abandoned and then archive"): the abandoned state is recorded by the archive's
   `archived_status: abandoned`, as for 081-S. The probe stays mandatory: if every probe step (a)-(d) succeeds, Ship
   records the probe result next to the approval and proceeds; if any step refuses, or (b) does not yield
   `archived_status: abandoned`, HALT `R16-PS5-UNPROVEN <step> <exit>` back to the operator; the advance approval does not
   cover an improvised fallback or any other command.
4. **Apply** (a)-(b) on the H3 branch in careful mode. Then re-read every member; a changed status or path: HALT
   `R15-142S-MEMBER-MOVED <ids>`. Re-read `142-F` (rollup guard 10.3).
5. **Restore path.** Before merge: `git revert` of the PS-5 commit on the H3 branch. After merge: a revert PR. Either way
   the index then needs a rebuild under a new approval.

### 7.6 PA5-P: PA-5 planning and harvest (Stage)

After the Revision 18 freeze (17), Stage runs impl-plan on the PA-5 deliberation (PA5-C, OD-5 answers, R-1, R-2), then plan-harden
in careful mode, then plan-review, then harvest under `142-F`. Inputs: the provisional PA5-T1 to PA5-T7 outline, re-cut so
that no PA-5 task edits the F54 file (R-2: PA5-T7 is Slot-20c, the F54 half of PA5-T6 is Slot-20d) and PA5-T4 is split
(SG-1). Every PA-5 task is its own Slot-19.k and passes SG-1; k is fixed here (`queue_position` 190 + k; more than 9: re-space as in 4). The PA-5 staging PR creates the PA-5 tasks, E6/E7/E9, and the
shipments Slot-19.k, Slot-20a-20d and Slot-21 with their mirrored edges and `queue_position`. Planning may overlap Ship's
execution of Slot-01 to Slot-15 under P-016 (no implementation branch or worktree); its harvest PR follows the assembly
merge. Until it merges, nothing after Slot-19 can be claimed (no shipment exists).

## 8. Assembly (Stage, one session)

A failed check is a HALT to the operator; nothing is repaired automatically.

1. **Preconditions.** Plan frozen at Revision 18 (17). H0, H1 and H3 merged; PS-5 `applied`; T0 local and
   remote.
2. **Read-only re-check** on fetched, clean `main`: `backlogit get` for every item in section 2 matches its Markdown (else
   HALT `R14-CACHE-DIVERGED <id>`); 142-S archived `abandoned`; R14.2 feasibility (`pre_task_completion` disabled; `move`
   covers subtask completion; else HALT `R14-SUBTASK-MOVE-UNSUPPORTED <case>`).
3. **Resets** PS-6, then the rollup guard.
4. **Splits** (4.3): create `142.061a`, `142.064a`, `142.064b`, `142.054a`, `142.054b`, `142.058a`, `142.058b`,
   `142.058c` (parent `142-F`); rewrite each parent to its own share; SG-1 and the AC map checks; rollup guard after each.
5. **Edges** in 5.1 order (E5, E1, E2, E3, E11, E8, E10, E12-E15), checks per 5.1.
6. **Content.** Rewrite each task's text to meet section 9. Then for every task: each requirement is present (else HALT
   `R16-CONTENT-CHECK <task> <req>`), and no U1 string remains in the description, acceptance or implementation sections
   (else HALT `R16-STALE-TEXT <task> <string>`). Rollup guard after each task.
7. **Shipments** Slot-01 to Slot-19 (24), in `queue_position` order: `backlogit_create_shipment` with `items: <task>`, title
   `142-F Slot-<NN>: <task> <unit>`, and a description with the slot, its predecessors, its 4.2 estimate and, where 7.3
   maps a parked commit, the T0 location. A refused task-only manifest: HALT `R15-SHIPMENT-SHAPE <slot> create`. Then the
   5.2 edges and `queue_position`.
8. **Lift comments.** Append `PA-6-LIFTED 2026-10-04 (OD-6)` to `142.055-T`, `142.056-T`, `142.057-T`, `142.060-T`,
   `142.063-T`, `142.058-T`, `142.059-T`; existing hold comments stay as history.
9. **Constitution Check** of the old plan replaced by section 13.
10. **Verify** each shipment (`backlogit shipment get`) and member: one member, a task, `queued` (CG-Q); else HALT
    `R15-SHIPMENT-SHAPE <slot> <found>`.
11. **Landing (CG-M).** Stage commits only; the operator merges. Afterwards, on fetched, clean `main`: cache rebuild run
    point 1, the queue-order check of 5.2, step 10 again under CG-S, and the slot-to-shipment-ID map in Stage memory as the
    handoff tokens.

## 9. Final per-task content requirements

### 9.1 Universal (every task)

* **U1** No text sets shipment, order or gates by group: no `S1`-`S5`, no `142-S` membership, no `PA-1` gating, no
  `pending-pa-1` label, no `EXPECTED_PENDING_RED` allowance mapped to another task, no `after-edit map`, `F54 settle`,
  `R12.4` or "frozen". History and comment sections and the U7 STATUS-RESET line are exempt.
* **U2** A line "Shipment: Slot-NN (one-task shipment, 142-F decomposition plan Revision 18)"; "Depends on" equals the
  task's edges after section 5; the 4.2 size and estimate.
* **U3** Step 4.3 and the final readiness run are `PASS` only; a task never relies on another task's pending RED.
* **U4** Route form stated (section 4). Route (c): the harness-architect writes the failing test at Step 2 against the
  task's own placeholder; build-feature never authors it (R12.5 A16 template, R11.10).
* **U5** Prose `Size | Complexity` kept, or set by 4.3 for a split task.
* **U6** No claim of ownership of another task's file and no signature freeze (X2). R12.3 still applies: no `#[path]`
  include of another task's production file.
* **U7** `142.054-T`-`142.059-T` carry the STATUS-RESET line (A16, R11.2). Formerly held tasks carry `PA-6-LIFTED`.
* **U8** Every slot PR merges with a merge commit and records its SHA. **U9** SB-1 is quoted in the task's notes.

### 9.2 Per task (old-plan source in brackets)

* **`142.060-T`** (Slot-01): depends on nothing; its SEQUENCING line says "Slot-01, claimed first (OD-1, 2026-10-04)",
  replacing "final code task of 142-S … adds it to active shipment 142-S (plan PA1)"; verification (3) reads "only code
  task of its shipment"; P-021 C3 cites the slot PR's residual-risk record; parked `a47b8aff` is reference. [M16, R6.7]
* **`142.061a`, `142.061-T`**: PRE-1 unit text split per S-02; universal otherwise. [R11.9]
* **`142.062-T`**: PRE-2 unit text; universal only (A1-A3 retired).
* **`142.064a`, `142.064b`, `142.064-T`** (S-04): `142.064a` "Depends on: PRE-2 (E5)"; `set_read_server_gate` and
  `read_server_gate` both `pub` (064b); no F54 evidence or determinism ownership in any of the three, consumer-baseline
  gates only [R6.9, R14.7 A25, R6.5 M1-M6, R7.1 M15]; the consumer baseline and pinned-read grep are 064a's, and 064b and
  `142.064-T` compare against 064a's record [M14, A17, A26]; each writes its own RED [A16]; the parent's transient retry
  and consumer evidence take three identical careful-mode runs of its own target, not F54 [M6, M15, R7.8].
* **`142.065-T`**: depends on 062 and 064 (E10); line 40 reads "Consumer regression targets unchanged against the PRE-3
  RED-phase baseline. HALT if any goes RED." (no F54 map, no PRE-3F); verification covers consumer targets only. [M7-M8]
* **`142.066-T`**: the inline two-observation rule (F1/G1, S1, 250 ms, S2, F2/G2; snapshots and status compared separately;
  no IPC between snapshots) [R7.7]; the harness lands `pub const GENERATION_PINNED_READS: &[&str] = &[];` and the
  implementation adds `"get_workspace_statistics"`; the test is `assert!(GENERATION_PINNED_READS.contains(
  &"get_workspace_statistics"), "Worker: 142.066-T GENERATION_PINNED_READS");` and acceptance says "contains", never
  "final value" or `assert_eq!` [R14.3 A23-A24, overridden]; no F54 ownership and no F54 RED text (line 52) [M9-M11].
* **`142.067-T`**: a feasibility row: call `probe_cli_read` first, then assert `PREFLIGHT_READ.mcp_tool` membership. [R14.6]
* **`142.068-T`**: inline extraction from `structuredContent` and the CLI command `engram --workspace <W> --json status`.
  [R7.1 M17]
* **`142.054a`, `142.054b`, `142.054-T`** (S-09): the harness-architect of each re-qualifies its F50 share on the slot's base
  tree; the PA-2b real-chain RED belongs to `142.054-T`; parked `c269fa79` and `41dd5081` are reference only; every slot's
  gates are `cargo test -p engram-indexer --all-targets --no-fail-fast` and its clippy gate GREEN. [R11.2, R10.5, A18-A20]
* **`142.069-T`**: a crate-local auto-discovered test through public `engram_indexer` paths, no `#[path]` include [A6-A8];
  A20 gate; line 36 reads "`stage_name` is exhaustive and equals the seven F50 `Failure` variants" (no F52/F53 comparison).
* **`142.070-T`**: a crate-local public-API test, no `#[path]` include, no lint attributes [A9-A10]; A20 gate.
* **`142.071-T`**: the crate-local harness command with its expected count [LD3]; A20 gate.
* **`142.072-T`**: `pub const RELAY_STAGES: [&str; 7]` holding the seven full lines `{"state":"Failed","stage":"<Stage>"}`;
  `normalize_verdict` returns the equal element and uses no other list [A21, A22]; the crate-local test proves two-way set
  equality between `RELAY_STAGES` and the lines built from `stage_name(f)` over every `Failure` variant [A11 rev. R14]; the
  variant guard is a `match` with no wildcard whose arms return distinct values (variant index 0-6, or `stage_name(f)`), or
  a single or-pattern arm, so `clippy::match_same_arms` stays clean; no `allow` attribute; no freeze text [A12-A13]; A20.
* **`142.073-T`**: engram-indexer all-targets test and clippy GREEN [A20]; CLI, parser and catalog parity kept without F54
  map claims [M13]; the `Cli::command().debug_assert()` check lives in a `#[cfg(test)]` module in `src/bin/engram.rs`
  (`struct Cli` is private, line 23), not under `tests/contract`.
* **`142.074-T`**: docs-only; no "Superseded by 142-F" note (old lines 27 and 35 removed); line 44 reads "Downstream: none".
* **`142.055-T`**: owns the fail-open assertion rewrite in `start_launcher_test.rs`; fail-closed launcher scope; route
  (a-1); parked `1dfc1b5b`, `4995d681` reusable as its harness records. [R11.6 P2-3, R11.9 step 5 W9]
* **`142.056-T`** (OD-3 B): files `start_launcher_failure_test.rs` plus one named placeholder (marker `Worker: 142.056-T`)
  in `start.ps1` or the test-support seam, placed by the harness-architect at Step 2; the matrix (seven-stage table, spaced
  paths, unowned descendant) fails first against that placeholder; build-feature replaces it; a case that passes at Step 2
  is recorded as characterization, not RED; no placeholder that makes a case fail first: HALT `R17-F52-NO-PLACEHOLDER`;
  parked `5760b948` is reference; no transferred fail-open criterion. [W9]
* **`142.057-T`**: F53 scope; parked `e24f5ae2`. [R11.2]
* **`142.063-T`**: the wholesale R9.6 text (PRE-3F HC/IC, pinned API, fixture, positive witness, three-run gate, AC1-AC8) on
  its own test target (PA-7); depends on 062, 066, 060; F54 barrier evidence for its own IC target only; no F54 file
  edit. [R9.6]
* **PA-5 tasks**: per the PA5-P harvest (7.6); each maps every F54 refusal and provenance case it enables to its 058 slot;
  pinned `get_workspace_status` changes only `code_graph` (R-1).
* **`142.058a`-`c`, `142.058-T`** (S-20): V1-V8 [R9.7] each mapped to one family slot; FL points and `FL-RED-HALT` [R11.6];
  addition 5 [R10.4]; T0 location and B0 path [R9.9]; the (d2) check; the drift ledger before each claim; FL takes each
  slot's share of the `6d216d19` body byte-identical except compile-only adaptations (paths, imports) listed in the harness
  record and proven by `cargo check --tests` before the RED run (that share is test code only, the F54 harness, so no
  production code precedes RED and RED-before-implementation holds); no `EXPECTED_PENDING_RED` mapping; `.001-ST` history
  kept. `142.058c` carries the reviewed real-fixture change [V5]; consumer-target evidence only [M6, V6].
* **`142.059-T`**: F55 docs, plus the moved note: append to
  `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md` a "Superseded by
  142-F" note citing this plan and `142.055-T`: guardrail 4 (fail-open to Copilot) is replaced by the fail-closed preflight;
  one shared budget and exact-child cleanup are retained.

## 10. Closure

### 10.1 Subtask close (after Slot-04c, Slot-09c, Slot-20d)

Rule R13.4; procedure R14.2 fix (b). After the family's last slot is finished (section 6): one worktree, fetched clean
`main`; on `chore/stage-142-f-subtask-close-<NN>` from `main`, read each subtask and its parent with `backlogit get` against
the Markdown (mismatch: HALT `R14-CACHE-DIVERGED <id>`); `backlogit move <id> --status done` per subtask, with a note citing
the carrying slot (4.3), its Step 4.3 record, implementation SHA and merge SHA; never `--force-gates` or `--gate-base`;
refusal: HALT `R14-SUBTASK-MOVE-HALT <subtask> <from>-><to> <exit>`; no sync; rollup guard after each move. Stage commits;
the operator merges (P-014, P-009). After the merge: re-read only; mismatch: HALT `R14-CACHE-DIVERGED <id>`. Options on any
`R14-SUBTASK-MOVE-*` halt: a scratch probe under `tmp/`, revisit Q4, or archive the subtasks with a CP-FINAL amendment.

### 10.2 CP-FINAL and `142-F`

After Slot-21 is finished, on `chore/stage-142-f-close` from `main`: if `142-F` is already `done` or archived, HALT
`R15-ROLLUP-DRIFT 142-F <status>`; read every child and grandchild in `queue/` and `archive/`; a subtask not `done`: HALT
`R14-SUBTASK-OPEN <ids>`; any other child not `done`: HALT `R14-142F-OPEN-CHILDREN <ids>`. Stage asks the operator
(**CP-FINAL**), records the answer verbatim, then runs `backlogit move 142-F --status done` citing every slot's merge SHA.
The operator merges. Until then P-001 blocks other release units (`R14-142F-CLOSE-PENDING`).

### 10.3 Rollup guard

After every write to a `142-F` descendant (resets, splits, edges, text edits, subtask moves, the PS-5 member re-read, every
slot's Step 6), the writer re-reads `142-F` and the item's parent task. Any status change the step didn't intend: HALT
`R15-ROLLUP-DRIFT <id> <from>-><to>`. Reverting it needs operator approval (backlogit changes parent status itself:
`.backlogit/logs/109-F.jsonl`, `087-F.jsonl`).

## 11. Cache rebuild

A plain `backlogit sync` can union a stale cache over the Markdown
(`docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md`). Run point 1 (assembly step 11, once) and run point 2
(the one CG-S re-apply for all of 142-F) are `approved` (2026-09-29 21:00); run point 3 (after subtask-close PRs) is
**withdrawn** (1.1). Any other rebuild needs a new approval. Residual risk: plain syncs at each safe-close (29 + k; 36 for k = 7) and at
each Ship session start can revert a reset or a subtask's `done`; CG-S, the 10.1 re-read and CP-FINAL detect it, each a
HALT.

Procedure (R14.4, careful mode): one worktree, fetched `main`, clean status; list `backlogit*` processes with
`Get-CimInstance Win32_Process`; `Stop-Process -Id` for in-workspace PIDs only; `Remove-Item -LiteralPath <file>
-ErrorAction Stop` for `.backlogit\backlogit.db`, `-wal`, `-shm` (sharing violation: HALT `R14-CACHE-LOCKED <file> <PIDs>`);
`Test-Path` `False` for all three (else `R14-CACHE-NOT-DELETED`); `backlogit sync` exits 0; `git status --short --
.backlogit/` empty (else `R14-SYNC-DIRTY`); re-read under CG-S. ActionRisk destructive; rollback none needed.

## 12. Claim gates (every slot)

* **P-001**: no other release unit `active`. **CG-D** (5.3) and **CG-Q** (one member, a task, `queued`).
* **T0 remote check** and R7.6 ancestry checks where 7.3 maps a parked commit (Slot-01, 09a-09c, 16-18, 20a-20d).
* **CG-H**: H0 is merged, so the slot runs under the `f6f3171f` harness rules.
* **CG-M**: every Stage or Ship backlog change the slot depends on is on `main` through a merged PR that passed P-014; where
  a hosted review was requested, P-018 applies (review `commit_id` equals HEAD). Clean worktree on `main`.
* **CG-S**: after any sync, re-read the member and re-run CG-Q. A reset task back at `active`: HALT `R12-RESET-REVERTED
  <ids>`; Stage may re-apply once in total for 142-F (run point 2); after that, the operator decides.
* **CG-B (baseline)**: Ship records the `main` full-suite result at A0. If it is red and Slot-01 isn't finished, no later
  slot is claimed; HALT `R15-SLOT-NOT-SELF-CONTAINED <task>` and the re-hold reserve (5.4) applies.
* **SB-1** (4.4) applies inside every slot session. Slot-20a: E6 recorded; drift ledger (R9.9, R10.2); FL text applied.

Ship side: Step 2 harnesses the one task, re-qualifying reset tasks' evidence on the base tree (R11.2). Route (c) F1-F3
verdicts per R13.6 and R14.6; F2/F3: HALT `R12-ROUTE-C-HALT <task> <test> <F2|F3>`. If a task can't reach `PASS` without a
later task: HALT `R15-SLOT-NOT-SELF-CONTAINED <task>` to Stage. Unexpected failures follow P-021 and Step 4.4a.

## 13. Constitution Check (replaces the old plan's section at step 8)

* **I Safety-first Rust**: unit design unchanged; recorded deviation: the test-only settle helper returns `Result<_, String>`.
* **II Test-first**: route (a-1) and (c) tests fail first on the task's own code or placeholder; F52 against its own
  placeholder (OD-3 B); split tasks write their own RED. Recorded R11.10 departure: `142.066-T`'s empty placeholder (R14.3).
* **III/IV Containment**: the PS-5 probe and the C1 carry stay under `tmp/` inside the workspace; the `debug_assert` test
  stays in `src/bin/engram.rs`.
* **V Observability, IX Persistence**: the Markdown is the record; the index is rebuilt only at run points 1-2.
* **VI Single responsibility**: one task per shipment; every slot passes SG-1.
* **VII Destructive approval**: PS-5 (`shipment block`, then `archive`) approved verbatim, conditional on the probe; the
  probe's deletion of its own scratch `tmp/ps5-probe/` is inside that scope (PS-5c, low); W4 deletion approved and applied;
  run points 1-2 approved; T0 deletion, re-setting `active` and any further rebuild need a new approval. Risky actions: H2a;
  PS-5; H0 conflict resolution; C1; six resets; up to 8 subtask moves; 29 + k safe-closes (36 for k = 7) with a plain sync
  (P-015, contained by 142-F byte-identity, CG-S and the rollup guard); the CP-FINAL `move`.
* **VIII Safety modes**: careful mode for H2a, PS-5, Slot-04a-04c and section 11.
* **X Context**: one document, 762 lines at Revision 18; SB-1 bounds every slot session.
* **XI Merge commits**: harness, staging, disposition, subtask-close, closure and slot PRs merge with a merge commit (P-009).
* **Task granularity**: SG-1 and the 4.2 table; the two `high` tasks are split; 04b keeps `high`, de-risked (4.2).
* **P-010**: Stage commits backlog and plan changes only; H0, C1, PS-5 and every safe-close are Ship's.
* **Stage template deviation**: manifests hold one task, not `[feature, tasks…]` (Ship's task-only contract, C1).
* **Cost**: about 29 + k short Ship cycles (36 for k = 7) instead of 21 longer ones; accepted under the operator's sizing requirement.

## 14. Decisions resolved and findings

| ID | Resolved (2026-10-04 12:32) | Effect |
|---|---|---|
| OD-1 | yes | Slot-01 created at assembly, claimed first (R-3) |
| OD-2 | W3 committed by the operator; W4 discard | H0 (PS-12); PS-10 `applied` |
| OD-3 | option B | `142.056-T` own placeholder (9.2) |
| OD-4 | yes | PS-5 `approved` (`shipment block`, then `archive`; Rev 18), conditional on probe PASS (7.5) |
| OD-5 | Q1-Q3 yes; PA-5 under 142-F | PA5-C, R-1, R-2, PA5-P (7.6) |
| OD-6 | lift, with best judgement | PA-6 lifted; re-hold reserve (5.4); PA-4 held |
| OD-7 | option A | section 17 |

**New open decisions:** none. R-1 to R-3 (1.1) are readings reported to the operator, not open questions.
**Findings still open:** attempt-13 P2-2, P2-4 to P2-10, P2-17 and P3s; attempt-14 P3s not addressed; earlier deferrals
(attempt-12 P2-1, P2-2, P2-6, P2-8, P2-11; stash `7BF90213`, `5AF5CD66`, `23E287C6`, `F99C705E`). Residual P2/P3 from attempts 13-15 go to the stash in the landing staging PR (18).

## 15. HALT token catalog

| Token | Raised at |
|---|---|
| `R12-RESET-REVERTED <ids>` | CG-S |
| `R12-ROUTE-C-HALT <task> <test> <F2\|F3>`, `R12-TEST-PATH-HALT`, `R12-LAYOUT-HALT <task> <item>` | Ship Step 2 |
| `R14-SUBTASK-MOVE-UNSUPPORTED <case>`, `R14-CACHE-DIVERGED <id>` | assembly step 2; 10.1 |
| `R14-SUBTASK-MOVE-HALT <subtask> <from>-><to> <exit>` | 10.1 |
| `R14-SUBTASK-OPEN <ids>`, `R14-142F-OPEN-CHILDREN <ids>`, `R14-142F-CLOSE-PENDING` | CP-FINAL |
| `R14-CACHE-LOCKED`, `R14-CACHE-NOT-DELETED`, `R14-SYNC-DIRTY` | section 11 |
| `FL-RED-HALT` | Slot-20a-20d |
| `R15-DAG-CYCLE <edge>`, `R15-DAG-ORDER <slot> <predecessor>` | assembly step 5; CG-D |
| `R15-SHIPMENT-SHAPE <slot> <found>` | assembly steps 7, 10 |
| `R15-142S-MEMBER-MOVED <ids>` | PS-5 step 4 |
| `R15-SLOT-NOT-SELF-CONTAINED <task>` | CG-B, Ship Step 2 |
| `R15-ROLLUP-DRIFT <id> <from>-><to>` | rollup guard, CP-FINAL |
| `R16-QUEUE-POSITION <found>`, `R16-WORKTREE-UNCLASSIFIED <path>`, `R16-PS5-UNPROVEN <step> <exit>` | 5.2; P0; PS-5 probe |
| `R16-CONTENT-CHECK <task> <req>`, `R16-STALE-TEXT <task> <string>` | assembly step 6 |
| `R17-W2-CARRY-MISMATCH <stage>`, `R17-H0-DIFF <paths>` (new) | C1, H3; H0 |
| `R17-SIZE-GATE <task> <metric>`, `R17-SPLIT-AC-UNMAPPED <task> <item>` (new) | assembly step 4, PA5-P |
| `R17-SLOT-BUDGET <slot> <a\|b\|c>`, `R17-F52-NO-PLACEHOLDER` (new) | any slot session (SB-1); Slot-17 Step 2 |

Retired: `R15-SIZE-SPLIT`, `R15-EDIT-ANCHOR`, `R15-Q5-UNCONFIRMED`, `R12-OWNERSHIP-HALT`, `R12-SIGNATURE-HALT`, and
`R14-SUBTASK-OPEN` as a claim gate.

## 16. Plan hardening

Requires plan hardening: yes. The record is the old plan's `## Plan Hardening`, Revisions 6-14, and sections 6, 7, 10.3 and
11 here. Risk surface: PS-5 (destructive, probed `shipment block` → `archive` per 081-S, advance approval bounded to the
two probed commands), H0 (a predicted
`config.yaml` conflict resolved by a fixed rule and a nine-path diff check), C1 (a hash-checked carry of one W2 file), the
per-slot manual safe-close with a plain sync, the hand-written `queue_position`, the `main` baseline (CG-B), and the splits
(AC map check, SG-1, SB-1).

## 17. Next step

Attempts 5-14 were ten consecutive FAILs; the circuit breaker stays: no further same-model re-review. Per OD-7 (option A),
one multi-model adversarial review of Revision 17 ran as attempt 15
(`docs/compound/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md`) and returned FAIL on one
MEDIUM-confidence P1 (A-1), closed in Revision 18 (18). **The plan is FROZEN at Revision 18** by Orchestrator judgement
under OD-7 option A: the only P1 was mechanical and was confirmed against the real tool (read-only `--help`) and the 081-S
precedent; no further plan review runs. Residual P2/P3 findings go to the stash, not into a Revision 19; Stage stashes them
in the landing staging PR (list in 18). A new P0/P1 found during execution goes to the operator. Then, in order: P0, A0,
C1, A0R, T0, H2a, H0, H1, H3, assembly, PA5-P.

## 18. Review history

### Attempt 15: FAIL (adversarial, multi-model)

Target: Revision 17 (713 lines), 2026-10-04. Three independent reviewers in parallel (Tier 1 gemini-3.7-flash, Tier 2
gpt-5.5, Tier 3 claude-opus-4.8); condensed in `docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md`.
Verdict **FAIL**: no HIGH-confidence finding; one MEDIUM-confidence P1 (A-1); four LOW P1s that the review judged to be
P2/P3; one LOW P3. All 11 attempt-14 P1 closures held.

| ID | Confidence, severity | Finding | Revision 18 disposition |
|---|---|---|---|
| A-1 | MEDIUM, P1 | PS-5 probe step (b) `backlogit move 142-S --status abandoned` unproven; contradicts "no abandon operation" and the 141-S exit-9 refusal | **Closed**: (b) removed; PS-5 = `shipment block` → `archive`, success = `archived_status: abandoned` (081-S); commands confirmed by the Orchestrator's read-only `--help` check; OD-4 coverage recorded (7.4, 7.5, 13, 14, 16) |
| B-1 | LOW, P1 as filed (P2) | CG-D vs the `blocks`-edge `shipped` rule | Closed: CG-D cites the `pre_claim` gate and 141-S (5.3) |
| B-2 | LOW, P1 as filed (P2) | `k ≤ 7` cap | Closed: k from PA5-P; positions 191-199; re-spacing in the PA5-P staging PR, no HALT (4, 7.6, 13) |
| B-3 | LOW, P1 as filed (P3) | Slot-20 body share applied before RED | Closed: the share is test code only (9.2) |
| B-4 | LOW, P1 as filed (P2) | `tmp/ps5-probe/` deletion outside the approval | Closed: inside the PS-5 probe scope; PS-5c, ActionRisk low (7.4, 7.5, 13) |
| C-2 | LOW, P3 | "at most 680 lines" | Closed: 13 X states the actual count |
| A15-O1 | Orchestrator observation, P3 | Slot-02/Slot-19 "S, 2 h" vs 4.1 (S ≤ 1.5 h) | Not fixed (label only); to the stash |

**Freeze.** The plan is FROZEN at Revision 18 by Orchestrator judgement under OD-7 option A: the only P1 (A-1) was
mechanical and was confirmed against the real tool and the 081-S precedent; no further plan review. **Residual P2/P3 to
the stash** (Stage stashes them in the landing staging PR, not now): attempt-13 P2-2, P2-4, P2-5, P2-6, P2-7, P2-8, P2-9,
P2-10, P2-17 and the attempt-13 P3s; attempt-14 Scope P3-1 and the unaddressed attempt-14 P3 bullets (A20 redundancy,
"Confirm C7", D-slot naming, `dead_code` risks of the settle helper and the `#[path]` include, E2/E8 order-only);
attempt-15 A15-O1. Already in the stash: `7BF90213`, `5AF5CD66`, `23E287C6`, `F99C705E`.

### Attempt 14: FAIL

Target: Revision 15 (587 lines), 2026-09-30. Five read-only personas; condensed in
`docs/scratch/2026-09-30-142-f-plan-review-attempt-14-findings.md`.

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | FAIL | 0 | 2 | 5 | 4 |
| Scope Boundary Auditor | FAIL | 0 | 1 | 5 | 5 |
| Architecture Strategist | FAIL | 0 | 3 | 5 | 3 |
| Constitution Reviewer | FAIL | 0 | 6 | 7 | 3 |
| Learnings Researcher | FAIL | 0 | 1 | 6 | 3 |

The eleven deduplicated P1s and where they are closed: (1) `shipped` unattainable → 6; (2) operator misquoted → 1.1; (3)
PS-5 mechanism and risk → 7.4, 7.5; (4) no path to `main` → 7.1, 7.2; (5) no rollup guard → 10.3; (6) F51/F52 test-first →
OD-3 B, 9.2, 13; (7) sizing halt and false claim → SG-1, 4.2; (8) order and hold in prose only → 5.2, 5.4; (9)
`142.074-T` documented a held change → 9.2 (note on `142.059-T`); (10) PRE-4 `assert_eq!` → 9.2 `142.066-T`; (11)
`match_same_arms` → 9.2 `142.072-T`. P2s addressed: R12 ownership and freeze (X2); CG-T (X3); D-NOTE (X4, U1); PS-7/PS-8
(X5); E10; E11; `142.069-T` line 36; PA-5 edges and the zero-task rule (5.1); the stale-text check; the parked commit map
(7.3); the `Cli` debug_assert location; the FL compile check; CG-S once in total; risky actions, owners and approvals
(7.4, 13); principles I-XI (13); the safe-close sync as a CG-S trigger (6); P-018 (CG-M); oscillation (17).

**Revision 16** (2026-09-30): attempt-14 fixes; one task per shipment; no review run. **Revision 17** (2026-10-04):
operator answers to OD-1 to OD-7 and the sizing requirement applied (H0, C1, SG-1, SB-1, four split families, PA-6 lift,
PA5-P); reviewed by attempt 15 (FAIL). **Revision 18** (2026-10-04): narrow attempt-15 fixes; frozen (17); no further
review.

<!-- plan-review-attempt: 14 -->
<!-- plan-review-attempt: 15 -->
