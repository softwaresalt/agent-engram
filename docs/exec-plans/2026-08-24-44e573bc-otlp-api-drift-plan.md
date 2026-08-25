---
title: "Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle"
type: implementation-plan
doc_type: plan
date: 2026-08-24
status: reviewed
source: docs/decisions/2026-08-24-otlp-api-drift-fix-decision.md
source_stash_id: "44E573BC"
---

# Align the tracing bridge and retain the OpenTelemetry 0.26 provider lifecycle

## Problem Frame

The `otlp-export` feature does not compile because `src/server/observability.rs` imports `SdkTracerProvider` and calls `SpanExporter::builder()`, neither of which exists in pinned 0.26. The supported API is `trace::TracerProvider` plus `new_exporter().tonic().build_span_exporter()` and an explicit batch runtime. The locked graph also contains `tracing-opentelemetry` 0.26 against OpenTelemetry 0.25, so bridge 0.27 is the compatible narrow alignment.

Compilation alone is insufficient. OpenTelemetry 0.26 tracers weak-reference their provider; the current local provider is dropped when `build_otlp_layer` returns, so attached layers emit no spans. The builder also has no production caller, `Config::otlp_endpoint` is unused, and `src/lib.rs::init_tracing` installs only the formatting layer. The repair must attach OTLP in production daemon tracing initialization, return retained provider ownership, keep that owner alive across daemon execution, and provide coordinated, bounded flush/shutdown with observable failure.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Test first | U1 records the compile/type-family RED and adds provider-retention, exported-span, shutdown, and failure contracts before production changes. |
| One OpenTelemetry type family | U2 aligns only `tracing-opentelemetry` 0.26 to 0.27 and reconciles the lockfile to 0.26. |
| Provider lifetime | U2 replaces the feature-enabled layer-only return with a lifecycle owner retaining `TracerProvider` through layer/subscriber use. |
| Production attachment | U2 routes the configured endpoint through daemon tracing initialization, attaches OTLP beside the stderr formatting layer, retains the owner across `engram::daemon::run`, and shuts it down on exit. |
| Coordinated shutdown | U2 exposes exactly-once flush/shutdown, finite export bounds, and propagated exporter/provider errors rather than relying on `Drop`. |
| Deterministic runtime proof | U1/U3 drive the production initialization seam with an in-process exporter and assert the exact emitted span plus exit-time shutdown without network or collector I/O. |
| All-features closure | U3 verifies dependency-tree, all-features compile/lint/tests, runtime export, bounded failure, and default-feature gates. |
| Width isolation | Scope remains OTLP manifest/lock reconciliation, the observability builder, endpoint plumbing, tracing initialization, the daemon command's owner lifetime/exit path, and focused tests; no workspace identity files. |

## Implementation Units

### U1 — RED: provider lifecycle and exported-span contract

Before any dependency or production edit, add feature-gated tests around a test-only in-process exporter and production-initialization seam. Record the current unresolved 0.26 API diagnostics, split dependency tree, absent `build_otlp_layer` caller, unused endpoint field, and formatting-only `init_tracing`. Define deterministic lifecycle scenarios that prove a configured production initialization attaches the returned layer while retaining its owner across daemon execution, emits one uniquely named span, and invokes explicit shutdown/flush on exit; shutdown delivers exactly that span before a bounded channel/timeout; an injected exporter flush/shutdown failure returns an error within the same bound. Also prove the unconfigured/default path remains formatting-only. The tests must use no socket, listener, collector, network, sleep polling, or unbounded retry. Test-only, focused files, target 110 minutes.

### U2 — GREEN: bridge alignment and retained provider lifecycle

`131.002-T` owns this entire implementation. Change only `tracing-opentelemetry` from 0.26 to 0.27 and reconcile `Cargo.lock` to the direct OpenTelemetry 0.26 family. In `src/server/observability.rs`, use `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().with_endpoint(...).build_span_exporter()`, and `with_batch_exporter(..., opentelemetry_sdk::runtime::Tokio)`.

Replace the feature-enabled layer-only return with a small lifecycle owner/guard that contains retained provider ownership and the layer attachment contract. Route the canonical configured endpoint into the daemon's production tracing initialization, attach the OTLP layer beside the existing stderr formatting layer, retain the guard across the complete `engram::daemon::run` future, and invoke its explicit shutdown on every daemon exit path. If daemon execution and telemetry shutdown both fail, retain the daemon error as primary and preserve the shutdown failure diagnostically; after a clean daemon exit, return shutdown failure. The shim and unconfigured/default build remain formatting-only.

The owner must expose exactly-once coordinated flush/shutdown through supported 0.26 provider APIs, propagate errors, use the finite SDK export bound with no retry loop, and not represent `Drop` as successful shutdown. A private/test-only exporter constructor must share the same owner/provider/layer and production-initialization path as endpoint construction. Scope is `Cargo.toml`, `Cargo.lock`, `src/server/observability.rs`, `src/lib.rs`, the daemon branch in `src/bin/engram.rs`, endpoint plumbing in `src/config/mod.rs` if needed, and focused tests; target 130 minutes.

### U3 — all-features and runtime verification

Run `cargo tree` to prove the OTLP graph no longer contains OpenTelemetry 0.25, then run the focused lifecycle tests, `cargo check --all-features`, all-features clippy with repository warning policy, relevant all-features tests, and default-feature gates. Independently drive the configured production tracing initialization and daemon-exit seam to prove that the layer is attached, the returned owner survives through daemon execution, one named span reaches the in-process exporter after explicit exit-time flush/shutdown, and injected failure returns within the bound and documented precedence. Prove the unconfigured/default path remains formatting-only. A live collector is neither required nor accepted as the sole oracle. Verification only, target 110 minutes.

## Dependency Graph

`U1 -> U2 -> U3`, represented by `131.001-T -> 131.002-T -> 131.003-T`. The RED-before-GREEN edge is mandatory. This release unit has no cross-shipment dependency beyond the `125-S` claim guard requiring PR #362 and this planning PR to integrate first.

## Decisions and Rationale

- Align bridge 0.27 to the already-pinned 0.26 family rather than widening all telemetry dependencies.
- Replace the invalid layer-only contract because the tracer does not strongly own the provider.
- Attach that owner/layer through the real daemon tracing path; a correct but uncalled builder cannot export production spans.
- Keep lifecycle ownership explicit: subscriber users retain the owner and call shutdown; provider `Drop` is not a flush guarantee.
- Share one provider/layer construction path between the endpoint exporter and test exporter so runtime tests cannot pass around production ownership logic.
- Use a local exporter plus bounded channel/timeout for deterministic exported-span evidence. External collectors add nondeterminism and do not prove ownership.
- Keep shutdown failure visible and bounded; do not hide it in best-effort cleanup or retries.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Provider drops before subscriber emits | Return an owner that retains provider state and make the lifecycle contract executable in U1/U3. |
| Batch export is asynchronous | Explicit flush/shutdown plus bounded exporter observation; no construction-only assertion. |
| Shutdown hangs or silently loses spans | Finite SDK export bound, no retries, surfaced error, and failure-injection timeout test. |
| Test bypasses production ownership | Private exporter injection reuses the same owner/provider/layer constructor. |
| Builder remains disconnected from production | RED and VERIFY drive the configured daemon tracing initialization and exit seam, not only a local subscriber. |
| Daemon and shutdown both fail | Preserve the daemon error as primary and retain shutdown failure diagnostically; return shutdown failure after clean daemon exit. |
| Lockfile update broadens scope | Exact `cargo tree` evidence and rejection of unrelated package changes. |
| Default build drifts | Run default and all-features gates. |

## Plan Hardening Signals

- Public API/schema/contract: present; the feature-enabled builder can no longer return only a layer.
- Security/auth/compliance: absent.
- Migration/destructive action: absent.
- External integration/checkpoint: present; OTLP is an external export surface even though deterministic verification is local.
- High runtime/rollback risk: present; incorrect lifetime or shutdown silently loses telemetry.

Requires plan hardening: yes

## Runtime Verification and Closure

Precheck the `otlp-export` feature and Tokio runtime. The required local harness drives the configured production tracing initialization, retains the owner across the daemon execution seam, emits a uniquely named span, exits, explicitly shuts down/flushes, and receives exactly that span before a deterministic timeout. The failure harness injects an exporter shutdown/flush error and requires the documented returned/diagnostic outcome before the same bound. Record dependency-tree and all-features/default gate outcomes. Roll back U2 if the production path never attaches the layer, the span is absent, shutdown exceeds the bound, errors are swallowed, OpenTelemetry 0.25 remains, or default features regress. Owner: Ship. Validation window: the focused test duration plus all-features CI; no external collector checkpoint.

## Plan Hardening

Hardening rerun: **required and satisfied** for the provider-lifecycle contract and external runtime surface.

Reinforcing context: `.github/instructions/strict-safety.instructions.md`, `.github/instructions/constitution.instructions.md`, the pinned dependency evidence, current `src/server/observability.rs`, and PR #363 review thread `discussion_r3848530320`.

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Replace the feature-enabled layer-only contract with retained provider lifecycle ownership | `src/server/observability.rs` | moderate | revert U2 and keep shipment unclaimed | no | planned |
| Attach the OTLP owner/layer in production and retain it across daemon execution | `src/lib.rs`, `src/bin/engram.rs`, endpoint plumbing | moderate | revert U2 and keep shipment unclaimed | no | planned |
| Coordinate finite flush/shutdown and propagate failure | OTLP lifecycle owner | moderate | revert U2; retain compile defect rather than ship silent loss | no | planned |
| Align only the tracing bridge to 0.27 | `Cargo.toml`, `Cargo.lock` | moderate | restore bridge 0.26 and split graph | no | planned |

Protected invariants: configured production daemon tracing actually attaches OTLP; provider outlives daemon subscriber use; shutdown is explicit and exactly once on exit; pending spans flush or an error is returned/preserved under documented precedence; failure is bounded; the unconfigured/default path remains formatting-only; no network-dependent test; no unrelated dependency or observability redesign; RED remains before GREEN.

## Plan Review

Gate: **PASS**. Standard plan review was rerun after the operator-authorized remediation pass. Hardening was required and is now present. Personas applied locally: constitution, Rust/API, architecture, scope boundary, test strategy, operational readiness, and learnings. Cross-model review was unavailable; it is preferred but not blocking for this non-security maintenance plan.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| L1 | P1 | The local provider dies when a layer-only builder returns, making later spans no-ops. | Resolved: U1 proves emission and U2 returns retained lifecycle ownership. |
| L2 | P1 | `131.002-T` preserved the invalid layer-only contract and did not own shutdown. | Resolved: U2 and `131.002-T` explicitly own provider retention and coordinated flush/shutdown. |
| V1 | P1 | Construction and compilation do not prove export behavior. | Resolved: the local exporter/subscriber harness deterministically asserts one named exported span. |
| B1 | P1 | Export/shutdown failure could hang or be silently swallowed. | Resolved: finite SDK bound, no retry loop, propagated error, and bounded failure injection. |
| D1 | P1 | Bridge 0.26 uses an incompatible OpenTelemetry 0.25 type family. | Resolved: bridge 0.27 alignment and U3 tree proof. |
| I1 | P1 | The OTLP builder has no production caller, the endpoint field is unused, and `init_tracing` installs only formatting, so the shipment could pass without exporting daemon spans. | Resolved in plan: U1 records the disconnected RED; U2 owns configured production attachment, daemon-lifetime retention, and exit-time shutdown; U3 verifies that exact path. |
| S1 | P2 | Lifecycle expansion could become a general tracing redesign. | Resolved by one module, one narrow owner, private test seam, and explicit exclusions. |

No unresolved standard-review P0/P1 findings remain. This operator-authorized remediation pass follows the prior three-cycle stop; it does not weaken or bypass the review gate. The reviewed plan remains eligible for the already-harvested `131-F` hierarchy and queued shipment `125-S` after its claim guards are satisfied.
