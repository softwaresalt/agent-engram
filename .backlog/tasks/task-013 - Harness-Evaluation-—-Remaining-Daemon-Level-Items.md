---
id: TASK-013
title: Harness Evaluation — Remaining Daemon-Level Items
status: To Do
assignee: []
created_date: '2026-03-29 04:57'
updated_date: '2026-03-30 02:00'
labels:
  - epic
  - harness
  - deferred
dependencies:
  - TASK-016
  - TASK-017
references:
  - .backlog/research/Agent-Harness-Evaluation-Report.md
priority: low
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Remaining harness improvement items from the AI Agent Harness Evaluation Report that require daemon code changes and cannot be implemented through agent/skill markdown files alone.

**Remaining deferred items:**
- **Primitive 5 (Tool Execution)**: Per-agent write policies via MCP policy engine — requires daemon code changes to `write.rs` to check agent identity and restrict file access per agent role
- **Primitive 7 (Observability)**: Automated prompt optimization — requires infrastructure to analyze token usage patterns and automatically suggest prompt revisions

**Why deferred:** These items require daemon code changes (violating the harness-only constraint) or depend on infrastructure not yet built.

**Already addressed by TASK-012 and TASK-014:**
- P1 (Context Management): compact-context skill, compaction trigger, memory advisory
- P2 (Task Granularity): 2-hour rule, width isolation, atomic milestone (TASK-014)
- P3 (Orchestration): Existing stop conditions documented, supervisor pattern rejected
- P4 (Tool Guardrails): Feature flag enforcement rule, protected file warnings
- P5 (Dynamic Injection): [REINFORCE] re-reads, DoD pre-flight check
- P6 (Observability): Review gate strengthened, metrics check, granularity compliance
<!-- SECTION:DESCRIPTION:END -->

## Implementation Notes

<!-- SECTION:NOTES:BEGIN -->
2026-03-30: Deferred daemon-level items now fully planned and decomposed into TASK-016 (MCP Sandbox Policy Engine) and TASK-017 (Observability and Evaluation Daemon Primitives). This task can be closed once those epics are in progress.
<!-- SECTION:NOTES:END -->
