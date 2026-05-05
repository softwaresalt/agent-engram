---
title: "Expose SQLITE_BUSY retry counters via AtomicU64 statics and a no-binding MCP tool"
domain: data-plane
tags: [sqlite, retry, cozo, observability, mcp-tool, atomics]
evidence: 040-F, PR #78, 38ae7e0
confidence: high
date: 2026-05-04
---

## Problem

After adding per-statement SQLITE_BUSY retry in `run_script_busy_retry_mutable`
(see `sqlite-busy-retry-granularity-2026-05-03.md`), operators had no way to
observe retry frequency without grepping logs. Silent retries made it impossible
to know whether the daemon was experiencing transient contention or persistent pressure.

## Solution Pattern

Add two `AtomicU64` process-global statics to `src/db/cozo_queries.rs`:

```rust
static MUTABLE_RETRY_COUNT: AtomicU64 = AtomicU64::new(0);
static MUTABLE_LAST_RETRY_EPOCH_MS: AtomicU64 = AtomicU64::new(0);
```

Increment them inside the retry branch (every retry, including the first):

```rust
MUTABLE_RETRY_COUNT.fetch_add(1, Ordering::Relaxed);
// `0` is the sentinel for "no retry yet"; clamp to at least 1ms to
// avoid writing the sentinel and masking a real retry.
let now_ms = u64::try_from(Utc::now().timestamp_millis())
    .unwrap_or(0)
    .max(1);
MUTABLE_LAST_RETRY_EPOCH_MS.store(now_ms, Ordering::Relaxed);
```

The `if attempt > 0` guard was a design iteration that was not shipped. In the
final implementation, `fetch_add` is called unconditionally within the retry
branch (guarded by `attempt + 1 < MAX_ATTEMPTS`). The `.max(1)` applies only
to the epoch timestamp, clamping it to ≥ 1 ms so the `0` sentinel is never
overwritten by a real retry.

Expose a snapshot via `get_mutable_script_retry_metrics` MCP tool (no workspace
binding required — reads process-global state directly):

```json
{
  "retry_count": 42,
  "last_retry_at": "2026-05-04T17:30:00.000Z"
}
```

`last_retry_at` is `null` when no retries have occurred in this process lifetime.

## Key Design Decisions

1. **`Ordering::Relaxed`**: Monotonic counters for telemetry — cross-thread
   ordering and immediate visibility don't matter; `fetch_add` is always
   atomic so no increment is ever lost, but a concurrent `load` may observe
   a slightly stale snapshot.
2. **No workspace binding required**: Counter reads don't touch the DB or workspace
   state. This makes the tool usable even before `set_workspace` succeeds.
3. **Process lifetime scope**: Counters reset on daemon restart. This is intentional —
   they indicate recent contention pressure, not historical totals.
4. **`epoch_ms != 0` sentinel**: `last_retry_at` is null when the static is still at
   its initialized-to-zero value. `chrono::DateTime::from_timestamp_millis(0)` returns
   `Some(epoch)`, not `None` — the sentinel check must happen before calling chrono.

## Test Invariants

- `retry_count` is a `u64` (non-negative integer ≤ u64::MAX) — validated with
  `as_u64().is_some()` in contract tests.
- Two consecutive reads of `retry_count` in a clean test must be non-decreasing
  (monotonicity invariant — validated in unit tests).

## Follow-up Opportunity

OTLP bridge: expose `MUTABLE_RETRY_COUNT` as an OpenTelemetry counter gauge
so dashboards can track retry rate over time without polling the MCP tool.
