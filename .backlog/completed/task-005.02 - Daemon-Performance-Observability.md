---
id: TASK-005.02
title: '005-02: Daemon Performance Observability'
status: Done
assignee: []
created_date: '2026-03-09'
labels:
  - feature
  - 005
  - userstory
  - p1
dependencies: []
references:
  - specs/005-lifecycle-observability/spec.md
parent_task_id: TASK-005
priority: high
---

## Description

<!-- SECTION:DESCRIPTION:BEGIN -->
As a developer running the engram daemon across multiple workspaces, I need visibility into daemon performance metrics — wake/sleep cycles, query latency, file watcher throughput, and memory consumption — so I can diagnose bottlenecks and verify the daemon is operating correctly during extended unattended sessions.

The daemon emits structured trace spans for all significant operations: startup, shutdown, tool call processing, file event handling, database queries, and hydration/dehydration cycles. These traces are available both in local log files and optionally exported to external observability collectors for aggregation.

**Why this priority**: The daemon runs as an unattended background service for hours or days. Without performance observability, diagnosing slow queries, stalled file watchers, or memory leaks requires reproducing the exact scenario. Structured traces are the primary diagnostic tool for production issues.

**Independent Test**: Can be fully tested by starting the daemon, performing several tool calls and triggering file events, then inspecting the structured log output for the expected trace spans with timing data. Delivers immediate value: operators can verify daemon health and diagnose performance issues without restarting.
<!-- SECTION:DESCRIPTION:END -->

## Acceptance Criteria
<!-- AC:BEGIN -->
- [x] #1 **Given** a running daemon with observability enabled, **When** a tool call is processed, **Then** the structured log contains a span with the tool name, execution duration, workspace ID, and success/failure status.
- [x] #2 **Given** a running daemon with a file watcher active, **When** a file change is detected and processed, **Then** the structured log contains spans for event detection, debounce processing, and database update, each with timing data.
- [x] #3 **Given** a daemon that has been idle and enters the TTL sleep cycle, **When** the daemon wakes on a new tool call, **Then** the structured log contains a span for the wake event including time-since-sleep and re-initialization duration.
- [x] #4 **Given** a daemon with optional trace export configured, **When** traces are generated, **Then** spans are exported to the configured collector endpoint in addition to local log output. ---
<!-- AC:END -->

## Implementation Plan

<!-- SECTION:PLAN:BEGIN -->
## Phase 4: User Story 2 — Daemon Performance Observability (Priority: P1)

**Goal**: Emit structured trace spans for all daemon operations with optional OTLP export

**Independent Test**: Start daemon, perform tool calls, inspect structured log for trace spans with timing data

### Tests for User Story 2 ⚠️

- [X] T027 [P] [US2] Contract test: tool call emits span with tool name, workspace_id, duration (S057) in tests/contract/observability_test.rs
- [X] T028 [P] [US2] Contract test: get_health_report returns all expected metrics (S056) in tests/contract/observability_test.rs
- [X] T029 [P] [US2] Contract test: get_health_report works without workspace binding (S060) in tests/contract/observability_test.rs

### Implementation for User Story 2

- [X] T030 [US2] Add latency tracking to AppState in src/server/state.rs — query_latencies VecDeque, tool_call_count AtomicU64, watcher_event_count AtomicU64, last_watcher_event RwLock
- [X] T031 [US2] Add #[instrument] tracing spans to all tool dispatch paths in src/tools/mod.rs — record tool name, workspace_id, duration
- [X] T032 [P] [US2] Add tracing spans to file watcher event processing in src/daemon/watcher.rs — event_detected, debounce_complete, db_update
- [X] T033 [P] [US2] Add tracing spans to TTL lifecycle events in src/daemon/ttl.rs — wake, sleep, expiry
- [X] T034 [US2] Implement get_health_report tool in src/tools/read.rs — returns version, uptime, memory, latency percentiles (p50/p95/p99), watcher status, connection count
- [X] T035 [US2] Register get_health_report in src/tools/mod.rs dispatch
- [X] T036 [US2] Implement OTLP export setup in src/server/observability.rs (behind otlp-export feature flag) — tracing-opentelemetry layer added to subscriber stack when ENGRAM_OTLP_ENDPOINT is set

**Checkpoint**: All daemon operations emit structured trace spans, health metrics available

---
<!-- SECTION:PLAN:END -->

