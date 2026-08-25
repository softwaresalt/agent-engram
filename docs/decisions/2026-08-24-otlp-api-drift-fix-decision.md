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

The optional `otlp-export` feature does not compile because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while Engram directly pins 0.26, and `src/server/observability.rs` uses APIs unavailable in 0.26. The runtime design is also incomplete: the builder drops its provider, has no production caller, inherits the SDK or environment export timeout, and has no explicit flush or shutdown path.

The first plan attempted one feature-enabled RED before the feature target itself compiled. Review 5015140545 correctly rejected that sequence: production compilation failed before tests could compile or execute, so it could never demonstrate a compiling-but-failing harness.

The daemon endpoint has no executable handoff. `Command::Daemon` is a unit variant that initializes tracing before `daemon::run`. `GlobalFlags` has no endpoint, the old `Config::otlp_endpoint` parser has no production caller, and `PluginConfig` loads too late. The shim child already inherits environment.

## Decision

Use four explicit RED boundaries and thirteen linear tasks.

1. Start with a compile-neutral integration meta-harness built without `otlp-export`. It invokes `cargo tree` and isolated `cargo check --lib` subprocesses and fails runtime assertions. It imports no current or future OTLP production symbol.
2. Align only `tracing-opentelemetry` to 0.27 and the generated lockfile, then repair only the 0.26 source compile baseline. The latter deliberately preserves provider drop and missing runtime behavior.
3. Introduce an explicit behavior-neutral exporter and tracing-pipeline seam before feature-enabled behavior tests. The seam is a separate task and cannot implement retention, timeout, endpoint propagation, attachment, or cleanup.
4. Add a compiling provider RED against that seam. Its exact command is `cargo test --no-default-features --features otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture`. It fails for no retained export, no application timeout, and no deterministic cancellation report.
5. Retain the provider and define `OTLP_EXPORT_TIMEOUT = Duration::from_secs(5)` in `src/server/observability.rs`. Build batch configuration from defaults, then call `with_max_export_timeout(OTLP_EXPORT_TIMEOUT)` so `OTEL_BSP_EXPORT_TIMEOUT` cannot override production policy. A test-only constructor may inject 25 ms for paused-time testing.
6. Add a compiling daemon RED before endpoint and attachment work. Make `Command::Daemon` the endpoint boundary, pass one typed value to the shared tracing seam, attach beside stderr formatting, and retain the owner through daemon use.
7. After the lifecycle runner exists, add a compiling cleanup RED. Then call force flush and shutdown exactly once each. Always attempt shutdown after flush failure. Each phase is bounded by the five-second provider setting, for a declared maximum of ten seconds across two sequential phases.
8. Report phase, limit, and source on cleanup failure. A clean daemon returns cleanup failure. If both daemon and cleanup fail, the daemon error remains primary and cleanup is preserved diagnostically.

The timeout is application source policy, not user configuration. The deterministic fake exporter remains pending, carries a drop token, and uses paused Tokio time to prove no cancellation at 24 ms and cancellation at 25 ms without sleep. No outer async timeout may claim cancellation while leaving a blocking SDK call running.

## Task and Shipment Decision

Exact task chain:

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T
```

Each task is 45 to 105 minutes, at most two files or evidence surfaces, at most four functions, at most three scenarios, one skill domain, and one atomic milestone. Shipment `125-S` contains `131-F` plus all thirteen tasks, stays queued and unclaimed, and remains subject to exact-head review and integration guards. Blocked shipments are unchanged.

## Runtime and Rollback Decision

All four RED commands are rerun unchanged in runtime verification. Ship then owns a 30-minute or three-controlled-exit observation window. Healthy state is zero OTLP timeout, export failure, or cleanup failure records and every controlled daemon exit below ten seconds. Roll back the owning GREEN commit or commits and disable `otlp-export` if an exit reaches ten seconds, cleanup failure is hidden, the expected focused span is absent, or default/all-features gates regress.

## Constraints

- No test may fail to compile because a planned production symbol does not exist.
- No external collector, socket, credential, network oracle, sleep polling, or retry.
- No hidden endpoint or timeout reread from CLI, environment, workspace config, or global state.
- No task mixes Cargo graph work with Rust source work.
- No task mixes provider construction with daemon CLI or cleanup coordination.
- The shim and absent-endpoint path remain formatting-only.
- PR 362 and blocked workspace-identity shipments remain untouched.

## References

- Review 5015140545 and OTLP threads `PRRT_kwDORJEduc6b8O89`, `PRRT_kwDORJEduc6b8UJJ`, and `PRRT_kwDORJEduc6b8UIv`
- `Cargo.toml`
- `src/server/observability.rs`
- `src/lib.rs`
- `src/bin/engram.rs`
- `docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md`
