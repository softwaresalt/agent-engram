---
title: Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed
source: docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md
source_stash_id: 44E573BC
review_source: PR 363 review 5015140545
---

# Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle

## Problem Frame

The optional `otlp-export` target currently cannot compile. `tracing-opentelemetry` 0.26 introduces OpenTelemetry 0.25 beside the direct 0.26 family, while `src/server/observability.rs` names APIs that are not available in 0.26. The current builder also returns only a layer, drops its provider, has no production caller, has no application-owned exporter timeout, and has no explicit cleanup path.

Review 5015140545 identified a sequencing defect: a feature-enabled test that imports future provider or lifecycle symbols cannot reach a compiling-but-failing state while the production target itself fails to compile. A compile error in either production or the test is not RED behavior evidence. The repaired sequence first runs a meta-harness with `otlp-export` disabled, then aligns the graph, then repairs only feature compilation, then introduces an explicit behavior-neutral seam, and only then adds feature-enabled provider and daemon RED tests against interfaces that already exist.

Read-only inspection confirms the runtime boundary. `Command::Daemon` is currently a unit variant, calls `init_tracing(LogFormat)` before `daemon::run`, and has no endpoint. `GlobalFlags` has no endpoint. `Config::otlp_endpoint` has no production parser caller. `PluginConfig::load` occurs too late. The shim child inherits environment. The daemon subcommand is therefore the sole endpoint boundary.

## Requirements Trace

| Requirement | Planned owner |
|---|---|
| Compiling initial RED | U1 uses a default-feature-disabled meta-harness and runtime subprocess assertions only. |
| One OpenTelemetry family | U2 changes only the bridge and lockfile. |
| Compiling optional target | U3 repairs supported 0.26 API use while preserving lifecycle defects. |
| Explicit test seam | U4 introduces shared exporter and tracing-pipeline interfaces with no behavior GREEN. |
| Compiling provider RED | U5 tests retained export and timeout through the existing seam. |
| Provider ownership and timeout | U6 retains the provider and applies the application timeout in provider construction. |
| Compiling daemon RED | U7 tests parser, exact endpoint handoff, attachment, and lifetime through existing APIs. |
| Endpoint propagation | U8 owns the daemon CLI and typed handoff. |
| Attachment and lifetime | U9 attaches the layer and retains the owner across daemon use. |
| Compiling cleanup RED | U10 tests the existing lifecycle runner before cleanup implementation. |
| Exactly-once cleanup | U11 owns flush, shutdown, failure precedence, and diagnostics. |
| Runtime proof | U12 reruns every RED command unchanged and verifies the complete path. |
| Closure | U13 records dependency, all-features, default, monitoring, and rollback evidence. |

## Exact RED Sequence

### RED A: compile-neutral graph and feature contract

U1 adds only `tests/otlp_feature_compile_contract_test.rs`. The test target is built without `otlp-export`, imports no production OTLP symbol, and then launches bounded subprocess checks.

```text
cargo test --no-default-features --test otlp_feature_compile_contract_test -- --nocapture
```

Assertions and expected failures at the starting implementation state:

1. `otlp_dependency_graph_uses_only_026` captures `cargo tree --no-default-features --features otlp-export` and asserts that no `opentelemetry v0.25` package is present. The harness compiles and the assertion fails because both 0.25 and 0.26 are present. U2 owns GREEN.
2. `otlp_export_feature_compiles` runs `cargo check --no-default-features --features otlp-export --lib` in an isolated temporary `CARGO_TARGET_DIR` and asserts success. The harness compiles and the assertion fails because the nested command exits 101 with the existing `SdkTracerProvider` and `SpanExporter::builder` unsupported-0.26 diagnostics. U3 owns GREEN.

The subprocess command is `cargo check --lib`, never `cargo test`, so the meta-harness cannot recurse. Invocation failure is distinct from the expected nonzero result, and captured stderr is printed in the assertion.

### RED B: provider ownership, export, and timeout

U4 first creates shared interfaces using already compiling 0.26 types. U5 then adds tests only to the existing observability test module.

```text
cargo test --no-default-features --features otlp-export --lib server::observability::tests::otlp_provider_red -- --nocapture
```

The library and tests must compile before these assertions run:

1. Emit `otlp-red-contract-span`, request flush, and assert `exported_span_names == [otlp-red-contract-span]`. Expected RED is an empty list because the layer-only path drops its provider.
2. Assert the production effective timeout is exactly `Duration::from_secs(5)` even when `OTEL_BSP_EXPORT_TIMEOUT` is set differently. Expected RED is the SDK or environment value because no application override is wired.
3. Inject a never-ready exporter with a cancellation token and test-only 25 ms limit. Pause Tokio time, advance 24 ms and assert not cancelled, then advance 1 ms and assert cancelled with a phase-and-limit failure. Expected RED is no bounded cancellation or report.

U6 owns GREEN without changing the tests.

### RED C: endpoint, attachment, and owner lifetime

```text
cargo test --no-default-features --features otlp-export otlp_daemon_red -- --nocapture
```

The lib and bin tests compile against U4 interfaces. `Cli::try_parse_from` and `Debug` output avoid direct access to a future enum field. Expected failures are a runtime clap unexpected-argument result for `--otlp-endpoint`, no recorded endpoint at the factory, and no exported span or retained owner. U8 makes endpoint assertions GREEN. U9 makes attachment and lifetime assertions GREEN.

### RED D: cleanup coordination

U9 creates the lifecycle runner while intentionally omitting cleanup. U10 then tests that current helper.

```text
cargo test --no-default-features --features otlp-export --bin engram otlp_cleanup_red -- --nocapture
```

The target and tests compile. Clean exit initially reports `(flush_calls, shutdown_calls) == (0, 0)` instead of `(1, 1)`. Combined daemon and cleanup failure lacks the cleanup diagnostic, and timeout failure is not surfaced. U11 owns GREEN without test edits.

## Timeout Ownership and Semantics

`src/server/observability.rs` owns `OTLP_EXPORT_TIMEOUT = Duration::from_secs(5)`. It is application source policy, not CLI, workspace config, environment, or operator input. Production provider construction starts from `BatchConfigBuilder::default()` and then calls `with_max_export_timeout(OTLP_EXPORT_TIMEOUT)`, so `OTEL_BSP_EXPORT_TIMEOUT` cannot override the final production value.

The SDK `force_flush()` and `shutdown()` calls are synchronous and accept no timeout. Their exporter work is bounded where the SDK supports cancellation: the batch processor export future. Each phase has a five-second application cap. Cleanup always attempts one flush and one shutdown, so the declared two-phase maximum is ten seconds. A flush error does not skip shutdown. There is no retry and no outer async wrapper that pretends to cancel a still-running blocking thread.

The test-only constructor can inject 25 ms but production cannot. A pending fake exporter future carries a drop token. Paused Tokio time proves it remains live at 24 ms and is cancelled at 25 ms. Failure reports include phase, configured limit, and source. A clean daemon returns cleanup failure. If daemon and cleanup both fail, the daemon error remains primary and the cleanup failure is emitted diagnostically with phase and limit.

## Implementation Units

### U1 / 131.001-T — RED compile-neutral meta-harness

One test file, at most three test/helper functions, two scenarios, 75 minutes. No manifest, lockfile, source, missing-symbol probe, external collector, socket, or nested test invocation.

### U2 / 131.002-T — GREEN bridge graph

Only `Cargo.toml` and `Cargo.lock`, zero functions, one graph scenario, 45 minutes. The graph assertion turns GREEN while feature compilation stays RED.

### U3 / 131.003-T — GREEN pinned-0.26 compile baseline

Only `src/server/observability.rs`, at most three functions, two compile scenarios, 75 minutes. Use `trace::TracerProvider`, `new_exporter().tonic().with_endpoint(...).build_span_exporter()`, and the supported Tokio batch runtime. Preserve layer-only provider drop, no application timeout, no caller, and no cleanup.

### U4 / 131.004-T — SCAFFOLD behavior-neutral seams

Only `src/server/observability.rs` and `src/lib.rs`, at most four interface/adapter functions, two behavior-preservation scenarios, 90 minutes. Add shared exporter-factory and tracing-pipeline injection boundaries. Existing production behavior remains layer-only and formatting-only. No RED test or behavior implementation is hidden here.

### U5 / 131.005-T — RED provider lifecycle

One test module, at most four test/helper functions, three scenarios, 100 minutes. The feature-enabled harness compiles and fails only for retained export, exact timeout, and deterministic cancellation/reporting behavior.

### U6 / 131.006-T — GREEN retained provider and timeout

Only `src/server/observability.rs`, at most four functions, three scenarios, 105 minutes. Retain the provider, expose layer plus explicit lifecycle results, and wire the five-second constant after environment-derived defaults. No production attachment.

### U7 / 131.007-T — RED daemon endpoint and attachment

Test-only modules in `src/bin/engram.rs` and `src/lib.rs`, at most four test/helper functions, three scenarios, 105 minutes. Tests compile against U4 and U6 interfaces and fail at runtime assertions only.

### U8 / 131.008-T — GREEN endpoint propagation

Only `src/bin/engram.rs` and `src/lib.rs`, at most four functions, three scenarios, 95 minutes. Add daemon-only flag and environment resolution and pass one typed value to the existing tracing seam. Attachment remains RED.

### U9 / 131.009-T — GREEN attachment and lifetime

Only `src/lib.rs` and the daemon arm/helper in `src/bin/engram.rs`, at most four functions, three scenarios, 105 minutes. Attach beside stderr formatting, return the owner, and retain it across the complete daemon future. Cleanup remains absent.

### U10 / 131.010-T — RED cleanup

One bin test module, at most four test/helper functions, three scenarios, 90 minutes. Test the existing runner for exactly-once clean, combined-failure, and timeout behavior.

### U11 / 131.011-T — GREEN cleanup coordination

Only the daemon runner/arm in `src/bin/engram.rs`, at most four functions, three scenarios, 105 minutes. Flush and shutdown once each on every exit, attempt shutdown after flush failure, and preserve failure precedence and diagnostics.

### U12 / 131.012-T — VERIFY runtime

At most two unchanged evidence surfaces, zero production functions, three scenario groups, 80 minutes. Rerun all four RED commands unchanged and prove endpoint-to-export-to-cleanup behavior without network.

### U13 / 131.013-T — VERIFY quality and operations

At most two evidence surfaces, zero functions, three gate groups, 90 minutes. Record dependency, all-features, focused, default, monitoring, and rollback evidence.

## Dependency Graph and Shipment

Strict chain:

```text
131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T
-> 131.006-T -> 131.007-T -> 131.008-T -> 131.009-T -> 131.010-T
-> 131.011-T -> 131.012-T -> 131.013-T
```

Shipment `125-S` contains parent `131-F` and all thirteen tasks in that order. It remains the sole queued and unclaimed shipment. Blocked shipments `126-S` through `129-S` are unchanged. PR 362 ordering is already satisfied; exact reviewed PR 363 integration and all claim guards remain required.

## Runtime Verification, Monitoring, and Rollback

Precheck the optional feature and Tokio runtime. U12 runs the exact RED commands unchanged. It must observe one uniquely named span, owner retention through daemon use, cancellation at the injected virtual limit, the production five-second per-phase value, at most ten seconds for two sequential phases, exactly one flush and one shutdown, and visible failure precedence.

Ship owns a 30-minute or three-controlled-daemon-exit observation window after deployment. The manual log query counts OTLP export failure, cleanup timeout, cleanup failure, and daemon exit duration. Baseline and healthy threshold are zero timeout or failure records and every controlled exit below ten seconds. Rollback triggers are any exit reaching ten seconds, any hidden cleanup failure, a missing expected span in focused verification, or any default/all-features regression. Rollback is to disable `otlp-export` and revert the owning GREEN commit or commits; keep `125-S` unclaimed or return the defect to Stage until evidence is clean.

## Plan Hardening

Hardening rerun: **required and satisfied** for external export, provider lifetime, synchronous cleanup, timeout ownership, and daemon error precedence.

Reinforcing context included the constitution, strict-safety and release-observability instructions, OpenTelemetry 0.26 API evidence in review 5015140545, and compound guidance that a scope-local timeout does not prove a wider operation bounded. The design therefore names both the five-second exporter phase cap and the derived ten-second two-phase maximum, and it avoids an outer timeout that would abandon a blocking thread without cancellation.

| ProposedAction | Targets | ActionRisk | Rollback | Approval required | ActionResult |
|---|---|---|---|---|---|
| Add compile-neutral RED meta-harness | One test file | moderate | Revert U1 | no | planned |
| Align dependency family | Cargo manifest and lockfile | moderate | Restore bridge 0.26 | no | planned |
| Repair compile baseline | Observability constructor | moderate | Revert U3 | no | planned |
| Add shared test seams | Observability and tracing init | moderate | Revert U4 | no | planned |
| Retain provider and own timeout | Observability provider | high | Revert U6 and keep feature disabled | preferred before rollout | planned |
| Wire endpoint and attachment | Daemon CLI and tracing init | moderate | Revert U8 and U9 | no | planned |
| Coordinate cleanup | Daemon lifecycle runner | high | Revert U11 and disable feature | preferred before rollout | planned |
| Verify and observe | Focused harness and logs | low | Return defect to Stage | no | planned |

Protected invariants: every behavior GREEN has a compiling runtime RED; U1 never enables the broken feature in its own compilation; U4 adds interface only; production timeout is application-owned and environment-independent; pending export is actually cancelled by the SDK timeout; provider ownership outlives daemon use; flush and shutdown are exactly once; shutdown follows flush failure; no network oracle or unrelated config redesign is introduced; all tasks remain under two hours.

## Plan Review

Gate: **PASS**. Standard review was rerun after hardening using constitution, Rust/API, architecture, scope, test-strategy, operational-readiness, learnings, and external-boundary security lenses. Cross-model and intercom dispatch were unavailable, so the local reviewer lenses are disclosed. This non-security maintenance plan has no unresolved P0 or P1 finding.

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| C1 | Constitution | P1 | Original U1 could not produce a compiling RED. | Resolved by U1 runtime meta-harness, U2 graph GREEN, and U3 compile GREEN. |
| T1 | Test strategy | P1 | Original provider and daemon RED named behavior before usable interfaces existed. | Resolved by explicit behavior-neutral U4 seam before U5 and U7. |
| R1 | Rust/API | P1 | Synchronous cleanup could inherit the operator-controlled SDK default timeout. | Resolved by U6 constant plus `with_max_export_timeout` after defaults. |
| A1 | Architecture | P1 | A broad combined repair would mix Cargo, provider, CLI, attachment, and lifecycle concerns. | Resolved by thirteen linear units with at most two files and one domain each. |
| O1 | Operational readiness | P1 | Timeout cancellation, reporting, monitoring, and rollback were incomplete. | Resolved by paused-time cancellation in U5/U6, cleanup reporting in U10/U11, and U13 observation and rollback gates. |
| S1 | Scope boundary | P2 | A seam task could hide implementation ahead of RED. | Resolved: U4 is behavior-preserving, has no tests, and forbids ownership, timeout, endpoint, attachment, and cleanup behavior. |
| T2 | Test strategy | P2 | A cargo subprocess test could recurse or fail to invoke. | Resolved: U1 invokes only `cargo tree` and `cargo check --lib`, uses an isolated target directory, and distinguishes invocation errors. |
| X1 | Security lens | P3 | OTLP tests could introduce network or secret handling. | Resolved by injected in-process exporters and explicit no-network/no-credential scope. |

The review confirms the exact thirteen-edge chain, fourteen-item parent-first roster, 45 to 105 minute estimates, application timeout ownership, deterministic cancellation test, monitoring and rollback plan, and queued/unclaimed `125-S` state.

## References

- PR 363 review 5015140545 and threads `PRRT_kwDORJEduc6b8O89`, `PRRT_kwDORJEduc6b8UJJ`, `PRRT_kwDORJEduc6b8UIv`
- Stash `44E573BC`; feature `131-F`; shipment `125-S`
- `docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md`
- `src/server/observability.rs`, `src/lib.rs`, and `src/bin/engram.rs`
- `docs/compound/workflow-issues/linked-worktree-shared-startup-deadline-exact-cleanup-2026-08-19.md`
- `.github/instructions/strict-safety.instructions.md`
- `.github/instructions/release-observability.instructions.md`
