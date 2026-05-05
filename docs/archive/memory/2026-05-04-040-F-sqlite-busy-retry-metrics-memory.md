---
title: "040-F SQLITE_BUSY Retry Metrics — Session Memory"
date: 2026-05-04
feature: 040-F
shipment: 023-S
branch: feat/040-F-sqlite-busy-retry-metrics
status: pr-ready
---

## Tasks Completed

- **040.001-T** — AtomicU64 retry counter in `cozo_queries.rs` — DONE
- **040.002-T** — `get_mutable_script_retry_metrics` MCP tool in `tools/read.rs` — DONE

## Files Modified

| File | Change |
|---|---|
| `src/db/cozo_queries.rs` | Added `MUTABLE_RETRY_COUNT`/`MUTABLE_LAST_RETRY_EPOCH_MS` statics; `RetryMetrics` struct; `mutable_script_retry_metrics()`; `reset_retry_metrics()` (cfg(test)); increment in `run_script_busy_retry_mutable` |
| `src/db/mod.rs` | Re-exported `RetryMetrics` and `mutable_script_retry_metrics` |
| `src/tools/read.rs` | Added `get_mutable_script_retry_metrics` handler |
| `src/tools/mod.rs` | Added dispatch arm |
| `src/shim/tools_catalog.rs` | Updated `TOOL_COUNT` 17→18; added catalog entry; updated dispatch name list |
| `tests/unit/retry_metrics_test.rs` | New external unit test (2 tests) |
| `tests/contract/retry_metrics_tool_test.rs` | New contract test (3 tests) |
| `tests/contract/metrics_tools_test.rs` | Updated tool count assertion 17→18 |
| `Cargo.toml` | Added `unit_retry_metrics` and `contract_retry_metrics_tool` test targets |

## Commits

1. `f94878e` — `test(build): scaffold harness for 040-F sqlite busy retry metrics`
2. `9adf98e` — `feat(db): implement SQLITE_BUSY mutable-script retry metrics (040-F)`

## Decisions

- Used `Ordering::Relaxed` for both statics — acceptable for independent monotonic telemetry counters (no cross-atomic invariants required)
- Used `unwrap_or(0)` for timestamp conversion — acceptable since 0 is the "no retry" sentinel and negative `timestamp_millis()` requires clock before epoch
- Used `json!()` macro directly in tool handler (infallible, matches codebase pattern)
- `#[allow(clippy::unused_async)]` on tool handler — required by dispatch contract

## Review Gate

PASS — no P0/P1 findings. Two P3 advisory: independent atomic sampling (documented in function doc), timestamp sentinel overlap (acceptable for telemetry).

## Pre-Existing Failures

`contract_shim_lifecycle` (6 tests) fail on baseline and in feature branch — daemon spawn fails in this environment. Not caused by this feature.

## Next Steps

- Create PR for `feat/040-F-sqlite-busy-retry-metrics`
- Await operator merge approval
- Post-merge closure on `post-merge/040-F-sqlite-busy-retry-metrics`
