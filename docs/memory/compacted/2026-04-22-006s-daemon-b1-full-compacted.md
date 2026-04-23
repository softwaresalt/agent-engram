---
title: "Compacted: 006-S Daemon Reliability B1 — full lifecycle (2026-04-22)"
date: 2026-04-23
compacted_from:
  - docs/memory/2026-04-22/carry-forward-everything-memory.md
  - docs/memory/2026-04-22/circuit-break-git-push-pr18.md
  - docs/memory/2026-04-22/ship-006-s-awaiting-merge-approval-memory.md
  - docs/memory/2026-04-22/ship-006-s-post-merge-closure-memory.md
  - docs/memory/2026-04-22/ship-006-s-pr18-green-memory.md
  - docs/memory/2026-04-22/ship-006-s-release-network-blocked-memory.md
  - docs/memory/2026-04-22/ship-006-s-reliability-b1-completion-memory.md
  - docs/memory/2026-04-22/ship-006-s-session-final-memory.md
  - docs/memory/2026-04-22/ship-006-s-shipment-reconcile-prep-memory.md
  - docs/memory/2026-04-22/stage-b2-harvest-memory.md
archived_to: docs/archive/memory/
completed_shipments: [006-S]
merge_sha: 091a164a405e42d55bc0345f35ce09f39e7d5500
pr: 18
status: compacted
---

# Compacted Memory — 006-S Daemon Reliability B1 (2026-04-22)

## Outcome

PR #18 merged to main (SHA `091a164a`). 12 backlog items shipped: 029.001–029.003 chores and 9 tasks.  
Shipment archived. 029-F B2 harvested and queued as 009-S for the next ship session.

## What Was Built

| Work Stream | Items | Key Files |
|---|---|---|
| WS-1 Version handshake + auto-respawn | 029.001-C, .001–.003-T | `src/shim/version.rs` (new), `src/daemon/version.rs` (new), `src/shim/ipc_client.rs`, `src/shim/lifecycle.rs` |
| WS-3 Self-healing PID/lock files | 029.002-C, .001–.003-T | `src/shim/pidfile.rs` (new), `src/daemon/lockfile.rs` |
| WS-5 `.workspace-id` persistent identity | 029.003-C, .001–.003-T | `src/db/workspace.rs`, `src/tools/lifecycle.rs` |

## Implementation Decisions (durable)

- `tempfile` promoted from dev-dep to normal dep — atomic PID writes require same-dir temp+persist
- PID file schema: `{pid, start_time_unix}` JSON — start-time match prevents PID reuse false-positives
- `ambiguous_bind` detection: shim-side only (no daemon protocol change), returns `WorkspaceError::AmbiguousBind`
- Version respawn trigger: fires on EITHER `IpcError::VersionMismatch` OR any initial-handshake failure — "stale binary" covers both cases
- `DirBuilder::mode()` no-op on pre-existing dirs documented; addressed in B2 (009-S)

## Failed Approaches / Hard-Won Lessons

- **Circuit-breaker (git push)**: GitHub connectivity failed mid-session; fix commit `2f2303c` was exported as `.bundle` + `.patch` to session workspace for cross-session recovery
- **tokio::test attribute change**: switching a sync test attribute to `#[tokio::test]` changes semantics — only async tests get `#[tokio::test]`
- **clippy rejection**: `unwrap_or_else(|_| default)` on non-Err paths rejected as unnecessary; use `unwrap_or(default)` or match directly
- **cozo-only CI import issue**: `daf8c7d` fixed CozoDB conditional import causing CI fail on default SurrealDB backend

## Copilot Review (PR #18)

5 review threads: all replied and resolved. 1 finding (Unix socket permission hardening) deferred as follow-up stash item `F7C8E121` — addressed in 009-S.

## Post-Merge Closure Actions

- `backlogit shipment ship` force-released covering feature `029-F` (known upstream bug) — restored `.backlogit/queue/029-F.md` from Git before B2 harvest
- Compound learning added: `docs/compound/workflow-issues/backlogit-shipment-ship-force-releases-covering-feature-2026-04-22.md`
- 3 stash follow-ups: Unix /tmp fallback socket permissions (→ addressed in 009-S), shim/daemon handshake smoke (→ addressed in 009-S), backlogit covering-feature overship bug (→ upstream)

## B2 Harvest (Stage)

029-F B2 (WS-2 doctor, WS-4 registry, WS-6 scan, WS-7 integration, WS-8 telemetry, WS-9 socket) harvested as 19-item shipment 009-S. Plan at `docs/exec-plans/2026-04-22-029-F-b2-observability-validation-plan.md`.
