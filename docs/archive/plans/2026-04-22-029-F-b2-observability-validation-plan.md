---
title: "029-F Shipment B2 — Observability & Validation"
description: "Implementation plan for WS-2 engram doctor, WS-4 registry validation, WS-6 background scan, WS-7 integration tests, WS-8 telemetry, plus 006-S stash follow-ups"
source_document: "docs/decisions/2026-04-22-029-F-b2-observability-validation-deliberation.md"
covering_feature: "029-F"
requires_plan_hardening: no
plan_review_attempts: 1
---

## Source

This plan operationalizes the B2 phase of `docs/decisions/2026-04-21-029-F-daemon-reliability-deliberation.md` (Option 2, Phase B2) plus two 006-S closure follow-ups captured in `docs/decisions/2026-04-22-029-F-b2-observability-validation-deliberation.md`.

## Primary Objective

Complete the daemon reliability program by adding the observability, validation, and diagnostics layer on top of B1's stable foundation: structured health diagnostics (`engram doctor`), strict registry validation, background offline-change scanning, remaining integration tests, failure-mode telemetry, Unix socket permission hardening, and an operator-facing smoke command.

## Constitution Check

* **I. Safety-First Rust**: All new code under `#![forbid(unsafe_code)]`. No `unwrap`/`expect`. Errors via `Result<T, EngramError>`.
* **II. Test-First**: Each unit starts with a red-phase harness task.
* **III. Workspace Isolation**: Socket path and registry path validation stays within workspace root.
* **IV. CLI Containment**: Doctor CLI and smoke command operate within cwd.
* **VI. Single Responsibility**: No new external dependencies required beyond existing tree.

## Implementation Units

### Unit 1 — Structured health diagnostics + doctor CLI (WS-2 + Stash S2)

Backlog target: `029.004-C` with 3 tasks.

**Approach**: Add a `health` field to `DaemonStatus` (structured report with red/yellow/green per diagnostic check). Implement `engram doctor` CLI subcommand that queries daemon status and runs additional local checks (pipe reachability, PID file validity, registry parse, workspace-id consistency). Absorb stash S2 (smoke command) as `doctor --smoke` flag that exercises a full handshake round-trip: spawn daemon, connect, exchange version, set_workspace, shut down.

Doctor MUST cover all eight 029-F failure modes:

| Check | Failure mode | Source |
|---|---|---|
| `binary_version` | Stale daemon binary vs current shim | WS-1 (B1) |
| `pid_liveness` | Stale `engram.pid` for a dead process | WS-3 (B1) |
| `workspace_identity` | Duplicate daemon from canonical-path drift | WS-5 (B1) |
| `pipe_reachability` | Daemon running but pipe unreachable | WS-3 (B1) |
| `registry_validity` | Registry referencing missing source paths | WS-4 (B2) |
| `offline_scan` | Offline-change detection looks like a hang | WS-6 (B2) |
| `session_resume` | Confused state after `/new` session switch | WS-7 (B2) |
| `telemetry_health` | Failure counters above threshold | WS-8 (B2) |

**Touched files**:
* `src/tools/lifecycle.rs` — extend `DaemonStatus` with `health: HealthReport`, add diagnostic logic
* `src/bin/engram.rs` — add `doctor` subcommand to clap CLI, `--smoke` flag
* `src/models/health.rs` (new) — `HealthReport`, `HealthCheck`, `HealthStatus` (red/yellow/green) types. All new response DTOs live in `src/models/` for consistency.
* `src/errors/mod.rs` — no new error codes needed; health checks report status, not errors

**Test posture**: `.001-T` is the red-phase harness task that scaffolds `tests/contract/doctor_health_test.rs` and `tests/integration/doctor_smoke_test.rs` as compiling-but-failing tests with minimal stubs. `.002-T` and `.003-T` depend on `.001-T` and make the harness pass. Apply `pub(crate)` visibility for test-accessed helpers per `docs/compound/best-practices/pub-visibility-for-external-test-harness-2026-04-20.md`. Follow `docs/compound/test-failures/tempdir-lifetime-in-contract-tests-2026-03-30.md` for TempDir lifetime in test helpers.

**Acceptance criteria**:
* `engram doctor` returns a structured report covering all eight failure modes listed above
* `doctor --smoke` exercises a full shim/daemon handshake and reports pass/fail
* `get_daemon_status` response includes a `health` field with per-check red/yellow/green status

### Unit 2 — Strict registry validation (WS-4)

Backlog target: `029.005-C` with 2 tasks.

**Approach**: Add a NEW `validate_sources_strict` function in `src/services/registry.rs` that fails on any non-optional missing source, alongside the existing best-effort `validate_sources`. The current call site in `ipc_server.rs` (`let _ = validate_sources(...)`) continues using the best-effort path for backward compatibility. A new call site in the doctor diagnostic path and in a startup "strict mode" uses `validate_sources_strict`. Surface known renamed-path migrations (`.backlog` → `.backlogit`) with a remediation hint. Add `doctor --fix` registry remediation path that auto-corrects known renames.

**Critical design note**: Do NOT modify the existing `validate_sources` signature or behavior — it is called with `let _ =` in production and changing it would break the startup path. Create a parallel strict function.

**Touched files**:
* `src/services/registry.rs` — add `validate_sources_strict` function, `optional` field support, known-rename detection, remediation hint text
* `src/models/registry.rs` — add `optional: bool` (defaults to `false`) to `ContentSource` struct
* `src/bin/engram.rs` — wire `--fix` flag to registry remediation

**Test posture**: `.001-T` is the red-phase harness task that scaffolds `tests/contract/registry_strict_validation_test.rs` as a compiling-but-failing test. `.002-T` depends on `.001-T` and makes the harness pass. Apply compound learnings for test helpers.

**Acceptance criteria**:
* `validate_sources_strict` on a registry with a non-existent, non-optional path returns a clear error with remediation hint
* Known renamed paths (`.backlog` → `.backlogit`) are detected with a specific migration suggestion
* `doctor --fix` applies known-rename corrections to `registry.yaml`
* Existing `validate_sources` call sites remain unaffected

### Unit 3 — Background offline-change scan (WS-6)

Backlog target: `029.006-C` with 3 tasks.

**Approach**: Refactor `set_workspace` to return immediately after binding with `pending_scan: true`. Offline-change detection moves to a background `tokio::spawn` task. The spawned task receives an `Arc<AppState>` (not a borrow) for shared state ownership. Progress is visible via `get_workspace_status` (new `scan_status: Option<ScanProgress>` field). Scan completion triggers re-index if changes were detected. Scan state lives in `server/state.rs` with a generation counter to handle concurrent `sync_workspace` requests.

**500ms SLA scope**: The 500ms acceptance criterion from 029-F covers the **bind response latency** — `set_workspace` returns the `WorkspaceBinding` within 500ms. Full hydration, code-graph indexing, and offline-change detection continue in the background. The existing heavy post-bind path (DB connect, hydration) already runs asynchronously in `ipc_server.rs`; this unit specifically backgrounds the offline-change scan that currently blocks within `set_workspace`.

**State ownership**: The background scan task MUST use `Arc<AppState>` (already the daemon's state type) — NOT a borrowed `&AppState`. Cancellation: scan tasks check a `CancellationToken` per scan generation so that a new `set_workspace` cancels a stale scan.

**Touched files**:
* `src/tools/lifecycle.rs` — add `pending_scan` to `WorkspaceBinding`, refactor offline detection into spawned task
* `src/tools/lifecycle.rs` — add `ScanProgress` to `WorkspaceStatus` (kept in `src/models/` for consistency with Unit 1 if the type is complex enough)
* `src/server/state.rs` — add scan state tracking with generation counter, `CancellationToken`
* `src/daemon/ipc_server.rs` — remove synchronous scan blocking from workspace init path

**Test posture**: `.001-T` is the red-phase harness task that scaffolds `tests/contract/background_scan_test.rs` as a compiling-but-failing test. `.002-T` and `.003-T` depend on `.001-T` and make the harness pass.

**Acceptance criteria**:
* `set_workspace` bind response returns within 500ms even on workspaces with thousands of changed files
* Response includes `pending_scan: true` when scan is in progress
* `get_workspace_status` reports scan progress and completion
* Scan completion triggers re-indexing when offline changes are detected
* Concurrent `set_workspace` or `sync_workspace` cancels a stale scan via generation counter

### Unit 4 — Remaining integration tests (WS-7)

Backlog target: `029.007-C` with 2 tasks.

**Approach**: Scaffold and implement the two remaining integration tests from the 029-F scope:
* `tests/integration/registry_missing_source_test.rs` — validates that a registry with a missing non-optional source path surfaces the correct error through the full daemon pipeline
* `tests/integration/multi_session_resume_test.rs` — validates that a daemon can serve a second shim session after the first disconnects, preserving workspace state

**Touched files**:
* `tests/integration/registry_missing_source_test.rs` (new)
* `tests/integration/multi_session_resume_test.rs` (new)
* `Cargo.toml` — register `[[test]]` entries

**Test posture**: red-phase task scaffolds both tests as compiling-but-failing harnesses. Implementation task makes them pass using the newly strict registry validation (Unit 2) and existing daemon lifecycle.

**Acceptance criteria**:
* `registry_missing_source_test` exercises the full daemon pipeline with a broken registry and asserts typed error with remediation hint
* `multi_session_resume_test` disconnects one shim session and reconnects a second, asserting workspace state is preserved
* Both tests pass in CI

### Unit 5 — Failure-mode telemetry (WS-8)

Backlog target: `029.008-C` with 2 tasks.

**Approach**: Add `ReliabilityCounters` struct with `AtomicU64` counters in daemon-owned state (`src/server/state.rs` or a new `src/services/reliability.rs` module). Counters: stale-PID-recovered, version-mismatch-respawn, registry-validation-failures, duplicate-daemon-detected. Daemon-side events (registry validation, duplicate-daemon) increment directly. Shim-side events (version-mismatch-respawn, stale-PID-recovered) are bridged to daemon state via a lightweight IPC notification after the recovery action completes (the shim already reconnects after respawn, so the notification piggybacks on the reconnect). Surface counters via `get_daemon_status` in the `HealthReport.telemetry` section (single telemetry surface — not a separate `get_daemon_metrics` tool).

**Counter ownership**: Counters live in `AppState` (daemon process), NOT in `src/services/metrics.rs` (which is workspace/usage-event oriented). This avoids mixing per-workspace tool-call metrics with process-level reliability counters.

**Touched files**:
* `src/server/state.rs` — add `ReliabilityCounters` struct with `AtomicU64` fields, accessor methods
* `src/tools/lifecycle.rs` — surface counters in `HealthReport` via `get_daemon_status`
* `src/daemon/ipc_server.rs` — increment `duplicate_daemon_detected` and `registry_validation_failures` at existing call sites
* `src/shim/lifecycle.rs` — after successful respawn or PID recovery, send a lightweight IPC notification that increments the daemon-side counter

**Test posture**: `.001-T` is the red-phase harness task that scaffolds `tests/contract/reliability_counters_test.rs` as a compiling-but-failing test. `.002-T` depends on `.001-T` and wires counter increments.

**Acceptance criteria**:
* Telemetry counters increment correctly when injected failure modes are exercised
* Counters are surfaced in `get_daemon_status` health report (single surface, not a separate tool)
* Counter operations are lock-free `AtomicU64` increments with no allocation on the hot path
* Shim-side recovery events are bridged to daemon counters via IPC notification

### Unit 6 — Unix socket permission hardening (Stash S1)

Backlog target: `029.009-T` (standalone task under 029-F).

**Approach**: In `ipc_endpoint_impl` (Unix), when using the `/tmp` fallback path, create the socket inside a private subdirectory (`/tmp/engram-{key}/`) with restrictive permissions. The private directory MUST be created atomically with `0o700` permissions at creation time using `std::fs::DirBuilder::new().mode(0o700).create()` — NOT created permissively and then tightened via `chmod`, which would leave a TOCTOU window. Socket file is then placed inside the private directory.

**Touched files**:
* `src/daemon/ipc_server.rs` — modify fallback path to use private subdirectory with restrictive permissions at creation time

**Test posture**: test-first — add a unit test in `tests/unit/` verifying the fallback path creates a directory with correct permissions before the socket bind.

**Acceptance criteria**:
* Fallback socket directory is created with `0o700` permissions at creation time (not post-creation chmod)
* Socket file is inside the private directory, not directly in `/tmp`
* Existing non-fallback (in-workspace) socket path behavior is unchanged

## Sequencing and Dependencies

1. **Unit 1** (doctor + health + smoke) — independent; can start immediately
2. **Unit 2** (strict registry) — independent; can start immediately
3. **Unit 3** (background scan) — independent; can start immediately
4. **Unit 4** (integration tests) — depends on Unit 2 (registry test needs strict validation) and Unit 3 (multi-session test benefits from background scan but doesn't strictly require it)
5. **Unit 5** (telemetry) — depends on Units 1-3 (wires counters into B1 recovery sites and new B2 validation paths)
6. **Unit 6** (socket permissions) — independent; can start immediately

Parallelism: Units 1, 2, 3, 6 can proceed in parallel. Unit 4 follows Unit 2. Unit 5 follows Units 1-3.

## Decisions and Rationale

| Decision | Rationale |
|---|---|
| Absorb stash S2 into Unit 1 as `doctor --smoke` | Smoke testing is a diagnostic — natural home in the doctor subcommand |
| Stash S1 as standalone Unit 6 | Socket permissions are a security concern orthogonal to observability |
| Background scan via `tokio::spawn` | Non-blocking approach preserves 500ms SLA without requiring a separate daemon process |
| Telemetry as atomic counters, not `UsageEvent` JSONL | Reliability counters need lock-free increment on hot paths; JSONL is for tool-call events |
| Private subdirectory for `/tmp` fallback instead of `fchmod` | Permission-at-creation prevents the TOCTOU window between socket creation and permission application |

## Risks and Caveats

| Risk | Mitigation |
|---|---|
| Doctor scope creep beyond 8 checks | Strict acceptance criteria; each check maps to one 029-F failure mode |
| Background scan race with manual `sync_workspace` | Scan state includes a generation counter; concurrent requests merge |
| Socket permission behavior differs macOS vs Linux | Test on Linux CI runner; document macOS `kern.posix.sem` differences if any |
| Telemetry counter memory leak on long-running daemons | Counters are fixed-size `AtomicU64`; no allocation growth |

## Plan Hardening Signals

* **Public API, schema, or contract change**: Yes — `DaemonStatus` gains a `health` field; `WorkspaceBinding` gains `pending_scan`; `WorkspaceStatus` gains `scan_status`. These are additive, not breaking.
* **Security, auth, permission, or compliance-sensitive behavior**: Yes — socket permission hardening (Unit 6) changes IPC security posture. Low blast radius (Unix-only fallback path).
* **Migration, backfill, destructive data/config action**: No.
* **External integration, operator checkpoint, or external dependency**: No.
* **High runtime, rollout, or rollback risk**: No — all changes are additive. Rollback is a revert of the branch.

`Requires plan hardening: no` — Changes are additive (new fields, new CLI subcommand, new tests). Security change (Unit 6) is scoped to a single Unix fallback path with low blast radius. No protocol breaks, no migrations, no external dependencies.

### Rollback and safety notes

* **Additive response fields**: `health`, `pending_scan`, `scan_status` are new fields. Agents that do not expect them will ignore them (JSON deserialization is lenient). Rollback: revert the branch removes the fields.
* **Doctor CLI**: new subcommand only. Rollback: revert removes the subcommand.
* **Strict registry validation**: parallel function, does not change existing best-effort path. Rollback: revert.
* **Background scan**: existing `set_workspace` synchronous behavior is the baseline. Rollback: revert restores synchronous scan.
* **Socket permissions (Unit 6)**: changes `/tmp` fallback path format from `/tmp/engram-{key}.sock` to `/tmp/engram-{key}/engram.sock`. Existing sockets are not migrated; new sockets use the new path. Rollback: revert restores flat path. Old sockets in `/tmp` are cleaned up by normal socket lifecycle.
* **Telemetry counters**: in-memory only, no persistence. Rollback: revert.

## Compound Learnings Applied

| Learning | Applied in |
|---|---|
| `tempdir-lifetime-in-contract-tests-2026-03-30.md` | Units 1, 2, 3, 4 — test helpers must return TempDir handle to caller |
| `pub-visibility-for-external-test-harness-2026-04-20.md` | Units 1, 2, 5 — test-accessed helpers use `pub(crate)` |
| `ci-rust-version-gap-clippy-lints-2026-04-20.md` | All units — task DoD includes `cargo clippy --target-dir target-redphase -- -D warnings -D clippy::pedantic` |
| `ship-shipment-overscoped-manifest-2026-04-20.md` | Plan scoped to single B2 phase (~15 tasks) |

## Runtime Verification and Closure

* **Unit 1** (doctor): runtime surface changed (new CLI subcommand, new API field). Verify `engram doctor` and `doctor --smoke` produce expected structured output on a real workspace.
* **Unit 2** (registry): verify a broken registry.yaml surfaces a human-readable error with remediation hint.
* **Unit 3** (background scan): verify `set_workspace` returns within 500ms on a workspace with many files; verify `get_workspace_status` shows scan progress.
* **Unit 5** (telemetry): verify counters are visible in daemon status after triggering known failure modes.
* **Unit 6** (socket permissions): verify on a Unix system that the fallback socket directory has `0o700` permissions.

## Plan Review

**Gate decision: PASS (after revision)**

Review attempt 1 returned FAIL with 7 P1 findings. All P1s were addressed in revision:

| P1 Finding | Resolution |
|---|---|
| Missing hardening section despite signals | Added `## Plan Hardening Signals` with rollback notes and safety analysis |
| Test-first sequencing weaker than B1 | Each multi-task unit now explicitly reserves `.001-T` as red-phase harness with hard dependency |
| `validate_sources` extension unsound | Replaced with parallel `validate_sources_strict` function; existing call sites unaffected |
| Background scan state ownership | Specified `Arc<AppState>` ownership, `CancellationToken`, generation counter |
| 500ms SLA not achievable | Clarified: 500ms covers bind response only; heavy work already backgrounded |
| WS-2/WS-8 requirements trace incomplete | Added explicit 8-check trace table in Unit 1; fixed single telemetry surface in Unit 5 |
| Counter placement architecturally wrong | Moved counters to `AppState`; shim events bridged via IPC notification |

P2 findings addressed: compound learnings table added, module boundary rule clarified (DTOs in `src/models/`), Unit 6 secure-creation requirement made explicit.

<!-- plan-review-attempt: 1 -->
