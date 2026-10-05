---
title: "142-F decomposition plan: review attempt 14 findings (raw persona output, condensed)"
date: 2026-09-30
source: Orchestrator-dispatched persona reviewers (read-only), target docs/exec-plans/2026-09-30-142-f-decomposition-plan.md (587 lines)
---

Dispatched directly by the Orchestrator after two Stage review sessions ended
without output. All five reviewers were read-only.

| Persona | Verdict | P0 | P1 | P2 | P3 |
|---|---|---|---|---|---|
| Rust Reviewer | FAIL | 0 | 2 | 5 | 4 |
| Scope Boundary Auditor | FAIL | 0 | 1 | 5 | 5 |
| Architecture Strategist | FAIL | 0 | 3 | 5 | 3 |
| Constitution Reviewer | FAIL | 0 | 6 | 7 | 3 |
| Learnings Researcher | FAIL | 0 | 1 | 6 | 3 |

## Deduplicated P1 findings (11)

1. **`shipped` vs `done` (Learnings P1-1, Constitution P1-3).** CG-D rule 2
   (plan:225) and 10.1 step 1 (plan:377-378) require earlier slots to be
   `shipped`. P-015 targeted manual safe-close always yields `status: archived`,
   `archived_status: done`; `move <S> --status shipped` exits 9 under 142-F
   (.backlogit/archive/141-S.md audit rationale; 133-S..141-S precedent). Fix:
   define a finished slot as archived with `archived_status` in {done, shipped}
   plus merge SHA and closure record; name manual safe-close (141-S procedure)
   as every slot's Step 6; forbid `backlogit shipment ship` (force-releases
   142-F, expands children).
2. **Operator decision misquoted (Scope P1-1).** §1.1 (L39-44) and the §3
   Revision 15 row quote a relayed summary as "verbatim". The operator's actual
   2026-09-30 19:09 PT words: "As recommended, let's split the work into one
   task per shipment with DAG ordering; optionally, run the three extra
   rebuilds now if that will actually help narrow the concerns. Use your best
   judgement." Record the rebuild decline / run point 3 withdrawal as
   Orchestrator judgement under "use your best judgement"; mark the PS
   apparatus and glossary work as Stage additions.
3. **PS-5 abandon of 142-S has no named mechanism, wrong risk, marked approved
   (Constitution P1-1; Learnings P2-5).** Plan:257 names no command. Registry
   has no shipment-abandon op (only return_blocked); rev15 memory says "No
   `shipment abandon` CLI verb"; precedent 081-S went active -> blocked ->
   archived (.backlogit/logs/081-S.jsonl:2-4). "No un-abandon" means
   irreversible -> ActionRisk destructive. Fix: name the exact command/tool
   sequence (check backlogit_block_shipment / normalize_blocked_shipment /
   return_blocked descriptions), prove on a scratch .backlogit copy inside the
   workspace that a former 142-S member can join a new shipment, snapshot
   142-S + members + 142-F first, add restore path, mark `planned` pending
   verbatim operator approval.
4. **No path to `main` for H1 and PS-5; PS-1/PS-7 contradiction (Constitution
   P1-2; Learnings P2-4).** PS-1 (plan:253) commits planning artifacts
   (incl. never-committed 142.061-T..142.074-T) on the parked branch; PS-7
   (plan:259) keeps that branch unmerged and forbids new commits after A0R;
   assembly runs on `chore/stage-142-f-assembly` from `main` (§8.2) where those
   tasks/plans don't exist and 142-S is still active -> 8.1 step 2 (plan:281)
   fails. Dirty worktree has no owner. Fix: classify the dirty worktree;
   Stage commits planning artifacts on a `chore/stage-*` branch from `main`,
   operator merges (P-009 merge commit, Copilot HEAD review gate); 142-S
   disposition recorded on its own chore branch through the P-014 gate; only
   then assembly. PS-1 owner is Stage.
5. **No child-status rollup guard (Constitution P1-4).** 10.3 (plan:409-410)
   assumes 142-F stays active; .backlogit/logs/109-F.jsonl (active->blocked,
   done->blocked->queued) and 087-F.jsonl (archived->active) show backlogit
   changes parent status itself. Fix: after every write to a 142-F descendant,
   re-read 142-F and the parent task; unexpected change -> HALT
   `R15-ROLLUP-DRIFT <id> <from>-><to>`, revert only with operator approval;
   CP-FINAL halts if 142-F is already done.
6. **F51/F52 cannot be test-first in separate slots (Constitution P1-5).**
   §13 (plan:506-507) claims Principle II compliance for test-only route (t),
   but old plan:2755-2757 says F52 (142.056-T) tests fail only against the
   pre-F51 start.ps1; CG-D puts D16 after D15 (F51, 142.055-T), so D16's tests
   pass at Step 2 and P-004's red phase is impossible. Fix options: (a) operator
   allows 055+056 as one slot, (b) operator approves F52 rerouting to its own
   placeholder marker, (c) fold 056's matrix into 055 as its RED harness. Both
   are in the PA-6-held range, so this can be an open decision bound to the
   PA-6 lift rather than a blocker for D1-D14.
7. **Sizing halt fires and the sizing claim is false (Constitution P1-6,
   Architecture P1-3).** §2/§4 (plan:177-183) say no task has size/complexity;
   142.055-T:31 "Size: S", 142.056-T:31 "Size: M | Complexity: medium",
   142.074-T:40 "Size: XS", 142.054-T:49 and 142.064-T:52 "Complexity: high"
   (both already split into subtasks). Registry update_task has no size field.
   Step 5b / `R15-SIZE-SPLIT` would halt. Fix: drop step 5b and the halt; state
   that a task already split into subtasks meets the bound; correct §2/§4/§13
   (plan:512-513).
8. **Slot order and PA-6 hold exist only in prose (Architecture P1-1).**
   Plan:188-190 edges are task-only; plan:224-227 CG-D rule 2. Backlogit
   Shipment Sequencing Protocol and Orchestrator Step 2
   (_orchestrator.agent.md:299-311) sequence by shipment-level `blocks` edges
   plus `queue_position`; Ship intake (_ship.agent.md:153-168) has no hold
   check, so §12 "Ship refuses to claim a held slot" is unenforced; D15/D17
   become eligible after D14. Fix: at assembly, add shipment-level
   `dep add <Dk> <Dj> --type blocks` mirroring task edges, set queue_position,
   and do NOT create held shipments (D15+) until PA-6 is lifted; reduce CG-D
   rule 2 to a tie-break.
9. **D14 documents a held change (Architecture P1-2).** 142.074-T.md:27,:35 add
   a "Superseded by 142-F" note claiming guardrail 4 (fail-open to Copilot) is
   replaced, citing 142.055-T, which is D15 and held; start.ps1 on main stays
   fail-open. Fix: add edge 074 -> 055 (and 057) and move 074 after D15 into the
   held range, or move the supersession note into 059 (F55).
10. **PRE-4 assertion fix not carried (Rust P1-1).** §9 A23/A24 (~l.355)
    applies R14.3 verbatim: `assert_eq!(GENERATION_PINNED_READS,
    ["get_workspace_statistics"], …)` (old plan ~4305-4307, A24 ~4346);
    142.066-T:25 says methods join later (86F93068), so the first PA-5 task
    breaks PRE-4. Fix: override A24 to
    `assert!(GENERATION_PINNED_READS.contains(&"get_workspace_statistics"), "Worker: 142.066-T GENERATION_PINNED_READS");`;
    A23 "final value" -> "contains"; rewrite the R14.3 Lint bullet.
11. **`clippy::match_same_arms` fix not carried (Rust P1-2).** A11 keeps R13.2
    guard "a `match` over `Failure` with one arm per variant and no wildcard"
    (old plan ~3827; R14.5 ~4436); identical arm bodies fail pedantic clippy;
    A20 gates D12 with `-D clippy::pedantic` and forbids `allow`. Fix: arms
    return distinct values (index 0-6 or `stage_name(f)`) or one or-pattern arm
    with no wildcard; add "no match_same_arms; no allow".

## P2 findings (condensed)

- Scope P2-1: R12.2-R12.5 ownership (O1-O4, A1-A5, A15) and R12.4 signature
  freeze still in force though they only served same-shipment RED mapping;
  retire them, drop R12.4 sentence from A23, fix L55.
- Scope P2-2: CG-T adds a non-DAG ordering gate with no enforcement point;
  put the subtask-close PR in the slot's Release Closure Completion Gate
  (R13.4 mechanism, old plan ~3962-3966) or check at CP-FINAL; drop CG-T.
- Scope P2-3 / Rust P2-1 / Arch P3: D-NOTE stacks an override instead of
  replacing text; A20 puts "or each failing test is a later S2 task's recorded
  pending RED" into acceptance of 054, 069-072; stale 142-S/PA-1 gating text
  and `pending-pa-1` labels on 060-074. Fix: write clean replacement text, no
  withdrawn text applied.
- Scope P2-4: PS-7/PS-8 are unrequested destructive proposals; keep T0
  indefinitely, leave branch untouched, no action.
- Arch P2-1: missing edge 065 -> 064 (142.065-T.md:40 needs PRE-3 RED baseline).
- Arch P2-2 / Rust P2-3: E8 and 060's edges to 054-057 are order-only; 060
  fixes an archive smoke check that may fail on main (Windows 8192-char
  truncation); if it fails on main every PASS-only slot before D18 is red.
  Record current result; move 060 early and drop order-only edges; ask the
  operator whether PA-6 covers non-launcher slots (060, 063, PA-5, 058).
- Arch P2-3: 142.069-T.md:36 compares with F52/F53 artifacts that land at
  D16/D17; reword to "equals the seven F50 `Failure` variants".
- Arch P2-4: PA-5 edges fixed before tasks exist; take edges from the harvest;
  add zero-PA-5-task rule.
- Arch P2-5: grep D1-D14 item text for `PRE-3F`, `after-edit map`, `F54
  settle` at assembly; halt on match (142.064-T.md:41,43-44; 142.065-T.md:40).
- Rust P2-2 / Arch P3: build the parked-commit-to-slot map from `git log
  --name-only main..<T0>`; a0ccdc27, 1dfc1b5b, 5760b948, a47b8aff unmapped; D18
  missing; D21 (059) has no commit.
- Rust P2-4: D13 `Cli::command().debug_assert()` can't run from
  tests/contract (struct Cli private, src/bin/engram.rs:23); use a
  `#[cfg(test)]` module in src/bin/engram.rs.
- Rust P2-5: 6d216d19 F54 body written against old main; FL byte-identity
  needs a compile check or allowed compile-only adaptations.
- Constitution P2-1: CG-S re-apply "once in total for 142-F".
- Constitution P2-2/P2-4/P2-5/P2-7: §13 risky-action list incomplete (21
  safe-closes deleting queue files + plain sync, subtask moves, CP-FINAL
  move); PS table lacks owner and approval_required; PS-3 rollback deletes a
  tag (needs approval); PS-6 rollback reuses an approval for the opposite
  change.
- Constitution P2-3: §13 omits principles I, III, IV, V, VI, VIII, XI (careful
  mode for PS-4/PS-5/§11; merge-commit rule for staging/closure PRs).
- Constitution P2-6: PS-5 approval cites frontmatter not the verbatim
  2026-09-27 22:54 Q1-Q8 text.
- Learnings P2-2: each safe-close Step 6 runs a plain `backlogit sync` (cache
  union landmine, docs/compound/backlogit-sync-cache-union-landmine-2026-07-02.md);
  name it a CG-S trigger.
- Learnings P2-3: Stage PRs merged by the operator need the Copilot
  commit_id == HEAD 4-point gate as CG-M evidence.
- Learnings P2-6: oscillation signature; certify once with multi-model
  adversarial review, freeze, route residual P2/P3 to backlog
  (docs/compound/single-model-plan-review-diverges-use-multi-model-adversarial-2026-07-23.md).

## P3 (selected)

- §8.4 dead (Scope P3-1); A20 "142.073-T final criteria" redundant; "Confirm
  C7" settled by C1 (record as Stage reading); single "slot done" definition
  needed; D-slot names clash with old "D2/D3/D4" labels in old plan and
  deliberation (rename, e.g. Slot-01).
- Re-check `pre_task_completion` disabled at every closure; canonical
  `{shipment}-{date}-post-merge-closure.md` per slot (140-S/141-S needed repair).
- Shared helper tests/helpers/activation_settle.rs dead_code risk; D8 `#[path]`
  include of preflight.rs dead_code risk and c269fa79 not salvageable at Step 2.
- §13 should record test-only `Result<_, String>` deviation for settle helper.
- E2 and E8 are order-only; say so.
- Ship's plain `backlogit sync` at session start up to 21 times: residual risk.
- State whether PA-5 tasks sit under 142-F.
