---
id: TASK-017.01
title: Metrics Attribution and Per-Agent Breakdown
status: Done
assignee: []
created_date: '2026-03-30 01:55'
labels:
  - epic
  - daemon
dependencies: []
parent_task_id: TASK-017
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Extend `UsageEvent` with call outcome tracking and add per-agent breakdown to `MetricsSummary`. Covers Plan 2 Units 1 and 6.

Includes:
- `outcome: String` field on UsageEvent (default "ok", error code on failure)
- `by_agent: BTreeMap<String, ToolMetrics>` on MetricsSummary
- Backward-compatible deserialization for existing JSONL files
- Updated `MetricsSummary::from_events` aggregation logic
<!-- SECTION:DESCRIPTION:END -->
