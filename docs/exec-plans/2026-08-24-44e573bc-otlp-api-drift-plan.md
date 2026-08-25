---
title: Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: blocked
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
| Correct graph and exact bridge evidence | U1/U2 pin one SDK 0.26 family and exact tracing bridge 0.27 retention source. |
| Compiling optional target | U3 repairs supported SDK 0.26 APIs without weakening bridge-retention truth. |
| Explicit tracing and subscriber seam | U4 introduces behavior-neutral exporter, pipeline, cfg-arm, and subscriber-isolation interfaces. |
| Compiling provider RED | U5 proves 0.27-bridge layer-held export under an isolated subscriber before lifecycle assertions fail. |
| Application lifecycle control | U6 returns an explicit provider handle and applies the per-export-future limit. |
| Compiling daemon RED | U7 uses child-process environment setup, redacted diagnostics, and isolated subscriber attachment. |
| Single endpoint authority | U8 reuses or retires existing endpoint authority and forbids duplicate flags. |
| Attachment and lifetime | U9 uses fallible global install for production and isolated Registry or `with_default` in tests. |
| Finite compiling cleanup RED | U10 tests returned and stalled cleanup with a test-side anti-hang watchdog. |
| Dedicated spawn error vocabulary | U11 adds truthful `CleanupWorkerSpawnFailed` variant and stable code without runtime use. |
| Behavior-neutral spawn seam | U12 adds injectable spawner and retained-owner sink without launching work. |
| Deterministic spawn-failure RED | U13 forces realistic `WouldBlock` and `OutOfMemory` errors after compilation. |
| Safe worker launch and error propagation | U14 uses `Builder::spawn`, dedicated `EngramError`, and owner retention without panic or fallback. |
| Honest application cleanup bound | U15 bounds daemon wait and defines all cleanup outcome precedence. |
| Runtime proof | U16 reruns all five RED commands unchanged and verifies static and behavioral safety. |
| Closure | U17 records all four quality gates, production diagnostic visibility, exit status, monitoring, and rollback or fails closed. |

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

U11 first defines exact `DaemonError::CleanupWorkerSpawnFailed { reason }` and a distinct stable code in the error files without using it. U12 supplies a behavior-neutral `CleanupWorkerSpawner` and injected retained-owner sink; the daemon still launches no worker. U13 then installs fake spawners that record one request and return realistic `WouldBlock` and `OutOfMemory` errors. Production bin and tests compile before assertions execute. The intended RED is missing spawner and sink invocation.

GREEN belongs only to U14. A clean daemon receives the dedicated `EngramError`, not daemon-process `SpawnFailed`. Diagnostic fields are `operation=otlp_cleanup_worker_spawn`, `cleanup_attempted=false`, `completion=not_started`, `worker_detached=false`, and `provider_residual=retained_for_process_lifetime`; URI userinfo and query never appear. Cleanup calls remain zero. Existing daemon error stays primary and one complete secondary diagnostic is emitted. No OS exhaustion, allocation pressure, sleep, panic hook, `unwrap`, `expect`, `panic`, `unsafe`, retry, or synchronous fallback is allowed.

## Cleanup Ownership and Honest Guarantees

`OTLP_EXPORT_TIMEOUT = 5s` limits each SDK exporter future. `OTLP_CLEANUP_WAIT_TIMEOUT = 5s` limits one daemon wait after successful worker launch. Neither limits synchronous SDK cleanup.

U11 provides a truthful cleanup-thread spawn error and distinct stable code. U12 provides an injectable spawner and retained-owner sink. The production adapter calls only `std::thread::Builder::new().name(...).spawn(job)` and returns the `Result`; the runner does not invoke it until U14. `std::thread::spawn`, `unwrap`, `expect`, `panic`, `unsafe`, `spawn_blocking`, runtime-owned cleanup tasks, joins, and retries are prohibited.

Before launch, caller and job hold separate `Arc` references to one ownership cell. On successful spawn, caller releases its duplicate and worker safely takes the owner. Lock poisoning or empty transfer is sent as a distinct worker failure and cannot become timeout. On spawn error, the injected sink receives the caller-held cell before return. Production retains it for the fatal fail-exit process lifetime; tests record retention without leaking. No cleanup method or provider Drop is used as fallback.

After launch, worker emits phases, calls `force_flush` once, and calls `shutdown` once only if flush returns. U10 test-side watchdog makes missing deadline fail at 5,001 ms rather than hang. U15 installs one monotonic production deadline. On timeout, receiver drops, native handle is not joined, and result says completion unknown and worker detached.

A pre-existing daemon error is always primary. For a clean daemon: launch failure precedes wait; worker panic or channel loss outranks ownership-transfer failure; transfer failure outranks returned flush or shutdown failure; observed cleanup failure outranks later timeout; pure timeout is last. Preserve all observed secondary detail in one redacted diagnostic.

Cleanup-worker spawn failure is fatal for a clean daemon and returns nonzero. U17 must validate exit code and restart-loop risk. It also fails if manual and shim-spawned production have no observable diagnostic sink. The bounded statement is only: after successful launch, cleanup contributes at most five seconds of daemon wait.

## Implementation Units

### U1 / 131.001-T — RED compile-neutral meta-harness

One test file, at most three test/helper functions, two scenarios, 75 minutes. Outer feature set: `cozo-backend`; nested OTLP commands: `cozo-backend,otlp-export`.

### U2 / 131.002-T — GREEN bridge graph

Only `Cargo.toml` and `Cargo.lock`, zero functions, one graph scenario, 55 minutes. The graph assertion turns GREEN while feature compilation stays RED.

### U3 / 131.003-T — GREEN pinned-0.26 compile baseline

Only `src/server/observability.rs`, at most three functions, two compile scenarios, 75 minutes. Preserve the layer-only return and its transitive tracer/provider clone, but no separately accessible application lifecycle handle, application timeout, caller, or cleanup.

### U4 / 131.004-T — SCAFFOLD behavior-neutral seams

Only `src/server/observability.rs` and `src/lib.rs`, at most four functions, two scenarios, 100 minutes. The behavior-neutral result/control seam can report lifecycle unavailable; existing production behavior remains layer-only and formatting-only while the layer continues to retain its tracer/provider clone.

### U5 / 131.005-T — RED provider lifecycle

One test module, at most four functions, three scenarios, 100 minutes. Prove the actual layer-retention/export baseline, then test missing explicit application lifecycle/flush control and the individual exporter timeout.

### U6 / 131.006-T — GREEN explicit lifecycle handle and per-export timeout

Only `src/server/observability.rs`, at most four functions, three scenarios, 105 minutes. Return a separately accessible application provider handle alongside the layer-held clone; the handle exists for explicit force-flush/shutdown control, not to make the layer live. Synchronous cleanup methods remain explicitly unbounded.

### U7 / 131.007-T — RED daemon endpoint and attachment

Test-only modules in `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenario groups, 105 minutes. Endpoint environment cases run only in self-relaunched child test processes using `Command::env`/`env_remove`.

### U8 / 131.008-T — GREEN endpoint propagation

Only `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenarios, 105 minutes.

### U9 / 131.009-T — GREEN attachment and lifetime

Only `src/lib.rs` and the daemon arm/helper in `src/bin/engram.rs`, at most four functions, three scenarios, 105 minutes. Cleanup remains absent.

### U10 / 131.010-T — RED cleanup isolation

One bin test module, at most four functions, three groups, 105 minutes. Deterministic barriers, paused Tokio time, and a 5,001 ms test watchdog keep every RED finite.

### U11 / 131.011-T — SCAFFOLD dedicated cleanup spawn error

Only `src/errors/mod.rs` and `src/errors/codes.rs`, at most three enum, code, display, or mapping changes, two groups, 60 minutes. Add truthful variant and stable code; no runtime call site.

### U12 / 131.012-T — SCAFFOLD injectable spawner and retained-owner sink

Only `src/bin/engram.rs`, at most four functions or trait methods, two groups, 60 minutes. Add named `Builder::spawn` adapter, Arc ownership cell, and injected production/test residual sinks; launch nothing.

### U13 / 131.013-T — RED deterministic spawn failure

One bin test module, at most four functions, three groups, 90 minutes. `WouldBlock` and `OutOfMemory` fakes compile then fail on missing invocation, dedicated error, zero cleanup, redaction, owner retention, and combined precedence.

### U14 / 131.014-T — GREEN safe worker launch and spawn-error propagation

Only `src/bin/engram.rs`, at most four functions or trait methods, three groups, 100 minutes. Launch once through `Builder::spawn`, preserve Result, map to dedicated `EngramError`, retain owner residual, and green returned cleanup plus spawn failure. Deadline remains RED but finite.

### U15 / 131.015-T — GREEN bounded wait and lifecycle diagnostics

Only `src/bin/engram.rs`, at most four functions, three groups, 105 minutes. Add one total daemon-wait deadline and green both stalls, transfer failure, channel loss or panic, returned failures, and precedence without changing U14 spawn handling.

### U16 / 131.016-T — VERIFY runtime

At most two unchanged evidence surfaces, zero production functions, three groups, 90 minutes. Rerun all five RED commands and verify subscriber isolation, redaction, error codes, and no forbidden launch, panic, unsafe, runtime blocking, join, or fallback path.

### U17 / 131.017-T — VERIFY quality and operations

At most two evidence surfaces, zero functions, three groups, 95 minutes. Run all four quality gates and record bridge evidence, fatal exit behavior, production diagnostic sink, baselines, named queries, monitoring, and rollback; fail closed if sink visibility is absent.

All estimates are 45-105 minutes. Error vocabulary, spawner seam, spawn RED, launch GREEN, bounded-wait GREEN, runtime proof, and closure are separate width-isolated concerns.

## Dependency Graph and Shipment

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T -> 131.014-T -> 131.015-T
-> 131.016-T -> 131.017-T
```

Seventeen tasks produce exactly sixteen task dependency edges. Shipment `125-S` contains parent `131-F` plus all seventeen tasks: eighteen items. Review `131.001-R` is outside roster. Feature, review, and shipment remain blocked after zero complete consensus-eligible responses. Shipments `126-S` through `129-S` remain blocked and untouched.

## Runtime Verification, Monitoring, and Rollback

Ship reruns all five RED commands. Evidence includes pinned bridge 0.27 tracer retention over SDK 0.26; isolated subscriber export; one endpoint authority and redaction; dedicated spawn error for realistic failure kinds; zero cleanup on failed spawn; retained owner residual; returned cleanup; and completion-unknown timeout.

Static and behavioral checks prove no duplicate endpoint flag, second global `init`, `std::thread::spawn`, `unwrap`, `expect`, `panic`, `unsafe`, `spawn_blocking`, runtime join, retry, or synchronous fallback. A test-side watchdog prevents pre-deadline tasks from hanging. Controlled child verification proves no join dependency, not SDK completion.

U17 runs format, clippy, tests, audit, and focused commands in policy order. It names a production diagnostic sink and exact queries for both manual and shim-spawned daemons, records baselines, and observes for 30 minutes or three exits. Null stdio with no durable sink is a blocking failure. Fatal spawn error, restart loop, hidden diagnostic, unexpected Drop fallback, timeout, exit over seven seconds, missing span, or regression disables `otlp-export` and reverts owning GREEN commits.

## Plan Hardening — Exact-Head Rerun

Hardening is **required and satisfied** for external export, provider lifetime, synchronous non-cancellable cleanup, process exit, and rollback.

Reinforcing context: strict-safety and release-observability instructions; pinned OpenTelemetry 0.26 source listed above; `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`, whose key lesson is that an inner timeout does not bound a wider operation.

| ProposedAction | Targets | ActionRisk | Rollback | Approval required | ActionResult |
|---|---|---|---|---|---|
| Add compile-neutral RED harness | One test file | moderate | Revert U1 | no | planned |
| Align dependency family | Cargo manifest and lockfile | moderate | Restore bridge 0.26 | no | planned |
| Expose explicit application lifecycle control and apply per-export policy | Observability provider | high | Revert U6; disable feature | preferred before rollout | planned |
| Wire endpoint and attachment | Daemon CLI and tracing init | moderate | Revert U8/U9 | no | planned |
| Add typed spawn error, safe fallible launch, and isolated cleanup | Error model and daemon runner | high | Revert U11-U15; disable feature | preferred before rollout | planned |
| Verify residual behavior and observe | Focused harness and logs | low | Return defect to Stage | no | planned |

Protected invariants: compiling REDs precede behavior GREENs; the layer-held tracer/provider clone is never misdescribed as dropped; failed spawn cannot drop the sole explicit owner or invoke synchronous fallback; all spawn errors propagate through `EngramError`; no `unwrap`, `expect`, `panic`, or `unsafe` exists in the production cleanup path; no test mutates process-global environment; no command omits `cozo-backend`; per-export and daemon-wait constants are never conflated; timeout diagnostics say completion unknown; no runtime-owned blocking task or join can extend daemon return; all work remains under two hours.

## Standard Plan Review History and Mandatory Escalation

Gate: **FAILED CLOSED / NO CONSENSUS**. Seven prior P1 findings crossed the escalation threshold; cleanup spawn failure added an eighth blocker. Their plan dispositions remain: actual provider retention baseline; child-only environment setup; per-export versus daemon-wait separation; detached native worker; finite barrier oracle; explicit residual operations; exact Cozo feature sets; and U11-U15 spawn-error sequencing.

### Authoritative dispatch receipt result

The configured Adversarial Review workflow reviewed commit `9d6c909e10cfc6ff836f464982145590d6d32a9e`, base `685f62668ac273a41a1f93fc9be2571510decae2`, 83 files, and manifest `5d062b33192e67e80fbfe5d283d3c4482974e65e8c74b6333d16cad4b6b618e9`. Top-level CLI events bind each simultaneous cohort response to session, model-call event, message ID, and model override with zero file changes.

| Slot | Session | Execution model | Model event | Message | Direct complete | Eligible |
|---|---|---|---|---|---|---|
| C | `363c0003-9d6c-4909-8003-000000000003` | `gemini-3.1-pro-preview` | `bda2942c-23f7-432b-9fd9-34f277f3128d` | `f7116735-43b4-4d29-a980-35beffddc000` | No; response synthesized without diff inspection | No |
| D | `363d0004-9d6c-4909-a004-000000000004` | `gpt-5.4-mini` | `0bac4840-14ef-41ec-a99d-3523dedf72d6` | `b2352ae7-bcff-484f-b8b5-f7fbeb610961` | No; response reported unavailable exact diff inspection | No |
| E | `363e0005-9d6c-4909-b005-000000000005` | `claude-sonnet-4.6` | `56ab5941-17d2-441e-a1c2-8a4c4ac344ae` | `aad7fe91-1857-4823-b29a-973eb12b1be5` | No; 23 of 83 files read in full | No |

Consensus-eligible reviewers: **0**. No HIGH, MEDIUM, or LOW confidence classification exists. Supplementary A and B receipts are retained in the closure but not counted because the first three-slot wave had one invocation failure; B raw findings remain unweighted.

### Bounded raw-finding remediations

Unweighted but source-supported observations caused bounded improvements: finite U10 watchdog; dedicated error code instead of daemon-process SpawnFailed; pinned bridge 0.27 retention evidence; isolated subscriber and fallible production install; single endpoint authority and redaction; explicit cleanup precedence; all four quality gates; fatal-exit and production diagnostic-sink gates; and corrected roster memories. The production sink is not assumed solved: U17 fails and returns to Stage if real topology discards diagnostics.

No rerun follows this remediation because unchanged reviewer tooling already failed complete direct coverage; another identical run would not create an eligible denominator. A future review must bind at least three responses and directly cover every changed file and manifest entry. Feature `131-F`, review `131.001-R`, and shipment `125-S` remain blocked.

Copilot review `5016087555` covered 78/78 files at source head `e00c650eb06073a67a9f228e1fd056c3c359ecb7`. This remains source-head-only evidence and is not inherited by later commits.

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
