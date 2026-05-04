---
title: "SQLITE_BUSY Retry Metrics & Alerting"
description: "Add a metrics counter for SQLITE_BUSY retry events to enable automated storm detection"
topic: "Structured alerting for SQLITE_BUSY retry rate"
depth: "lightweight"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-05-04-sqlite-busy-retry-metrics-plan.md"
tags:
  - "observability"
  - "sqlite-busy"
  - "metrics"
  - "daemon-reliability"
source_stash_ids:
  - "51B936CD"
---

## Problem Frame

The daemon's `run_script_busy_retry_mutable` helper (src/db/cozo_queries.rs)
emits `tracing::warn!` on each SQLITE_BUSY retry attempt. This provides
visibility via log scraping but does not support automated alerting. Operators
cannot detect retry storms without manually watching logs.

A metrics counter would allow threshold-based alerts (e.g., "more than N
retries in M seconds") to fire automatically, improving production reliability
without requiring human log monitoring.

Source: `51B936CD` from docs/closure/2026-05-04-039-F-daemon-reliability-phase3-closure.md

## Research Findings

- **Existing retry mechanism**: `run_script_busy_retry_mutable` at
  src/db/cozo_queries.rs:310-344 — 5 attempts, 50ms→500ms exponential backoff,
  `tracing::warn!` on each retry with attempt count, max_attempts, delay_ms, error.
- **Existing metrics module**: src/services/metrics.rs — focused on tool-call
  token usage via JSONL append + mpsc channel. Not designed for daemon-internal
  counters but the pattern (mpsc sender, background writer) is reusable.
- **OpenTelemetry support**: Optional `otlp-export` feature in Cargo.toml
  (tracing-opentelemetry 0.26). When enabled, tracing spans and events are
  forwarded to an OTLP collector, meaning the existing `tracing::warn!` events
  would already be observable via OTLP. A dedicated counter metric would provide
  cleaner alerting than parsing warn-level log events.
- **Error codes**: 13xxx range reserved for metrics subsystem.

## Options Evaluated

### Option A: Add a dedicated `AtomicU64` counter + MCP query tool

Increment an in-process atomic counter on each retry. Expose via a new MCP
tool (`get_retry_metrics`) for on-demand query. No external dependencies.

- **Pros**: Zero new dependencies, simple, works without OTLP
- **Cons**: Counter resets on daemon restart, no automatic alerting without
  external polling
- **Effort**: Low
- **Fit**: Good for local diagnostics; requires external polling for alerts

### Option B: Emit an OpenTelemetry counter metric via tracing layer

Use the existing `tracing::warn!` span with a structured field that
tracing-opentelemetry maps to a metric. When OTLP export is enabled, the
counter flows to the collector automatically.

- **Pros**: Leverages existing infrastructure, automatic export, no new tools
- **Cons**: Only works when `otlp-export` feature is enabled; invisible otherwise
- **Effort**: Low
- **Fit**: Best when OTLP infrastructure exists

## Decision

**Option A** — Add an `AtomicU64` retry counter in the CozoDB query module,
exposed via a new MCP tool `get_retry_metrics`. This provides immediate value
regardless of whether OTLP export is configured. Future work can bridge this
counter to OpenTelemetry when the `otlp-export` feature is active.

Rationale: The daemon is local-first and may run without external observability
infrastructure. An in-process counter with MCP query access gives operators
immediate visibility without requiring OTLP setup.

## Rejected Alternatives

Option B rejected for now because it only helps when OTLP is configured. The
local-first daemon philosophy means core observability should work without
external dependencies. Option B can be added later as a bridge.

## Unresolved Questions

- Should the counter include a sliding-window rate (retries/sec) or just a
  monotonic total? Decision: start with monotonic total + last-retry timestamp;
  rate calculation is a client concern.
- Should a threshold-exceeded event emit a higher-severity tracing event
  (e.g., `tracing::error!` when > N retries in M seconds)? Decision: defer to
  a follow-up task; keep scope minimal for this shipment.

## Risks and Mitigations

| Risk | Mitigation |
|------|-----------|
| Counter resets on restart | Acceptable for local daemon; document in tool response |
| AtomicU64 contention | Negligible — single increment per retry, low frequency |
| MCP tool adds surface area | Minimal — read-only query, no state mutation |
