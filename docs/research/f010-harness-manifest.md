---
title: F010 Harness Manifest
date: 2026-03-27
---

# F010 Harness Manifest

**Feature**: 010 — Effectiveness Metrics & Token Usage Tracking
**Generated**: 2026-03-27
**Branch**: 010-effectiveness-metrics-token-usage-tracking
**Compilation**: PASS
**Red Phase**: CONFIRMED (9 tests fail with unimplemented!/not-registered)

## Test Files

| Tier | Path | Test Count |
|------|------|------------|
| unit | tests/unit/metrics_model_test.rs | 6 |
| unit | tests/unit/metrics_collector_test.rs | 4 |
| contract | tests/contract/metrics_contract_test.rs | 3 |
| integration | tests/integration/metrics_persistence_test.rs | 4 |
| contract | tests/contract/metrics_tools_test.rs | 7 |

## Stub Files

| Path | Symbols |
|------|---------|
| src/models/metrics.rs | MetricsMessage, UsageEvent, MetricsSummary, ToolMetrics, SymbolCount, TimeRange, MetricsConfig, MetricsSummary::from_events |
| src/services/metrics.rs | record, compute_summary, compute_and_write_summary |

## Subtask Mapping

| Subtask | Title | Test Function(s) | Harness Command | Status |
|---------|-------|------------------|-----------------|--------|
| TASK-010.01 | UsageEvent Model & MetricsConfig | t010_01_usage_event_serde_round_trip, t010_01_usage_event_none_connection_id_omitted, t010_01_metrics_summary_from_events, t010_01_metrics_config_defaults, t010_01_metrics_config_partial_toml, t010_01_btreemap_deterministic_ordering | `cargo test --test unit_metrics_model` | RED |
| TASK-010.02 | Metrics Collector Service | t010_02_record_does_not_block_when_full, t010_02_usage_event_to_jsonl, t010_02_compute_summary_aggregation, t010_02_compute_summary_partial_line_tolerance | `cargo test --test unit_metrics_collector` | RED |
| TASK-010.03 | Dispatch Instrumentation | t010_03_dispatch_records_usage_event_for_read_tools, t010_03_dispatch_skips_lifecycle_tools, t010_03_estimated_tokens_equals_bytes_div_4 | `cargo test --test contract_metrics` | RED (placeholders) |
| TASK-010.04 | Branch-Aware Persistence & Flush | t010_04_flush_creates_summary_json, t010_04_branch_isolation, t010_04_append_after_restart, t010_04_metrics_dir_not_in_gitignore | `cargo test --test integration_metrics_persistence` | RED |
| TASK-010.05 | MCP Tools | t010_05_get_branch_metrics_returns_summary, t010_05_get_branch_metrics_not_found, t010_05_get_branch_metrics_no_workspace, t010_05_get_branch_metrics_compare, t010_05_get_token_savings_report, t010_05_health_report_includes_metrics, t010_05_tool_count_matches_catalog | `cargo test --test contract_metrics_tools` | RED |
| TASK-010.06 | Error Codes (13xxx) | (errors validated via tools tests) | `cargo test --test contract_metrics_tools -- t010_05_get_branch_metrics_not_found` | RED |
| TASK-010.07 | Baseline Analysis Script | N/A (PowerShell) | N/A | SKIPPED |
| TASK-010.08 | Calibration Report Script | N/A (PowerShell) | N/A | SKIPPED |

## Cargo.toml Registration

```toml
[[test]]
name = "unit_metrics_model"
path = "tests/unit/metrics_model_test.rs"

[[test]]
name = "unit_metrics_collector"
path = "tests/unit/metrics_collector_test.rs"

[[test]]
name = "contract_metrics"
path = "tests/contract/metrics_contract_test.rs"

[[test]]
name = "integration_metrics_persistence"
path = "tests/integration/metrics_persistence_test.rs"

[[test]]
name = "contract_metrics_tools"
path = "tests/contract/metrics_tools_test.rs"
```

## Notes

- Contract tests for dispatch instrumentation (TASK-010.03) use placeholder assertions because the MetricsCollector infrastructure to capture/inspect emitted events does not exist yet. The Worker must add test inspection hooks during implementation.
- TASK-010.06 (Error Codes) is validated transitively through the metrics tools contract tests which assert specific error codes (13002 for not-found, 1003 for no-workspace).
- TASK-010.07 and TASK-010.08 are PowerShell scripts — no Rust harness generated. They have Pester/-Validate test requirements in their task descriptions.
- Execution posture for all tasks: test-first.
