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

Compilation alone is insufficient. OpenTelemetry 0.26 tracers weak-reference their provider; the current local provider is dropped when `build_otlp_layer` returns, so attached layers emit no spans. The repair must return retained provider ownership, keep that owner alive through subscriber use, and provide coordinated, bounded flush/shutdown with observable failure.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Test first | U1 records the compile/type-family RED and adds provider-retention, exported-span, shutdown, and failure contracts before production changes. |
| One OpenTelemetry type family | U2 aligns only `tracing-opentelemetry` 0.26 to 0.27 and reconciles the lockfile to 0.26. |
| Provider lifetime | U2 replaces the feature-enabled layer-only return with a lifecycle owner retaining `TracerProvider` through layer/subscriber use. |
| Coordinated shutdown | U2 exposes exactly-once flush/shutdown, finite export bounds, and propagated exporter/provider errors rather than relying on `Drop`. |
| Deterministic runtime proof | U1/U3 use an in-process exporter and local subscriber to assert the exact emitted span without network or collector I/O. |
| All-features closure | U3 verifies dependency-tree, all-features compile/lint/tests, runtime export, bounded failure, and default-feature gates. |
| Width isolation | Scope remains OTLP manifest/lock reconciliation, one observability module, and its focused tests; no workspace identity files. |

## Implementation Units

### U1 — RED: provider lifecycle and exported-span contract

Before any dependency or production edit, add feature-gated tests around a test-only in-process exporter seam. Record the current unresolved 0.26 API diagnostics and split dependency tree. Define at most three deterministic lifecycle scenarios: attach the returned layer to a local subscriber while retaining its owner and emit one uniquely named span; explicit shutdown/flush delivers exactly that span before a bounded channel/timeout; injected exporter flush/shutdown failure returns an error within the same bound. The tests must use no socket, listener, collector, network, sleep polling, or unbounded retry. Test-only, one focused file, target 100 minutes.

### U2 — GREEN: bridge alignment and retained provider lifecycle

`131.002-T` owns this entire implementation. Change only `tracing-opentelemetry` from 0.26 to 0.27 and reconcile `Cargo.lock` to the direct OpenTelemetry 0.26 family. In `src/server/observability.rs`, use `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().with_endpoint(...).build_span_exporter()`, and `with_batch_exporter(..., opentelemetry_sdk::runtime::Tokio)`.

Replace the feature-enabled layer-only return with a small lifecycle owner/guard that contains retained provider ownership and the layer attachment contract. The owner must remain live until subscriber use ends, expose explicit exactly-once coordinated flush/shutdown through supported 0.26 provider APIs, propagate errors, use the finite SDK export bound with no retry loop, and not represent `Drop` as successful shutdown. A private/test-only exporter constructor must share the same owner/provider/layer path as endpoint construction. Preserve the non-feature no-op behavior. Cargo manifest, derived lock reconciliation, and one production module; fewer than five production functions; target 110 minutes.

### U3 — all-features and runtime verification

Run `cargo tree` to prove the OTLP graph no longer contains OpenTelemetry 0.25, then run the focused lifecycle tests, `cargo check --all-features`, all-features clippy with repository warning policy, relevant all-features tests, and default-feature gates. Independently prove that the returned owner survives construction through local subscriber use, one named span reaches the in-process exporter only after explicit flush/shutdown, and injected failure returns within the bound. A live collector is neither required nor accepted as the sole oracle. Verification only, target 100 minutes.

## Dependency Graph

`U1 -> U2 -> U3`, represented by `131.001-T -> 131.002-T -> 131.003-T`. The RED-before-GREEN edge is mandatory. This release unit has no cross-shipment dependency beyond the `125-S` claim guard requiring PR #362 and this planning PR to integrate first.

## Decisions and Rationale

- Align bridge 0.27 to the already-pinned 0.26 family rather than widening all telemetry dependencies.
- Replace the invalid layer-only contract because the tracer does not strongly own the provider.
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

Precheck the `otlp-export` feature and Tokio runtime. The required local harness attaches the returned layer, retains the owner, emits a uniquely named span, explicitly shuts down/flushes, and receives exactly that span before a deterministic timeout. The failure harness injects an exporter shutdown/flush error and requires a returned error before the same bound. Record dependency-tree and all-features/default gate outcomes. Roll back U2 if the span is absent, shutdown exceeds the bound, errors are swallowed, OpenTelemetry 0.25 remains, or default features regress. Owner: Ship. Validation window: the focused test duration plus all-features CI; no external collector checkpoint.

## Plan Hardening

Hardening rerun: **required and satisfied** for the provider-lifecycle contract and external runtime surface.

Reinforcing context: `.github/instructions/strict-safety.instructions.md`, `.github/instructions/constitution.instructions.md`, the pinned dependency evidence, current `src/server/observability.rs`, and PR #363 review thread `discussion_r3848530320`.

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Replace the feature-enabled layer-only contract with retained provider lifecycle ownership | `src/server/observability.rs` | moderate | revert U2 and keep shipment unclaimed | no | planned |
| Coordinate finite flush/shutdown and propagate failure | OTLP lifecycle owner | moderate | revert U2; retain compile defect rather than ship silent loss | no | planned |
| Align only the tracing bridge to 0.27 | `Cargo.toml`, `Cargo.lock` | moderate | restore bridge 0.26 and split graph | no | planned |

Protected invariants: provider outlives layer/subscriber use; shutdown is explicit and exactly once; pending spans flush or an error is returned; failure is bounded; no network-dependent test; no unrelated dependency or observability redesign; RED remains before GREEN.

## Plan Review

Gate: **PASS**. Standard plan review was rerun after the operator-authorized remediation pass. Hardening was required and is now present. Personas applied locally: constitution, Rust/API, architecture, scope boundary, test strategy, operational readiness, and learnings. Cross-model review was unavailable; it is preferred but not blocking for this non-security maintenance plan.

| ID | Severity | Finding | Disposition |
|---|---|---|---|
| L1 | P1 | The local provider dies when a layer-only builder returns, making later spans no-ops. | Resolved: U1 proves emission and U2 returns retained lifecycle ownership. |
| L2 | P1 | `131.002-T` preserved the invalid layer-only contract and did not own shutdown. | Resolved: U2 and `131.002-T` explicitly own provider retention and coordinated flush/shutdown. |
| V1 | P1 | Construction and compilation do not prove export behavior. | Resolved: the local exporter/subscriber harness deterministically asserts one named exported span. |
| B1 | P1 | Export/shutdown failure could hang or be silently swallowed. | Resolved: finite SDK bound, no retry loop, propagated error, and bounded failure injection. |
| D1 | P1 | Bridge 0.26 uses an incompatible OpenTelemetry 0.25 type family. | Resolved: bridge 0.27 alignment and U3 tree proof. |
| S1 | P2 | Lifecycle expansion could become a general tracing redesign. | Resolved by one module, one narrow owner, private test seam, and explicit exclusions. |

No unresolved standard-review P0/P1 findings remain. This operator-authorized remediation pass follows the prior three-cycle stop; it does not weaken or bypass the review gate. The reviewed plan remains eligible for the already-harvested `131-F` hierarchy and queued shipment `125-S` after its claim guards are satisfied.
