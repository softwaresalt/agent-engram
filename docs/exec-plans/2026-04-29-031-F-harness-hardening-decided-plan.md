---
title: "031-F Harness Hardening — Decided Plan"
description: "Condensed implementation decisions for 031-F: file-load verification, bug capture, file-first content, and workflow policies"
feature_id: "031-F"
shipment_id: "008-S"
original_plan: "docs/archive/plans/2026-04-21-031-F-harness-hardening-plan.md"
plan_review: PASS
merged: "567cd51571a1048e07a4addeaff02ac5e96680de"
date: 2026-04-29
---

## Final Decisions

| Decision | Outcome |
|---|---|
| Bug capture format | `docs/compound/bugs/` with `type: bug` in frontmatter; no H1 when `title:` present |
| File-first threshold | >500 tokens AND Tier 1 subagent; success = ≥30% context reduction |
| Target skill for file-first | learnings-researcher in `compound/SKILL.md` |
| Branch discipline | P-012 in `workflow-policies.md`; exception list covers docs-only and hotfix |
| Decomposition policy | P-011 in `workflow-policies.md`; research → plan → feature → task pipeline is universal |
| Bug entry format in `docs/compound/bugs/` | MUST NOT include H1 heading when frontmatter `title:` is set (Copilot review finding) |

## Implementation Units and Outputs

| Chore | Tasks | Outputs |
|---|---|---|
| 031.001 File-load verification | 031.001.001-T, 031.001.002-T | `agent-engram.instructions.md` "Verifying File Indexed" section; verification step in deliberate/impl-plan/spike |
| 031.002 Bug capture | 031.002.001-T, 031.002.002-T | `docs/decisions/2026-04-29-bug-capture-format-decision.md`; observe skill `source: bug`; ship agent Step 4.5/5.5 wiring; first example at `docs/compound/bugs/stale-engram-citation-2026-04-29.md` |
| 031.003 File-first content | 031.003.001-T, 031.003.002-T | `.github/instructions/file-first-content.instructions.md`; learnings-researcher spawn contract in compound SKILL.md |
| 031.004 Policy formalization | 031.004.001-T, 031.004.002-T | `workflow-policies.md` v1.6.0 (P-011 + P-012); ship Step 1 P-012 gate; stage Step 5.0 P-011 check |

## Dependencies Executed

- Lane A (031.001 → 031.003): sequential as planned; 031.001 fully complete before 031.003 started
- Lane B (031.002): independent; executed in parallel
- Lane C (031.004): 031.004.001-T before 031.004.002-T; cross-references correct

## Justified Deviations

| Principle | Deviation | Justification |
|---|---|---|
| II. Test-First | Docs-only exception | No `cargo test` targets for markdown; verification is structural |
| Task <3 files | 031.001.002-T touches 3 skills; 031.004.002-T touches 3 files | Mechanically identical one-subsection edits across cohesive groups |

## Rollback

`git revert -m 1 567cd51571a1048e07a4addeaff02ac5e96680de`

## Monitoring

7-day behavioral observation window: agents invoke file-load verification, route bugs to
`docs/compound/bugs/`, Ship checks P-012, Stage checks P-011 orphans.
