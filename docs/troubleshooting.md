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

## Stdio initialize startup failures (early-child-exit diagnostics)

If an MCP client (for example GitHub Copilot CLI) reports a failure while
initializing the `engram` MCP server over stdio — a message like `failed to
initialize MCP client`, `Send message error`, or a Windows `os error 232`
("the pipe is being closed") — the shim itself did not crash mid-handshake.
Since 124-F (stash 870B1AFF), `engram shim` always binds the MCP stdio
transport and answers `initialize` before evaluating whether the workspace is
admissible, the daemon is ready, or the IPC endpoint can be derived. If one of
those preconditions fails, the session stays up in a **degraded** state:
`tools/list` still returns the full catalog, but every `tools/call` fails with
a structured error naming the real cause. The pipe-closed symptom a client
reports today is almost always caused by a stale installed binary that
predates this fix (see the version-skew signal below), not by a live defect
in the current build.

### Exit-code taxonomy

When the shim process exits, its exit code classifies why:

| Exit code | Class | Meaning |
|---|---|---|
| `0` | — | Clean shutdown; no precondition ever failed |
| `10` | `admission_failure` | The workspace path does not exist, is not a Git repository root, or the working directory could not be resolved |
| `11` | `readiness_timeout` | The daemon did not reach a ready state (spawn failure, respawn shutdown timeout, or the readiness budget was exceeded) |
| `12` | `endpoint_derivation_failure` | The IPC endpoint (named pipe or Unix socket path) could not be derived for the workspace |
| `13` | `transport_failure` | The MCP stdio transport itself failed to bind, or the session ended with a protocol error (for example, no client ever sent `initialize` before disconnecting) |

These codes apply even when the MCP protocol exchange itself looked
successful to the client (the session answered `initialize`/`tools/list`
normally but every `tools/call` failed) — the exit code reflects whether the
session was ever degraded, independent of what the client observed over the
wire. A wrapper script or CI job that checks `$LASTEXITCODE` (or `$?` on
POSIX shells) after a manual `engram shim` invocation can rely on these codes
without parsing stderr.

### Startup-failure record

Every degraded startup writes one JSON line to:

```text
<workspace>/.engram/diagnostics/shim-startup-failures.jsonl
```

Each record contains exactly four fields — a `timestamp` (RFC 3339), the
`binary_version` (the build's `ENGRAM_BUILD_HASH`, falling back to the crate
version), the `failure_class` (one of the four names above), and a sanitized
`message`. The record never contains credentials, tokens, environment
variable values, or paths outside the workspace; persisting the record is
best-effort and failures to write it are swallowed (the process exit code and
stderr line remain the primary signal).

### stdout purity invariant

`engram shim` reserves stdout exclusively for JSON-RPC framing bytes. No log
line, banner, or diagnostic text is ever written there — all human-readable
output, including tracing output enabled via `RUST_LOG`, goes to stderr. This
holds even when `RUST_LOG=engram=debug` and `ENGRAM_LOG_FORMAT=json` are set.
If you ever observe a non-JSON-RPC byte on the shim's stdout, treat it as a
regression and file it against 124-F — it corrupts the MCP framing channel
and is the exact hazard this contract exists to prevent.

### Daemon log destination change (stdout → stderr)

The daemon's tracing writer is now pinned to stderr for every log format
(previously the `fmt` layer's default, stdout, was used for
`engram daemon`). If a log-capture script or supervisor redirects the
daemon's stdout to a file, update it to capture stderr instead.

### Version-skew signal

The `binary_version` field in the startup-failure record (and the `version`
field in `engram daemon-status` / `engram workspace-status` output) reports
the exact build hash running. Compare it against the latest released build.
An operator hitting a pre-initialize pipe closure on a binary whose version
predates a relevant fix (for example, worktree admission changes) should
reinstall or update (`engram update`) before investigating further — this was
the immediate trigger behind the original 870B1AFF report.

### Correlating `/mcp show engram` with the record

When your MCP client exposes a `/mcp show engram` (or equivalent) diagnostic
command, cross-reference its reported server status with the startup-failure
record:

1. Run `/mcp show engram` (or the client's equivalent) to see the client's
   view of the server (connected, degraded, or failed).
2. Read the most recent line of
   `<workspace>/.engram/diagnostics/shim-startup-failures.jsonl` for the
   `failure_class` and sanitized `message`.
3. Match the `failure_class` against the exit-code taxonomy above to decide
   the next action: `admission_failure` → verify the workspace path and Git
   root; `readiness_timeout` → see
   [Daemon startup or IPC timeout](#daemon-startup-or-ipc-timeout);
   `endpoint_derivation_failure` → verify the workspace path is valid UTF-8
   and within platform path-length limits; `transport_failure` → verify the
   client is actually sending `initialize` and not closing the connection
   early.
4. If the `binary_version` in the record predates your expected release, see
   [Version-skew signal](#version-skew-signal) above.
