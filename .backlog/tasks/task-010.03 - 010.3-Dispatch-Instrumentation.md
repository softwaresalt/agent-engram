---
id: TASK-010.03
title: '010.3: Dispatch Instrumentation'
status: To Do
assignee: []
created_date: '2026-03-27 05:50'
labels:
  - task
dependencies:
  - TASK-010.01
  - TASK-010.02
parent_task_id: TASK-010
priority: medium
ordinal: 3000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Modify `dispatch()` in `src/tools/mod.rs` to measure response payload size after tool execution and emit a UsageEvent to the MetricsCollector.

**Measurement point** (after tool returns Result<Value>, before function returns):
1. Execute tool (existing match block)
2. On `Ok(value)`:
   a. Measure bytes: `value.to_string().len()` (infallible `Display` impl — do NOT use `serde_json::to_string()` which is fallible)
   b. Estimate tokens = bytes / 4
   c. Extract symbols_returned/results_returned from value (tool-specific)
   d. Get branch from `state.snapshot_workspace()`
   e. Send UsageEvent to MetricsCollector via `record()`
3. Record latency (existing)
4. Return result (existing)

**Tool-specific result counting** (via `value.get("field")` on serde_json::Value):
- `map_code`: neighbors array length + 1 (root)
- `list_symbols`: read `total_count` field
- `unified_search`: results array length
- `impact_analysis`: code_neighborhood array length
- `query_memory`: results array length
- `query_graph`: read `row_count` field
- Lifecycle/write tools: skip metrics recording (non-goals)

**Note:** `connection_id` is omitted in Phase 1 (field is Optional). Threading it through dispatch would change the signature and all call sites.

**Files to modify:** `src/tools/mod.rs` (edit)
**Test file:** `tests/contract/metrics_contract_test.rs` (new)
**Cargo.toml:** Add `[[test]]` block: `name = "contract_metrics"`, `path = "tests/contract/metrics_contract_test.rs"`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 dispatch records UsageEvent with correct tool_name and non-zero response_bytes for read tools
- [ ] #2 dispatch does NOT record UsageEvent for lifecycle/write tools
- [ ] #3 estimated_tokens equals response_bytes / 4
- [ ] #4 symbols_returned correctly extracted per tool type (map_code neighbors, list_symbols total_count, etc.)
<!-- AC:END -->
