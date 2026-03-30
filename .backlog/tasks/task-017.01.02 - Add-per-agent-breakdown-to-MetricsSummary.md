---
id: TASK-017.01.02
title: Add per-agent breakdown to MetricsSummary
status: To Do
assignee: []
created_date: '2026-03-30 01:57'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-017.01.01
  - TASK-016.04
parent_task_id: TASK-017.01
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add per-agent metrics breakdown to `MetricsSummary`.

**Files to modify:**
- `src/models/metrics.rs` — add `by_agent: BTreeMap<String, ToolMetrics>` to MetricsSummary
- `src/services/metrics.rs` — update `MetricsSummary::from_events` to aggregate by agent_role

**Implementation:**
Add field to MetricsSummary:
```rust
#[serde(skip_serializing_if = "BTreeMap::is_empty", default)]
pub by_agent: BTreeMap<String, ToolMetrics>,
```

Per review F5: Use `skip_serializing_if` so the field only appears when agent-attributed data exists. This preserves backward compatibility of the `get_branch_metrics` response.

In `from_events`, group events by `agent_role`. Events without agent_role are grouped under `"anonymous"`.

**Test scenarios:**
- MetricsSummary with agent-attributed events produces correct by_agent breakdown
- MetricsSummary with no agent_role events omits by_agent from JSON
- Mixed events (some with, some without agent_role) handled correctly
- Existing MetricsSummary tests continue to pass
<!-- SECTION:DESCRIPTION:END -->
