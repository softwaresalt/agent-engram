---
title: "Code Review: Feature 010 Effectiveness Metrics & Token Usage Tracking"
date: 2026-03-27
mode: interactive
gate: pass
reviewers: [Rust Safety Reviewer, Constitution Reviewer, MCP Protocol Reviewer, Concurrency Reviewer, Learnings Researcher]
---

# Code Review: Feature 010 Effectiveness Metrics & Token Usage Tracking

## Summary

| Severity | Count | Action                       |
|----------|-------|------------------------------|
| P0       | 0     |                              |
| P1       | 1     | Fixed (spawn_blocking)       |
| P2       | 3     | 2 fixed (saturating), 1 advisory |
| P3       | 0     |                              |

## Findings

### src/services/metrics.rs — Blocking I/O in async context (P1, fixed)

`compute_summary()` performs synchronous `std::fs::File::open` + `BufReader` reads. Called from async tool handlers without `spawn_blocking`, this blocks the tokio executor thread pool.

**Resolution:** Wrapped all `compute_summary` calls in `tokio::task::spawn_blocking` within `compute_and_write_summary`, `get_health_report`, `get_branch_metrics`, and `get_token_savings_report`.

### src/models/metrics.rs — Unchecked integer addition (P2, fixed)

`from_events()` used `+=` for `total_tokens` and per-tool counters without overflow protection.

**Resolution:** Replaced with `saturating_add()` on lines 127, 135, 136.

### src/tools/read.rs — Delta subtraction edge case (P2, fixed)

Token savings delta used unchecked subtraction between `i64` values derived from `u64::try_from` with `i64::MAX` fallback.

**Resolution:** Replaced with `saturating_sub()` on lines 865-868.

### src/services/metrics.rs — Initialize-shutdown race window (P2, advisory)

Between `shutdown()` completing and new sender installation, concurrent `record()` calls silently drop events. The window is narrow (single mutex swap) and workspace rebind is rare.

**Resolution:** Documented as advisory. Low practical risk for v1. Can be addressed by preparing the new channel before shutdown if event loss during rebind becomes measurable.

## Learnings Applied

No prior compound documents found in `.backlog/compound/`.

## Residual Work

The initialize-shutdown race window (P2 advisory) remains as a known limitation. Consider addressing if metrics accuracy during workspace rebind becomes important.
