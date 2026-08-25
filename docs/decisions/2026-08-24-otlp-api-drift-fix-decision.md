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

The optional `otlp-export` feature does not compile because `src/server/observability.rs` uses API names unavailable in pinned OpenTelemetry 0.26 while `tracing-opentelemetry` 0.26 resolves against the 0.25 type family. A layer-only repair is also runtime-invalid: the tracer weak-references its provider, the local provider is dropped when `build_otlp_layer` returns, the builder has no production caller, and `src/lib.rs::init_tracing` installs formatting only.

The configured endpoint currently has no executable production handoff. `src/config/mod.rs::Config::otlp_endpoint` exists, but `Config::parse` has no production caller. The actual binary parses `src/bin/engram.rs::Cli` plus `src/cli/flags.rs::GlobalFlags`; `GlobalFlags` has no endpoint. `Command::Daemon` calls `init_tracing(LogFormat)` before `engram::daemon::run`. Workspace `PluginConfig::load` occurs inside `daemon::run`, after tracing starts, and has no OTLP field. The shim-spawned daemon receives only `daemon --workspace`, with environment inheritance available.

## Evidence

`cargo tree --features otlp-export` shows `tracing-opentelemetry` 0.26 pulling OpenTelemetry 0.25 while Engram and `opentelemetry-otlp` use 0.26. Bridge 0.27 depends on OpenTelemetry 0.26. Local 0.26 APIs expose `trace::TracerProvider`, `new_exporter().tonic().build_span_exporter()`, explicit Tokio batch runtime configuration, and provider flush/shutdown. Engram indexed search found `build_otlp_layer` and `init_tracing` but no caller edges; targeted source reads confirmed the exact `Cli`/daemon/config/shim flow above.

## Decision

Keep the direct SDK/exporter stack at 0.26 and align only `tracing-opentelemetry` to 0.27. Make `Command::Daemon` the canonical endpoint boundary by adding daemon-subcommand `--otlp-endpoint` parsing with `ENGRAM_OTLP_ENDPOINT` fallback. Clap explicit-flag precedence is authoritative. Pass the resolved `Option<&str>` to a daemon-only tracing initializer in `src/lib.rs`; retain the existing formatting-only initializer for the shim. The normal shim path needs no lifecycle edit because its daemon child inherits `ENGRAM_OTLP_ENDPOINT`. The dead flat `Config` field and late workspace `PluginConfig` are not configuration sources for tracing startup.

Migrate the provider constructor to supported 0.26 APIs and require it to accept that propagated endpoint argument without rereading environment, CLI, or config. Return an owner that contains the layer attachment and strongly retains the provider. Attach OTLP beside stderr formatting, retain the owner across the full `engram::daemon::run` future, and invoke exactly-once bounded cleanup on exit. A daemon error remains primary if both run and cleanup fail; cleanup failure after a clean run is returned. The default build, absent-endpoint path, and shim remain formatting-only.

Use a test-first in-process exporter path shared with production construction. Prove explicit flag/environment endpoint propagation, exact-value consumption, retained ownership, one named exported span, and bounded failure behavior without sockets, external collectors, credentials, or network access.

## Constraints

- Independent shipment; no workspace-identity or blocked-security files.
- `131.001-T` records the complete endpoint/API/lifecycle/export RED harness before any manifest or production edit.
- `131.002-T` changes only `Cargo.toml` and `Cargo.lock`.
- `131.003-T` owns only `src/bin/engram.rs` plus `src/lib.rs` daemon endpoint propagation.
- `131.004-T` owns only `src/server/observability.rs` provider construction and consumes the supplied endpoint.
- `131.005-T` owns production attachment and retention in `src/lib.rs` plus the daemon arm.
- `131.006-T` owns bounded daemon cleanup; `131.007-T` owns runtime proof; `131.008-T` owns closure gates.
- Every task is at most 105 minutes, 2 files, 4 functions, 3 scenarios, one skill domain, and one atomic milestone.
- Exact dependency chain: `131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T -> 131.008-T`.
- An external collector is neither required nor sufficient as the runtime oracle.

## References

- Stash `44E573BC`
- Feature `131-F`; tasks `131.001-T` through `131.008-T`; shipment `125-S`
- `Cargo.toml` OpenTelemetry entries and split `cargo tree` evidence
- `src/bin/engram.rs::{Cli, Command::Daemon, main}`
- `src/cli/flags.rs::GlobalFlags`
- `src/config/mod.rs::Config::otlp_endpoint` (unused legacy declaration, not the selected boundary)
- `src/models/config.rs::PluginConfig::load` (late workspace config, not the selected boundary)
- `src/shim/lifecycle.rs` daemon spawn and environment inheritance
- `src/lib.rs::init_tracing`
- `src/server/observability.rs::build_otlp_layer`
- `docs/research/doc-005 - F005-Research.md`
