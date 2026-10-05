---
title: "142-F decomposition plan, Revision 17: attempt 15 multi-model adversarial review"
date: 2026-10-04
target: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md (Revision 17, 713 lines)"
mode: read-only plan review
reviewers: 3 (Tier 1 gemini-3.7-flash, Tier 2 gpt-5.5, Tier 3 claude-opus-4.8), dispatched in parallel, each independent
verdict: FAIL
---

## Summary

| Confidence | P0 | P1 | P2 | P3 |
|---|---|---|---|---|
| HIGH (3/3) | 0 | 0 | 0 | 0 |
| MEDIUM (2/3) | 0 | 1 | 0 | 0 |
| LOW (1/3) | 0 | 4 | 0 | 1 |

Severity mapping: CRITICAL = P0, MAJOR = P1, MINOR = P2/P3. The table keeps the reviewers' severities. The orchestrator's
notes on the LOW P1s appear under each finding and do not change the confidence tiers.

**Verdict: FAIL.** One P1 at MEDIUM confidence (A-1). It is narrow and cheap to fix, and it sits in PS-5 (7.5). No
HIGH-confidence finding was raised. All 11 attempt-14 P1 closures mapped in section 18 held up: no reviewer reopened one.

**Evidence limitation.** None of the reviewers could run a shell in this session (a separate command-runner agent also
reported no execution interface). So no reviewer ran `backlogit --help`, `backlogit shipment --help` or the git
verification commands. Tool-existence claims were checked only against files: `.autoharness/backlog-registry.yaml`,
`.backlogit/archive/141-S.md`, `.backlogit/archive/081-S.md`, `.backlogit/logs/081-S.jsonl` and
`.github/instructions/backlogit.instructions.md`.

## 1. Consensus findings (HIGH): none

## 2. Majority findings (MEDIUM)

### A-1 [P1, MEDIUM, Reviewers A + C] PS-5 step (b) `backlogit move 142-S --status abandoned` is unproven, contradicts the plan, and predictably HALTs H3

* **Plan:** 7.5 preamble (line ~412): "no abandon operation exists". Probe step 2(b) (line ~418): `backlogit move 142-S
  --status abandoned`. Step 3 (lines ~420-423): any refusal is HALT `R16-PS5-UNPROVEN`, and "the advance approval does not
  cover an improvised fallback or any other command". The assembly precondition "PS-5 `applied`" (line ~443) makes the whole
  landing and assembly sequence depend on this probe.
* **Evidence:**
  * `.backlogit/archive/141-S.md` (description, "Established tool rejection"): a direct shipment status `move` is refused
    with `shipment must be shipped via ShipShipment, not a direct status update` (exit 9). The plan restates this at line
    ~309.
  * The cited precedent `081-S` reached `archived_status: abandoned` through active → blocked → archive, with no `move
    --status abandoned` step (`.backlogit/logs/081-S.jsonl` lines 2-4; `.backlogit/archive/081-S.md` front matter).
  * `.autoharness/backlog-registry.yaml` `move_task` is generic, and nothing shows it accepts `abandoned` for a shipment.
  * If `move` to `abandoned` worked, it would be the "abandon operation" that line ~412 says does not exist.
* **Impact:** If the tool behaves as these records suggest, the mandatory probe fails at (b) and H3 HALTs before
  assembly. The operator then has to decide something mid-sequence that could be settled now. The probe and the HALT keep
  this safe (nothing is mutated on `main`), but it is a predictable block on the landing sequence.
* **Reviewer A's extra claims, not verified:** Reviewer A also said `shipment block` and `backlogit archive` do not exist,
  because the registry does not list them. The registry is not a complete listing (it also has no archive operation, yet
  081-S was archived by `actor: backlogit`). Plan line ~107 lists `shipment block` for 1.11.0, and the 081-S comment records
  an active → blocked transition. These claims are treated as unverified and are not counted in A-1. Check them with
  `backlogit shipment --help` before freeze.
* **Fix:** Make the disposition match the 081-S precedent: (a) `shipment block` → (c) archive, with the success criterion
  `archived_status: abandoned`. Either drop (b), or mark it "attempt; a refusal of (b) alone is not a HALT if (a) → (c)
  yields `archived_status: abandoned`". Record that this path still carries out OD-4 ("mark 142-S as abandoned and then
  archive"), so the advance approval covers it. Reconcile line ~412. Before freeze, confirm (a) and (c) exist with
  read-only `backlogit shipment --help` and `backlogit --help`.
* **Action class:** `gated_auto` (Stage plan edit, then operator confirmation that the approval covers the two-step path).

## 3. Unique findings (LOW)

### B-1 [P1 as filed, LOW, Reviewer B] Shipment `blocks` edges vs the manual safe-close `archived_status: done`

* **Plan:** 5.2 (lines ~290-303) mirrors every task edge as a shipment `blocks` edge. CG-D (lines ~305-309) counts a
  predecessor as finished under the section 6 definition (archived `done`).
* **Evidence:** `.github/instructions/backlogit.instructions.md` lines ~64-66 and ~98-101 say a successor is eligible only
  when every `blocks` predecessor has `status == "shipped"`, and section 6 forbids `shipment ship`.
* **Orchestrator note:** Mitigated by precedent. `.backlogit/archive/141-S.md` ("Successor eligibility clarification")
  records that `autoharness gate pipeline-topology --phase pre_claim` treats `status: archived` as satisfied, and that
  142-S was claimed with `done`-archived predecessors. Plan line ~303 cites this, and the prose mismatch is already stashed
  (`3A963D34`). Treat as P2: in 5.3, name the pre_claim gate as the check that CG-D relies on.
* **Fix:** In CG-D, cite `autoharness gate pipeline-topology --phase pre_claim` and the 141-S clarification as the rule
  that an archived predecessor satisfies a `blocks` edge.

### B-2 [P1 as filed, LOW, Reviewer B] PA-5 cap `k ≤ 7` vs the PA5-T4 split

* **Plan:** Slot table row 19.k (line ~180) and the slot count (line ~187) both cap at 7. Sizing row 19.k (line ~215)
  and 7.6 (lines ~432-433) require PA5-T4 to be split.
* **Orchestrator note:** With PA5-T7 moved to Slot-20c and the F54 half of PA5-T6 moved to Slot-20d (R-2), T1-T6 plus a
  two-way T4 split gives exactly 7, so the cap holds for a two-way split. A three-way split, or a later SG-1 split, has no
  defined slot. Queue positions 191-199 still fit below Slot-20a (200). Treat as P2.
* **Fix:** Replace `k ≤ 7` with "k assigned at PA5-P; k ≤ 9 (queue_position 191-199); more needs a renumber and a new
  review", and make the 36-slot total "36 or more".

### B-3 [P1 as filed, LOW, Reviewer B] Slot-20 applies the parked body share before the RED run

* **Plan:** 9.2, `142.058a`-`c`/`142.058-T` (lines ~541-543): "FL takes each slot's share of the `6d216d19` body
  byte-identical except compile-only adaptations ... proven by `cargo check --tests` before the RED run".
* **Evidence:** `.github/agents/_ship.agent.md` ~311 (Step 2 RED). Constitution II (lines 19-22).
* **Orchestrator note:** Probably a false positive. The parked `6d216d19` body is F54 test-file content (the F54 file is a
  test target; "F54a harness core (FL)"), not production code. Compiling test code before observing RED is ordinary test
  first, and `FL-RED-HALT` guards against a share that passes on arrival. Treat as P3: state in 9.2 that the body share is
  test code only.
* **Fix:** Add "the `6d216d19` share is test code; no production code is applied before the RED run".

### B-4 [P1 as filed, LOW, Reviewer B] Deleting `tmp/ps5-probe/` is outside the PS-5 advance approval

* **Plan:** 7.5 step 2 (line ~418): "Then delete `tmp/ps5-probe/`". Step 3 says the approval covers exactly (a)-(c) and "not
  ... any other command".
* **Evidence:** Constitution VII (lines ~80-86): deleting files or directories is destructive and needs operator approval.
  The constitution check (13, VII) does not list this deletion.
* **Orchestrator note:** Valid internal inconsistency with low practical risk (a gitignored scratch copy that Ship
  created). Treat as P2.
* **Fix:** Keep `tmp/ps5-probe/` until a separately approved cleanup, or add the deletion to PS-5's ProposedAction and to
  the 13/VII risky-action list.

### C-2 [P3, LOW, Reviewer C] Constitution check X states "at most 680 lines"; the file is 713 lines

* **Plan:** section 13 X (line ~620).
* **Fix:** Correct the bound to the actual length, or trim the plan.

### Orchestrator observation (not a reviewer finding, not counted)

* 4.2: Slot-02 (`142.061-T`) and Slot-19 (`142.063-T`) are labeled "S, 2 h", but 4.1 defines S ≤ 1.5 h and M ≤ 2 h.
  This is a label mismatch only: both fit the 2 h ceiling (Slot-02 is split anyway, and Slot-19 keeps the HC cap). P3.

## 4. Remediation plan (priority = confidence × severity)

| # | Finding | Score | Action class | Owner |
|---|---|---|---|---|
| 1 | A-1 PS-5 (b) unproven; align with the 081-S block → archive path; confirm the commands with read-only `--help` | 2 × 3 = 6 | `gated_auto` | Stage (plan) + operator (approval scope) |
| 2 | B-1 CG-D: cite the pre_claim gate and the archived-equals-satisfied precedent | 1 × 3 = 3 | `manual` (orchestrator: P2) | Stage |
| 3 | B-2 PA-5 `k` cap | 1 × 3 = 3 | `manual` (orchestrator: P2) | Stage |
| 4 | B-3 Slot-20 body share is test code only | 1 × 3 = 3 | `advisory` (orchestrator: likely false positive) | Stage |
| 5 | B-4 Probe directory deletion approval | 1 × 3 = 3 | `manual` (orchestrator: P2) | Stage |
| 6 | C-2 Line-count claim | 1 × 2 = 2 | `advisory` | Stage |

Remediation 1 is a local edit to 7.5 (plus line ~412 and, optionally, 13/VII). Remediations 2-6 can go in the same edit
or, per OD-7, go to the stash after the operator confirms.

## 5. Backlog entries (P1 findings; create only after operator confirmation)

```yaml
- type: bug
  title: "PS-5: probe step (b) move --status abandoned unproven; align with 081-S block→archive"
  description: "7.5 step 2(b) uses `backlogit move 142-S --status abandoned`, contradicting 7.5's 'no abandon operation exists' and the 141-S exit-9 direct-status-update refusal; 081-S reached abandoned via block→archive. A refusal halts H3 (R16-PS5-UNPROVEN) and blocks assembly."
  file: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  line: 418
  severity: "MAJOR"
  confidence: "MEDIUM"
  fix: "Drop (b) or make it non-halting when (a)→(c) yields archived_status: abandoned; record OD-4 coverage; verify commands via read-only --help."
  linked_review: "docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md"
- type: bug
  title: "CG-D: archived-done predecessors vs blocks-edge 'shipped' rule"
  description: "Shipment blocks edges are gated on shipped per backlogit.instructions; slots safe-close to archived done. Mitigated by the 141-S pre_claim precedent."
  file: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  line: 305
  severity: "MAJOR"
  confidence: "LOW"
  fix: "Cite the autoharness pre_claim gate and the 141-S clarification in CG-D."
  linked_review: "docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md"
- type: bug
  title: "Slot-19.k: k ≤ 7 cap has no rule for further PA-5 splits"
  description: "PA5-T4 split plus possible further SG-1 splits can exceed 7 slots with no defined queue_position/edges."
  file: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  line: 180
  severity: "MAJOR"
  confidence: "LOW"
  fix: "Assign k at PA5-P with bound 9 (191-199); beyond requires renumber and review."
  linked_review: "docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md"
- type: bug
  title: "Slot-20: state that the 6d216d19 share applied before RED is test code only"
  description: "Reviewer read the parked body share as implementation before RED; likely test-only, but the plan does not say so."
  file: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  line: 541
  severity: "MAJOR"
  confidence: "LOW"
  fix: "Add: no production code is applied before the RED run."
  linked_review: "docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md"
- type: bug
  title: "PS-5: tmp/ps5-probe deletion outside the advance approval"
  description: "Directory deletion is destructive (Constitution VII) but not in the (a)-(c) approval or 13/VII list."
  file: "docs/exec-plans/2026-09-30-142-f-decomposition-plan.md"
  line: 418
  severity: "MAJOR"
  confidence: "LOW"
  fix: "Retain the directory or add the deletion to the PS-5 ProposedAction."
  linked_review: "docs/scratch/2026-10-04-142-f-plan-review-attempt-15-adversarial.md"
```

## 6. Raw reviewer outputs (condensed)

* **Reviewer A (Tier 1, gemini-3.7-flash):** 1 MAJOR. PS-5 commands (block, move abandoned, archive) are not executable;
  the `move` part is merged into A-1, and the other claims are unverified.
* **Reviewer B (Tier 2, gpt-5.5):** 4 MAJOR (B-1 to B-4).
* **Reviewer C (Tier 3, claude-opus-4.8):** 1 MAJOR (A-1, step (b)), 1 MINOR (C-2). Reported having no shell.

**Final verdict: FAIL** (one MEDIUM-confidence P1: A-1).
