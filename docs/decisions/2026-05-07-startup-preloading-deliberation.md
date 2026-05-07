---
title: "Startup Script Engram Pre-Loading"
description: "Integrate engram index/sync into start.ps1 to preload the database before Copilot launches"
topic: "Engram startup pre-loading integration"
depth: "lightweight"
decision_status: "decided"
promoted_to: "plan"
linked_artifacts:
  - "docs/exec-plans/2026-05-07-startup-preloading-plan.md"
tags:
  - "cli"
  - "startup"
  - "preloading"
source_stash: "B59D87CA"
---

## Problem Frame

The engram MCP daemon performs workspace indexing on first binding. When Copilot
starts and immediately invokes engram MCP tools, the initial indexing can exceed
the MCP response timeout, causing tool call failures. Pre-loading the database
from start.ps1 before launching Copilot eliminates this cold-start penalty.

**Success criteria**: After running start.ps1, the engram database is populated
so that the first MCP tool call against the daemon receives indexed data without
waiting for an initial full index.

**Constraints**: Non-fatal — if engram is not installed or sync fails, Copilot
must still launch. Follow the existing backlogit sync pattern in start.ps1.

## Research Findings

- `start.ps1` already follows a pattern: detect tool → try sync → warn on failure → proceed
- The backlogit section uses: `Get-Command backlogit`, `try/catch`, `Write-Warning`
- `engram sync` (incremental) calls `sync_workspace` via IPC to the running daemon
- `engram index` (full) calls `index_workspace` via IPC
- Both commands use the `--workspace` global flag to target a specific directory
- The CLI already handles daemon auto-spawn: if no daemon is running, the shim spawns one
- The `--quiet` flag suppresses non-error output (ideal for startup scripts)

## Options Evaluated

### Option A: `engram sync` (incremental)

Fast incremental sync. If the workspace was previously indexed, only processes
changed files. If never indexed, the daemon auto-indexes on first bind anyway.

- **Pros**: Fast for warm starts, minimal overhead
- **Cons**: On first-ever run with no prior index, sync alone may not trigger a
  full index (depends on daemon auto-bind behavior)

### Option B: `engram sync --full` / `engram index` (full re-index)

Forces a complete re-parse of all workspace source files every startup.

- **Pros**: Guarantees fresh state
- **Cons**: Slower (~5-15s depending on workspace size); wasteful on warm starts

## Decision

**Chosen: Option A (`engram sync`)** with graceful fallback.

Rationale: The daemon already handles cold-start by auto-indexing when a
workspace is first bound. The startup script's job is to ensure the daemon is
running and the workspace is incrementally synced so that subsequent MCP calls
are fast. A full re-index on every startup is wasteful. If the user needs a full
index, they can run `engram index` manually.

The implementation follows the identical pattern as the existing backlogit sync
section in start.ps1.

## Covering Feature Scope

This is a single-task feature: "Startup Script Engram Pre-Loading". The task
adds an engram sync call to start.ps1, following the established backlogit sync
pattern. No additional tasks are needed — the CLI infrastructure already exists
from 042-F.

## Risks and Mitigations

| Risk | Mitigation |
|---|---|
| Daemon not installed | `Get-Command` check; skip with no warning |
| Daemon spawn takes too long | Non-blocking; backgound daemon spawn is handled by the shim already |
| Sync fails (no prior index) | `try/catch` with `Write-Warning`; Copilot launches anyway |
