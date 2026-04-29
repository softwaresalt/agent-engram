---
title: "008-S / 031-F Operational Closure: Agent Harness Engram-Aware Workflow Hardening"
type: closure
shipment_id: 008-S
feature_id: 031-F
merge_sha: 567cd51571a1048e07a4addeaff02ac5e96680de
pr_number: 46
pr_url: https://github.com/softwaresalt/agent-engram/pull/46
branch: feature/031-F-harness-hardening
status: READY
mode: post-merge
date: 2026-04-29
---

## Summary

Shipped feature 031-F — a cross-cutting documentation-only hardening pass across
the agent harness that tightens engram-aware workflows, bug capture, file-first
content production, and decomposition/branch policy. All 8 tasks landed in PR #46
(9 commits, including a Copilot review fix), merged to main at `567cd51`.

## Scope

13 manifest items across 4 chores:

| Chore | Scope |
|---|---|
| 031.001 | File-load verification — engram instructions + 3 skills (deliberate, impl-plan, spike) |
| 031.002 | Bug capture format — decision record + observe/ship wiring |
| 031.003 | File-first content protocol — new instruction file + compound skill |
| 031.004 | P-011/P-012 policies + ship/stage cross-references |

## CI and Review

- **CI**: green on both `surreal-backend` and `cozo-backend` at merge
- **Copilot review**: 1 finding addressed (H1 heading in bug entry with frontmatter `title:`) — fixed in `4a77831`, thread resolved via GraphQL
- **Flaky test**: `c018_06_policy_denied_call_records_metrics_with_denied_outcome` flapped once on re-run; confirmed pre-existing and unrelated to docs-only changes
- **Unresolved review items**: none

## Runtime Surfaces Affected

None. All 21 changed files are markdown artifacts (instructions, skills, agents, policies, decisions, docs). No Rust source changed, no binary behavior altered.

## Runtime Verification

**Verdict**: PASS (structural)

Documentation-only change. No runtime surfaces changed. Verification is structural:

- All modified instruction/skill/policy files are syntactically valid markdown
- No `cargo test` targets exist for markdown content — harness exception applied per plan Constitution Check
- Behavioral adoption of new protocols is observable in future agent runs (see Monitoring section)

## Invariants to Preserve

1. Engram verification step in deliberate/impl-plan/spike MUST NOT block on engram unavailability — protocols remain optional when daemon is unreachable
2. `docs/compound/bugs/` entries MUST have `type: bug` in frontmatter and MUST NOT include an H1 heading when `title:` is present
3. File-first threshold (>500 tokens + Tier 1 subagent) MUST be preserved in the instruction file
4. P-011 and P-012 policies in `workflow-policies.md` remain as v1.6.0 additions

## Pre-Deploy Checks

| Check | Status |
|---|---|
| Both CI backends green | ✓ |
| No Rust code changed (docs-only exception) | ✓ |
| Copilot review comment resolved | ✓ |
| Backlogit archive integrity (GI/GR reconcile) | ✓ PROCEED |
| Branch protection merge strategy (P-009) | ✓ merge commit only |

## Deployment Path

Merge-only. No deployment or canary required — docs-only change with no runtime behavior change.

## Post-Deploy Checks

1. Verify `docs/compound/bugs/` is scanned by the learnings-researcher in the next compound skill invocation
2. Confirm file-first protocol instructions appear in subagent prompts when `compound/SKILL.md` is loaded
3. Validate P-011 orphan check appears in Stage Step 5.0 in the next Stage-executed plan
4. Confirm P-012 branch-per-feature check appears in Ship Step 1 in the next Ship execution

## Risky Action Record

| Action | Risk | Approval | Result |
|---|---|---|---|
| Update agent definitions (ship.agent.md, stage.agent.md) | moderate | inline plan review (PASS) | applied |
| Add P-011 + P-012 to workflow-policies.md | moderate | inline plan review (PASS) | applied |
| Create new instruction file (file-first-content.instructions.md) | low | N/A | applied |
| Create new bug entry in docs/compound/bugs/ | low | N/A | applied |

No destructive actions taken. No rollback required.

## Healthy Signals

- Future agent sessions reference the engram verification step before treating file content as authoritative
- Bug discoveries are routed to `docs/compound/bugs/` rather than scattered in memory files
- Learnings-researcher reads `docs/compound/bugs/` entries during compound skill execution
- Ship Step 1 logs a P-012 branch-per-feature check; Stage Step 5.0 runs the P-011 orphan check

## Failure Signals

- Agent ignores the new verification step — indicates instructions were not loaded
- Bug entries created in non-standard locations (e.g., `docs/bugs/` or backlog) — indicates protocol not propagated
- P-011/P-012 checks absent in future Ship/Stage runs — indicates agent definitions not loaded

## Monitoring Plan

Documentation-only changes have no metric dashboards or alert thresholds. Monitoring is
behavioral — observe the next 2–3 agent sessions that invoke deliberate, impl-plan, spike,
compound, ship, or stage to confirm the new protocols are followed. No automated alert needed.

## Rollback Trigger

N/A for documentation-only. If a new protocol proves counterproductive (e.g., engram
verification step causes unnecessary agent stalls), revert via `git revert 567cd51` which
will re-open PR #46 changes for removal.

## Rollback Procedure

```bash
git revert -m 1 567cd51571a1048e07a4addeaff02ac5e96680de
# Creates a revert commit on a branch; open a PR against main
```

## Validation Window

2–3 agent sessions observing the four affected workflows (deliberate/impl-plan/spike,
compound, ship, stage). Owner: repository maintainer.

## Source Artifact Traceability

| Field | Value |
|---|---|
| `source_stash_id` | Not present in 031-F — item did not originate from stash |
| Deliberation | `docs/decisions/2026-04-21-031-F-harness-hardening-deliberation.md` (`decision_status: decided`, `promoted_to: plan`) |
| Execution plan | `docs/exec-plans/2026-04-21-031-F-harness-hardening-plan.md` (plan-review PASS) |

## Follow-Up Items

1. **Stash SQL improvements** — 4 stash entries for SQL reference resolution remain active; candidate for next shipment (see deliberation recommendation to group as new shipment)
2. **Flaky test** — `c018_06_policy_denied_call_records_metrics_with_denied_outcome` should be tracked as a known flaky test and stabilized separately

## Status

**READY** — post-merge closure complete. No open conditions. Monitoring is behavioral for next 2–3 agent sessions.
