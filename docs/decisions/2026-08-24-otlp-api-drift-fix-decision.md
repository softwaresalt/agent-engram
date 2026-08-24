---
title: "Repair pinned OpenTelemetry 0.26 API usage"
type: decision
doc_type: decision
source: "stash 44E573BC"
date: 2026-08-24
status: decided
source_stash_id: "44E573BC"
promoted_to:
  - docs/exec-plans/2026-08-24-44e573bc-otlp-api-drift-plan.md
---

# Repair pinned OpenTelemetry 0.26 API usage

## Problem Frame

The optional `otlp-export` feature does not compile under all-features because `src/server/observability.rs` uses API names from a newer OpenTelemetry release: `SdkTracerProvider` and `SpanExporter::builder()`.

## Evidence

The lockfile and manifest pin OpenTelemetry 0.26. Local crate source exposes `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().build_span_exporter()`, and `TracerProvider::builder().with_batch_exporter(exporter, runtime)`. No dependency bump is needed. No existing test directly covers `build_otlp_layer`.

## Decision

Keep the pinned dependency set and migrate only the feature-gated implementation to the 0.26 API. Add a feature-gated construction contract first, capture the current compile failure as RED, then make the smallest production edit and run all-features compile/lint/tests.

## Constraints

- Independent shipment; no workspace-identity files.
- No exporter connection or external collector is required to construct the layer.
- RED commit precedes GREEN implementation.
- No opportunistic OpenTelemetry version upgrade or observability redesign.

## References

- Stash `44E573BC`
- `Cargo.toml` OpenTelemetry 0.26 entries
- `src/server/observability.rs`
- local `opentelemetry-otlp` 0.26 documentation
