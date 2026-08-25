---
title: Align the tracing bridge and repair OpenTelemetry 0.26 lifecycle usage
type: decision
doc_type: decision
source: stash 44E573BC
date: 2026-08-24
status: decided
source_stash_id: 44E573BC
promoted_to:
  - docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md
---

# Align the tracing bridge and repair OpenTelemetry 0.26 lifecycle usage

## Problem Frame

The optional `otlp-export` feature does not compile because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while Engram directly pins 0.26, and the observability builder uses APIs unavailable in 0.26. The runtime design also drops its provider and lacks explicit cleanup ownership.

Pinned SDK 0.26 source shows that `force_flush()` and `shutdown()` enqueue messages and block on untimed oneshot responses. `max_export_timeout` races each exporter batch future only. It cannot establish a whole-call or two-call bound.

## Decision

Use four compiling RED boundaries and thirteen linear tasks.

1. Build the compile-neutral outer meta-harness with `--no-default-features --features cozo-backend`; nested OTLP tree/check commands use `--features cozo-backend,otlp-export`.
2. Align only `tracing-opentelemetry` to 0.27 and the lockfile, then repair only the pinned-0.26 source compile baseline.
3. Add a behavior-neutral exporter/tracing seam before feature-enabled tests.
4. Run the provider RED with `cargo test --no-default-features --features cozo-backend,otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture`.
5. Retain the provider and set `OTLP_EXPORT_TIMEOUT = 5s` after defaults. This bounds each exporter future only and may drop a never-ready export; it does not bound synchronous cleanup.
6. Add the daemon endpoint/attachment RED and sequential endpoint and retention GREENs.
7. Add a cleanup RED using deterministic synchronous fake methods and phase barriers.
8. Move the explicit provider owner into one dedicated detached `std::thread`. The worker calls `force_flush` once and calls `shutdown` once only after flush returns. The daemon waits once for at most `OTLP_CLEANUP_WAIT_TIMEOUT = 5s` on a phase/completion channel.
9. On deadline, do not join or claim cancellation/completion. Return/log `completion=unknown`, last phase, wait limit, and detached-worker/resource residual. A clean daemon returns cleanup failure; a daemon error remains primary when both fail.

A native detached worker is deliberate: Tokio `spawn_blocking` tasks may delay runtime shutdown, while a dropped native `JoinHandle` is not joined and does not keep the daemon process alive after main returns. A timed-out SDK call may continue until normal process termination; an embedding that keeps the process alive is outside this design and prohibited without a reaper.

## Task and Shipment Decision

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T
```

Thirteen tasks create exactly twelve task dependency edges. Each task is 45-115 minutes, at most two files/evidence surfaces, at most four functions, at most three scenarios, one domain, and one atomic milestone. Cleanup isolation fits U11 at 115 minutes, so no extra task is needed. Shipment `125-S` remains fourteen items, sole queued, and unclaimed.

## Runtime and Rollback Decision

Rerun all four corrected RED commands unchanged. Deterministic tests separately prove per-export cancellation and a bounded daemon wait whose timeout leaves synchronous completion unknown. A controlled child process held past the cleanup wait must exit within five seconds plus a two-second harness allowance, proving no join/runtime-blocking dependency rather than SDK completion.

For 30 minutes or three controlled exits, observe export failures, worker spawn/loss/panic, cleanup failures, cleanup-wait timeouts, detached-worker outcomes, and total exit latency. Disable `otlp-export` and revert the owning GREEN commits on any failure/timeout, hidden residual, child exit beyond seven seconds, missing span, or feature-gate regression.

## Constraints

- Never describe `force_flush()` or `shutdown()` as cancellable or operation-bounded in SDK 0.26.
- Never infer a cleanup-call deadline from `max_export_timeout`.
- No `spawn_blocking`, runtime-owned cleanup task, or production join.
- No external collector, socket, credential, network oracle, sleep polling, or retry.
- No test command may omit required `cozo-backend` when defaults are disabled.
- No task mixes Cargo graph, provider construction, endpoint wiring, and cleanup coordination.
- PR #362 and blocked workspace-identity shipments remain untouched.

## References

- PR 363 reviews `5015373740` and `5015447062`
- `Cargo.toml`, `Cargo.lock`
- Pinned `opentelemetry_sdk` 0.26 `trace/provider.rs` and `trace/span_processor.rs`
- `src/server/observability.rs`, `src/lib.rs`, `src/bin/engram.rs`
- `docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md`
