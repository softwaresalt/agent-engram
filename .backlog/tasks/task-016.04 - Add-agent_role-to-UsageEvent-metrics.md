---
id: TASK-016.04
title: Add agent_role to UsageEvent metrics
status: Done
implementation_note: Implemented in commit f57baab
assignee: []
created_date: '2026-03-30 01:54'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-016.02.02
parent_task_id: TASK-016
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `agent_role` field to `UsageEvent` and thread it through the metrics pipeline. This task is shared between the Sandbox Policy Engine and Observability features.

**Files to modify:**
- `src/models/metrics.rs` — add `agent_role: Option<String>` with `#[serde(skip_serializing_if = "Option::is_none", default)]`
- `src/tools/mod.rs` — pass agent_role from `ToolCallContext` into `UsageEvent` during metrics recording

**Backward compatibility:** `#[serde(default)]` ensures existing JSONL files without `agent_role` deserialize without error.

Per review F5: Use `#[serde(skip_serializing_if = "Option::is_none")]` so the field only appears in output when set.

**Test scenarios:**
- UsageEvent serializes with agent_role present
- UsageEvent serializes without agent_role (field omitted in JSON)
- Existing JSONL without agent_role deserializes correctly
- Metrics recording captures agent_role from ToolCallContext
<!-- SECTION:DESCRIPTION:END -->
