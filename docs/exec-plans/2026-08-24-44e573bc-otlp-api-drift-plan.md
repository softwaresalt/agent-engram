---
title: Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: review
source: docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md
source_stash_id: 44E573BC
review_source: PR 363 reviews 5015373740, 5015447062, 5015636140, 5015710467, 5015926424, and 5016087555; backlog review 131.001-R
---

# Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle

## Problem Frame

The optional `otlp-export` target cannot compile. `tracing-opentelemetry` 0.26 introduces OpenTelemetry 0.25 beside the direct 0.26 family, and `src/server/observability.rs` names APIs unavailable in 0.26. The builder returns only a tracing layer and has no production caller or explicit application cleanup handle. Pinned SDK semantics show that the layer owns a tracer, the tracer owns a cloned provider, and ending the constructor-local provider binding therefore does not stop span processing. The defect is inaccessible application lifecycle/flush ownership, not provider liveness.

Earlier revisions incorrectly treated `BatchConfigBuilder::with_max_export_timeout` as a whole-call deadline for synchronous `force_flush()` and `shutdown()`. It is not. This revision keeps the useful per-export timeout, moves synchronous cleanup to an isolated native worker, and bounds only how long the daemon awaits the worker. It never claims the SDK calls are canceled or complete after that wait expires.

Read-only inspection also confirms that `Command::Daemon` is the sole executable endpoint boundary. The shim remains formatting-only and its child inherits environment.

## Pinned OpenTelemetry 0.26 Evidence

`Cargo.lock:2776-2779` pins `opentelemetry_sdk` 0.26.0 (checksum `d2c627d9f4c9cdc1f21a29ee4bfbd6028fcb8bcf2a857b43f3abdf72c9c862f3`), and `Cargo.lock:4612-4615` pins `tracing-opentelemetry` 0.26.0 (checksum `5eabc56d23707ad55ba2a0750fc24767125d5a0f51993ba41ad2c441cc7b8dea`). Read-only pinned source establishes the contract:

* `opentelemetry_sdk-0.26.0/src/trace/provider.rs:55-65` states that cloning/dropping one provider does not stop processing and that shutdown requires an explicit call or dropping every provider clone.
* `provider.rs:216-221` constructs each tracer with `self.clone()`; `trace/tracer.rs:29-49` stores that `TracerProvider` clone in `Tracer`.
* `tracing-opentelemetry-0.26.0/src/layer.rs:37-44` stores `tracer: T`, and `layer.rs:575-588` moves the supplied tracer into the returned layer.
* Therefore the returned layer transitively retains a provider clone. Dropping only the constructor-local provider variable is not a valid RED oracle. A separate application handle is still required to invoke and observe explicit flush/shutdown orchestration.

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
| Correct isolated feature graph | U1/U2/U3 use `cozo-backend,otlp-export` for nested graph and compile commands. |
| One OpenTelemetry family | U2 changes only the bridge and lockfile. |
| Compiling optional target | U3 repairs supported 0.26 APIs while preserving lifecycle defects. |
| Explicit tracing seam | U4 introduces behavior-neutral exporter and tracing-pipeline interfaces. |
| Compiling provider RED | U5 proves layer-held export, then fails only for missing application control and per-export policy. |
| Application lifecycle control | U6 returns an explicit provider handle and applies the per-export-future limit. |
| Compiling daemon RED | U7 uses child-process environment setup and tests endpoint handoff, attachment, and lifetime. |
| Endpoint propagation | U8 owns daemon CLI and typed handoff. |
| Attachment and lifetime | U9 retains the owner through daemon use. |
| Compiling cleanup RED | U10 tests returned and stalled synchronous cleanup through existing interfaces. |
| Behavior-neutral spawn seam | U11 adds an injectable spawner and safe `Builder::spawn` adapter without launching work. |
| Deterministic spawn-failure RED | U12 forces an `io::Error` after compilation and names U13 as GREEN owner. |
| Safe worker launch and error propagation | U13 launches through `Builder::spawn`, maps failure to `EngramError`, and preserves owner residuals without panic or fallback. |
| Honest application cleanup bound | U14 bounds only daemon wait and owns stall, channel-loss, panic, and precedence GREEN. |
| Runtime proof | U15 reruns all five RED commands unchanged and verifies static safety. |
| Closure | U16 records dependency, quality, monitoring, and rollback evidence. |

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

The compiling tests have three behaviorally distinct scenarios. First, a controlled local simple exporter proves the layer-held tracer/provider clone exports `otlp-layer-owned-baseline-span` after the constructor-local provider binding ends; this baseline must already PASS. Second, a controlled local batch exporter emits `otlp-red-contract-span`, requests `force_flush` through the U4 control seam, and expects a separately accessible application lifecycle handle, a successful flush result, and that exact exported span. The intended RED is `LifecycleUnavailable`/no explicit application handle, not provider drop. Third, a never-ready fake exporter is dropped at an injected 25 ms under paused Tokio time; this RED is the missing source-owned per-export timeout and makes no cleanup-call claim.

### RED C: endpoint, attachment, and owner lifetime

```text
cargo test --no-default-features --features cozo-backend,otlp-export otlp_daemon_red -- --nocapture
```

The lib and bin tests compile against U4 interfaces. A parent test relaunches the current test executable for flag-plus-environment, environment-only, and absent cases using `std::process::Command::env`/`env_remove` plus a private child marker. Child cases only read inherited environment and call `Cli::try_parse_from`; no in-process `set_var`/`remove_var`, unsafe block, serial lock, or cross-test race is permitted. Before U8, the exact command fails at runtime with the captured unexpected-argument or missing-resolved-value assertion, never during compilation. Separate recording-factory and attachment/lifetime assertions remain RED for U8/U9.

### RED D: cleanup isolation and daemon-wait semantics

```text
cargo test --no-default-features --features cozo-backend,otlp-export --bin engram otlp_cleanup_red -- --nocapture
```

U10 uses fake synchronous cleanup methods with entered/release barriers. It proves returned flush results lead to one shutdown attempt; force-flush and shutdown stalls make the daemon wait time out at the injected/paused five-second deadline; and the result says completion unknown with a detached worker. The test releases and reaps the fake worker only after the production-facing assertion so the test process does not leak a thread. Current code is RED because U9 retains the owner but starts no cleanup worker.

### RED E: cleanup-worker spawn failure

```text
cargo test --no-default-features --features cozo-backend,otlp-export --bin engram otlp_cleanup_spawn_failure_red -- --nocapture
```

U11 first supplies a behavior-neutral injectable `CleanupWorkerSpawner`; the daemon still launches no worker. U12 then installs a fake spawner that records one request and returns `std::io::ErrorKind::Other` with marker `forced cleanup-worker spawn failure`. The production bin and test compile before assertions execute. The intended RED is that the spawner is not called and no typed failure exists.

GREEN belongs only to U13. A clean daemon must receive `EngramError::Daemon(DaemonError::SpawnFailed { reason })` or the repository-equivalent typed error. The diagnostic records `operation=otlp_cleanup_worker_spawn`, `cleanup_attempted=false`, `completion=not_started`, `worker_detached=false`, and `provider_residual=retained_for_process_lifetime`. `force_flush` and `shutdown` remain at zero. If a daemon error already exists, it remains primary and the complete spawn diagnostic is emitted once. No OS thread exhaustion, allocator pressure, sleep, panic hook, `unwrap`, `expect`, `panic`, `unsafe`, retry, or synchronous cleanup fallback is allowed.

## Cleanup Ownership and Honest Guarantees

Two independent constants have non-overlapping meanings:

* `OTLP_EXPORT_TIMEOUT = 5s` in `src/server/observability.rs` limits each exporter future or batch in SDK 0.26.
* `OTLP_CLEANUP_WAIT_TIMEOUT = 5s` in `src/bin/engram.rs` limits one daemon wait for the complete cleanup-worker sequence after successful launch.

U11 introduces an injectable spawner whose production adapter calls only `std::thread::Builder::new().name(...).spawn(job)` and returns the `Result`. The runner does not invoke it until U13. `std::thread::spawn`, `unwrap`, `expect`, `panic`, `unsafe`, `spawn_blocking`, runtime-owned cleanup tasks, joins, and retries are prohibited.

Before launch, caller and job hold separate `Arc` references to an ownership cell containing the explicit provider owner. After successful spawn, the caller releases its duplicate and the worker safely takes the owner. Lock poisoning or failed ownership transfer is propagated and diagnosed, never unwrapped. If `Builder::spawn` returns `Err`, dropping the failed job cannot drop the sole owner because the caller still retains it. The runner places that cell in an explicit safe process-lifetime residual holder, returns a typed `EngramError`, and logs that cleanup was not attempted. It never invokes synchronous cleanup or treats provider Drop as success.

After successful launch, the worker emits phase signals, calls `force_flush()` once, and calls `shutdown()` once only if flush returns, whether success or error. If flush never returns, shutdown is not reported as attempted. The daemon awaits only phase and completion messages under one monotonic deadline.

On deadline, the receiver is dropped, the native `JoinHandle` is not joined, and the result records `last_phase`, `wait_limit`, `completion=unknown`, and `worker_detached=true`. Queued spans, exporter I/O, SDK completion, and resource release may remain unresolved. Worker panic or channel close is distinct from spawn failure and timeout. A clean daemon returns cleanup failure, spawn failure, channel failure, or timeout. If a daemon error already exists, it remains primary and the complete cleanup diagnostic is emitted once.

The bounded statement is: **after successful worker launch, OTLP cleanup contributes at most five seconds of daemon wait.** Spawn failure returns immediately. Neither statement claims that flush or shutdown completed or was canceled. Normal binary exit may terminate a detached worker; embedding this runner in a continuing process is prohibited without a reaper or process-isolation redesign.

## Implementation Units

### U1 / 131.001-T — RED compile-neutral meta-harness

One test file, at most three test/helper functions, two scenarios, 75 minutes. Outer feature set: `cozo-backend`; nested OTLP commands: `cozo-backend,otlp-export`.

### U2 / 131.002-T — GREEN bridge graph

Only `Cargo.toml` and `Cargo.lock`, zero functions, one graph scenario, 45 minutes. The graph assertion turns GREEN while feature compilation stays RED.

### U3 / 131.003-T — GREEN pinned-0.26 compile baseline

Only `src/server/observability.rs`, at most three functions, two compile scenarios, 75 minutes. Preserve the layer-only return and its transitive tracer/provider clone, but no separately accessible application lifecycle handle, application timeout, caller, or cleanup.

### U4 / 131.004-T — SCAFFOLD behavior-neutral seams

Only `src/server/observability.rs` and `src/lib.rs`, at most four functions, two scenarios, 90 minutes. The behavior-neutral result/control seam can report lifecycle unavailable; existing production behavior remains layer-only and formatting-only while the layer continues to retain its tracer/provider clone.

### U5 / 131.005-T — RED provider lifecycle

One test module, at most four functions, three scenarios, 100 minutes. Prove the actual layer-retention/export baseline, then test missing explicit application lifecycle/flush control and the individual exporter timeout.

### U6 / 131.006-T — GREEN explicit lifecycle handle and per-export timeout

Only `src/server/observability.rs`, at most four functions, three scenarios, 105 minutes. Return a separately accessible application provider handle alongside the layer-held clone; the handle exists for explicit force-flush/shutdown control, not to make the layer live. Synchronous cleanup methods remain explicitly unbounded.

### U7 / 131.007-T — RED daemon endpoint and attachment

Test-only modules in `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenario groups, 105 minutes. Endpoint environment cases run only in self-relaunched child test processes using `Command::env`/`env_remove`.

### U8 / 131.008-T — GREEN endpoint propagation

Only `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenarios, 95 minutes.

### U9 / 131.009-T — GREEN attachment and lifetime

Only `src/lib.rs` and the daemon arm/helper in `src/bin/engram.rs`, at most four functions, three scenarios, 105 minutes. Cleanup remains absent.

### U10 / 131.010-T — RED cleanup isolation

One bin test module, at most four functions, three scenario groups, 105 minutes. Deterministic barriers plus paused Tokio time distinguish caller timeout from worker completion.

### U11 / 131.011-T — SCAFFOLD injectable cleanup-worker spawner

Only `src/bin/engram.rs`, at most three functions or trait methods, two scenario groups, 60 minutes. Add the behavior-neutral injection point, safe named `Builder::spawn` adapter, and Arc-backed ownership-cell shape; launch no worker.

### U12 / 131.012-T — RED deterministic spawn failure

One bin test module, at most four functions, three scenario groups, 90 minutes. The fake spawner returns a forced `io::Error`; compilation passes before missing invocation, typed-error, zero-cleanup, ownership-residual, and combined-precedence assertions fail.

### U13 / 131.013-T — GREEN safe worker launch and spawn-error propagation

Only `src/bin/engram.rs`, at most four functions or trait methods, three scenario groups, 100 minutes. Launch once through `Builder::spawn`, preserve the returned `Result`, map failure into `EngramError`, retain the owner residual, and green nonstalling cleanup plus spawn-failure behavior. Deadline behavior remains RED.

### U14 / 131.014-T — GREEN bounded wait and lifecycle diagnostics

Only `src/bin/engram.rs`, at most four functions, three scenario groups, 105 minutes. Add the one total daemon-wait deadline and green both stall phases, channel loss or panic, cleanup failure, and combined-error precedence without changing U13 spawn handling.

### U15 / 131.015-T — VERIFY runtime

At most two unchanged evidence surfaces, zero production functions, three scenario groups, 90 minutes. Rerun all five corrected RED commands and verify no forbidden launch, panic, unsafe, blocking-runtime, join, or fallback path.

### U16 / 131.016-T — VERIFY quality and operations

At most two evidence surfaces, zero functions, three gate groups, 90 minutes. Record feature graph, quality, runtime, spawn-failure residual, cleanup residual, monitoring, and rollback evidence.

All estimates are 45-105 minutes and remain below the two-hour limit. Spawn abstraction, spawn-failure RED, worker-launch GREEN, and bounded-wait GREEN are separate width-isolated concerns.

## Dependency Graph and Shipment

Strict chain:

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T -> 131.014-T -> 131.015-T
-> 131.016-T
```

Sixteen tasks produce exactly fifteen task dependency edges. Shipment `125-S` contains parent `131-F` plus all sixteen tasks in that order: seventeen items total. Review `131.001-R` is outside the shipment roster and is the mandatory gate. Feature `131-F` and shipment `125-S` are blocked and unclaimable pending accepted escalation review. Shipments `126-S` through `129-S` remain blocked and untouched.

## Runtime Verification, Monitoring, and Rollback

Ship reruns all five RED commands unchanged. Deterministic tests observe layer-held export, explicit application-handle flush/export, subprocess endpoint precedence, deterministic fake-spawner failure, zero cleanup calls after failed spawn, typed `EngramError`, retained provider residual, returned cleanup sequencing, and timeout results that say completion unknown.

Static and behavioral checks prove the production launcher uses `Builder::spawn` and contains no `std::thread::spawn`, `unwrap`, `expect`, `panic`, `unsafe`, `spawn_blocking`, runtime-owned join, retry, or synchronous fallback. Controlled subprocess verification holds a fake cleanup method past the deadline and proves exit within the five-second daemon wait plus a two-second harness allowance. That proves no join dependency, not SDK completion.

For 30 minutes or three controlled exits, Ship monitors export failure, cleanup-worker spawn failure, worker channel loss or panic, cleanup failure, cleanup-wait timeout, retained or detached residual outcome, and exit duration. Healthy state is zero failure or timeout records and exits below seven seconds. Any failure, unexpected provider Drop fallback, hidden residual, missing span, or feature-gate regression disables `otlp-export` and reverts the owning GREEN commits. Closure records failed-spawn retention and whether timed-out cleanup remained unresolved until process exit.

## Plan Hardening — Exact-Head Rerun

Hardening is **required and satisfied** for external export, provider lifetime, synchronous non-cancellable cleanup, process exit, and rollback.

Reinforcing context: strict-safety and release-observability instructions; pinned OpenTelemetry 0.26 source listed above; `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`, whose key lesson is that an inner timeout does not bound a wider operation.

| ProposedAction | Targets | ActionRisk | Rollback | Approval required | ActionResult |
|---|---|---|---|---|---|
| Add compile-neutral RED harness | One test file | moderate | Revert U1 | no | planned |
| Align dependency family | Cargo manifest and lockfile | moderate | Restore bridge 0.26 | no | planned |
| Expose explicit application lifecycle control and apply per-export policy | Observability provider | high | Revert U6; disable feature | preferred before rollout | planned |
| Wire endpoint and attachment | Daemon CLI and tracing init | moderate | Revert U8/U9 | no | planned |
| Add safe fallible worker launch and isolate synchronous cleanup | Daemon lifecycle runner | high | Revert U11-U14; disable feature | preferred before rollout | planned |
| Verify residual behavior and observe | Focused harness and logs | low | Return defect to Stage | no | planned |

Protected invariants: compiling REDs precede behavior GREENs; the layer-held tracer/provider clone is never misdescribed as dropped; failed spawn cannot drop the sole explicit owner or invoke synchronous fallback; all spawn errors propagate through `EngramError`; no `unwrap`, `expect`, `panic`, or `unsafe` exists in the production cleanup path; no test mutates process-global environment; no command omits `cozo-backend`; per-export and daemon-wait constants are never conflated; timeout diagnostics say completion unknown; no runtime-owned blocking task or join can extend daemon return; all work remains under two hours.

## Standard Plan Review History and Mandatory Escalation

Gate: **BLOCKED PENDING MANDATORY ADVERSARIAL ESCALATION**. The prior local PASS language is withdrawn as an executable gate. Seven P1 findings were accumulated before review `5016087555`, crossing the repository threshold of three. Review `131.001-R` therefore requires the configured Adversarial Review workflow over the complete pinned PR planning scope.

| ID | Lens | Severity | Finding | Current plan disposition |
|---|---|---|---|---|
| R3 | Rust/API | P1 | Dropping the constructor-local provider is not a RED because the layer retains a tracer and provider clone. | U5 proves retained export, then fails on unavailable application control; U6 owns the handle. |
| T4 | Test strategy | P1 | Rust 2024 forbids unsafe in-process environment mutation and parallel tests would race. | U7 uses only child-process `Command::env` and `env_remove`; U8 owns GREEN. |
| R2 | Rust/API | P1 | SDK max export timeout does not bound synchronous cleanup. | U6 limits its claim to each export future; U10/U14 bound daemon wait only. |
| A2 | Architecture | P1 | Async timeout around blocking cleanup could leave runtime-owned work and delay shutdown. | U13 uses a detached native worker; U14 waits only on a channel with no join. |
| T3 | Test strategy | P1 | Timeout tests could pass without proving the worker remained blocked. | U10 uses entered and release barriers, paused time, 4,999 ms pending proof, and test-only reap. |
| O2 | Operations | P1 | Timed-out resources and telemetry loss were undocumented. | U14-U16 require explicit unknown, retained or detached residual diagnostics, monitoring, and rollback. |
| C2 | Constitution | P1 | OTLP no-default commands omitted required Cozo surfaces. | Every outer or nested command uses the exact required `cozo-backend` feature set. |
| SPAWN-1 | Rust safety and lifecycle | P1 | Worker creation had no deterministic test-first failure seam and could panic or silently drop into cleanup. | U11 scaffolds injection, U12 compiles then fails deterministically, U13 owns safe `Builder::spawn` and `EngramError` GREEN, and U14 owns timeout behavior. |

Secondary remediations remain in force: compile-first behavior failures name their GREEN owner; endpoint failures distinguish child invocation from assertion failure; the provider-retention baseline is already GREEN; and child stderr remains captured. The graph now has sixteen tasks, fifteen linear edges, seventeen shipment items, and estimates of 45-105 minutes.

### Escalation evidence gate

At least three independent reviewers must each directly cover architecture, security and TOCTOU, concurrency and lifecycle, Rust safety, scope and width, constitution, and every table entry above. Every counted response requires authoritative execution-system metadata that binds a stable task or response ID to the explicit invocation and model override, exact reviewed commit, reviewer slot, and exact instruction manifest. Checked-in routing, requested labels, and reviewer self-assertion are not sufficient.

HIGH-confidence P0/P1 blocks. Every MEDIUM finding must be fixed or explicitly deferred with rationale. LOW remains advisory. If fewer than three eligible receipts exist, no confidence calculation or consensus claim is permitted and `131-F`, `131.001-R`, and `125-S` remain blocked.

Copilot review `5016087555` directly covered 78 of 78 files at source head `e00c650eb06073a67a9f228e1fd056c3c359ecb7`. That is source-head evidence only. This remediation expands the diff; final review evidence must state its own pinned commit and actual file count rather than inheriting 78/78.

## References

- PR 363 reviews `5015373740`, `5015447062`, `5015636140`, `5015710467`, `5015926424`, and `5016087555`; unresolved source-head comments `3850407649`, `3850407688`, and `3850544771`
- Stash `44E573BC`; feature `131-F`; review `131.001-R`; shipment `125-S`
- `Cargo.toml`, `Cargo.lock`
- `src/server/observability.rs`, `src/lib.rs`, `src/bin/engram.rs`
- `docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md`
- `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`
- `docs/compound/best-practices/rust-2024-set-var-unsafe-2026-05-07.md`
- `.github/instructions/strict-safety.instructions.md`
- `.github/instructions/release-observability.instructions.md`
