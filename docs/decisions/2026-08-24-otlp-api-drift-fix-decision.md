---
title: Align the tracing bridge and repair OpenTelemetry 0.26 lifecycle usage
type: decision
doc_type: decision
source: stash 44E573BC
date: 2026-08-24
status: blocked
source_stash_id: 44E573BC
promoted_to:
  - docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md
---

# Align the tracing bridge and repair OpenTelemetry 0.26 lifecycle usage


> [!IMPORTANT]
> **CURRENT FAIL-CLOSED AUTHORITY.** This decision is planning/history only. All 131 artifacts and shipment 125-S are blocked; no implementation claim is permitted until a future, separately staged release obtains three eligible complete-coverage reviewers. See [PR #363 fail-closed planning authority](2026-08-25-pr-363-fail-closed-planning-authority.md).

## Problem Frame

The optional `otlp-export` feature does not compile because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while Engram directly pins 0.26, and the observability builder uses APIs unavailable in 0.26. The runtime design returns only a layer and lacks a separately accessible application cleanup handle. Read-only pinned source corrects the prior assumption: `TracerProvider::library_tracer` passes `self.clone()` into `Tracer`, `Tracer` stores that provider clone, and `OpenTelemetryLayer` stores the tracer. Ending only the constructor-local provider binding therefore does not stop span processing.

Pinned SDK 0.26 source shows that `force_flush()` and `shutdown()` enqueue messages and block on untimed oneshot responses. `max_export_timeout` races each exporter batch future only. It cannot establish a whole-call or two-call bound.

## Decision

Use five compiling RED boundaries and seventeen linear tasks.

1. Build the compile-neutral outer meta-harness with `--no-default-features --features cozo-backend`; nested OTLP tree/check commands use `--features cozo-backend,otlp-export`.
2. Align only `tracing-opentelemetry` to 0.27 and the lockfile, record exact 0.27 checksum and source proof that its layer stores the tracer/provider clone, then repair only the SDK 0.26 compile baseline.
3. Add behavior-neutral exporter, pipeline-control, cfg-arm, and subscriber-injection seams before tests. Tests use an injected Registry or `tracing::subscriber::with_default`; production uses fallible global installation and never calls a second panicking `init`.
4. Run the provider RED with `cargo test --no-default-features --features cozo-backend,otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture`. A controlled local exporter first proves layer-held-clone export as an already-GREEN baseline; RED then fails on `LifecycleUnavailable`/missing explicit flush control and the missing source-owned per-export timeout.
5. Return a separately accessible application provider handle alongside the attachable layer and set `OTLP_EXPORT_TIMEOUT = 5s` after defaults. The layer already retains a clone; the separate handle exists to invoke and observe force flush/shutdown. The timeout bounds each exporter future only and may drop a never-ready export; it does not bound synchronous cleanup.
6. Add daemon endpoint and attachment RED plus sequential GREENs. Child processes own environment variance; attachment uses isolated subscribers; one existing endpoint authority is reused or retired; duplicate flags and unredacted URI credentials are forbidden.
7. Add a finite cleanup RED with deterministic barriers, paused time, and a test-side 5,001 ms watchdog.
8. Add truthful `DaemonError::CleanupWorkerSpawnFailed` plus a distinct stable code; do not reuse daemon-process `SpawnFailed`.
9. Add a behavior-neutral injectable spawner and retained-owner sink whose production adapter uses only `std::thread::Builder::spawn` and returns its `Result`.
10. Add compile-then-fail RED using realistic `WouldBlock` and `OutOfMemory`; require zero cleanup calls, dedicated `EngramError`, redacted diagnostics, and caller-retained ownership.
11. Make launch GREEN through `Builder::spawn`. On failure, send the caller-held cell to the residual sink before fatal nonzero return. No provider Drop or synchronous cleanup fallback.
12. After successful launch, call `force_flush` once and call `shutdown` only after flush returns. Bound only daemon wait by five seconds and define cleanup outcome precedence.
13. Run all four quality gates and fail closure if manual or shim-spawned production lacks an observable diagnostic sink, if fatal exit can restart-loop, or if monitoring baselines and named queries are absent.

A native detached worker is deliberate: Tokio `spawn_blocking` tasks may delay runtime shutdown, while a dropped native `JoinHandle` is not joined and does not keep the daemon process alive after main returns. A timed-out SDK call may continue until normal process termination; an embedding that keeps the process alive is outside this design and prohibited without a reaper.

## Task and Shipment Decision

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T -> 131.014-T -> 131.015-T
-> 131.016-T -> 131.017-T
```

Seventeen tasks create sixteen edges and eighteen shipment items with `131-F`. Each task is 45-105 minutes, at most two files or evidence surfaces, at most four functions, at most three groups, one domain, and one atomic milestone. Review `131.001-R`, feature `131-F`, every task `131.001-T` through `131.017-T`, and shipment `125-S` are blocked after the receipt-bound three-reviewer cohort produced zero complete eligible responses. PR #363 is planning/history only and cannot authorize implementation. A future, separately staged release must obtain three eligible complete-coverage reviewers before creating executable scope.

## Runtime and Rollback Decision

Rerun all five corrected RED commands unchanged and run format, clippy, tests, and audit in repository order. Deterministic tests separately prove layer-held provider-clone export, explicit application-handle flush/export, per-export cancellation, subprocess-isolated endpoint precedence, realistic forced `Builder::spawn` failures with dedicated `CleanupWorkerSpawnFailed`, retained ownership, redaction, and isolated subscriber behavior, and a bounded daemon wait whose timeout leaves synchronous completion unknown. A controlled child process held past the cleanup wait must exit within five seconds plus a two-second harness allowance, proving no join/runtime-blocking dependency rather than SDK completion.

For 30 minutes or three controlled exits, use a named observable sink and exact queries for manual and shim-spawned daemons to observe export failures, worker spawn, transfer, loss or panic, cleanup failures, timeouts, residuals, fatal exit status, restart loops, and latency. Null stdio with no durable sink fails closure. Disable `otlp-export` and revert the owning GREEN commits on any failure/timeout, hidden residual, child exit beyond seven seconds, missing span, or feature-gate regression.

## Constraints

- Never describe dropping one local provider binding as stopping span processing; the layer transitively retains a provider clone.
- Never use `std::env::set_var`, `std::env::remove_var`, an unsafe block, or a process-global environment lock in these tests; use `Command::env`/`env_remove` before child startup.
- Never describe `force_flush()` or `shutdown()` as cancellable or operation-bounded in SDK 0.26.
- Never infer a cleanup-call deadline from `max_export_timeout`.
- No `std::thread::spawn`, `unwrap`, `expect`, `panic`, `unsafe`, `spawn_blocking`, runtime-owned cleanup task, production join, retry, or synchronous cleanup fallback.
- `Builder::spawn` failure must return dedicated `CleanupWorkerSpawnFailed`, emit one redacted residual diagnostic, call no cleanup method, retain the explicit owner through an injected sink, and return fatal nonzero for a clean daemon.
- No second global subscriber `init`; tests use isolated Registry or `with_default`, and production installation is fallible.
- One endpoint authority only; URI userinfo and query never enter diagnostics.
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
