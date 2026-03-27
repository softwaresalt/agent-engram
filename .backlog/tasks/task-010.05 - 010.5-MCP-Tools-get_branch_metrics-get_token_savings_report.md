---
id: TASK-010.05
title: '010.5: MCP Tools (get_branch_metrics, get_token_savings_report)'
status: To Do
assignee: []
created_date: '2026-03-27 05:50'
labels:
  - task
dependencies:
  - TASK-010.04
parent_task_id: TASK-010
priority: medium
ordinal: 5000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Implement two new MCP tools and extend the health report with metrics data.

**`get_branch_metrics`** (R10):
- Parameters: `branch_name: Option<String>` (defaults to current), `compare_to: Option<String>`
- Returns `MetricsSummary` for requested branch via `compute_summary()`
- If `compare_to` provided, returns both summaries plus a delta section
- No workspace → error 1001 (`WORKSPACE_NOT_SET`)
- No metrics for branch → error 13002 (`METRICS_NOT_FOUND`)

**`get_token_savings_report`** (R12):
- Parameters: none (always uses current branch)
- Returns formatted text: "On branch {branch}, engram delivered {N} tokens across {M} tool calls. Average {avg} tokens per call. Most-queried symbols: {top 5}."
- Phase 2 extension point: append savings sentence when multipliers configured

**Registration:**
- Add both tools to `all_tools()` in `src/shim/tools_catalog.rs` with JSON schemas
- Add dispatch entries in `src/tools/mod.rs`
- Increment `TOOL_COUNT` from 14 to 16

**Health report extension** (R11):
- Add `metrics_summary` field to `get_health_report` response: current branch, total tokens delivered, total tool calls tracked, time range
- Return `null` if no metrics exist yet

**Files to modify:** `src/tools/read.rs` (edit), `src/tools/mod.rs` (edit), `src/shim/tools_catalog.rs` (edit)
**Test file:** `tests/contract/metrics_tools_test.rs` (new)
**Cargo.toml:** Add `[[test]]` block: `name = "contract_metrics_tools"`, `path = "tests/contract/metrics_tools_test.rs"`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 get_branch_metrics returns valid MetricsSummary after recording events
- [ ] #2 get_branch_metrics with non-existent branch returns error 13002 (METRICS_NOT_FOUND)
- [ ] #3 get_branch_metrics without workspace returns error 1001 (WORKSPACE_NOT_SET)
- [ ] #4 get_branch_metrics with compare_to returns both summaries and delta
- [ ] #5 get_token_savings_report returns formatted text summary
- [ ] #6 get_health_report includes metrics_summary field
- [ ] #7 Tool catalog count matches dispatch table (TOOL_COUNT = 16)
<!-- AC:END -->
