---
id: TASK-014
title: Task Granularity and Horizon Scoping
status: Done
assignee: []
created_date: '2026-03-29 06:54'
labels:
  - epic
  - harness
dependencies: []
references:
  - .backlog/research/Agent-Harness-Evaluation-Report.md
  - .backlog/plans/2026-03-29-task-granularity-plan.md
  - .backlog/reviews/2026-03-29-task-granularity-plan-review.md
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Enforce task granularity and horizon scoping across the agent harness pipeline to prevent sequential error compounding. Based on METR Time Horizons research: agent reliability drops below 50% for tasks >2 hours.

**Three rules enforced:**
1. **2-Hour Rule**: Every task scoped to ~2 hours human effort (heuristic: <3 files, <5 functions, <4 test scenarios)
2. **Width Isolation**: Single skill domain per task (no mixing code + docs, schema + API)
3. **Atomic Milestone**: Every task produces a verifiable state change (passing test, successful build, measurable output)

**Enforcement points:**
- `impl-plan` — Core Principle #7, Plan Quality Bar, unit template fields
- `backlog-harvester` — Authoritative granularity validation (Step 3.2b)
- `harness-architect` — Advisory secondary check (Step 4.3)
- `build-orchestrator` — Atomic milestone gate (Step 4.4)
- `build-feature` — Inherently compliant (harness loop)
- `AGENTS.md` — Task Granularity section under Development Workflow
<!-- SECTION:DESCRIPTION:END -->
