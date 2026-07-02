---
title: Engram Troubleshooting Guide
description: Symptom-based troubleshooting for workspace setup, indexing, client wiring, and runtime diagnostics.
---

## Overview

Use this page when Engram does not behave the way the quickstart or workflows
guide suggests. Start with the symptom that matches what you are seeing, then
move to [docs/log-observation-guide.md](log-observation-guide.md) when you need
deeper operational detail.

## Fast checks

Run these first from the workspace root:

```bash
engram daemon-status --format text
engram workspace-status --format text
engram health --format text
engram manifest --format text
```

Those commands quickly tell you whether the binary is callable, the daemon is
reachable, the workspace is bound, and the tool catalog is present.

## Symptom guide

| Symptom | What to check | Likely next step |
|---|---|---|
| Client launches but no tools appear | Confirm the client uses the `engram` binary as a stdio MCP server | Fix the client config or point it at the correct binary path |
| `bind` fails | Confirm you are in a Git repository or passed `--workspace` explicitly | Run from the repo root or supply the path |
| Search returns nothing | Confirm the workspace is bound and the code graph exists | Run `engram sync` or `engram index` |
| Results look stale after a branch switch | Remember that Engram stores branch-aware indexed state | Run `engram sync` after checkout or force a rebuild |
| The daemon will not stay healthy | Inspect `.engram/logs/` and verify idle-timeout or ready-timeout overrides | Review runtime env vars or reinstall the workspace artifacts |
| Semantic search is missing or degraded | Check how the binary was built | Rebuild with default features or use symbol-only surfaces |
| Old HTTP/SSE instructions fail | Verify whether you actually built with `legacy-sse` | Prefer the stdio shim path unless compatibility requires otherwise |

## Client wiring problems

The most common failure is a client configuration that points to an old HTTP/SSE
endpoint or a stale binary path.

For manual setups, prefer a stdio entry like:

```json
{
  "mcpServers": {
    "engram": {
      "type": "stdio",
      "command": "/absolute/path/to/engram",
      "args": [],
      "cwd": "/absolute/path/to/workspace"
    }
  }
}
```

If the client can start the binary but tool calls fail, run `engram bind` and
`engram workspace-status` from a terminal in the same workspace to confirm the
runtime path is healthy outside the client.

## Indexing problems

When indexing is the issue, use this order:

1. `engram workspace-status --format text`
2. `engram sync --format text`
3. `engram sync --full --format text` or `engram index --format text`
4. `engram index --direct --format text` (or set `ENGRAM_DIRECT=1`) when daemon startup or the sync IPC call times out

`engram index` and `engram sync --full` are both full rebuild paths. Use them
when incremental sync is not enough.

### Daemon startup or IPC timeout

When the daemon is slow to reach its ready state, or the sync or index IPC call
times out (for example, a `Daemon failed to reach Ready state` error), reach for
direct mode as the escape hatch. It indexes in the current process without
spawning or waiting on a daemon:

```bash
engram index --direct --format text
engram sync --full --direct --format text
ENGRAM_DIRECT=1 engram sync --format text
```

Direct mode acquires the workspace lock first, so stop any running daemon before
you retry; otherwise the command exits with a lock error. See
[configuration.md](configuration.md#daemonless-direct-indexing) for the full
reference.

## Runtime cleanup

When runtime artifacts are suspect:

* use `engram update` to refresh generated artifacts while preserving data
* use `engram reinstall` to rebuild runtime directories and regenerate the starter registry
* use `engram uninstall --keep-data` when you need to remove runtime wiring but keep workspace data

Move to the operations guide when you need to inspect logs, latency, or report
surfaces in more detail.
