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

The optional `otlp-export` feature does not compile because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while Engram directly pins 0.26, and the observability builder uses APIs unavailable in 0.26. The runtime design returns only a layer and lacks a separately accessible application cleanup handle. Read-only pinned source corrects the prior assumption: `TracerProvider::library_tracer` passes `self.clone()` into `Tracer`, `Tracer` stores that provider clone, and `OpenTelemetryLayer` stores the tracer. Ending only the constructor-local provider binding therefore does not stop span processing.

Pinned SDK 0.26 source shows that `force_flush()` and `shutdown()` enqueue messages and block on untimed oneshot responses. `max_export_timeout` races each exporter batch future only. It cannot establish a whole-call or two-call bound.

## Decision

Use five compiling RED boundaries and sixteen linear tasks.

1. Build the compile-neutral outer meta-harness with `--no-default-features --features cozo-backend`; nested OTLP tree/check commands use `--features cozo-backend,otlp-export`.
2. Align only `tracing-opentelemetry` to 0.27 and the lockfile, then repair only the pinned-0.26 source compile baseline.
3. Add a behavior-neutral exporter/tracing result/control seam before feature-enabled tests. It preserves the layer-held tracer/provider clone and reports the separate application lifecycle handle as unavailable.
4. Run the provider RED with `cargo test --no-default-features --features cozo-backend,otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture`. A controlled local exporter first proves layer-held-clone export as an already-GREEN baseline; RED then fails on `LifecycleUnavailable`/missing explicit flush control and the missing source-owned per-export timeout.
5. Return a separately accessible application provider handle alongside the attachable layer and set `OTLP_EXPORT_TIMEOUT = 5s` after defaults. The layer already retains a clone; the separate handle exists to invoke and observe force flush/shutdown. The timeout bounds each exporter future only and may drop a never-ready export; it does not bound synchronous cleanup.
6. Add the daemon endpoint/attachment RED and sequential endpoint and retention GREENs. Flag/environment/absence precedence runs only in self-relaunched child test processes configured with `Command::env`/`env_remove`; no process-global mutation or serial lock is allowed.
7. Add a cleanup RED using deterministic synchronous fake methods and phase barriers.
8. Add a behavior-neutral injectable cleanup-worker spawner whose production adapter uses only `std::thread::Builder::spawn` and returns its `Result`; launch no worker yet.
9. Add a compile-then-fail RED whose fake spawner deterministically returns `std::io::Error`. Require zero cleanup calls, typed `EngramError`, exact spawn-failure diagnostics, no panic/fallback, and caller-retained provider ownership.
10. Make worker launch GREEN through `Builder::spawn`. Share an Arc-backed ownership cell until spawn succeeds. On spawn failure, retain the cell for process lifetime, return immediately, and never use provider Drop or synchronous cleanup as fallback.
11. After successful launch, call `force_flush` once and call `shutdown` once only after flush returns. Bound only daemon wait for phase/completion by `OTLP_CLEANUP_WAIT_TIMEOUT = 5s`.
12. On deadline, do not join or claim cancellation/completion. Return/log `completion=unknown`, last phase, wait limit, and detached-worker/resource residual. Worker panic/channel loss and spawn failure remain distinct. A clean daemon returns cleanup failure; a daemon error remains primary when both fail.

A native detached worker is deliberate: Tokio `spawn_blocking` tasks may delay runtime shutdown, while a dropped native `JoinHandle` is not joined and does not keep the daemon process alive after main returns. A timed-out SDK call may continue until normal process termination; an embedding that keeps the process alive is outside this design and prohibited without a reaper.

## Task and Shipment Decision

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T -> 131.014-T -> 131.015-T
-> 131.016-T
```

Sixteen tasks create exactly fifteen task dependency edges. Each task is 45-105 minutes, at most two files or evidence surfaces, at most four functions, at most three scenarios, one domain, and one atomic milestone. Spawn abstraction, forced-failure RED, safe launch GREEN, and bounded-wait GREEN are separate concerns. Shipment `125-S` contains seventeen items and is blocked pending mandatory review `131.001-R`.

## Runtime and Rollback Decision

Rerun all five corrected RED commands unchanged. Deterministic tests separately prove layer-held provider-clone export, explicit application-handle flush/export, per-export cancellation, subprocess-isolated endpoint precedence, forced `Builder::spawn` failure with typed `EngramError` and retained ownership, and a bounded daemon wait whose timeout leaves synchronous completion unknown. A controlled child process held past the cleanup wait must exit within five seconds plus a two-second harness allowance, proving no join/runtime-blocking dependency rather than SDK completion.

For 30 minutes or three controlled exits, observe export failures, worker spawn/loss/panic, cleanup failures, cleanup-wait timeouts, detached-worker outcomes, and total exit latency. Disable `otlp-export` and revert the owning GREEN commits on any failure/timeout, hidden residual, child exit beyond seven seconds, missing span, or feature-gate regression.

## Constraints

- Never describe dropping one local provider binding as stopping span processing; the layer transitively retains a provider clone.
- Never use `std::env::set_var`, `std::env::remove_var`, an unsafe block, or a process-global environment lock in these tests; use `Command::env`/`env_remove` before child startup.
- Never describe `force_flush()` or `shutdown()` as cancellable or operation-bounded in SDK 0.26.
- Never infer a cleanup-call deadline from `max_export_timeout`.
- No `std::thread::spawn`, `unwrap`, `expect`, `panic`, `unsafe`, `spawn_blocking`, runtime-owned cleanup task, production join, retry, or synchronous cleanup fallback.
- `Builder::spawn` failure must return through `EngramError`, emit one actionable residual diagnostic, call no cleanup method, and retain the explicit provider owner for process lifetime.
- No external collector, socket, credential, network oracle, sleep polling, or retry.
- No test command may omit required `cozo-backend` when defaults are disabled.
- No task mixes Cargo graph, provider construction, endpoint wiring, and cleanup coordination.
- PR #362 and blocked workspace-identity shipments remain untouched.

## References

- PR 363 reviews `5015373740`, `5015447062`, `5015636140`, `5015710467`, `5015926424`, and `5016087555`; review artifact `131.001-R`
- `Cargo.toml`, `Cargo.lock`
- `Cargo.lock:2776-2779` and pinned `opentelemetry_sdk` 0.26 `trace/provider.rs:55-65,216-221`, `trace/tracer.rs:29-49`, and `trace/span_processor.rs`
- `Cargo.lock:4612-4615` and pinned `tracing-opentelemetry-0.26.0/src/layer.rs:37-44,575-588`
- `docs/compound/best-practices/rust-2024-set-var-unsafe-2026-05-07.md`
- `src/server/observability.rs`, `src/lib.rs`, `src/bin/engram.rs`
- `docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md`
