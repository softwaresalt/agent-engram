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

The `otlp-export` feature does not compile because `src/server/observability.rs` uses API names that are unavailable in pinned OpenTelemetry 0.26, while `tracing-opentelemetry` 0.26 resolves against OpenTelemetry 0.25. Bridge 0.27 is the narrow compatible alignment. Compilation alone is insufficient: the current builder drops its local provider, has no production caller, and returns only a layer.

Read-only inspection at PR #363 HEAD `9dcd33f5e49583f8138f4896b70c89c00251e25f` establishes the real endpoint/config flow. `src/bin/engram.rs::Cli` parses `GlobalFlags`, whose `src/cli/flags.rs` definition has no OTLP field. `Command::Daemon` has no endpoint field and calls `engram::init_tracing(daemon_log_format())` before extracting `--workspace` and awaiting `engram::daemon::run`. `src/config/mod.rs::Config::otlp_endpoint` exists, but `Config::parse` has no production caller. `src/models/config.rs::PluginConfig::load` runs inside `daemon::run`, after tracing initialization, and has no OTLP field. `src/shim/lifecycle.rs` spawns `engram daemon --workspace <path>` and inherits process environment. `src/lib.rs::init_tracing` accepts only `LogFormat` and installs stderr formatting, while `build_otlp_layer` has no caller.

The executable repair therefore makes the daemon subcommand, not dead `Config` or workspace `PluginConfig`, the canonical endpoint boundary. A dedicated `Command::Daemon` option resolves `--otlp-endpoint` with `ENGRAM_OTLP_ENDPOINT` fallback; clap gives the explicit flag precedence. The daemon match arm passes that typed value into a daemon-only tracing initializer. A shim-spawned daemon receives the environment value through existing process inheritance, while the shim itself keeps its formatting-only initializer. No hidden config, GlobalFlags, PluginConfig, or shim-lifecycle work is required.

## Requirements Trace

| Requirement | Implementation action |
|---|---|
| Test first | U1 records endpoint propagation, API, ownership, attachment, export, and cleanup RED evidence before manifest or production edits. |
| One OpenTelemetry type family | U2 changes only the tracing bridge manifest entry and generated lockfile. |
| Explicit endpoint propagation | U3 adds daemon-subcommand flag/environment resolution and passes `Option<&str>` to a daemon-only tracing initialization seam. |
| Pinned provider construction | U4 consumes the supplied endpoint argument and returns retained provider/layer ownership without reading config or environment. |
| Production attachment and retention | U5 attaches OTLP in the daemon initializer and retains the returned owner across `daemon::run`. |
| Bounded daemon shutdown | U6 performs exactly-once bounded cleanup with documented error precedence. |
| Runtime proof | U7 verifies the complete configured endpoint-to-export path and failure behavior without network. |
| Closure | U8 records dependency, all-features, focused, and default-feature evidence without implementation edits. |
| Width isolation | Every unit is at most 105 minutes, touches at most 2 production or evidence surfaces, changes at most 4 functions, covers at most 3 scenarios, and has one skill domain and atomic milestone. |

## Implementation Units

### U1 — RED: endpoint, API, lifecycle, and export contract harness

Before any manifest or production edit, add the feature-gated failing contract and deterministic in-process exporter harness. Record the split dependency graph; unsupported 0.26 APIs; dead `Config::otlp_endpoint`; absent `GlobalFlags` endpoint; `Command::Daemon -> init_tracing(LogFormat) -> daemon::run` order; inherited shim-child environment; uncalled builder; provider drop; and missing cleanup. Scenario 1 covers explicit daemon flag precedence, inherited environment, absent endpoint, and arrival at a daemon-only initializer. Scenario 2 covers provider construction, production attachment, retained ownership, and one named exported span. Scenario 3 covers bounded exactly-once cleanup and combined-failure precedence.

Skill domain: test contract/harness. Maximum width: 2 test files, 4 test/helper functions, 3 scenarios. Estimate: 105 minutes. Atomic milestone: reproducible RED evidence is attached to `131.001-T`. No collector, socket, network, sleep polling, or unbounded retry is permitted.

### U2 — GREEN: align the tracing bridge dependency graph

Change only `tracing-opentelemetry` from 0.26 to 0.27 and reconcile `Cargo.lock` to the existing direct OpenTelemetry 0.26 family. Reject unrelated package drift and leave every endpoint/runtime RED contract unchanged.

Skill domain: Cargo dependency management. Maximum width: exactly `Cargo.toml` and `Cargo.lock`, 0 changed functions, fewer than 5 dependency entries, 1 scenario. Estimate: 45 minutes. Atomic milestone: `cargo tree` shows one OpenTelemetry 0.26 family.

### U3 — GREEN: propagate the configured endpoint to daemon tracing

In `src/bin/engram.rs`, add `otlp_endpoint: Option<String>` to `Command::Daemon` with `#[arg(long, env = "ENGRAM_OTLP_ENDPOINT")]`, destructure it in the daemon match arm, and pass `as_deref()` into a dedicated daemon tracing initializer before `engram::daemon::run`. In `src/lib.rs`, add that daemon-only initialization seam while retaining the existing formatting-only `init_tracing(LogFormat)` used by `src/shim/mod.rs`. The seam receives the typed endpoint but does not construct or attach a provider yet. The existing shim spawn needs no edit because its daemon child inherits `ENGRAM_OTLP_ENDPOINT`. Do not use dead `Config::otlp_endpoint`, add a GlobalFlags field, load PluginConfig early, or edit shim lifecycle.

Skill domain: daemon CLI/config propagation. Maximum width: exactly `src/bin/engram.rs` and `src/lib.rs`, 4 functions, 3 direct-flag/inherited-environment/absent scenarios. Estimate: 95 minutes. Run the U1 harness unchanged. Atomic milestone: one explicit typed endpoint reaches the daemon tracing initialization boundary and only exporter assertions remain RED.

### U4 — GREEN: migrate the OpenTelemetry 0.26 provider constructor

In `src/server/observability.rs`, replace unsupported APIs with `opentelemetry_sdk::trace::TracerProvider`, `opentelemetry_otlp::new_exporter().tonic().with_endpoint(...).build_span_exporter()`, and the supported Tokio batch runtime. Replace the layer-only return with a narrow lifecycle owner that strongly retains the provider and exposes layer attachment plus explicit cleanup. The constructor accepts the U3 endpoint argument and must not read CLI, environment, `Config`, `PluginConfig`, or global state. The private exporter seam shares the same ownership path.

Skill domain: Rust observability-provider construction. Maximum width: 1 production file, 4 functions, 3 scenarios. Estimate: 90 minutes. Atomic milestone: provider construction consumes the supplied endpoint and returns retained ownership while attachment remains RED.

### U5 — GREEN: attach OTLP and retain ownership across daemon use

In `src/lib.rs`, consume the existing daemon initializer `Option<&str>`, pass the exact value to the U4 constructor, attach its layer beside stderr formatting, and return the live lifecycle owner. In the `src/bin/engram.rs` daemon arm, store that owner across the complete `engram::daemon::run` future. The absent-endpoint, feature-disabled, and shim paths remain formatting-only. No endpoint reread or config edit is allowed.

Skill domain: production tracing attachment/retention. Maximum width: exactly `src/lib.rs` and the daemon arm in `src/bin/engram.rs`, 4 functions, 3 scenarios. Estimate: 105 minutes. Atomic milestone: the propagated value drives production attachment and the owner survives daemon use.

### U6 — GREEN: bound daemon flush and shutdown

In the daemon arm of `src/bin/engram.rs`, use the retained U5 owner to invoke flush/shutdown exactly once on every exit path within a finite bound. A daemon error remains primary when run and cleanup both fail, with cleanup failure preserved diagnostically; cleanup failure after a clean run is returned. No retry loop, sleep polling, Drop-only success, endpoint parsing, provider construction, or attachment work is included.

Skill domain: daemon lifecycle/error propagation. Maximum width: 1 production file, 4 functions, 3 scenarios. Estimate: 105 minutes. Atomic milestone: all U1 cleanup contracts are GREEN and finite.

### U7 — VERIFY: prove configured runtime export and failure behavior

Without implementation edits, drive the complete daemon CLI/environment -> tracing initializer -> provider -> attachment/retention -> cleanup path through the U1 in-process exporter. Prove the exact endpoint value reaches the shared constructor, one uniquely named span exports while ownership survives daemon use, cleanup occurs exactly once within the bound, combined failure follows the documented precedence, and the absent-endpoint control constructs no exporter.

Skill domain: runtime verification. Maximum width: 2 unchanged evidence surfaces, 0 changed production functions, 3 scenarios. Estimate: 80 minutes. Atomic milestone: exact focused commands and outcomes are recorded on `131.007-T`.

### U8 — VERIFY: close all-features and default quality gates

Without source, test, manifest, lockfile, or config edits, record the dependency-tree proof, all-features check/clippy/focused tests, configured endpoint/lifecycle/export results, and default-feature gates. Return any distinct defect to Stage rather than widening the release unit.

Skill domain: quality/closure evidence. Maximum width: 2 evidence surfaces, 0 changed functions, 3 gate groups. Estimate: 90 minutes. Atomic milestone: passing closure evidence is recorded on `131.008-T`.

## Dependency Graph

Strict order: U1 -> U2 -> U3 -> U4 -> U5 -> U6 -> U7 -> U8, represented by `131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T -> 131.008-T`. Every unit is blocked by its immediate predecessor. PR #362 ordering is satisfied by merged commit `685f62668ac273a41a1f93fc9be2571510decae2`. Shipment `125-S` remains queued and unclaimed until the exact final reviewed PR #363 head is integrated and all claim guards pass.

## Decisions and Rationale

- Make the actual daemon subcommand the endpoint boundary because it owns tracing initialization; the old flat `Config` parser is unused and workspace `PluginConfig` loads too late.
- Keep endpoint scope off `GlobalFlags`: OTLP config is daemon-startup-only, while shim behavior remains formatting-only.
- Rely on existing child-process environment inheritance for normal shim startup; no shim lifecycle edit is needed.
- Align bridge 0.27 to the pinned 0.26 family rather than broadening the telemetry upgrade.
- Require provider construction to accept the propagated endpoint argument; no hidden environment/config read is permitted.
- Separate propagation, construction, attachment/retention, cleanup, runtime proof, and closure so each milestone remains executable and width compliant.
- Use one deterministic in-process exporter path; an external collector does not prove ownership or cleanup.

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Dead config field is mistaken for a live source | U1 records that `Config::parse` is unused; U3 names `Command::Daemon` as the sole canonical boundary. |
| Explicit shim CLI override is lost | Supported normal shim contract is inherited `ENGRAM_OTLP_ENDPOINT`; direct `--otlp-endpoint` belongs to the daemon subcommand and is tested there. |
| Provider rereads hidden global state | U4 accepts only the U3 endpoint argument; U5 passes the exact value. |
| Provider drops before export | U4 returns retained ownership; U5 stores it across daemon use. |
| Cleanup hangs or hides failure | U6 requires exactly-once finite cleanup and explicit error precedence. |
| Runtime test bypasses production | U7 starts at the real daemon endpoint boundary and reuses the production constructor. |
| Default build or dependency graph regresses | U8 separates all-features and default closure. |

## Plan Hardening Signals

- Public API/schema/contract: present; tracing initialization and provider ownership contracts change behind a feature flag.
- Security/auth/compliance: absent.
- Migration/destructive action: absent.
- External integration/checkpoint: present; OTLP is an external export surface, though verification is in process.
- High runtime/rollback risk: present; wrong endpoint flow, lifetime, or cleanup silently loses telemetry.

Requires plan hardening: yes

## Runtime Verification and Closure

Precheck the `otlp-export` feature and Tokio runtime. Runtime verification must start from the real daemon endpoint boundary, prove exact-value propagation into the shared provider constructor, retain ownership through daemon execution, emit one uniquely named span, and complete explicit cleanup before a deterministic timeout. Failure injection must prove the daemon-versus-cleanup precedence. Roll back the owning GREEN unit and keep `125-S` unclaimed if endpoint propagation diverges, no span arrives, ownership ends early, cleanup exceeds the bound, failures are swallowed, OpenTelemetry 0.25 remains, or default features regress. Owner: Ship. Validation window: focused bounded harness plus all-features CI; no external collector checkpoint.

## Plan Hardening

Hardening rerun: **required and satisfied** after the endpoint flow was traced and the plan expanded to eight units. Reinforcing context included the constitution width rules, strict-safety vocabulary, current `Cli`/`GlobalFlags`/`Command::Daemon` path, dead flat `Config`, late workspace `PluginConfig`, shim spawn inheritance, `init_tracing`, and the uncalled builder. Engram indexed search and call mapping were used first; targeted source reads closed graph edges that the index did not expose.

| ProposedAction | Targets | ActionRisk | Rollback | Approval required | ActionResult |
|---|---|---|---|---|---|
| Author endpoint/API/lifecycle/export RED contracts | At most 2 focused test files | moderate | Revert U1; keep shipment unclaimed | no | planned |
| Align the bridge dependency family | `Cargo.toml`, `Cargo.lock` | moderate | Restore bridge 0.26 | no | planned |
| Propagate daemon endpoint config | `src/bin/engram.rs`, `src/lib.rs` | moderate | Revert U3; preserve formatting-only init | no | planned |
| Build retained provider from supplied endpoint | `src/server/observability.rs` | moderate | Revert U4 | no | planned |
| Attach and retain the owner | `src/lib.rs`, daemon arm | moderate | Revert U5; keep cleanup blocked | no | planned |
| Coordinate bounded cleanup | daemon arm | moderate | Revert U6; keep shipment unclaimed | no | planned |
| Verify runtime endpoint-to-export behavior | Unchanged harness/evidence | low | Return defect to Stage | no | planned |
| Run final closure gates | Verification evidence | low | Leave feature queued | no | planned |

Protected invariants: RED precedes every implementation edit; explicit flag precedence and inherited environment are tested; the endpoint flows once as a typed value and is never reread; shim and absent-endpoint paths remain formatting-only; each task stays within 105 minutes, 2 files, 4 functions, 3 scenarios, one skill domain, and one atomic milestone; provider ownership outlives subscriber and daemon use; cleanup is explicit, exactly once, and finite; no network oracle or unrelated config redesign is introduced. Any width breach returns to Stage for re-harvest.

## Plan Review

Gate: **PASS**. Standard review was rerun after hardening and eight-unit re-harvest. Local constitution, Rust/API, architecture, scope, test-strategy, operational-readiness, learnings, and external-boundary security lenses reviewed the actual current source flow. Cross-model persona and intercom tooling were unavailable; this is disclosed and non-blocking for this non-security maintenance plan.

| ID | Persona | Severity | Finding | Disposition |
|---|---|---|---|---|
| E1 | Architecture | P1 | Former U4 could not obtain an endpoint within `src/lib.rs` only; dead `Config::otlp_endpoint`, missing `GlobalFlags`, and pre-`daemon::run` initialization made it non-executable. | Resolved: U3 owns the exact two-file daemon-subcommand -> daemon-initializer handoff. |
| E2 | Scope boundary | P1 | Returning ownership to the daemon while forbidding daemon edits was contradictory. | Resolved: U5 explicitly owns `src/lib.rs` and the `src/bin/engram.rs` daemon arm. |
| T1 | Test strategy | P1 | Endpoint precedence and inherited-environment behavior lacked a RED oracle. | Resolved: U1 scenario 1 covers direct flag, inherited environment, absent endpoint, and typed arrival at the initializer. |
| R1 | Rust/API | P1 | Provider construction could hide a second config lookup. | Resolved: U4 accepts the propagated endpoint argument only; U5 passes the exact value. |
| C1 | Constitution | P1 | Adding plumbing to the former seven-unit plan could overload existing tasks. | Resolved: eight units cap at 105 minutes, 2 files, 4 functions, 3 scenarios, one domain, and one milestone. |
| V1 | Operational readiness | P1 | Runtime export proof and broad closure are independent milestones. | Resolved: U7 owns runtime behavior; U8 owns dependency/all-features/default closure. |
| S1 | Scope boundary | P2 | The unused flat `Config` field could remain misleading. | Accepted as explicit non-authoritative context; removing the legacy parser is unrelated cleanup and not required by the executable daemon path. |
| X1 | Security lens | P3 | External export tests must not introduce credentials or network dependency. | Resolved by the in-process exporter and no-secret/no-network scope. |

No unresolved P0 or P1 finding remains. The accepted P2 does not hide implementation work: U3 names the sole canonical endpoint boundary and all files/functions needed for propagation. Review confirms `131.001-T -> 131.002-T -> 131.003-T -> 131.004-T -> 131.005-T -> 131.006-T -> 131.007-T -> 131.008-T` and approves the updated roster in queued, unclaimed shipment `125-S`.
