---
title: Engram Operations and Diagnostics Guide
description: Logging, health checks, report surfaces, and runtime diagnostics for Engram.
---

## Overview

This guide covers the operational surfaces you use after Engram is installed:
logs, health signals, branch metrics, and report commands. It complements the
symptom-oriented [docs/troubleshooting.md](troubleshooting.md) page.

## Runtime artifacts to know

| Path | What to look for |
|---|---|
| `.engram/logs/` | Structured daemon logs |
| `.engram/run/` | IPC endpoints and runtime locks |
| `.engram/db/` | Workspace-local database files when the default data dir is used |

## Useful diagnostic commands

```bash
engram daemon-status --format text
engram workspace-status --format text
engram stats --format text
engram health --format text
engram branch-metrics --format text
engram report token-savings --format text
engram report eval --format text
engram report retry-metrics --format text
```

Use them together:

* `daemon-status` tells you whether the daemon is alive and which runtime build is active
* `workspace-status` tells you whether the workspace is bound and whether the code graph looks current
* `stats` and `health` expose workspace and runtime health signals
* `branch-metrics` and the `report` subcommands expose delivery and usage telemetry

## Running the daemon manually

When you need to observe the daemon directly, run it with an explicit workspace:

```bash
ENGRAM_LOG_FORMAT=pretty RUST_LOG=engram=debug engram daemon --workspace /path/to/workspace
```

On Windows PowerShell:

```powershell
$env:ENGRAM_LOG_FORMAT = "pretty"
$env:RUST_LOG = "engram=debug"
engram daemon --workspace D:\path\to\workspace
```

The daemon creates `.engram/logs/` when it starts. That directory is the first
place to look when a client-driven session behaves differently from a manual
terminal run.

## Health signals that matter

| Signal | Healthy shape | Investigate when |
|---|---|---|
| `daemon-status` | Version and uptime return immediately | The command cannot reach the daemon |
| `workspace-status` | Bound workspace and non-empty graph after sync | The workspace is unset or still empty after indexing |
| `health` | Tool traffic and latency values continue moving | Latency spikes or counts stay flat during real usage |
| `branch-metrics` | Branch summary matches the branch you expect | Metrics look tied to the wrong branch or stay empty |
| `report retry-metrics` | Retry count stays low and recent failures are explainable | Retry counts rise unexpectedly or never reset |

## Log-reading tips

Use `pretty` logs when you are tracing a single local issue. Use `json` logs
when you want to pipe output into another tool.

```bash
ENGRAM_LOG_FORMAT=json RUST_LOG=engram=debug engram daemon --workspace /path/to/workspace
```

Focus first on:

* daemon startup and lock acquisition
* IPC listener readiness
* workspace bind or workspace-status failures
* sync or index failures
* health-report or report-command anomalies

## Compatibility note

Legacy HTTP/SSE is an optional feature-gated path. If you are debugging the
default installation, stay on the stdio shim plus IPC daemon path first and
reach for `legacy-sse` only when you have an explicit compatibility need.
