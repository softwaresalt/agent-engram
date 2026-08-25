---
title: "Align the tracing bridge and repair OpenTelemetry 0.26 lifecycle usage"
type: decision
doc_type: decision
source: "stash 44E573BC"
date: 2026-08-24
status: decided
source_stash_id: "44E573BC"
promoted_to:
  - docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md
---

# Align the tracing bridge and repair OpenTelemetry 0.26 lifecycle usage

## Problem Frame

The optional `otlp-export` feature does not compile under all-features because `src/server/observability.rs` uses API names from a newer OpenTelemetry release: `SdkTracerProvider` and `SpanExporter::builder()`. A code-only migration would still fail because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while the direct SDK/exporter stack is 0.26. A layer-only repair is also runtime-invalid: the 0.26 tracer weak-references its provider, so dropping the local provider when `build_otlp_layer` returns turns later spans into no-ops.

## Evidence

`cargo tree --features otlp-export` shows two incompatible type families: `tracing-opentelemetry` 0.26 pulls `opentelemetry` and `opentelemetry_sdk` 0.25, while Engram and `opentelemetry-otlp` use 0.26. `tracing-opentelemetry` 0.27 depends on the 0.26 family, making it the narrow bridge alignment. Local 0.26 APIs expose `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().build_span_exporter()`, explicit batch runtime configuration, and provider flush/shutdown. Engram has no current caller that retains the provider and no test that emits an OTLP-backed span.

## Decision

Keep the direct OpenTelemetry SDK/exporter stack at 0.26, align only `tracing-opentelemetry` from 0.26 to 0.27, and migrate the feature-gated implementation to the 0.26 API. Replace the feature-enabled layer-only return with an explicit lifecycle owner that contains the layer attachment and retains `TracerProvider` through subscriber use. The owner must provide exactly-once coordinated flush/shutdown, propagate exporter or shutdown failure, use a finite export bound with no unbounded retry, and never treat provider `Drop` as successful shutdown.

Add test-first contracts using a test-only in-process exporter path shared with the production provider/layer constructor. Attach the layer to a local subscriber, emit one uniquely named span, retain the lifecycle owner, invoke explicit shutdown/flush, and deterministically assert the exported span through a bounded channel/timeout. No socket, external collector, or network oracle is required.

## Constraints

- Independent shipment; no workspace-identity files.
- RED lifecycle contracts precede manifest, lockfile, or production changes.
- Dependency scope is exactly `tracing-opentelemetry` 0.26 to 0.27 plus resulting lockfile reconciliation; no broader OpenTelemetry upgrade.
- `131.002-T` owns provider retention and shutdown/flush implementation; `131.003-T` independently verifies all-features and runtime export behavior.
- Provider ownership must outlive layer/subscriber use; explicit shutdown must flush or return a bounded error.
- The deterministic local exporter is the runtime oracle; an external collector is neither required nor sufficient.

## References

- Stash `44E573BC`
- Feature `131-F`; shipment `125-S`
- `Cargo.toml` OpenTelemetry 0.26 entries
- `cargo tree --features otlp-export` split 0.25/0.26 evidence
- `tracing-opentelemetry` 0.27 dependency metadata
- `src/server/observability.rs`
- local `opentelemetry-otlp` 0.26 documentation
