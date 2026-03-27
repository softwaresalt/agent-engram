---
id: TASK-010
title: 'Feature 010: Effectiveness Metrics & Token Usage Tracking'
status: Done
assignee: []
created_date: '2026-03-27 05:48'
updated_date: '2026-03-27 21:24'
labels:
  - feature
dependencies: []
references:
  - .backlog/plans/2026-03-27-engram-effectiveness-metrics-plan.md
  - .backlog/brainstorm/2026-03-27-engram-effectiveness-metrics-requirements.md
  - .backlog/reviews/2026-03-27-engram-effectiveness-metrics-plan-review.md
priority: medium
ordinal: 1000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Engram returns precise, pre-indexed results to AI coding assistants, but there is no way to measure whether this saves tokens compared to file-based search. This feature adds an accounting layer that connects tool responses to context window impact, persists measurements per feature branch as Git-trackable JSONL, and supports comparison against non-engram baselines captured from real Copilot CLI sessions.

**Two-phase approach:**
- Phase 1: Track engram volume only (no savings claims). Every MCP tool response records a UsageEvent to `.engram/metrics/{branch}/usage.jsonl`.
- Phase 2: Apply empirically calibrated multipliers derived from Copilot CLI baseline data to produce savings ratios with stated confidence levels.

**Key deliverables:**
- UsageEvent model and MetricsConfig
- Async MetricsCollector service with tokio::sync::mpsc channel
- Dispatch-level instrumentation for all MCP read tools
- Branch-aware JSONL persistence with summary computation on flush
- Two new MCP tools: `get_branch_metrics`, `get_token_savings_report`
- Health report extension with `metrics_summary`
- Post-hoc baseline extraction script for Copilot CLI session store
- Calibration report script for multiplier derivation

**Plan:** `.backlog/plans/2026-03-27-engram-effectiveness-metrics-plan.md`
**Requirements:** `.backlog/brainstorm/2026-03-27-engram-effectiveness-metrics-requirements.md`
**Review:** `.backlog/reviews/2026-03-27-engram-effectiveness-metrics-plan-review.md`
<!-- SECTION:DESCRIPTION:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented feature 010 end to end: metrics models, collector, dispatch instrumentation, branch-aware persistence, MCP reporting tools, baseline/calibration scripts, and focused verification. Verified with focused cargo tests, script validation, cargo fmt --check, and cargo clippy --all-targets -- -D warnings.
<!-- SECTION:FINAL_SUMMARY:END -->
