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
| Test first | U1 records the API/type-family RED and authors the lifecycle/export harness before any dependency or production edit. |
| One OpenTelemetry type family | U2 changes only the tracing bridge manifest entry and generated lockfile, then proves the graph is entirely OpenTelemetry 0.26. |
| Pinned API/provider construction | U3 migrates the provider/exporter constructor to supported 0.26 APIs and returns explicit provider ownership. |
| Production attachment and retention | U4 attaches the configured layer beside formatting and returns ownership from production tracing initialization. |
| Bounded daemon shutdown | U5 retains ownership across daemon execution and implements exactly-once bounded flush/shutdown with documented error precedence. |
| Deterministic runtime proof | U6 drives the completed production seam with the U1 in-process exporter and proves the expected span and bounded failure behavior. |
| All-features closure | U7 runs dependency, all-features, focused-test, and default-feature gates and records closure evidence without implementation edits. |
| Width isolation | Every unit is at most 105 minutes, modifies fewer than 3 files, changes fewer than 5 functions, covers fewer than 4 scenarios, stays in one skill domain, and ends in an atomic verifiable milestone. |

## Implementation Units

### U1 — RED: OTLP API, lifecycle, and export contract harness

Before any manifest, lockfile, or production edit, add the feature-gated failing contract and deterministic in-process exporter harness. Record the current unsupported 0.26 API diagnostics, split dependency tree, absent production caller, unused endpoint, formatting-only initialization, provider-drop defect, and missing shutdown path. Limit the executable contract to three scenarios: provider construction/retention, configured production attachment with one uniquely named exported span, and bounded exit flush/shutdown including injected failure precedence. No socket, listener, collector, network, sleep polling, or unbounded retry is permitted.

Skill domain: test contract/harness. Maximum width: 2 test files, 4 test/helper functions, 3 scenarios. Estimate: 105 minutes. Atomic milestone: the harness compiles as far as the known defect permits, fails for the recorded pre-implementation reasons, and its exact RED commands/diagnostics are attached to 131.001-T.

### U2 — GREEN: align the tracing bridge dependency graph

Change only tracing-opentelemetry from 0.26 to 0.27 and reconcile the generated lockfile to the existing direct OpenTelemetry 0.26 family. Reject any unrelated package drift. Do not alter Rust source or tests in this unit.

Skill domain: Cargo dependency management. Maximum width: exactly Cargo.toml and Cargo.lock, fewer than 5 dependency entries, 1 dependency-tree scenario. Estimate: 45 minutes. Atomic milestone: cargo tree shows bridge 0.27 resolving only against the OpenTelemetry 0.26 family.

### U3 — GREEN: migrate the OTLP 0.26 provider constructor

In src/server/observability.rs, replace unsupported API names with opentelemetry_sdk::trace::TracerProvider, opentelemetry_otlp::new_exporter().tonic().with_endpoint(...).build_span_exporter(), and the supported Tokio batch runtime. Replace the layer-only return with a narrow owner value that strongly retains the provider and exposes the layer attachment plus explicit lifecycle operations. Reuse the U1 private exporter injection seam; do not attach it to production initialization or daemon exit in this unit.

Skill domain: Rust observability provider construction. Maximum width: 1 production file, 4 functions, 3 constructor/owner scenarios; the U1 tests are run but not broadened. Estimate: 90 minutes. Atomic milestone: the provider-construction and owner-retention portion of the U1 harness passes while production attachment and shutdown remain RED.

### U4 — GREEN: attach the OTLP layer and retain ownership in production initialization

In src/lib.rs, consume the canonical configured OTLP endpoint, attach the U3 layer beside the existing formatting layer, and return the lifecycle owner to the daemon caller. Preserve formatting-only behavior when the endpoint or feature is absent and preserve shim behavior. Existing Config::otlp_endpoint is the boundary; any additional config plumbing is out of scope and must return to Stage rather than widening this unit.

Skill domain: production tracing initialization. Maximum width: 1 production file, 4 functions, 3 configured/unconfigured ownership scenarios; run the unchanged U1 harness. Estimate: 100 minutes. Atomic milestone: production initialization uses the shared provider constructor and returns a live owner for the configured path while the default path stays formatting-only.

### U5 — GREEN: bound daemon flush and shutdown lifecycle

In the daemon branch of src/bin/engram.rs, retain the U4 owner across the complete engram::daemon::run future and invoke exactly-once bounded flush/shutdown on every exit path. A daemon error remains primary when run and telemetry cleanup both fail, with cleanup failure preserved diagnostically; a cleanup failure after a clean run is returned. Do not add retry loops, sleep polling, Drop-only success, or unrelated command lifecycle changes.

Skill domain: daemon lifecycle/error propagation. Maximum width: 1 production file, 4 functions, 3 exit/failure-precedence scenarios; run the unchanged U1 harness. Estimate: 105 minutes. Atomic milestone: all U1 lifecycle contracts turn GREEN within the finite bound.

### U6 — VERIFY: prove runtime exported-span behavior

Without implementation edits, independently drive the configured production initialization and daemon-exit seam through the U1 in-process exporter. Verify three scenarios only: one exact named span arrives while the owner survives daemon use; explicit exit flush/shutdown completes exactly once within the bound; injected failure returns the documented precedence outcome while the unconfigured control constructs no exporter. A live collector is neither required nor accepted as the oracle.

Skill domain: runtime verification. Maximum width: 2 verification/test evidence surfaces, 0 changed production functions, 3 scenarios. Estimate: 80 minutes. Atomic milestone: exact focused commands and deterministic outcomes are recorded on 131.006-T.

### U7 — VERIFY: close all-features and default quality gates

Without source, test, manifest, or lockfile edits, run the final dependency-tree proof, all-features check and clippy policy, relevant all-features/focused tests, and default-feature gates. Record exact command outcomes, confirm no OpenTelemetry 0.25 package remains in the OTLP graph, and close the feature only if every preceding task is complete. Return any distinct defect to Stage rather than broadening this task.

Skill domain: quality and closure evidence. Maximum width: 2 evidence surfaces, 0 changed functions, 3 gate groups. Estimate: 90 minutes. Atomic milestone: complete closure evidence is recorded on 131.007-T with every required gate passing.

## Dependency Graph

Strict execution order is U1 -> U2 -> U3 -> U4 -> U5 -> U6 -> U7, represented by 131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T. Every GREEN or verification unit is blocked by its immediate prerequisite and therefore transitively by the RED contract. PR #362 ordering is satisfied by merged commit 685f62668ac273a41a1f93fc9be2571510decae2. Shipment 125-S remains queued and unclaimed until the exact final reviewed PR #363 head is integrated and its remaining claim guards pass.

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
| Provider drops before subscriber emits | Return an owner that retains provider state in U3 and make the lifecycle contract executable in U1/U6. |
| Batch export is asynchronous | Explicit flush/shutdown plus bounded exporter observation; no construction-only assertion. |
| Shutdown hangs or silently loses spans | Finite SDK export bound, no retries, surfaced error, and failure-injection timeout test. |
| Test bypasses production ownership | Private exporter injection reuses the same owner/provider/layer constructor. |
| Builder remains disconnected from production | U1 records the disconnected path, U4 attaches it, U5 closes its exit lifecycle, and U6 verifies the complete production seam. |
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

Precheck the `otlp-export` feature and Tokio runtime. The required local harness drives the configured production tracing initialization, retains the owner across the daemon execution seam, emits a uniquely named span, exits, explicitly shuts down/flushes, and receives exactly that span before a deterministic timeout. The failure harness injects an exporter shutdown/flush error and requires the documented returned/diagnostic outcome before the same bound. Record dependency-tree and all-features/default gate outcomes. Roll back the affected GREEN unit if the production path never attaches the layer, the span is absent, shutdown exceeds the bound, errors are swallowed, OpenTelemetry 0.25 remains, or default features regress. Owner: Ship. Validation window: the focused test duration plus all-features CI; no external collector checkpoint.

## Plan Hardening

Hardening rerun: **required and satisfied** after the OTLP task decomposition changed. Triggers remain a feature-gated public contract change, an external export surface, asynchronous runtime ownership, and exit-time failure handling. The constitution width rules and strict-safety action vocabulary were re-read. Engram call-graph evidence confirms build_otlp_layer has no caller and init_tracing is isolated in src/lib.rs. A docs/compound search found only an unrelated retry-metrics OTLP note, so no prior implementation pattern governs this repair.

| ProposedAction | targets | ActionRisk | rollback | approval_required | ActionResult |
|---|---|---|---|---|---|
| Author the failing API/lifecycle/export contract before implementation | at most 2 focused test files | moderate | revert U1; keep shipment unclaimed | no | planned |
| Align only the tracing bridge dependency family | Cargo.toml, Cargo.lock | moderate | restore bridge 0.26 and split graph | no | planned |
| Replace layer-only construction with retained 0.26 provider ownership | src/server/observability.rs | moderate | revert U3; keep later tasks blocked | no | planned |
| Attach configured OTLP through production tracing initialization | src/lib.rs | moderate | revert U4; preserve formatting-only initialization | no | planned |
| Retain the owner and coordinate bounded daemon exit cleanup | daemon branch in src/bin/engram.rs | moderate | revert U5; keep shipment unclaimed | no | planned |
| Verify deterministic runtime export and bounded failure | unchanged U1 harness and task evidence | low | return a distinct defect to Stage | no | planned |
| Run final dependency/all-features/default closure | verification commands and task evidence | low | leave 131-F queued and unclosed | no | planned |

Protected invariants: U1 is committed before any manifest or production change; each later unit has one skill domain and an immediate prerequisite; no task exceeds 105 minutes, 2 modified files, 4 changed functions, or 3 scenarios; configured production tracing actually attaches OTLP; the provider outlives subscriber and daemon use; shutdown is explicit, exactly once, and finite; pending spans flush or a visible error follows the documented precedence; the default and shim paths remain formatting-only; no network-dependent oracle or unrelated dependency/config redesign is introduced.

Verification and rollback are now staged rather than concentrated in one GREEN task. U2 must prove the dependency graph before U3 begins. U3 must pass constructor/owner contracts before U4 attaches production tracing. U4 must return ownership before U5 changes daemon exit. U6 independently proves the completed runtime path, and U7 closes all-features/default gates. Any unit that needs a third modified file, fifth function, fourth scenario, or a second skill domain stops and returns to Stage for re-harvest. Runtime owner: Ship. Validation window: focused bounded harness plus all-features CI. Rollback trigger: missing span, owner drop before daemon completion, cleanup beyond the finite bound, swallowed failure, residual OpenTelemetry 0.25, or default-feature regression.

## Plan Review

Gate: **PASS**. Standard review was rerun after hardening and seven-unit re-harvest. Hardening was required and is materially complete. Local personas applied: constitution, Rust/API, architecture, scope boundary, test strategy, operational readiness, learnings, and security lens for the external export boundary. Cross-model persona invocation and intercom broadcast were unavailable; this is disclosed but is not blocking for a non-security maintenance plan.

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| C1 | Constitution | P1 | The former U2 changed at least six files, mixed dependency, provider, initialization, daemon lifecycle, and test concerns, and exceeded two hours. | Resolved: U1 through U7 each cap at 105 minutes, fewer than 3 files, fewer than 5 functions, fewer than 4 scenarios, one skill domain, and one atomic milestone. |
| T1 | Test strategy | P1 | RED contracts could be weakened if dependency or production edits begin before the complete lifecycle/export harness is recorded. | Resolved: 131.001-T is the sole root and every later task is directly or transitively blocked by it. |
| R1 | Rust/API | P1 | Dependency alignment, pinned API migration, provider ownership, production attachment, and daemon cleanup have different failure modes. | Resolved: U2, U3, U4, and U5 isolate those boundaries and preserve explicit error precedence. |
| A1 | Architecture | P1 | Production attachment and shutdown cannot remain hidden in a provider-construction task. | Resolved: U4 owns initialization/retention and U5 alone owns daemon exit coordination. |
| V1 | Operational readiness | P1 | Runtime export proof and broad quality gates are independent closure milestones. | Resolved: U6 owns deterministic exported-span/failure verification; U7 owns dependency/all-features/default closure. |
| S1 | Scope boundary | P2 | Endpoint plumbing might widen U4 into config work. | Resolved in plan: existing Config::otlp_endpoint is the boundary; any new config work returns to Stage rather than broadening U4. |
| L1 | Learnings | P3 | No directly applicable compound learning exists for provider lifecycle ownership. | Acknowledged; the plan relies on pinned API evidence, current call graph, and executable contracts. |
| X1 | Security lens | P3 | OTLP crosses an external boundary, but tests must not introduce collector credentials or network dependence. | Resolved by the in-process exporter oracle and explicit no-network/no-secret scope. |

No unresolved P0 or P1 finding remains. The P2 endpoint-plumbing risk has an explicit stop boundary. Review confirms the exact dependency chain 131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T and approves re-harvest into existing feature 131-F and queued, unclaimed shipment 125-S.
