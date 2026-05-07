---
title: "Decided Plan — 029-F B1 Daemon Reliability (Shipment 006-S)"
date: 2026-04-23
compacted_from: docs/exec-plans/2026-04-21-029-F-b1-foundational-reliability-plan.md
archived_to: docs/archive/plans/
shipment: 006-S
feature: 029-F
status: shipped
merge_sha: 091a164a405e42d55bc0345f35ce09f39e7d5500
---

# Decided Plan — 029-F B1 Daemon Reliability

**Status**: SHIPPED. All 3 units implemented in PR #18 → `main`.

## Scope

WS-1 version handshake + auto-respawn, WS-3 self-healing PID/lock, WS-5 `.workspace-id` persistent identity. (B2 WS-2/4/6/7/8 shipped separately in 009-S.)

## Final Decisions

### Unit 1 — Version handshake + auto-respawn

- Shared `version` module in both binaries; IPC handshake exchanges versions
- Respawn trigger: fires on EITHER `IpcError::VersionMismatch` OR any initial handshake failure (connection-reset, deserialization, timeout) — "stale binary" covers both cases
- Bounded: max 1 respawn attempt; if retry also fails, surface original error
- New files: `src/shim/version.rs`, `src/daemon/version.rs`
- Probe timeout: 100ms `pub(crate) const`; max respawn attempts: 1 `pub(crate) const`

### Unit 2 — Self-healing PID/lock files

- PID schema: `{pid, start_time_unix}` JSON — start-time match prevents PID reuse false positives
- Atomic writes: `tempfile::NamedTempFile::new_in(pid_dir).persist()` — cross-volume rename rejected (fails on Windows)
- `tempfile` promoted from dev-dep to normal dep
- New file: `src/shim/pidfile.rs`

### Unit 3 — `.workspace-id` persistent identity

- Persist UUIDv4 to `.engram/.workspace-id` on first bind; daemon key switches from path-hash to workspace-id
- `ambiguous_bind` detection: shim-side only (no daemon protocol change)
- Error: `WorkspaceError::AmbiguousBind { expected_id, found_id, path }` in `src/errors/mod.rs`
- Legacy workspaces fall back to path-hash with deprecation log

## Critical Constraints

- All new on-disk writes resolve through `canonicalize_workspace` before create/rename
- `WorkspaceError::PathEscape` if any path escapes workspace root
- CI matrix MUST include windows-latest + ubuntu-latest (pipe primitives differ)
- Each unit starts with `.001-T` red-phase harness (failing tests + minimum stubs) before implementation

## Rejected Alternatives

- ~~Daemon-side `ambiguous_bind` detection~~ — keeps Unit 3 from leaking into Unit 1's protocol surface
- ~~Direct file writes for PID~~ — cross-volume rename fails on Windows; `tempfile::new_in` required
- ~~Post-creation chmod for socket dir~~ — TOCTOU window; creation-time mode required (addressed in B2)

## Rollback Triggers (reference only — observation window elapsed)

- `set_workspace` p95 > 1500ms → revert Unit 3
- Shim respawn rate > 2/hour → revert Unit 1
- Stale-PID false positives in CI → revert Unit 2
- Revert order: Unit 3 → Unit 2 → Unit 1
