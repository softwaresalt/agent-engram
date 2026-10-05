---
title: "142-F Revision 18 execution rulings (R-A1 to R-A5)"
description: "Operator rulings on five P1 execution defects found in the frozen 142-F decomposition plan after PR #410; they replace the named plan steps at H3 and assembly without a Revision 19"
status: accepted
date: 2026-10-04
decided_by: operator
decided_at: "2026-10-04T23:16-07:00"
applies_to: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md (Revision 18, status frozen-rev18)"
supersedes_steps:
  - "7.4 PS-5 row (action, approval condition)"
  - "7.4 PS-6 row (targets)"
  - "7.5 PS-5 mechanism: the 'PS-5 uses no move' sentence and steps 2, 3 and 4 (commands)"
  - "16 risk-surface wording for PS-5 (block, then archive)"
  - "5.1 E1 row (removed edges)"
  - "4.3 common rules: the predecessor-copy rule and the acceptance-criterion map (S-04 only)"
  - "8 steps 3 (PS-6 targets), 4 (S-04 edge copy), 6 (U1 and U7 checks) and 8 (U7 check point)"
  - "9.1 U1 (exemption) and U7 (check point)"
  - "9.2 142.068-T (CLI command)"
source_evidence: "Orchestrator review of the 35 PR #410 files Copilot skipped, section A and the A1 scratch probe of 2026-10-04 21:47-21:54 -07:00 (session ea5dda5b)"
related_stash: "EEF658F3 (resolved by R-A2)"
related_pr: "#410 (merge c59499736e93e7f34399851f8cd0680414de7664)"
tags:
  - "142-F"
  - "142-S"
  - "execution-ruling"
---

## Context

PR #410 landed the 142-F planning work (landing step H1). The decomposition plan is frozen at Revision 18, and its
section 17 routes any new P0 or P1 found during execution to the operator instead of a Revision 19. The Orchestrator
found five P1 defects (A1 to A5) that would halt H3 or assembly. The operator approved all five recommended rulings on
2026-10-04 at 23:16 -07:00. This record states them so the executors can apply them; the plan file itself stays
unchanged.

## How executors use this

* Ship at H3 and Stage at assembly (ASM) MUST read this record together with Revision 18 before their first step.
* Where this record and Revision 18 conflict, this record wins for the steps named in `supersedes_steps` only. Every
  other step, HALT token, approval and guard in Revision 18 applies unchanged.
* Each executor cites the ruling ID (R-A1 to R-A5) in its run record wherever it applies one.
* A new defect outside these five still goes to the operator under section 17. Neither executor extends a ruling by
  analogy.

## R-A1: abandon 142-S without `shipment block` (H3, PS-5)

**Defect.** Section 7.5 steps 2(a)-(b) and 4, the PS-5 row of 7.4 and section 16 abandon 142-S by `backlogit shipment
block`, then `backlogit archive`, on the strength of the 081-S precedent. The precedent does not prove it: `081-S.jsonl`
holds only a Ship comment "active -> blocked" and no status event, and its `archived_status: abandoned` was set out of
band. The 2026-10-04 scratch probe on a copy of `.backlogit/` (`tmp/ps5-probe/`, backlogit 1.11.0) showed:

| Probe step | Result |
|---|---|
| `shipment block 142-S` | `blocked`; members and `142-F` unchanged |
| `archive 142-S` while blocked | Refused: "archive blocked shipment aggregate outside its lifecycle operation: shipment status conflict" |
| `move 142-S --status abandoned` while blocked | Refused: "must be blocked/unblocked via the governed BlockShipment/UnblockShipment seam" |
| `shipment unblock 142-S --to active --confirm --by ship`, then `move 142-S --status abandoned`, then `archive 142-S` | `archive/142-S.md`: `status: archived`, `archived_status: abandoned`, `archived_from` set; `142-F` `active`; members unchanged |

The probe log records the abandon `move` as an `artifact_mutation` by actor `backlogit`, so the move carries no Ship
attribution of its own. Revision 18's sequence would halt H3 with `R16-PS5-UNPROVEN` after the W2 commit.

**Ruling.** Ship skips `shipment block`. Starting from 142-S in its current `active` state, Ship runs, in careful mode
on the H3 branch, after 7.5 step 1 (snapshot):

1. `backlogit comment add 142-S --actor ship --comment "<split reason: 142-S abandoned and its scope split into 142-F one-task slots per the 2026-09-27 split decision and OD-4 (2026-10-04 12:32); executed under R-A1>"`.
   The comment supplies the attribution the `move` event lacks.
2. `backlogit move 142-S --status abandoned`.
3. `backlogit archive 142-S`.

**Success** means `.backlogit/archive/142-S.md` has `archived_status: abandoned` and `archived_from` set, and the status
and path of `142-F` and of every 142-S member are unchanged. 7.5 step 4's member re-read (`R15-142S-MEMBER-MOVED`) and
the 10.3 rollup guard on `142-F` run after step 3 as written. Any refusal, or an archive without
`archived_status: abandoned`, is HALT `R16-PS5-UNPROVEN <step> <exit>` to the operator.

**Precondition.** 142-S must be `active` when Ship starts. Any other status is HALT `R16-PS5-UNPROVEN precondition
<status>`. The probe's unblock path proves recovery from `blocked`, but this ruling doesn't authorize it.

**Probe.** The 7.5 step 2 probe is satisfied by the evidence above; Ship doesn't re-run it at H3. The scratch copy
`tmp/ps5-probe/` is already deleted with the operator's approval (PS-5c applied). Probe step (c) (`shipment create`
from a reset former member) moves to Stage under R-A5.

**ProposedAction (replaces the PS-5 row of 7.4; strict-safety ActionRisk per command).**

| ID | Owner | Action and target | change_kind | ActionRisk | approval_required | Approval / ActionResult | Rollback / containment |
|---|---|---|---|---|---|---|---|
| PS-5 (1) | Ship | `comment add 142-S` with the split reason | backlog log append, additive | low | no (inside PS-5) | OD-4 and R-A1; `approved` | none needed; a comment is history |
| PS-5 (2) | Ship | `move 142-S --status abandoned` | backlog state, irreversible in-tool | **destructive** | **yes, verbatim** | OD-4 ("mark 142-S as abandoned and then archive") and the operator's R-A1 approval, 2026-10-04 23:16; `approved`, no longer conditional on a probe | 7.5 step 5 restore path |
| PS-5 (3) | Ship | `archive 142-S` | backlog state, irreversible in-tool | **destructive** | **yes, verbatim** | as PS-5 (2); `approved` | 7.5 step 5 restore path |

The advance approval covers exactly these three commands on 142-S. It doesn't cover `shipment block`, `shipment
unblock`, a hand edit of `archive/142-S.md`, or any other fallback.

## R-A2: E1 also removes `142.064a` → `142.063-T` (assembly steps 4 and 5)

**Defect.** Section 4.3 says the first family task copies the parent's open predecessor edges. At assembly step 4,
`142.064a` therefore copies `142.064-T` → `142.063-T`. E1 (5.1) removes only `142.064-T` → `142.063-T`. With E2
(`142.063-T` → `142.066-T`), the existing `142.066-T` → `142.064-T` and E13 (`142.064-T` → `142.064b` → `142.064a`),
step 5 closes the cycle `142.064a` → `142.063-T` → `142.066-T` → `142.064-T` → `142.064b` → `142.064a` and halts with
`R15-DAG-CYCLE`.

**Ruling.**

* E1 removes both `142.064-T` → `142.063-T` and `142.064a` → `142.063-T`.
* The 4.3 predecessor-copy step skips any edge that 5.1 removes. For S-04 that means `142.064a` never receives the
  `142.063-T` edge. If it was already copied, E1 removes it at step 5.
* The 4 slot-table row for Slot-04a governs: after E5 and E1, `142.064a` depends on `142.062-T` only. Stage confirms it
  with `backlogit_get_dependencies`; any other predecessor is HALT `R15-DAG-ORDER Slot-04a <predecessor>`.

This ruling addresses the dependency-cycle finding in active stash entry `EEF658F3`, filed independently by Copilot.
The stash entry remains active pending Stage's disposition; this record does not archive it. Stage may cite this ruling
in its disposition.

## R-A3: `142.068-T` uses `workspace-status` (assembly step 6, 9.2)

**Defect.** Section 9.2 copies R7.1 M17 into `142.068-T`: "the CLI command `engram --workspace <W> --json status`". The
binary has no `status` subcommand. `src/bin/engram.rs` declares `#[command(name = "workspace-status")]` for
`WorkspaceStatus`, which sends `get_workspace_status`.

**Ruling.** The `142.068-T` rewrite uses `engram --workspace <W> --json workspace-status`. The M17 sentence "`stats` is
the CLI subcommand that sends `get_workspace_statistics`" is correct and stays as written. The 9.2 content check for
`142.068-T` passes on `workspace-status` and fails (`R16-CONTENT-CHECK 142.068-T M17`) on a bare `status`.

## R-A4: assembly content and split checks (steps 4, 6 and 8)

**Defect.** Three checks halt on text the plan itself requires:

* U1 (9.1) bans `F54 settle`, but 9.2 requires the wholesale R9.6 text for `142.063-T`, which states the barrier's
  `F54 settle:` stderr line. Step 6 would raise `R16-STALE-TEXT 142.063-T F54 settle`.
* The 4.3 map rule requires every acceptance criterion and verification item of `142.064-T` and its subtasks to map to
  one family task. The F54 items moved out of the PRE-3 family (R6.5 M1-M6 and R7.1 M14-M15, owned by `142.058-T` since
  R9.5) map to none, so step 4 would raise `R17-SPLIT-AC-UNMAPPED`.
* U7 requires `PA-6-LIFTED`, which step 8 writes, but step 6 checks it.

**Ruling.**

* **U1.** In `142.063-T` only, U1 exempts text that states the required `F54 settle:` stderr contract string (R9.6,
  for example `F54 settle: <NoActivator|Active> after <ms> ms` and `F54 settle: Active`). The R9.6 group wording
  ("first code task of S4", "S4 base") is not exempt: Stage rewrites it in slot terms (Slot-19, the Slot-19 branch base),
  and U1 still checks `142.063-T` for `S1`-`S5`. The exemption doesn't extend to any other task.
* **4.3 map.** For a `142.064-T` or `142.064.00x-ST` item that moved out of the PRE-3 family, the entry "moved to
  `142.058-T` per M-n", naming the R6.5 or R7.1 row that moved it, counts as its one mapping. A moved item mapped to a
  family task as well is double-mapped and still halts.
* **U7.** Step 6 checks the STATUS-RESET half of U7. The `PA-6-LIFTED` half is checked right after step 8 for the seven
  tasks step 8 names. A miss is HALT `R16-CONTENT-CHECK <task> U7`.

## R-A5: PS-6 resets the subtasks; Stage creates shipments (assembly step 3, 7.5)

**Defect.** PS-6 (7.4, assembly step 3) resets `142.054-T` to `142.059-T` from `active` to `queued`, but their open
subtasks were activated with them in 142-S and stay `active`. Section 7.5 step 2(c) has Ship run `backlogit shipment
create`, which is outside Ship's role boundary (P-010).

**Ruling.**

* PS-6 also resets `142.054.001-ST`, `142.054.002-ST`, `142.054.003-ST`, `142.058.002-ST` and `142.058.003-ST` from
  `active` to `queued` with their parent tasks. `142.058.001-ST` is archived `done` and is untouched. Stage resets each
  family's subtasks first, then the parent; the 10.3 rollup guard runs after each write, and a parent that rolls up to
  `queued` on its own counts as its intended reset. PS-6 keeps ActionRisk moderate, recorded approval (2026-09-29 18:14,
  extended by the operator's R-A5 approval, 2026-10-04 23:16).
* Any step that creates a shipment is run by Stage, never Ship. Probe step (c) of 7.5 is no longer run at H3 (R-A1); the
  shipment-from-a-former-member proof is assembly step 7, where Stage creates the slot shipments for the reset tasks and
  a refusal is HALT `R15-SHIPMENT-SHAPE <slot> create`.

## Approval record

| When (-07:00) | Who | Decision |
|---|---|---|
| 2026-10-04 23:16 | Operator | Approved the Orchestrator's five recommended rulings for findings A1-A5 (recorded here as R-A1 to R-A5) |
