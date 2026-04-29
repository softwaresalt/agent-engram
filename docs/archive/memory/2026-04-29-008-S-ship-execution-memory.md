---
title: "008-S Ship Execution Memory"
date: 2026-04-29
session: f91de172-89c0-47dc-bbe3-9a5ca7bc302b
shipment: 008-S
feature: 031-F
branch: feature/031-F-harness-hardening
pr: https://github.com/softwaresalt/agent-engram/pull/46
pr_number: 46
status: awaiting-merge-approval
---

## Status

PR #46 is open, CI green (both backends), awaiting user merge approval.

## Tasks Completed (8/8)

| ID | Title | Commit |
|---|---|---|
| 031.001.001-T | File-load verification in agent-engram instructions | d4a000a |
| 031.001.002-T | Verification step in deliberate/impl-plan/spike skills | 59d9b99 |
| 031.002.001-T | Bug capture format decision + example entry | b8218c6 |
| 031.002.002-T | Wire bug capture into observe skill + ship agent | b7844e2 |
| 031.003.001-T | file-first-content.instructions.md new instruction file | 4a3960c |
| 031.003.002-T | Apply file-first to learnings-researcher in compound skill | 9f6dbdc |
| 031.004.001-T | P-011 + P-012 policies in workflow-policies.md v1.6.0 | cad8fd4 |
| 031.004.002-T | Cross-reference P-011/P-012 from ship and stage agents | f42f3ca |

## Files Modified

- `.github/instructions/agent-engram.instructions.md`
- `.github/instructions/constitution.instructions.md`
- `.github/instructions/file-first-content.instructions.md` (new)
- `.github/skills/deliberate/SKILL.md`
- `.github/skills/impl-plan/SKILL.md`
- `.github/skills/spike/SKILL.md`
- `.github/skills/observe/SKILL.md`
- `.github/skills/compound/SKILL.md`
- `.github/agents/ship.agent.md`
- `.github/agents/stage.agent.md`
- `.github/policies/workflow-policies.md`
- `docs/decisions/2026-04-29-bug-capture-format-decision.md` (new)
- `docs/compound/bugs/stale-engram-citation-2026-04-29.md` (new)

## Key Decisions

1. **Bug capture format**: `docs/compound/bugs/` with `type: bug` frontmatter — visible to learnings-researcher automatically
2. **File-first threshold**: >500 tokens, Tier 1 subagents only, ≥30% reduction required
3. **Learnings-researcher baseline**: 23 files / ~18,000 tokens inline → ~500–1,500 tokens after query-mediated delivery
4. **P-011/P-012 placement**: P-011 orphan check added to Stage Step 5.0; P-012 + P-010 gate added to Ship Step 1

## Next Steps (post-merge)

1. Create `post-merge/031-F-harness-hardening` branch
2. Close shipment 008-S via `backlogit_ship_shipment`
3. Mark 031-F and all 4 chores done in backlog
4. Run `compound-refresh` skill to check if new docs supersede any existing compound entries
5. Run `compact-context` skill
6. Push post-merge closure branch and merge

## Outstanding Pre-Merge Items

- None — all 8 tasks done, CI green, PR open
