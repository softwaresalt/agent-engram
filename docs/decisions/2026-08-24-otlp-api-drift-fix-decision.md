---
title: "Align the tracing bridge and repair OpenTelemetry 0.26 API usage"
type: decision
doc_type: decision
source: "stash 44E573BC"
date: 2026-08-24
status: decided
source_stash_id: "44E573BC"
promoted_to:
  - docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md
---

# Align the tracing bridge and repair OpenTelemetry 0.26 API usage

## Problem Frame

The optional `otlp-export` feature does not compile under all-features because `src/server/observability.rs` uses API names from a newer OpenTelemetry release: `SdkTracerProvider` and `SpanExporter::builder()`. A code-only migration would still fail because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while the direct SDK/exporter stack is 0.26.

## Evidence

`cargo tree --features otlp-export` shows two incompatible type families: `tracing-opentelemetry` 0.26 pulls `opentelemetry` and `opentelemetry_sdk` 0.25, while Engram and `opentelemetry-otlp` use 0.26. `tracing-opentelemetry` 0.27 depends on `opentelemetry` and `opentelemetry_sdk` 0.26, making it the narrow bridge alignment. Local 0.26 crate source exposes `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().build_span_exporter()`, and `TracerProvider::builder().with_batch_exporter(exporter, runtime)`. No existing test directly covers `build_otlp_layer`.

## Decision

Keep the direct OpenTelemetry SDK/exporter stack at 0.26, align only `tracing-opentelemetry` from 0.26 to 0.27, and migrate the feature-gated implementation to the 0.26 API. Add a feature-gated construction contract first, capture the current compile failure and split dependency graph as RED evidence, then apply the exact manifest/lock/source changes and run dependency-tree plus all-features compile/lint/tests.

## Constraints

- Independent shipment; no workspace-identity files.
- No exporter connection or external collector is required to construct the layer.
- RED commit precedes GREEN implementation.
- Dependency scope is exactly `tracing-opentelemetry` 0.26 to 0.27 plus the resulting lockfile reconciliation; no broader OpenTelemetry upgrade or observability redesign.

## References

- Stash `44E573BC`
- `Cargo.toml` OpenTelemetry 0.26 entries
- `cargo tree --features otlp-export` split 0.25/0.26 evidence
- `tracing-opentelemetry` 0.27 dependency metadata
- `src/server/observability.rs`
- local `opentelemetry-otlp` 0.26 documentation
