---
id: TASK-017.02.03
title: Add evaluation config to WorkspaceConfig
status: Done
assignee: []
created_date: '2026-03-30 01:58'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-017.02.01
parent_task_id: TASK-017.02
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add `[evaluation]` section to `WorkspaceConfig`.

**Files to modify:**
- `src/models/config.rs` — add `evaluation: EvaluationConfig` field with `#[serde(default)]`

**Config format (`.engram/engram.toml`):**
```toml
[evaluation]
max_token_ratio = 10.0
max_error_rate = 0.3
min_tool_diversity = 2
slow_query_threshold_ms = 200

[evaluation.weights]
token_efficiency = 0.4
error_rate = 0.3
diversity = 0.15
latency = 0.15
```

All fields use `#[serde(default)]` for backward compatibility.

**Test scenarios:**
- WorkspaceConfig deserializes with [evaluation] section
- WorkspaceConfig deserializes without [evaluation] (defaults applied)
- Custom weights override defaults correctly
<!-- SECTION:DESCRIPTION:END -->
