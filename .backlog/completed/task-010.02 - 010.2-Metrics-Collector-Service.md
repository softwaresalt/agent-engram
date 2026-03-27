---
id: TASK-010.02
title: '010.2: Metrics Collector Service'
status: Done
assignee: []
created_date: '2026-03-27 05:49'
updated_date: '2026-03-27 21:24'
labels:
  - task
dependencies:
  - TASK-010.01
parent_task_id: TASK-010
priority: medium
ordinal: 2000
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
Create the `MetricsCollector` service in `src/services/metrics.rs` (new file) with a `tokio::sync::mpsc` bounded channel for async event recording.

**Architecture:**
- **Channel type**: `MetricsMessage` enum (from Unit 1): `Event(UsageEvent)`, `SwitchBranch(String)`, `Shutdown`
- **Sender side**: `record(event: UsageEvent)` — wraps in `MetricsMessage::Event`, calls `try_send`. If full, drops with `tracing::trace!("metrics_event_dropped")`. Zero latency impact.
- **Receiver side**: Background task via `tokio::spawn`. Reads messages, appends JSONL lines, switches branch output path, or shuts down.
- **Global singleton**: `OnceLock<mpsc::Sender<MetricsMessage>>` — no Mutex needed since `try_send` takes `&self`
- **JoinHandle**: Store in `AppState` for graceful shutdown (send `Shutdown`, await handle)

**Branch path management:**
- `WorkspaceSnapshot.branch` is ALREADY sanitized — do NOT call `sanitize_branch_for_path()` again
- Directory: `{workspace}/.engram/metrics/{snapshot.branch}/`
- Create directory on first write via `tokio::fs::create_dir_all`

**Summary computation:**
- `compute_summary(workspace_path, branch) -> Result<MetricsSummary>` reads `usage.jsonl` line-by-line
- Silently discards final line if parse fails (concurrent-append tolerance)
- Write `summary.json` via `atomic_write` from `src/services/dehydration.rs`

**Tracing instrumentation:**
- `#[instrument]` on background writer loop
- `tracing::trace!` on event drop (channel full)
- `tracing::info!` on branch switch
- `tracing::warn!` on write failures

**Lifecycle:**
- Initialize at server startup in `bin/engram.rs` after `AppState::new()`
- On shutdown: send `Shutdown` message, await `JoinHandle`, then `flush_all_workspaces()`

**Files to modify:** `src/services/metrics.rs` (new), `src/services/mod.rs` (edit), `src/server/state.rs` (edit — add JoinHandle)
**Test file:** `tests/unit/metrics_collector_test.rs` (new)
**Cargo.toml:** Add `[[test]]` block: `name = "unit_metrics_collector"`, `path = "tests/unit/metrics_collector_test.rs"`
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [ ] #1 record() does not block when channel is full
- [ ] #2 UsageEvent serializes to valid JSONL line (one JSON object per line)
- [ ] #3 compute_summary() produces correct aggregates from test JSONL string
- [ ] #4 Background writer correctly handles MetricsMessage::SwitchBranch
- [ ] #5 Background writer correctly handles MetricsMessage::Shutdown (drains buffer)
- [ ] #6 Partial-line tolerance: compute_summary discards unparseable final line
<!-- AC:END -->

## Final Summary

<!-- SECTION:FINAL_SUMMARY:BEGIN -->
Implemented the async metrics collector, recent-event ledger, branch switching, summary computation, and summary persistence helpers.
<!-- SECTION:FINAL_SUMMARY:END -->
