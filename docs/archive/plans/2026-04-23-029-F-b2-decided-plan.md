---
title: "Decided Plan — 029-F B2 Observability & Validation (Shipment 009-S)"
date: 2026-04-23
compacted_from: docs/exec-plans/2026-04-22-029-F-b2-observability-validation-plan.md
archived_to: docs/archive/plans/
shipment: 009-S
feature: 029-F
status: shipped
merge_sha: 0d831cad050c57ff538867a8025f46834ac1018f
---

# Decided Plan — 029-F B2 Observability & Validation

**Status**: SHIPPED. All 6 units implemented in PR #21 → `main`.

## Scope

WS-2 doctor health diagnostics, WS-4 registry strict validation, WS-6 background scan, WS-7 integration tests, WS-8 telemetry counters, WS-9 Unix socket permission hardening.

## Final Decisions

### Unit 1 — Structured health diagnostics + doctor CLI (WS-2)

- `HealthReport` / `HealthCheck` / `HealthStatus` (Green/Yellow/Red) in `src/models/health.rs` (new)
- `derive_overall` maps per-check statuses: `Unknown` treated as `Yellow` (not ignored)
- `get_health_report_for_daemon` in `src/tools/doctor.rs` (new) — covers all 8 failure modes
- Error handling: match+log for health check errors (no silent discards)
- `get_daemon_status` response includes `health` field

### Unit 2 — Strict registry validation (WS-4)

- NEW `validate_sources_strict` parallel function — NEVER modify existing `validate_sources`
- `validate_sources` call sites (`let _ = ...`) remain unchanged (backward compat)
- `ContentSource.optional: bool` (defaults false) added to `src/models/registry.rs`
- Known rename detection: `.backlog` → `.backlogit` with remediation hint
- `doctor --fix` flag for registry auto-correction

### Unit 3 — Background offline-change scan (WS-6)

- `set_workspace` returns within 500ms SLA — scan is fully backgrounded via `tokio::spawn`
- Background task uses `Arc<AppState>` (not borrow) for shared state ownership
- `CancellationToken` per scan generation — new `set_workspace` cancels stale scan
- `clear_hydration_ready()` called BEFORE `tokio::spawn(background_db_hydration)` — race-free
- `pending_scan: true` in `WorkspaceBinding` response when scan in progress

### Unit 4 — Remaining integration tests (WS-7)

- `tests/integration/registry_missing_source_test.rs` (new)
- `tests/integration/multi_session_resume_test.rs` (new)

### Unit 5 — Failure-mode telemetry (WS-8)

- `ReliabilityCounters` struct with `AtomicU64` fields lives in `src/server/state.rs` — NOT `src/services/metrics.rs`
- Counters surfaced via `get_daemon_status` health report (single surface, not a new tool)
- Shim-side events bridged to daemon counters via IPC notification after reconnect
- Double metrics init bug fixed: removed `metrics::initialize` from `background_db_hydration`

### Unit 6 — Unix socket permission hardening (WS-9)

- Private subdirectory `/tmp/engram-{key}/` created with `DirBuilder::mode(0o700)` at creation time
- Post-create permission verification required: `fs::metadata().permissions().mode() & 0o777 == 0o700`
- Reason: `DirBuilder::mode()` has no effect on pre-existing directories (compound learning documented)

## Critical Constraints

- `validate_sources` call signature/behavior MUST NOT change
- `ReliabilityCounters` in `AppState`, not `services/metrics.rs`
- `clear_hydration_ready()` order: before spawn, not after (race condition)
- Socket dir permissions: creation-time mode + post-create verification (not TOCTOU-vulnerable chmod)
- All 8 failure modes must be covered in doctor health report

## Deferred Follow-Ups (stashed, not blocking)

1. Scan race: `begin_scan_generation()` vs `clear_hydration_ready()` ordering needs mutex-guarded invariant
2. Workspace traversal: pre-check that `.engram/` dir exists before scan generation starts
3. Registry counter: wire `registry_validation_errors` counter in `validate_sources_strict`

## Rejected Alternatives

- ~~`ReliabilityCounters` in `services/metrics.rs`~~ — mixing per-workspace tool metrics with process-level reliability counters
- ~~`get_daemon_metrics` as separate MCP tool~~ — single surface via `get_daemon_status` health report
- ~~Post-create chmod for socket dir~~ — TOCTOU window; creation-time mode with post-create verification
