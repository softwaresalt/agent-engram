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

The optional `otlp-export` feature does not compile under all-features because `src/server/observability.rs` uses API names from a newer OpenTelemetry release: `SdkTracerProvider` and `SpanExporter::builder()`. A code-only migration would still fail because `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25 while the direct SDK/exporter stack is 0.26. A layer-only repair is also runtime-invalid: the 0.26 tracer weak-references its provider, so dropping the local provider when `build_otlp_layer` returns turns later spans into no-ops. Finally, `build_otlp_layer` has no production caller, `Config::otlp_endpoint` is unused, and `src/lib.rs::init_tracing` installs only the formatting layer, so repairing the builder alone would still export no daemon spans.

## Evidence

`cargo tree --features otlp-export` shows two incompatible type families: `tracing-opentelemetry` 0.26 pulls `opentelemetry` and `opentelemetry_sdk` 0.25, while Engram and `opentelemetry-otlp` use 0.26. `tracing-opentelemetry` 0.27 depends on the 0.26 family, making it the narrow bridge alignment. Local 0.26 APIs expose `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().build_span_exporter()`, explicit batch runtime configuration, and provider flush/shutdown. Call-graph inspection finds no caller of either feature variant of `build_otlp_layer`; the daemon command calls `init_tracing` before `engram::daemon::run`, while `init_tracing` installs only the stderr formatting layer. The original research decision also requires attaching OTLP beside that formatting layer.

## Decision

Keep the direct OpenTelemetry SDK/exporter stack at 0.26, align only `tracing-opentelemetry` from 0.26 to 0.27, and migrate the feature-gated implementation to the 0.26 API. Replace the feature-enabled layer-only return with an explicit lifecycle owner that contains the layer attachment and retains `TracerProvider` through subscriber use. Wire the configured OTLP endpoint into the daemon's production tracing initialization, attach the resulting layer beside the stderr formatting layer, retain the owner across the full `engram::daemon::run` future, and invoke its bounded shutdown path on daemon exit. A daemon-run error retains precedence if both run and telemetry shutdown fail; a shutdown failure after a clean daemon exit is returned.

The owner must provide exactly-once coordinated flush/shutdown, propagate exporter or shutdown failure, use a finite export bound with no unbounded retry, and never treat provider `Drop` as successful shutdown. The default build and the shim remain formatting-only when no configured endpoint is supplied.

Add test-first contracts using a test-only in-process exporter path shared with the production provider/layer constructor. Attach the layer to a local subscriber, emit one uniquely named span, retain the lifecycle owner, invoke explicit shutdown/flush, and deterministically assert the exported span through a bounded channel/timeout. No socket, external collector, or network oracle is required.

## Constraints

- Independent shipment; no workspace-identity files.
- `131.001-T` captures the complete API/lifecycle/export RED harness before manifest, lockfile, or production changes; every later task is dependency-blocked by that prerequisite chain.
- Dependency scope is exactly `tracing-opentelemetry` 0.26 to 0.27 plus resulting lockfile reconciliation; no broader OpenTelemetry upgrade.
- Width-isolated execution is mandatory: `131.002-T` aligns only Cargo dependencies; `131.003-T` migrates provider construction; `131.004-T` owns production layer attachment and provider retention; `131.005-T` owns bounded daemon flush/shutdown; `131.006-T` verifies runtime span export; and `131.007-T` closes all-features/default quality gates.
- Provider ownership must outlive layer/subscriber use; explicit shutdown must flush or return a bounded error.
- The configured endpoint must reach production daemon tracing initialization; no builder-only or test-only integration is accepted.
- The deterministic local exporter in `131.001-T` is the runtime oracle reused by `131.006-T`; an external collector is neither required nor sufficient.

## References

- Stash `44E573BC`
- Feature `131-F`; tasks `131.001-T` through `131.007-T`; shipment `125-S`
- `Cargo.toml` OpenTelemetry 0.26 entries
- `cargo tree --features otlp-export` split 0.25/0.26 evidence
- `tracing-opentelemetry` 0.27 dependency metadata
- `src/server/observability.rs`
- `src/lib.rs::init_tracing`; `src/bin/engram.rs` daemon lifecycle
- `src/config/mod.rs::Config::otlp_endpoint`
- `docs/research/doc-005 - F005-Research.md`
- local `opentelemetry-otlp` 0.26 documentation
