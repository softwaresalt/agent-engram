---
id: decision-018
title: Engram usage MEASUREMENT — close the emission-to-measurement gap
date: 2026-07-05
status: Accepted
author: orchestrator
source_stash: A7F72BC0
shipment: 075-S
feature: 073-F
related:
  - decision-017 - Engram-usage-telemetry-emit-not-ingest-pivot.md
  - docs/exec-plans/2026-07-02-engram-usage-telemetry-emit-plan.md
  - docs/design-docs/engram-usage-telemetry-consumption-contract.md
---

## Context

Shipment 067-S delivered usage-telemetry **emission**: every measured engram
tool call appends a `UsageEvent` to `.engram/metrics/{branch}/usage.jsonl` with a
caller-supplied `correlation_id`, `latency_ms`, `workspace`, coarse
`params_summary`, and a pinned `schema_version`. Emission alone does not answer
the operator's question: **how much does autoharness actually use engram?**

Stash `A7F72BC0` asked to close the gap between EMISSION and MEASUREMENT across
four pillars: coverage, correlation, measurement/reporting, and verification.

## Decision

Ship a bounded MEASUREMENT slice (075-S) rather than a new telemetry subsystem.

### 1. Coverage

- The dispatch choke point (`src/tools/mod.rs::dispatch`) is the single emission
  site; every allowlisted MCP tool flows through it. Audit confirmed the
  allowlist covers all agent-facing query/lifecycle tools.
- **Close the `index_git_history` gap** — it is an agent-facing analysis tool, so
  it now emits under the same `git-graph` cfg gate as `query_changes`.
- **Keep `flush_state` excluded** — it is an internal lifecycle operation, not an
  agent-facing query, so it is intentionally outside the adoption-measurement
  surface. Recording it would inflate adoption counts with daemon housekeeping.

### 2. Correlation

Correlation threading (MCP `_meta.correlation_id` + CLI `--correlation-id` /
`ENGRAM_CORRELATION_ID`, persisted on both the daemon and `--direct` paths) was
verified intact. `correlation_id` is the join key that ties engram usage back to
a specific harness task/session.

### 3. Measurement / reporting

Extend the existing `get_token_savings_report` MCP tool with an **additive**
structured `metrics` object (the prose `report` and `branch` fields are
unchanged). The aggregation (`MetricsSummary::from_events`) now also computes two
cheap scalar counts:

- `unique_tools_exercised` — adoption **breadth** (distinct tools touched).
- `distinct_correlation_ids` — adoption **reach** (distinct harness
  tasks/sessions that invoked engram at least once).

The heavy per-correlation breakdown is computed by a dedicated
`correlation_metrics(&events)` function and surfaced **only** by
`get_token_savings_report`:

- `by_correlation_id` — per-task/session rollup (`call_count`, `unique_tools`,
  `time_range`) so usage can be attributed to individual harness work units.

Keeping the map off the shared `MetricsSummary` struct prevents it from bloating
frequently-polled tools (`get_health_report`, `get_branch_metrics`) or the
persisted `summary.json` — the two scalar counts are cheap enough to appear
everywhere `MetricsSummary` is serialized. `session_count` (derived from a
`connection_id` that no production path currently sets) is deliberately excluded
from the adoption `metrics` object because it is always `0` today.

Events without a `correlation_id` still count toward the top-level totals but do
not create a `by_correlation_id` bucket.

### 4. Verification

Unit tests assert the correlation aggregation (including the empty-id edge case);
a contract test asserts the extended report shape end-to-end.

## Alternatives considered

- **New `get_usage_report` MCP tool** — rejected for now: adds tool-registration,
  manifest, and contract surface without extra value over extending the existing
  report. The stash explicitly sanctioned extending `get_token_savings_report`.
- **Ingest telemetry back into engram's graph** — already rejected by
  decision-017 (emit-not-ingest pivot). Not revisited.

## Consequences

- Autoharness can read adoption breadth/reach/per-task usage from a single tool
  call, or compute the same metrics directly from `usage.jsonl` using the pinned
  schema (see the consumption contract).
- The report output shape grows by one additive field; existing consumers keep
  working.

## Out of scope

- The `flush_state` exclusion could be revisited if a future consumer wants raw
  lifecycle-call counts.
- Longitudinal cross-branch aggregation (usage.jsonl is per-branch) is left to
  the consumer.
