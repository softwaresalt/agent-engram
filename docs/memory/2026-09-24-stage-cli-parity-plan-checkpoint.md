# Stage Checkpoint: CLI Parity Plan (Circuit Open at Plan Review)

> **Status (2026-10-04, PR #410 review): historical checkpoint, superseded.** The "plan CLI parity before 142-S
> continues" directive below was superseded the same day by the operator's Path S decision
> (`docs/memory/2026-09-24-stage-path-s-handoff.md`), and 142-S itself (still active) is now planned to be abandoned at H3 under
> `docs/exec-plans/2026-09-30-142-f-decomposition-plan.md` Revision 18. Do not resume from this checkpoint; see 035-D and
> the parity plan's status note.

- **Date**: 2026-09-24 (about 15:40 -07:00)
- **Agent**: Stage
- **Session**: stage-2026-09-24-cli-parity-docs-harness (resumed)
- **Status**: mid-session checkpoint (not context overflow)

## Goal and Acceptance

The operator said: "Restore checkpoint-20260924-185101.json and resume Stage with option 2; plan CLI parity before 142-S continues."

- 036-D is decided as Option 2: a durable harness-policy change for docs-only tasks. There is no one-off waiver for 142.059-T.
- 035-D full CLI parity is an approved requirement. It must be planned now, before 142-S continues.
- Stage must flag honestly if parity cannot be implemented before 142-S without pausing or changing the active release. It must not downgrade parity to "after 142-S".
- Constraints:
  - Do not modify the 142-S manifest.
  - No source, test, config, template, or agent edits. No build, test, or lint runs. No parallel branch or worktree.
  - Preserve the dirty files left by the operator and Ship.
  - Use the backlogit CLI and the engram CLI.
- Output: the plan and harvest as far as policy allows, then a concise status: artifacts, IDs, order, whether a checkpoint is pending, and the exact next step.

## Established Facts

- **Recovery**: done.
  - All 30 checkpoints were enumerated with no filter. There were 0 quarantined.
  - Sole active checkpoint: checkpoint-20260924-185101.json. Owner is stage, and it is valid.
  - The operator confirmed the restore. Engram was reachable, so prune-on-restore ran as a bounded summary.
  - The checkpoint is NOT resolved yet. Resolve it only after a successful resume, which means the session actually delivers its outcome.
- **Tool gate**: DEGRADED_MODE. The backlogit MCP tools are not available, so Stage uses the backlogit CLI fallback. Each call takes about 9 seconds.
  - `backlogit sync`: INDEX_SYNC_OK.
  - Hooks poll returned no events.
  - Config: `.autoharness/config.yaml` parsed OK.
- **142-S**: status active.
  - All six 142-F tasks are active. Five code tasks are harness-ready; 142.059-T is labeled harness-not-applicable.
  - There is no open PR. The branch is local only and not pushed. There is a single worktree.
  - The 142-S harness still contains `assert!(!...Cli)` for get_retrieval_eval_report in the dirty file `tests/contract/read_server_cli_mcp_parity_test.rs`. Fixing it is Ship's job.
- **Shipment transitions**: backlogit supports only queued→active, active→shipped, and active→abandoned. There is no pause. Ship's claim step requires the shipment to be the sole active one.
- **Parity gaps (verified)**:
  - Tool level: get_retrieval_eval_report (default build), query_changes, and index_git_history (both git-graph).
  - Parameter level: impact_analysis.powerbi_node_id and flush_state.force. The flush handler ignores force.
- **Ship per-task gate**: Step 4.3 runs `cargo dev-test`, which uses default features only.
- **CI**: no git-graph lane. `ci.yml` has `paths-ignore` for docs.

## Completed Changes (Stage artifacts only)

- **Plan** (revision 3): `docs/exec-plans/2026-09-24-full-cli-mcp-parity-plan.md`.
  - Units U1–U9 form a linear chain:
    - U1: get_retrieval_eval_report descriptor and catalog
    - U2: `engram report retrieval-eval`
    - U3: git-graph descriptors
    - U4: `query-changes`
    - U5: `index-git-history`
    - U6: `impact --powerbi-node-id`
    - U7: `flush --force`
    - U8: CI config for the git-graph lane
    - U9: docs
  - The plan also includes the harness specification (H-PARAM, H-REG, H-ORACLE, H-F23R, H-RETR, H-GIT, H-PARAM2, H-CI, H-DOCS), the pending-harness contract, Path P/S execution ordering, the Constitution Check, Plan Hardening, and review records for attempts 1 and 2. The markers `<!-- plan-review-attempt: 1 -->` and `2` are present.
- **035-D decision doc**: set to decided, `promoted_to` points at the plan, and the superseded note is added.
- **036-D decision doc**: set to decided (Option 2) and the Amendment Specification is added (label `harness-verification-gated`).
- **Backlog records**: `backlogit update 035-D` and `backlogit update 036-D` set the chosen-direction, open-questions, and notes sections.

## Plan Review Attempt 3: FAIL (circuit open)

**P1 (Rust reviewer).** `tests/fixtures/mcp_tool_catalog.expected.json` has no query_changes or index_git_history entries, and `mcp_catalog_oracle_test.rs` is not cfg-aware. The git-graph lane is therefore already red for exact name-set and drift, and U8 would make that red gate CI.

- **Proposed fix**: add both entries with `"feature":"git-graph"`, and have `load_expected` drop them when `!cfg!(feature="git-graph")`. Mark them red for U3. Record the current failure as pre-existing. Relabel the `_description_macros` removal as hygiene.

**Other attempt-3 findings (P2/P3):**

- The reverse task move active→queued needs explicit operator authorization. Ship's role boundary does not list it.
- Precheck that the tasks can be reassigned after 142-S is abandoned.
- Name the P-004 git-graph-lane substitution as a deviation.
- Run the git-graph lane from U3 through U9.
- Add an F54 pending-red row for Path S.
- Add the F54 target to H-CI for Path S.
- 035-D Options text and status are stale.
- F55 changes by label only.
- H-CI must reject `continue-on-error`.

Constitution, Architecture, and Scope all passed.

**Escalation (P-013.6).** The escalation route resolved from the fresh config is `escalation: gpt-6-sol/openai`. That differs from the Stage route (claude-opus-5.5/anthropic), so this is not ESCALATION_DEGRADED.

- Required: compile the escalation payload and hand it to engram for analysis, then halt. Do not re-run plan-review without the operator.
- Harvest, shipment assembly, stash archive, and 090.004-T archive are BLOCKED because the review gate did not pass.

## Rejected Approaches

| Approach | Reason |
|---|---|
| Fold parity into 142-S | Would modify the active manifest |
| One-off waiver for F55 | The operator chose Option 2 |
| Dependency edge between 142-S and the parity shipment | Would mutate 142-S or silently encode Path S |
| Units U01 and U02 as separate tasks | They had no red harness and broke the 2-hour rule |
| "Park" 142-S | No shipment pause exists |

## Remaining Work

1. Write the escalation payload into the plan: threshold is plan-review, 3 consecutive FAILs, route gpt-6-sol/openai. Include refs, then halt.
2. Optionally record the proposed revision-4 fixes as unreviewed. Do not re-review.
3. Create a Stage checkpoint through the backlogit CLI with `phase: plan-review-circuit-open`. Leave checkpoint-20260924-185101 active, or supersede it only after the resume outcome is delivered.
4. Final memory note and `backlogit sync`.
5. Give the operator the status:
   - Planning is done up to the review gate. Harvest is blocked pending operator authorization of re-review attempt 4 after the oracle fix.
   - Path P needs `skip_policy: P-001` plus authorization for Ship to abandon 142-S, move its tasks to queued, and ship a successor.
   - Path S keeps the original order.
   - The 036-D amendment goes through the template channel, owned by the operator.
   - Ship owes the F54 harness fix.

## Stopping Criterion

Stop when the escalation payload is recorded, the checkpoint and memory are written, sync has run, and the concise status has been delivered. Do not harvest.

## Next Action

Append the escalation payload and the revision-4 proposal (unreviewed) to the plan file.
