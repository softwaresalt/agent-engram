---
title: "008-S Ship Execution — Compacted Memory"
type: compacted
shipment: 008-S
feature: 031-F
merged: "567cd51"
pr: 46
date: 2026-04-29
original: "docs/archive/memory/2026-04-29-008-S-ship-execution-memory.md"
---

## Outcome

031-F fully shipped. PR #46 merged to main at `567cd51`. All 8 tasks done. Both CI backends green.

## Key Decisions

1. Bug format → `docs/compound/bugs/` + `type: bug`; no H1 with frontmatter `title:`
2. File-first threshold → >500 tokens, Tier 1 subagents, ≥30% reduction
3. P-011 orphan check → Stage Step 5.0; P-012 + P-010 gate → Ship Step 1
4. Learnings-researcher → file-first spawn contract in compound SKILL.md

## Commits

* `d4a000a` `031.001.001-T`
* `59d9b99` `031.001.002-T`
* `b8218c6` `031.002.001-T`
* `cad8fd4` `031.004.001-T`
* `b7844e2` `031.002.002-T`
* `4a3960c` `031.003.001-T`
* `f42f3ca` `031.004.002-T`
* `9f6dbdc` `031.003.002-T`
* `4a77831` `fix(docs) H1 removal`

## Post-Merge Closure

- 008-S archived via `backlogit shipment ship` (claim then ship)
- Reconcile reports: `008-S-pre-20260429.md` + `008-S-post-20260429.md` (both PROCEED)
- Decided-plan: `docs/exec-plans/2026-04-29-031-F-harness-hardening-decided-plan.md`
- Closure: `docs/closure/2026-04-29-008-S-harness-hardening-closure.md` (READY)
