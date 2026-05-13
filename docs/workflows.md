---
title: Engram Workflows Guide
description: Common Engram command sequences for setup, indexing, search, graph traversal, and diagnostics.
---

## Overview

Use this page as the task cookbook after the quickstart is done. Each workflow
starts from the current workspace root unless a command shows an explicit path.

## Bring a workspace online

```bash
engram install
engram bind
engram sync
```

That sequence installs workspace artifacts, binds the workspace, and ensures the
first index exists.

## Refresh the index

| Goal | Command | Notes |
|---|---|---|
| Normal refresh | `engram sync` | Incremental path through the daemon |
| Forced rebuild | `engram sync --full` | Full rebuild without changing the command shape |
| Alias for full rebuild | `engram index` | Same intent as `sync --full` |
| Prewarm without daemon lifecycle | `engram sync --direct` | Runs in-process and exits when complete |

Use `sync` as the routine command. Use `index` or `sync --full` when you want a
clean rebuild.

## Search by concept or symbol

```bash
engram search "workspace lifecycle" --format text
engram query-memory "release workflow" --format text
engram symbols --type function --prefix run_ --format text
```

Use `search` when you know the concept. Use `symbols` when you already know the
kind or name shape you want.

## Walk graph relationships

```bash
engram map-code run --depth 2 --format text
engram impact run --depth 2 --format text
engram query-graph --operation neighborhood --root fn:abc123 --max-depth 2 --format text
```

Use:

* `map-code` to inspect local callers, callees, and usages
* `impact` to estimate blast radius before a change
* `query-graph` when you need a structured path or neighborhood traversal

## Check health and delivery signals

```bash
engram daemon-status --format text
engram workspace-status --format text
engram health --format text
engram branch-metrics --format text
engram report token-savings --format text
```

These commands are the fastest way to confirm that the daemon is healthy, the
workspace is current, and usage telemetry is being recorded.

## Refresh or remove the installation

```bash
engram update
engram reinstall
engram uninstall --keep-data
```

Use `update` when you want fresh generated artifacts. Use `reinstall` when the
runtime directories need a clean rebuild. Use `uninstall --keep-data` when you
want to remove wiring without discarding the workspace data.
