---
title: Engram usage-telemetry consumption contract
date: 2026-07-05
status: stable
schema_version: 2
shipment: 075-S
related:
  - decision-017 - Engram-usage-telemetry-emit-not-ingest-pivot.md
  - docs/decisions/2026-07-05-engram-usage-measurement-decision.md
  - src/models/metrics.rs
---

# Engram usage-telemetry consumption contract

This document is the **stable, versioned contract** autoharness (or any consumer)
uses to measure how much it exercises engram. It describes the on-disk usage
record, the derived adoption metrics, and where to read them.

## 1. Source of truth: `usage.jsonl`

Engram appends one JSON object per line to:

```text
{workspace}/.engram/metrics/{branch}/usage.jsonl
```

- **Append-only**, one `UsageEvent` per line. Rotated to `usage.N.jsonl` when it
  exceeds the configured `max_file_bytes` (default 10 MiB, up to
  `max_rotated_files` generations).
- Records are **tool-managed state** — do not hand-edit.
- The pinned schema version is `2` (`USAGE_SCHEMA_VERSION` in
  `src/models/metrics.rs`). New fields are **additive-only**; existing fields are
  never renamed or removed, so older records keep deserializing.

### 1.1 `UsageEvent` fields (consumer-relevant subset)

| Field | Type | Meaning |
|---|---|---|
| `tool_name` | string | MCP tool method (e.g. `unified_search`, `map_code`). |
| `timestamp` | string (RFC 3339 UTC) | When the call was dispatched. |
| `correlation_id` | string, optional | Caller-supplied join key that ties the call to a harness task/session. Omitted when neither the MCP `_meta.correlation_id` nor the CLI `--correlation-id` / `ENGRAM_CORRELATION_ID` was supplied. |
| `latency_ms` | u64 | Dispatch latency of the call. |
| `branch` | string | Git branch the call was recorded under. |
| `workspace` | string | Resolved workspace root. |
| `outcome` | string | `success` or `error`. |
| `estimated_input_tokens` / `estimated_output_tokens` | u64 | Coarse token estimates (`bytes / 4`). |
| `result_count` | u32 | Canonical result-item count. |
| `params_summary` | object, optional | Coarse, privacy-preserving summary (`query_hash`, `query_len`, `limit`) — never raw query text. |
| `schema_version` | u32 | Record schema version (`2`). |

## 2. Which tools are measured

Emission is gated at the dispatch choke point by an allowlist
(`should_record_metrics` in `src/tools/mod.rs`). All agent-facing query and
lifecycle tools emit, including `set_workspace`, `sync_workspace`,
`index_workspace`, `unified_search`, `query_memory`, `map_code`, `list_symbols`,
`impact_analysis`, `query_graph`, the observability/report tools, and (under the
`git-graph` build) `query_changes` and `index_git_history`.

**Intentionally excluded:** `flush_state` — an internal lifecycle operation, not
an agent-facing query. Excluding it keeps adoption counts free of daemon
housekeeping.

## 3. Adoption metrics

Two equivalent ways to obtain the metrics:

### 3.1 Ask engram (preferred)

Call the `get_token_savings_report` MCP tool. In addition to the legacy `branch`
and prose `report` fields, the response carries an additive `metrics` object:

```jsonc
{
  "branch": "main",
  "report": "On branch main, engram handled ...",
  "metrics": {
    "schema_version": 2,
    "total_tool_calls": 42,
    "unique_tools_exercised": 7,        // adoption BREADTH
    "distinct_correlation_ids": 5,      // adoption REACH
    "time_range": { "start": "...", "end": "..." },
    "by_tool": { "map_code": { "call_count": 12, ... }, ... },
    "by_correlation_id": {
      "task-042.003.001-T": {
        "call_count": 9,
        "unique_tools": 4,
        "time_range": { "start": "...", "end": "..." }
      }
    }
  }
}
```

**Surface placement.** The two cheap scalar counts (`unique_tools_exercised`,
`distinct_correlation_ids`) also appear additively on `MetricsSummary` wherever
it is serialized — `get_branch_metrics`, `get_health_report`'s `metrics_summary`,
and the on-disk `summary.json`. The heavy `by_correlation_id` map is emitted
**only** by `get_token_savings_report`, so frequently-polled tools stay lean.

> **Note on `session_count`.** `MetricsSummary` also carries a `session_count`
> field derived from `connection_id`. No production emit path currently sets
> `connection_id`, so `session_count` is effectively always `0` today. It is
> intentionally **not** part of the adoption `metrics` object. Do not use it as
> an adoption signal until connection plumbing lands.

### 3.2 Compute from `usage.jsonl` directly

A consumer that prefers to aggregate itself derives the same metrics per branch:

| Metric | Definition |
|---|---|
| `total_tool_calls` | count of records |
| `unique_tools_exercised` | count of distinct `tool_name` |
| `distinct_correlation_ids` | count of distinct non-empty `correlation_id` |
| `by_correlation_id[cid].call_count` | records with that `correlation_id` |
| `by_correlation_id[cid].unique_tools` | distinct `tool_name` within that `correlation_id` |
| `by_correlation_id[cid].time_range` | min/max `timestamp` within that `correlation_id` |

Records with no `correlation_id` count toward `total_tool_calls` and
`unique_tools_exercised` but do **not** create a `by_correlation_id` bucket.

## 4. Measuring "how much does autoharness use engram?"

The intended reading:

- **Breadth** — `unique_tools_exercised` shows how much of engram's surface the
  harness touches. A harness that only calls one tool is under-adopting.
- **Reach** — `distinct_correlation_ids` shows how many harness tasks/sessions
  reached for engram at all. Compare against the number of harness tasks run to
  get an adoption rate.
- **Depth** — `by_correlation_id[*].call_count` and `unique_tools` show how
  intensively a single task used engram.

For a reliable adoption rate, autoharness MUST pass a stable `correlation_id`
per task/session on every engram call (MCP `_meta.correlation_id` or the CLI
`--correlation-id` flag / `ENGRAM_CORRELATION_ID`).

## 5. Stability guarantees

- `schema_version` is bumped only on breaking changes to the record shape.
- Fields are additive-only within a schema version.
- The `get_token_savings_report` `metrics` object is additive to the existing
  response; the `branch` and `report` fields are preserved.
