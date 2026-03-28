---
id: TASK-010.01
title: '010.1: UsageEvent Model & MetricsConfig'
status: Done
assignee: []
created_date: '2026-03-27 05:49'
updated_date: '2026-03-27 21:24'
labels:
  - task
dependencies: []
parent_task_id: TASK-010
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Define core data structures for the metrics subsystem in `src/models/metrics.rs` (new file).

**Structs to create:**
- `MetricsMessage` enum: `Event(UsageEvent)`, `SwitchBranch(String)`, `Shutdown` — channel item type for background writer
- `UsageEvent`: tool_name, timestamp (RFC3339), response_bytes (u64), estimated_tokens (u64), symbols_returned (u32), results_returned (u32), branch, connection_id (Option)
- `MetricsSummary`: total_tool_calls, total_tokens, by_tool (`BTreeMap<String, ToolMetrics>` for deterministic ordering), top_symbols (`Vec<SymbolCount>`), time_range (`TimeRange`), session_count
- `SymbolCount`: name, count — named struct instead of tuple for clear JSON field names
- `TimeRange`: start, end — named struct instead of tuple
- `ToolMetrics`: call_count, total_tokens, avg_tokens (f64, needs `#[allow(clippy::cast_precision_loss)]`)
- `MetricsConfig`: enabled (bool, default true), buffer_size (usize, default 1024)

**Key patterns:**
- All structs derive `Debug, Clone, Serialize, Deserialize`
- Use `#[serde(skip_serializing_if = "Option::is_none")]` for optional fields
- Use `BTreeMap` not `HashMap` for `by_tool` (Constitution IX: sorted keys for Git diffability)
- Add `MetricsConfig` to `WorkspaceConfig` in `src/models/config.rs` with `#[serde(default)]`
- Add `///` rustdoc on all public items following `src/models/function.rs` style
- Re-export from `src/models/mod.rs`

**Files to modify:** `src/models/metrics.rs` (new), `src/models/mod.rs` (edit), `src/models/config.rs` (edit)
**Test file:** `tests/unit/metrics_model_test.rs` (new)
**Cargo.toml:** Add `[[test]]` block: `name = "unit_metrics_model"`, `path = "tests/unit/metrics_model_test.rs"`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 UsageEvent serializes to JSON and round-trips via serde_json
- [ ] #2 MetricsSummary computes correctly from a Vec of UsageEvents
- [ ] #3 MetricsConfig defaults to enabled=true buffer_size=1024
- [ ] #4 MetricsConfig deserializes from partial TOML (missing fields get defaults)
- [ ] #5 BTreeMap produces deterministic key ordering in serialized summary
- [ ] #6 Proptest: UsageEvent and MetricsSummary round-trip through serde_json
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented UsageEvent and MetricsConfig wiring, including config defaults and metrics summary aggregation updates.
<!-- SECTION:FINAL_SUMMARY:END -->
