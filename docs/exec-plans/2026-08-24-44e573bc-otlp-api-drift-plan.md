---
title: "Align the tracing bridge and repair optional OTLP against OpenTelemetry 0.26"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed
source: docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md
source_stash_id: "44E573BC"
---

# Align the tracing bridge and repair optional OTLP against OpenTelemetry 0.26

## Problem Frame

The `otlp-export` feature does not compile because `src/server/observability.rs` imports `SdkTracerProvider` and calls `SpanExporter::builder()`, neither of which exists in pinned 0.26. The supported API is `trace::TracerProvider` plus `new_exporter().tonic().build_span_exporter()` and an explicit batch runtime. The locked graph also contains `tracing-opentelemetry` 0.26 against OpenTelemetry 0.25, so its tracer type cannot accept the direct 0.26 SDK tracer; bridge 0.27 is the compatible narrow alignment.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Test first | U1 adds a feature-gated construction contract and captures the current compile failure plus split dependency graph. |
| One OpenTelemetry type family | U2 aligns only `tracing-opentelemetry` 0.26 to 0.27 and reconciles the lockfile to 0.26. |
| Pinned API compatibility | U2 migrates imports/builder/runtime without changing the direct 0.26 stack. |
| All-features closure | U3 checks the dependency tree, then runs compile, lint, and tests with all features. |
| Width isolation | Only Cargo.toml, Cargo.lock, and OTLP observability code/test; no workspace identity files. |

## Implementation Units

### U1 — RED: feature-gated OTLP layer construction contract

Add one `#[cfg(feature = "otlp-export")]` unit or focused test that calls `build_otlp_layer` with a loopback OTLP URI and requires construction to succeed without connecting to a collector. Capture RED with the pinned all-features compile failure: unresolved `SdkTracerProvider` and missing `SpanExporter::builder`. Record `cargo tree` evidence that bridge 0.26 introduces OpenTelemetry 0.25 beside the direct 0.26 stack. Test-only, one file, one scenario, target 60 minutes.

### U2 — GREEN: align the tracing bridge and migrate to the 0.26 API

Change only `tracing-opentelemetry` from 0.26 to 0.27 in `Cargo.toml` and reconcile `Cargo.lock` so the bridge uses OpenTelemetry 0.26. In `src/server/observability.rs`, use `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().with_endpoint(...).build_span_exporter()`, and `with_batch_exporter(exporter, opentelemetry_sdk::runtime::Tokio)`. Keep the returned layer contract and all other dependency versions unchanged. U1 compiles and passes. Cargo.toml, Cargo.lock, and one production file, target 90 minutes.

### U3 — all-features verification

Run `cargo tree` to prove the OTLP feature graph no longer contains OpenTelemetry 0.25, then run the focused feature test, `cargo check --all-features`, all-features clippy with repository warning policy, and relevant all-features tests. Confirm default-feature gates remain green and no collector/network dependency is introduced. Verification only, target 90 minutes.

## Dependency Graph

`U1 -> U2 -> U3`. Independent release unit with no cross-shipment dependency.

## Decisions and Rationale

- Align the tracing bridge to the release that uses the already-pinned 0.26 type family rather than upgrading the four direct telemetry crates.
- Construction-only test avoids external collector flakiness.
- Do not redesign provider lifetime or global subscriber behavior unless RED evidence proves another defect.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Layer construction starts network work | Use local URI and assert construction only; no export. |
| Provider runtime requires Tokio | Use the enabled `rt-tokio` feature and explicit runtime required by 0.26. |
| Lockfile update broadens beyond the bridge | Require exact `cargo tree` evidence and reject unrelated package changes. |
| Fix drifts default build | Run default and all-features gates. |

## Plan Hardening Signals

- Public API/schema/contract: absent; one optional dependency version aligns existing type families.
- Security/auth/compliance: absent.
- Migration/destructive action: absent.
- External integration/checkpoint: absent for construction; no live collector.
- High runtime/rollback risk: absent; optional compile-only repair.

Requires plan hardening: no

## Runtime Verification and Closure

Runtime surface is optional trace export. Before closure, construct the layer under the feature and, only if an approved local collector is available, emit one smoke span; collector absence does not block compile repair. Rollback is the U2 commit. Trigger: all-features compile/lint failure or default-feature regression.

## Plan Review

Gate: **PASS**. Hardening not required because every signal is absent. Personas applied: constitution, Rust/API, architecture, scope, tests, operational readiness, and learnings.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| T1 | P1 | A code-only compile fix would violate repository test-first policy. | Resolved: U1 adds the construction contract before U2. |
| R1 | P1 | The 0.26 batch exporter requires an explicit runtime argument. | Resolved in U2. |
| A1 | P2 | A broad dependency upgrade would widen the lockfile and compatibility surface. | Resolved: align only `tracing-opentelemetry` 0.26 to 0.27; keep the direct stack at 0.26. |
| D1 | P1 | A code-only migration leaves bridge 0.26 on OpenTelemetry 0.25, incompatible with the direct 0.26 tracer. | Resolved: U1 records the split graph; U2 aligns bridge 0.27 and U3 proves one 0.26 type family. |
| O1 | P3 | A live collector smoke is useful but environment-dependent. | Optional closure evidence, not a gate for the compile defect. |

No unresolved P0/P1 findings. Review-fix cycles: 2 of 3. This plan remains approved for harvest.
