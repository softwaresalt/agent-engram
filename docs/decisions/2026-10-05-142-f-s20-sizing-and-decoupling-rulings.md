---
title: "142-F Revision 18 execution rulings R-A6 to R-A8 (S-20 sizing and decoupling, 142.066-T CLI name)"
description: "Operator rulings on HALT R17-SIZE-GATE 142.058a FL-share (R-A6, R-A7: count only authored or adapted harness lines against the 400-line bounds, and move the S-20 split from assembly to PA5-P), plus an Orchestrator-recorded R-A8 pending operator confirmation"
status: mixed
date: 2026-10-05
decided_by: mixed
rulings:
  R-A6:
    status: accepted
    decided_by: operator
    decided_at: "2026-10-05T22:58-07:00"
  R-A7:
    status: accepted
    decided_by: operator
    decided_at: "2026-10-05T22:58-07:00"
  R-A8:
    status: pending-operator-confirmation
    decided_by: orchestrator
    decided_at: "2026-10-06T00:45-07:00"
    confirmation_point: "operator review of the 142-F assembly PR (#413); the operator may reverse it there"
applies_to: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md (Revision 18, status frozen-rev18)"
companion_record: "docs/decisions/2026-10-04-142-f-rev18-execution-rulings.md (R-A1 to R-A5, still in force)"
supersedes_steps:
  - "4.2 rows 20a-20d (the 'FL share ≤ 400 lines' condition)"
  - "4.3 S-20 last sentence (share measurement and re-split)"
  - "4.4 SB-1 (c) for the 142.058 family"
  - "5.1 E3 and the 142.058 chain of E15 (moved from assembly step 5 to PA5-P)"
  - "7.6 PA5-P (gains the S-20 split, E3, E15 and the 142.058 family content)"
  - "8 steps 4, 5 and 6 (142.058a-c, E3, E15 and the 142.058 family content leave assembly)"
  - "9.2 142.058a-c, 142.058-T (written at PA5-P)"
  - "9.2 142.066-T (CLI command name, R-A8)"
source_evidence: "docs/memory/2026-10-05-stage-142-f-asm-memory.md, step 4 HALT measurement of git show 6d216d19:tests/contract/read_server_cli_mcp_parity_test.rs (1,633 lines; about 1,145 lines of shared harness core)"
related_halt: "R17-SIZE-GATE 142.058a FL-share"
tags:
  - "142-F"
  - "execution-ruling"
---

## Context

Stage ran 142-F assembly (plan section 8) on branch `chore/stage-142-f-assembly` and halted at step 4 with
`R17-SIZE-GATE 142.058a FL-share`. Every daemon case in the parked F54 parity harness (`6d216d19`, 1,633 lines) runs
through about 1,145 lines of shared harness core. The core can't be split by case class, and landing it early without
callers trips `dead_code` under `-D warnings` with no allowed escape (R11.10 L1, A15). So no S-20 share can meet the
400-line bound as written. Stage offered options A, B and C. The operator chose A and C together on 2026-10-05 at
22:58 -07:00.

## How executors use this

* Stage at the resumed assembly, Stage at PA5-P and Ship at Slot-20a to Slot-20d MUST read this record together with
  Revision 18 and R-A1 to R-A5.
* Where this record and Revision 18 conflict, this record wins for the steps named in `supersedes_steps` only. Every
  other step, HALT token, approval and guard in Revision 18 and R-A1 to R-A5 applies unchanged.

## R-A6: the 400-line bounds count authored and adapted lines only (option A)

**Defect.** The 400-line bounds in 4.2 (rows 20a-20d), 4.3 S-20 and SB-1 (c) count every line a 142.058 family slot
lands. The verbatim load from `6d216d19` is already-reviewed test code, but the first daemon-case slot must carry the
whole shared core, so the bound can't be met.

**Ruling.**

* For the 142.058 family only (`142.058a`, `142.058b`, `142.058c`, `142.058-T`), the 400-line bounds count only the
  lines the harness-architect authors or adapts. An adapted line is any line that differs from the `6d216d19` blob,
  including the compile-only adaptations (paths, imports) that 9.2 already requires.
* The verbatim load is measured and recorded, not capped. Each slot's harness record lists the `6d216d19` line ranges it
  loads, their line count, and every adaptation. It also includes the diff of the landed file against the matching
  `6d216d19` regions, which shows that only the listed adaptations differ. An unlisted difference counts as authored.
* 9.2 still applies: the load is byte-identical except the listed adaptations, and `cargo check --tests` proves them
  before the RED run.
* The S-20 share measurement (4.3, last sentence) uses the authored and adapted count. A share over 400 lines is still
  split again by case class before creation. SB-1 (c) uses the same count for these four slots. A slot whose authored or
  adapted lines pass 400 still raises `R17-SLOT-BUDGET <slot> c`.
* This ruling doesn't change the bound for any other task. `142.063-T`'s HC keeps its 2-hour, 400-line cap as written.

## R-A7: the S-20 split moves from assembly to PA5-P (option C)

**Defect.** Section 8 steps 4 to 6 create and write the 142.058 family at assembly. Nothing in Slot-01 to Slot-19
depends on that family, and the PA5-P staging PR already creates Slot-20a to Slot-20d. Keeping S-20 at assembly ties
the Slot-01 to Slot-19 landing to the 058 sizing work.

**Ruling.**

* **Assembly step 4** creates `142.061a`, `142.064a`, `142.064b`, `142.054a` and `142.054b` only, and rewrites only
  `142.061-T`, `142.064-T` and `142.054-T` to their shares. It doesn't create `142.058a` to `c` and doesn't split
  `142.058-T`. `142.058-T` stays `queued` with its subtasks `142.058.002-ST` and `142.058.003-ST` as they are.
* **Assembly step 5** skips E3 and the 142.058 chain of E15 (`058a` → 066 and 065, `058b` → `058a`, `058c` → `058b`,
  `058-T` → `058c`). E11, including its removal of `142.060-T` → `142.058-T`, still runs. The other edges run as
  planned, and the step 5 cycle check covers the resulting graph.
* **Assembly step 6** writes only U1 and the U7 STATUS-RESET line for `142.058-T`, plus one implementation-notes line:
  "S-20 split, Slot-20 content and E3/E15 deferred to PA5-P (R-A7, 2026-10-05)". Step 6 checks `142.058-T` for U1 and
  the U7 STATUS-RESET half only. U2 to U6, U8, U9 and the 9.2 S-20 content are written and checked at PA5-P.
  `142.059-T` gets its full section 9 content at assembly. Its edge to `142.058-T` is unchanged.
* **Assembly step 8** still appends `PA-6-LIFTED` to `142.058-T`, and the R-A4 U7 check after step 8 still covers it.
* **PA5-P (7.6)** adds these to its staging PR: the S-20 split per 4.3 under R-A6, measured from `6d216d19`; the SG-1
  and acceptance-map checks for the family, with the same HALT tokens; E3 and the 142.058 chain of E15 with the 5.1
  checks; the 9.2 content for `142.058a` to `c` and `142.058-T`, plus U2 to U9 for all four; and the Slot-20a to
  Slot-20d shipments it already creates. The E6 edges (`142.058a` → each PA-5 sink) follow the split.
* No 142.058 family task can be claimed before the PA5-P PR merges, because no Slot-20 shipment exists until then.
  Moving the split later therefore costs no schedule.

## R-A8: R-A3 also applies to `142.066-T` (assembly step 6)

**Defect.** Stage found this during step 6, batch 2. `142.066-T` scenarios 2 and 3 name a CLI command `status`:
"the harness sends only `stats`, `status`, and `_health`", and "`daemon-status` and `status` are accepted". Next to
`stats` and `daemon-status`, these are CLI subcommand names. The binary has no `status` subcommand
(`src/bin/engram.rs` declares `workspace-status` for `WorkspaceStatus`). This is the R-A3 defect in a second task.

**Ruling.** In the `142.066-T` rewrite, the CLI name `status` becomes `workspace-status` in scenarios 2 and 3.
Prose that means the `get_workspace_status` method, such as "status call 1" in the R7.7 rule, keeps its wording.
The 9.2 content check for `142.066-T` fails (`R16-CONTENT-CHECK 142.066-T R-A8`) if a bare CLI `status` remains.

This applies an operator-approved ruling (R-A3) to a second instance of the same defect. The Orchestrator recorded it
to keep assembly moving. The operator can reverse it at the assembly PR review.

## Approval record

| When (-07:00) | Who | Decision |
|---|---|---|
| 2026-10-05 22:58 | Operator | Chose options A and C for HALT `R17-SIZE-GATE 142.058a FL-share` (recorded here as R-A6 and R-A7) |
| 2026-10-06 00:45 | Orchestrator | Recorded R-A8 under the R-A3 precedent; the operator can reverse it at PR review |
