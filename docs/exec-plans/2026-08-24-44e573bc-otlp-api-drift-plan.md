---
title: Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed
source: docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md
source_stash_id: 44E573BC
review_source: PR 363 reviews 5015373740 and 5015447062
---

# Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle

## Problem Frame

The optional `otlp-export` target cannot compile. `tracing-opentelemetry` 0.26 introduces OpenTelemetry 0.25 beside the direct 0.26 family, and `src/server/observability.rs` names APIs unavailable in 0.26. The builder also drops its provider, has no production caller, and has no explicit application cleanup ownership.

Earlier revisions incorrectly treated `BatchConfigBuilder::with_max_export_timeout` as a whole-call deadline for synchronous `force_flush()` and `shutdown()`. It is not. This revision keeps the useful per-export timeout, moves synchronous cleanup to an isolated native worker, and bounds only how long the daemon awaits the worker. It never claims the SDK calls are canceled or complete after that wait expires.

Read-only inspection also confirms that `Command::Daemon` is the sole executable endpoint boundary. The shim remains formatting-only and its child inherits environment.

## Pinned OpenTelemetry 0.26 Evidence

Pinned `opentelemetry_sdk` 0.26 source establishes the contract:

* `trace/span_processor.rs:253-272` enqueues `Flush`/`Shutdown` and synchronously `block_on`s an untimed oneshot response.
* `trace/span_processor.rs:390-396` performs shutdown only after the worker reaches that message and flushes.
* `trace/span_processor.rs:408-425` races each individual `exporter.export(batch)` future against `max_export_timeout`; it does not wrap the whole processor call.
* `trace/span_processor.rs:599-603` documents the setting as the maximum duration to export a batch.
* `trace/provider.rs:144-172` iterates processors synchronously for `force_flush` and `shutdown`.

A queue can contain multiple batches or an in-flight export before the cleanup response. Therefore a five-second per-export value cannot prove a five-second cleanup phase or a ten-second two-call total.

## Requirements Trace

| Requirement | Planned owner |
|---|---|
| Compiling initial RED | U1 uses `cozo-backend` while keeping `otlp-export` disabled in the outer meta-harness. |
| Correct isolated feature graph | U1/U2/U3 use `cozo-backend,otlp-export` for nested graph/compile commands. |
| One OpenTelemetry family | U2 changes only the bridge and lockfile. |
| Compiling optional target | U3 repairs supported 0.26 APIs while preserving lifecycle defects. |
| Explicit test seam | U4 introduces behavior-neutral exporter and tracing-pipeline interfaces. |
| Compiling provider RED | U5 tests retained export and per-export timeout through the existing seam. |
| Provider ownership and per-export timeout | U6 retains the provider and applies source-owned batch-export policy. |
| Compiling daemon RED | U7 tests parser, exact endpoint handoff, attachment, and lifetime. |
| Endpoint propagation | U8 owns daemon CLI and typed handoff. |
| Attachment and lifetime | U9 retains the owner through daemon use. |
| Compiling cleanup RED | U10 tests returned and stalled synchronous cleanup through existing interfaces. |
| Honest application cleanup bound | U11 isolates cleanup in one detached native worker and bounds only daemon wait. |
| Runtime proof | U12 reruns every RED command unchanged and verifies residual semantics. |
| Closure | U13 records dependency, quality, monitoring, and rollback evidence. |

## Exact RED Sequence and Correct Feature Sets

### RED A: compile-neutral graph and feature contract

The outer test must compile Engram's required backend while intentionally leaving OTLP disabled:

```text
cargo test --no-default-features --features cozo-backend --test otlp_feature_compile_contract_test -- --nocapture
```

Its runtime subprocesses use the minimal feature pair needed by the OTLP target:

```text
cargo tree --no-default-features --features cozo-backend,otlp-export
cargo check --no-default-features --features cozo-backend,otlp-export --lib
```

`otlp_dependency_graph_uses_only_026` initially fails because 0.25 and 0.26 coexist. `otlp_export_feature_compiles` initially fails with the existing unsupported-0.26 diagnostics. Invocation failure is distinct from the intended nonzero result, stderr is retained, and the nested command is `cargo check`, never recursive `cargo test`.

### RED B: provider ownership, export, and per-export timeout

```text
cargo test --no-default-features --features cozo-backend,otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture
```

The compiling tests prove retained export and that `OTLP_EXPORT_TIMEOUT = 5s` overrides `OTEL_BSP_EXPORT_TIMEOUT` for each exporter future. A never-ready fake export is dropped at an injected 25 ms under paused Tokio time. That test proves only SDK cancellation of one export future; it does not make a cleanup-call claim.

### RED C: endpoint, attachment, and owner lifetime

```text
cargo test --no-default-features --features cozo-backend,otlp-export otlp_daemon_red -- --nocapture
```

The lib and bin tests compile against U4 interfaces. Expected failures are runtime parser/handoff/attachment/lifetime assertions. U8 and U9 own sequential GREEN states.

### RED D: cleanup isolation and daemon-wait semantics

```text
cargo test --no-default-features --features cozo-backend,otlp-export --bin engram otlp_cleanup_red -- --nocapture
```

U10 uses fake synchronous cleanup methods with entered/release barriers. It proves returned flush results lead to one shutdown attempt; force-flush and shutdown stalls make the daemon wait time out at the injected/paused five-second deadline; and the result says completion unknown with a detached worker. The test releases and reaps the fake worker only after the production-facing assertion so the test process does not leak a thread. Current code is RED because U9 retains the owner but starts no cleanup worker.

## Cleanup Ownership and Honest Guarantees

Two independent constants have non-overlapping meanings:

* `OTLP_EXPORT_TIMEOUT = 5s` in `src/server/observability.rs`: maximum for each exporter future/batch as implemented by SDK 0.26. It can cancel/drop that future.
* `OTLP_CLEANUP_WAIT_TIMEOUT = 5s` in `src/bin/engram.rs`: one total monotonic deadline for the daemon to await the complete cleanup worker sequence. It cancels only the channel wait.

After the daemon future ends, the lifecycle runner launches exactly one `std::thread` worker and transfers the explicit provider owner into it. The worker emits phase signals, calls `force_flush()` once, and calls `shutdown()` once only if flush returns, whether success or error. If flush never returns, shutdown cannot be reached and is not reported as attempted. The daemon awaits completion/phase signals under the total wait deadline.

On timeout the receiver is dropped, the native `JoinHandle` is not joined, and the caller returns a diagnostic with `last_phase`, `wait_limit`, `completion=unknown`, and `worker_detached=true`. There is no retry. Queued spans, exporter I/O, SDK method completion, and resource release may remain unresolved. A worker panic or channel close is reported distinctly.

The launcher must not use Tokio `spawn_blocking`, a runtime-owned `JoinSet`, or any drop path that waits for the worker. A thread-spawn failure must return immediately and avoid treating provider Drop as successful cleanup; a process-lifetime ownership cell/leak is acceptable on this rare fail-exit path and must be logged as an unresolved resource residual. The global subscriber remains process-static rather than becoming a caller-side cleanup oracle.

The bounded statement is: **OTLP cleanup contributes at most five seconds of daemon wait after successful worker launch.** It is not: "flush completed in five seconds", "shutdown completed in five seconds", or "the calls were canceled". The main future can return at the deadline because no runtime blocking task or join remains. Rust process termination does not wait for a detached native thread; normal binary exit terminates it and the OS reclaims process resources. If this runner were embedded in a process that continues running, the detached worker could continue indefinitely; such reuse is prohibited without a separate reaper/process-isolation design.

A clean daemon returns cleanup failure/timeout. If daemon and cleanup both fail, the daemon error remains primary and the complete cleanup diagnostic is emitted to stderr before return.

## Implementation Units

### U1 / 131.001-T — RED compile-neutral meta-harness

One test file, at most three test/helper functions, two scenarios, 75 minutes. Outer feature set: `cozo-backend`; nested OTLP commands: `cozo-backend,otlp-export`.

### U2 / 131.002-T — GREEN bridge graph

Only `Cargo.toml` and `Cargo.lock`, zero functions, one graph scenario, 45 minutes. The graph assertion turns GREEN while feature compilation stays RED.

### U3 / 131.003-T — GREEN pinned-0.26 compile baseline

Only `src/server/observability.rs`, at most three functions, two compile scenarios, 75 minutes. Preserve layer-only provider drop, no application timeout, no caller, and no cleanup.

### U4 / 131.004-T — SCAFFOLD behavior-neutral seams

Only `src/server/observability.rs` and `src/lib.rs`, at most four functions, two scenarios, 90 minutes. Existing production behavior remains layer-only and formatting-only.

### U5 / 131.005-T — RED provider lifecycle

One test module, at most four functions, three scenarios, 100 minutes. Test retained export and the individual exporter timeout only.

### U6 / 131.006-T — GREEN retained provider and per-export timeout

Only `src/server/observability.rs`, at most four functions, three scenarios, 105 minutes. Synchronous cleanup methods remain explicitly unbounded.

### U7 / 131.007-T — RED daemon endpoint and attachment

Test-only modules in `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenarios, 105 minutes.

### U8 / 131.008-T — GREEN endpoint propagation

Only `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenarios, 95 minutes.

### U9 / 131.009-T — GREEN attachment and lifetime

Only `src/lib.rs` and the daemon arm/helper in `src/bin/engram.rs`, at most four functions, three scenarios, 105 minutes. Cleanup remains absent.

### U10 / 131.010-T — RED cleanup isolation

One bin test module, at most four functions, three scenario groups, 105 minutes. Deterministic barriers plus paused Tokio time distinguish caller timeout from worker completion.

### U11 / 131.011-T — GREEN detached cleanup worker

Only `src/bin/engram.rs`, at most four functions, three scenario groups, 115 minutes. One native worker, one total daemon-wait deadline, no join, honest residual reporting, and existing error precedence.

### U12 / 131.012-T — VERIFY runtime

At most two unchanged evidence surfaces, zero production functions, three scenario groups, 90 minutes. Rerun all four corrected commands and verify no `spawn_blocking`/join path.

### U13 / 131.013-T — VERIFY quality and operations

At most two evidence surfaces, zero functions, three gate groups, 90 minutes. Record feature graph, quality, runtime, cleanup residual, monitoring, and rollback evidence.

All estimates are 45-115 minutes and remain below the two-hour limit. Cleanup isolation remains one width-isolated implementation task, so no fourteenth task is required.

## Dependency Graph and Shipment

Strict chain:

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T
```

Thirteen tasks produce exactly twelve task dependency edges. Shipment `125-S` contains parent `131-F` plus all thirteen tasks in that order: fourteen items total. It remains sole queued and unclaimed. Blocked shipments `126-S` through `129-S` remain blocked.

## Runtime Verification, Monitoring, and Rollback

Ship reruns the corrected RED commands unchanged. Deterministic tests must observe one span, owner retention, one-export-future cancellation at the injected limit, returned cleanup sequencing, and timeout results that explicitly say completion unknown. Controlled subprocess verification holds a fake cleanup method past the deadline and proves the child exits within the five-second application wait plus a two-second harness allowance. That proves process exit does not join the worker; it does not prove SDK cleanup completed.

For 30 minutes or three controlled exits, Ship monitors stderr/logs for export failure, cleanup-worker spawn failure, worker channel loss/panic, cleanup failure, cleanup-wait timeout, detached-worker outcome, and daemon exit duration. Healthy state is zero failure/timeout records and controlled exits below seven seconds. Any such failure, hidden residual, missing span, or feature-gate regression disables `otlp-export` and reverts the owning GREEN commits. Closure records whether any timed-out cleanup remained unresolved until process exit.

## Plan Hardening — Exact-Head Rerun

Hardening is **required and satisfied** for external export, provider lifetime, synchronous non-cancellable cleanup, process exit, and rollback.

Reinforcing context: strict-safety and release-observability instructions; pinned OpenTelemetry 0.26 source listed above; `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`, whose key lesson is that an inner timeout does not bound a wider operation.

| ProposedAction | Targets | ActionRisk | Rollback | Approval required | ActionResult |
|---|---|---|---|---|---|
| Add compile-neutral RED harness | One test file | moderate | Revert U1 | no | planned |
| Align dependency family | Cargo manifest and lockfile | moderate | Restore bridge 0.26 | no | planned |
| Retain provider and apply per-export policy | Observability provider | high | Revert U6; disable feature | preferred before rollout | planned |
| Wire endpoint and attachment | Daemon CLI and tracing init | moderate | Revert U8/U9 | no | planned |
| Isolate synchronous cleanup in a detached native worker | Daemon lifecycle runner | high | Revert U11; disable feature | preferred before rollout | planned |
| Verify residual behavior and observe | Focused harness and logs | low | Return defect to Stage | no | planned |

Protected invariants: compiling REDs precede behavior GREENs; no command omits `cozo-backend`; per-export and daemon-wait constants are never conflated; no synchronous call is described as cancellable; timeout diagnostics say completion unknown; no runtime-owned blocking task or join can extend daemon return; all work remains under two hours.

## Standard Plan Review — Exact-Head Rerun

Gate: **PASS**. Intercom and cross-model dispatch were unavailable, so the Stage caller ran constitution, Rust/API, architecture, scope, test-strategy, operational-readiness, learnings, and external-boundary security lenses locally. No unresolved P0/P1 remains.

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| R2 | Rust/API | P1 | SDK max export timeout does not bound synchronous provider cleanup. | Resolved: U6 limits its claim to each export future; U10/U11 bound only daemon wait and report completion unknown. |
| A2 | Architecture | P1 | An async timeout around a blocking call would abandon runtime-owned work and could delay runtime shutdown. | Resolved: one detached `std::thread`, no `spawn_blocking`, no join, explicit process-exit residual. |
| T3 | Test strategy | P1 | The timeout oracle could pass without proving the worker remained blocked. | Resolved: entered/release barriers, paused time, pending-at-4,999-ms check, and test-only release/reap. |
| O2 | Operations | P1 | Timed-out resources and telemetry loss were undocumented. | Resolved: structured unknown/detached diagnostic, controlled child exit check, monitoring, rollback, and closure residual. |
| C2 | Constitution | P1 | OTLP-only no-default commands compile unrelated missing Cozo surfaces. | Resolved: outer meta-harness uses `cozo-backend`; every OTLP graph/check/test command uses `cozo-backend,otlp-export`. |
| S2 | Scope | P2 | Cleanup isolation might require another task. | Resolved: U11 stays one file, four functions, three scenario groups, 115 minutes; roster remains thirteen tasks. |
| L1 | Learnings | P2 | Prior startup learning rejects inner-timeout-as-outer-bound reasoning. | Resolved in hardening and the two-constant design. |
| X2 | Security | P3 | External exporter tests could cross network/credential boundaries. | Resolved by injected in-process fakes and no-network scope. |

Review confirms a fourteen-item roster, thirteen tasks, exactly twelve dependency edges, 45-115 minute estimates, one sole queued/unclaimed shipment, and no claim that SDK cleanup completed after the daemon deadline.

## References

- PR 363 reviews `5015373740`, `5015447062`; threads `PRRT_kwDORJEduc6b8xO2`, `PRRT_kwDORJEduc6b8_IN`
- Stash `44E573BC`; feature `131-F`; shipment `125-S`
- `Cargo.toml`, `Cargo.lock`
- `src/server/observability.rs`, `src/lib.rs`, `src/bin/engram.rs`
- `docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md`
- `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`
- `.github/instructions/strict-safety.instructions.md`
- `.github/instructions/release-observability.instructions.md`
