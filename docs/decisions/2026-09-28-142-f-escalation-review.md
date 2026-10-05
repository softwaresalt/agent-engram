---
title: "142-F launcher preflight: P-013.6 escalation review"
date: 2026-09-28
type: escalation-review
plan: docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md
escalation_route: gpt-6-sol/openai/xhigh
trigger: 3 consecutive plan-review FAILs (attempts 5-7)
verdict: RESTRUCTURE
---

## Executive summary

* **P1-F is a real blocker in Revision 8:** Ship cannot call the IC suite
  `EXPECTED_PENDING_RED` while the later task's owned RED file has changed.
* Re-recording a file hash at HC and IC is not, by itself, a new, valid
  `142.058-T` RED harness or proof that its implementation has not begun.
* The operator **did grant** the narrow invariant-6 edit exception. Only the
  PA-1 admission to the old `142-S` was withheld; no Step 4.3 waiver was granted.
* Three reviews found different symptoms of the same coupling: two tasks
  mutate one test crate while four of its tests must stay RED across a gate.
* **Restructure, do not patch the baseline:** give PRE-3F its own test
  target and support; keep all changes to the F54 test file with `142.058-T`.
* Keep F54's real test body off S4 until its owner starts after PA-5.
  PA-6 continues to hold S3 and therefore S4.
* Obtain the operator's PA-7 decision below before Stage writes Revision 9;
  no edge, shipment, backlog, or git mutation follows from this review.

## Evidence key and gate verdict

`P` means
`docs/exec-plans/2026-09-26-142-f-launcher-preflight-command-plan.md`;
`S` means `.github/agents/_ship.agent.md`; `Q58` and `Q63` mean the
corresponding `.backlogit/queue/142.058-T.md` and `142.063-T.md`.
The citations below use **file:line** ranges in the current working tree.

At `S:440-461`, Step 4.3 permits `EXPECTED_PENDING_RED` only if **all**
six checks pass. Condition 3 demands the exact failed names and observed
markers in a *previously recorded, compiling* RED harness for a
**different, later** task, with its owned-test-file baseline. Condition 4
requires that task's build-feature implementation not to have begun and
**every** Owned file still to equal its recorded pre-implementation
content/type baseline. An unmet or unreadable check blocks; the final
task/PR cannot use pending RED (`S:452-461`).

At IC, rows 2-5 remain RED and are assigned to later `142.058-T`
(`P:1126-1159`); `142.063-T` owns only row 6, which becomes GREEN.
Yet HC replaces the F54 placeholder and IC changes the fixture and
barrier in that same `142.058-T` Owned file (`P:1018-1055,698-751`;
`Q58:65-73`; `Q63:26-39`). The prior F54 harness was recorded at
`7bd9e504` and extended by completed subtask `.001-ST` at `6d216d19`
(`docs/memory/2026-09-26-ship-142-s-f54-red-f50-gate-blocked.md:30-47`;
`.backlogit/archive/142.058.001-ST.md:1-13`). Its old file baseline
cannot equal the HC/IC blob. Thus **P1-F blocks this plan as written**
(`P:4345-4376`), regardless of whether the new row has one owner.

Stage's proposed automatic re-recording (`P:4365-4376`) is **not enough**:

* HC rows 2-5 fail at PRE-3F's stub, with a new `F54-BLOCK: F54-RED:`
  marker (`P:1130-1142,1179-1197`), not because the later F54 work is
  missing. Labeling that HC snapshot the later task's RED harness does
  not establish condition 3. HC is a RED-phase record, not a Step 4.3
  completion verdict for `142.063-T` (`S:440-442`; `P:698-751`).
* At IC, replacing a hash *after* another task edits the file proves
  only equality to the replacement. To count as a new baseline, Ship
  would have to establish an independently observed, compiling F54 RED
  harness at the IC content, record each exact marker **before** the IC
  Step 4.3 verdict, retain and explain the previous records, prove that
  `142.058-T`'s build-feature implementation has not begun, and show
  that its pending RED assertions were not edited to achieve the verdict
  (`S:452-461`). A repeated "re-record after any edit" rule does none
  of this and operates as the waiver Step 4.3 forbids.
* `.001-ST` is a **done subtask**, not automatically proof that the
  parent task's build-feature phase has begun. But it is completed
  implementation of this very test file. The parent remains `active`
  and `harness-ready` (`Q58:23-29`; archive `.001-ST:1-13`).
  Neither "`active`" nor a new definition of "unstarted" resolves that
  provenance question; Ship must verify the actual task/build history
  under condition 4 (`S:455-459`). Do not deem it unstarted by fiat.

This does not assert that a *genuinely new*, frozen, pre-`142.058-T`
implementation RED record could never meet the literal conditions.
It says that Revision 8 and Stage's HC/IC rolling-baseline proposal
neither establish such a record nor authorize editing a pending task's
RED tests to manufacture one (`S:452-461`).

## PA-1 and the ownership exception

**Yes, the narrow historical permission survived; replace its use, not
its history.** The primary approval granted the invariant-6 edit
exception separately and modified **only** PA-1's admission of every
harvested unit to active `142-S`
(`docs/decisions/2026-09-26-142-f-launcher-preflight-command-deliberation.md:606-633`;
`P:3402-3414`). Revision 6 expressly preserves it for S4
(`P:390-396`). Revision 7 withdraws **R6.4's test-first exception**,
not PA-1's file permission (`P:637-643`); T1 and T10 already say the
2026-09-27 permission stands (`P:676-687`). The older PRE-3F sentence
"without PA-1 this task must not start" and current queued task text
are stale in that respect (`P:1746-1753`; `Q63:26-39`).

The permission is limited to the PRE-3F diff contract. It does **not**
alter the Step 4.3 baseline or allow a pending F54 RED test to be
suppressed. Record the following **PA-7**, if the operator agrees:

> I confirm that my 2026-09-27 PA-1 approval retained the invariant-6
> PRE-3F file-edit permission; I withheld only admission of the
> harvested units to the old 142-S. I grant no waiver of Ship Step 4.3,
> P-002, or P-004. For the four-shipment plan, I authorize Stage to
> propose moving PRE-3F's positive test and activation-settle helper
> into a separate test target and support owned by 142.063-T, with
> necessary Cargo registration. The F54 test file, including its
> barrier call, identity change, and real-snapshot fixture change, is
> edited only by 142.058-T after PA-5; the historical cross-task edit
> permission is not to be exercised. Do not mutate items, edges,
> shipments, files, or git until the revised plan passes review and
> the normal owning workflow authorizes execution.
> PA-6 remains Hold S3.

## Why the reviews did not converge

| Review | Newly exposed P1 | Common mechanism |
|---|---|---|
| Attempt 5 | S1/S2 still require F54 evidence; PRE-3F lacks a valid RED-before-`harness-ready` gate (`P:3840-3886`) | Splitting the release moved the F54 file into S4 without moving all verification and RED-phase obligations |
| Attempt 6 | HC stub fails strict compilation; `unknown_ipc` is assigned to both PRE-3F and F54 (`P:4051-4103`) | One shared test crate and helper changes produce failures outside the positive test |
| Attempt 7 | Owners are unique, but `142.063-T` changes the later `142.058-T` Owned-file baseline (`P:4345-4376`) | Per-test ownership cannot satisfy a per-**file**, per-task Step 4.3 invariant |

R6.2 deliberately carries the F54 RED harness only in S4
(`P:331-367`), but R7.3 and R8.3 still put its HC/IC work in a
different earlier task from four RED cases (`P:698-751,1126-1159`).
That is the structural flaw. Fixing marker text, table labels, or
hashes in isolation moves the contradiction to the next gate.
The additional PA-5 fixture change would also touch F54's file unless
ownership is made explicit (`P:545-577`).

## Alternatives and their cost

Here `A → B` means **A depends on B** (`P:370-384,580-602,843-852,880-883`).
All alternatives leave PA-6 **Hold S3**, and thus S4, unchanged
(`P:1221-1253`). None may be executed on this review's authority.

| Approach | Edges and S1-S4 | Operator decision and assessment |
|---|---|---|
| Rolling HC/IC baseline, unchanged task split | E1-E8 and S1-S4 unchanged | **Reject.** A new hash is not the later task's qualifying RED record; repeated recapture weakens Step 4.3 (`S:452-461`). A new PA cannot waive that gate |
| Move **only row 6** into a separate `142.063-T` test file | E1-E8 and S1-S4 could remain | **Insufficient:** the barrier and fixture edits still touch F54's Owned file (`P:1729-1768,1810-1818`). Its fixture is private inside the existing integration-test crate (`tests/contract/read_server_cli_mcp_parity_test.rs:151-160`), with an explicit Cargo target (`Cargo.toml:1275-1277`). Isolation must cover the helper, fixture, and timing of the F54 cherry-pick too |
| **Recommended: fully isolate PRE-3F** | **E1-E8 unchanged** (E5, E1, E2, E3, E4, E8 at assembly; E6/E7 at PA-5 harvest, E4 before E8). S1-S4 membership/order and the S3/S4 PA-6 hold remain (`P:341-367,843-883,1270-1278`). `142.063-T` owns a new positive-test target and self-contained/shared support; `142.058-T` alone edits F54 after PA-5 | Needs PA-7, new explicit Cargo target, a bounded test-support extraction, and a reviewed change to both tasks' Owned files and harness records. F54 stays a GREEN placeholder throughout PRE-3F and PA-5, so their Step 4.3 suites have no F54 pending RED. The later F54 edit includes barrier wiring, identity, and real-snapshot fixture; final `142.058-T` suite must be GREEN |
| Reverse `142.058-T` and `142.063-T` without isolation | Reverse E3/E7 and alter S4 order | **Reject:** F54 runs after PRE-3 with no barrier, reopening L3-1 (`P:370-389,780-797`); it cannot close GREEN until PA-5 and the barrier exist |
| Fold `142.063-T` into `142.058-T` | E1, E4, E5, E6 stay; retire E2/E3/E7 and old `063→062`; redirect E8 to `058→060` after E4. S1-S3 unchanged; S4 becomes PA-5 → F54 → docs → feature | **Not minimal:** `142.058-T` is already `active`/`harness-ready`, and `.001-ST` is done (`Q58:23-29`; archive `.001-ST:1-13`). Adding the new positive test after that label risks a fresh P-002/P-004 failure (`.github/policies/workflow-policies.md:36-103`). It would also combine two tasks' work in one two-hour unit. A replacement task with a newly confirmed RED phase and explicit disposition of existing task history would require a broader operator-approved replan |

The fully isolated route is **not** "only move row 6." PRE-3F
must prove the positive test using its own fixture and a shared
settle helper in a new target; specify an importable test-support API
that F54 can use later without modifying PRE-3F's Owned files.
The F54 fixture is untouched until F54's own task. The real-snapshot
change belongs to that later file owner, not to a PA-5 task
(`P:545-577`). PA-5's production/read
conversions need independent GREEN harnesses, without loading F54's
RED body. The existing F54 placeholder starts no daemon
(`P:1018-1055`). When `142.058-T` replaces it, the barrier call must
arrive in the **same commit** as the real body, before any F54 run
(`P:780-797`). Preserve `.001-ST` as done history.

## Attempt 7 P2 disposition

These are P2s, not independent authorization to expand implementation.
**Put all four dispositions into Revision 9** because this revision
changes the same execution boundary; execute their steps only at the
normal later gates.

| Finding | Revision 9 disposition |
|---|---|
| Positive test may trip `too_many_lines` (`P:4382-4392`) | Allow bounded private positive-test helpers in PRE-3F's new target at HC, unchanged in IC; preserve strict clippy and disallow lint suppression. The current T6 no-helper diff contract otherwise leaves HC no repair path |
| S3 hold lacks queue/dark-mode enforcement (`P:4393-4407`) | Specify Orchestrator skips held S3 and dependent S4; Ship refuses both claims; exclude them from dark scope; require a later PA-6 decision and P-014 approval before S3. Claiming unrelated work remains subject to P-001, not a new exception (`.github/policies/workflow-policies.md:16-32,396-415,497-530`) |
| No hold marker on F52/F53 (`P:4408-4412`) | At assembly, comment on **all** S3 tasks `142.055-T`–`142.057-T`, `142.060-T`, and shipment S3; no individually ready S3 task starts while held (`P:1244-1253`) |
| Local T0 may be the sole reachable copy during an indefinite hold (`P:4413-4420`) | Before H2, require a durable protected copy of T0 and the B0/full-output record: an operator-authorized remote push **or** a verified durable bundle. Record its location and recovery check; no abandonment if either is absent (`P:799-829,869-879,4158-4170`). Resolve later legitimate-main-drift attribution before S4 claim (`P:4141-4150`) |

The remaining Attempt 7 P3 wording and lint-command suggestions
(`P:4422-4480`) are advisory, except where a stale sentence would
contradict the newly authoritative ownership, gate, or PA-6 hold:
correct those references in the same Revision 9.

## Minimum coherent Revision 9 directive

This is a **structural** revision, not the proposed one-paragraph
baseline patch. Stage should make these plan-text changes **only after
PA-7**, then run **one** review scoped to the changed contracts:

1. **R6.2/R6.3 and the dependency graph (`P:341-389,2818-2860`):**
   keep S1-S4, their membership, and PA-6. The F54 parked-branch
   commits move from `142.063-T` HC to `142.058-T`'s own later
   harness/integration; S4 remains PRE-3F → PA-5 → F54 → docs →
   feature. No real F54 test body or fixture edit enters S4 before F54.
2. **R6.8, R7.3-R7.5, R8.1-R8.5, PRE-3F, and F54
   (`P:545-577,698-852,1018-1220,1705-1818`):**
   make `142.063-T` own an explicitly registered new Rust test
   target and minimal shared test support, **not** the F54 file.
   Pin the helper signature, fixture ownership, and compiling HC
   stub (the current `&mut ReadServerFixture` method cannot simply
   be imported from the private F54 crate). HC proves
   only its positive test RED with the pinned compiling stub before
   `harness-ready`; `cargo check --all-targets` and strict pedantic
   clippy pass at HC and IC. IC implements the settle helper and
   makes its full `cargo dev-test --no-fail-fast` suite GREEN. At the later
   `142.058-T` task, load the F54 body with the completed helper's
   `ensure_daemon` call in one commit, and perform its identity and
   PA-5 real-snapshot fixture edits in that task only. Retain the
   historical `.001-ST` and original F54 RED evidence; revalidate
   F54's exact harness/ownership provenance before its work starts,
   and halt if the old `harness-ready` record no longer qualifies.
   Its final full suite must be GREEN. Rework the R8.3 table to
   distinguish separate targets rather than assigning temporary HC
   failures of F54 rows 2-5 to `142.058-T`.
3. **R6.9/R7.2/R7.5-R7.6/R8.7-R8.8 and task-text manifest
   (`P:580-612,669-695,843-883,1257-1278`):**
   rewrite T1-T18 and the PRE-3F diff contract for new Owned files
   and the `Cargo.toml` registration; stop claiming HC makes no Cargo
   change. The original F54 target remains registered as the
   placeholder until `142.058-T`. Keep E1-E8 and their apply order;
   update the S4 hold to require PA-5 tasks and E6/E7 before claim.
   PA-5 RED harnesses travel with their own tasks; do not install
   them ahead of PRE-3F's full-suite gate. No review verdict may use
   a RED F54 file while its owner is pending.
4. **R7.4/R7.6/R8.6, Constitution, Runtime Verification, and PA-6
   notes (`P:799-829,869-879,1221-1253,2980-2991,3060-3073`):**
   carry the four P2 dispositions above, relocate B0-to-IC
   PA-5/main attribution to F54's own verification, and require
   full-suite GREEN for PRE-3F, PA-5 tasks, and F54 at their gates.
   The PA-6 hold remains a claim gate, not a status/edge workaround.

**Decisions first:** the operator records PA-7 (including the
PRE-3F test-target/Owned-file expansion), authorizes Stage to prepare
this Revision 9 and one scoped review, and chooses a T0 durability route
before H2. PA-5 still needs its separately reviewed task plan;
PA-6 needs a **later** explicit decision to release S3. A review PASS
does not itself authorize shipment, backlog, git, or merge operations
(`P:995-1016,1221-1253`; `.github/policies/workflow-policies.md:396-415`).

**Confidence:** moderate (~0.7) that one scoped review of a *fully
synchronized* Revision 9 will clear this Step 4.3 blocker. The
new test-support boundary, old `142.058-T` harness provenance, and
unreviewed PA-5 task boundaries are residual review risks. If a
self-contained positive fixture cannot be scoped within PRE-3F's
two-hour limit, replan the test support explicitly; do not restore
the two-owner RED/GREEN overlap.
