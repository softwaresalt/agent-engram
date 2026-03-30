---
id: TASK-017.03.01
title: Implement get_evaluation_report handler and routing
status: To Do
assignee: []
created_date: '2026-03-30 01:58'
updated_date: '2026-03-30 01:59'
labels:
  - daemon
dependencies:
  - TASK-017.02.02
  - TASK-017.02.03
parent_task_id: TASK-017.03
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Add the `get_evaluation_report` MCP tool to the daemon.

**Files to modify:**
- `src/tools/read.rs` — add `get_evaluation_report` handler function
- `src/tools/mod.rs` — add routing for `"get_evaluation_report"` in dispatch match, add to `should_record_metrics`

**Handler implementation:**
```rust
pub async fn get_evaluation_report(
    state: SharedState,
    params: Option<Value>,
) -> Result<Value, EngramError>
```

**Parameters:**
- `branch` (optional): Branch to evaluate. Defaults to active branch from workspace snapshot.
- `include_recommendations` (optional, default true): Include recommendation strings.

**Logic:**
1. Get workspace snapshot (or return WorkspaceNotSet)
2. Determine branch from params or snapshot
3. Read usage events via `metrics::compute_summary` path (read JSONL)
4. Load `EvaluationConfig` from workspace config
5. Call `evaluation::evaluate(events, config)`
6. Serialize `EvaluationReport` to JSON

Per review F7: For v1, compute on every call. Caching deferred to follow-up.

**Test scenarios:**
- Tool returns valid JSON with efficiency_score 0–100
- Tool works with empty metrics (baseline score)
- Tool works with metrics lacking agent_role (attributes to "anonymous")
- Tool appears in dispatch routing
- Invalid branch parameter returns helpful error
<!-- SECTION:DESCRIPTION:END -->
