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

## Stdio initialize and readiness failures

If an MCP client (for example GitHub Copilot CLI) reports a failure while
initializing the `engram` MCP server over stdio — a message like `failed to
initialize MCP client`, `Send message error`, or a Windows `os error 232`
("the pipe is being closed") — the shim itself did not crash mid-handshake.
Since 124-F (stash 870B1AFF), `engram shim` always binds the MCP stdio
transport and answers `initialize` before evaluating whether the workspace is
admissible, the daemon is ready, or the IPC endpoint can be derived. If one of
those preconditions fails permanently, the session stays up in a **degraded**
state: `tools/list` still returns the full catalog, but every `tools/call`
fails with a structured error naming the real cause.

A readiness-budget expiry is different. The error includes
`recoverable: true` and `retry_after_ms`, and the same stdio process continues
probing the named-pipe or Unix-socket daemon. Later calls are forwarded as soon
as that daemon reports ready; the MCP client does not need to restart the
session. Spawn, protocol, shutdown, admission, and endpoint failures remain
terminal and report `recoverable: false`.

### Exit-code taxonomy

When the shim process exits, its exit code classifies why:

| Exit code | Class | Meaning |
|---|---|---|
| `0` | — | Clean shutdown; startup succeeded or a transient readiness timeout recovered before disconnect |
| `10` | `admission_failure` | The workspace path does not exist, is not a Git repository root, or the working directory could not be resolved |
| `11` | `readiness_timeout` | The daemon did not reach a ready state before disconnect, or daemon startup failed permanently |
| `12` | `endpoint_derivation_failure` | The IPC endpoint (named pipe or Unix socket path) could not be derived for the workspace |
| `13` | `transport_failure` | The MCP stdio transport itself failed to bind, or the session ended with a protocol error (for example, no client ever sent `initialize` before disconnecting) |
| `14` | `protocol_incompatible` | The daemon's protocol or `_health` contract is incompatible with this shim version — retrying will never succeed |

These codes apply even when the MCP protocol exchange itself looked successful
to the client. The exit code reflects the state when the client disconnected:
a recovered session exits `0`, while a session that remained degraded exits
with its classified failure. A wrapper script or CI job that checks
`$LASTEXITCODE` (or `$?` on POSIX shells) after a manual `engram shim`
invocation can rely on these codes without parsing stderr.

### Transient vs terminal health classification

The shim's late-readiness recovery path classifies `_health` probe failures
into two categories:

| Category | Wire code | `failure_class` | `recoverable` | `retry_after_ms` | Exit code | Operator action |
|---|---|---|---|---|---|---|
| **Transient** | `15002` | `readiness_timeout` | `true` | `250` | `11` | Wait and retry — the daemon is warming up, unreachable, timed out, reset, or answering a non-ready status |
| **Terminal** | `15005` | `protocol_incompatible` | `false` | *(absent)* | `14` | Upgrade or replace the daemon — retrying will never succeed |

**Agent integration contract:** `recoverable` is the sole authoritative retry
signal — never key retry logic off `failure_class` alone; treat `failure_class`
as diagnostic-only. Check `retry_after_ms` for *key presence*, not truthiness
or non-null: the key is present only on the recoverable/transient branch and
is never `null` or `0` when present. If `recoverable` is itself missing or of
an unexpected type (a malformed or unrecognized future payload), fail closed
and treat the call as non-retryable — do not infer retryability from
`failure_class` or any other field. Example payloads
(`tools/call` response, abbreviated):

```text
// Transient
{
  "result": {
    "isError": true,
    "structuredContent": {
      "engram_code": 15002,
      "failure_class": "readiness_timeout",
      "message": "daemon did not reach a ready state within the configured budget",
      "recoverable": true,
      "retry_after_ms": 250
    }
  }
}

// Terminal
{
  "result": {
    "isError": true,
    "structuredContent": {
      "engram_code": 15005,
      "failure_class": "protocol_incompatible",
      "message": "daemon protocol version 999 is incompatible (expected 1)",
      "recoverable": false
    }
  }
}
```

**What makes a probe terminal (proven incompatibility only):**

- Daemon returned JSON-RPC error `-32601` Method Not Found for `_health`
- Daemon response omitted the `result` payload entirely
- Daemon `result` could not be decoded as a valid health response
- Daemon protocol version does not match the shim's expected version

**What stays transient (transport errors, by construction):**

- Connection refused, timeout, reset, EOF, truncated response
- JSON-RPC error codes other than `-32601` (including `-32603`, `-32700`, `-32600`)
- Version-compatible daemon reporting a non-ready status

### Startup-failure record

Every startup deadline or terminal startup failure writes one JSON line to:

```text
<workspace>/.engram/diagnostics/shim-startup-failures.jsonl
```

Each record contains exactly four fields — a `timestamp` (RFC 3339), the
`binary_version` (the build's `ENGRAM_BUILD_HASH`, falling back to the crate
version), the `failure_class` (one of `admission_failure`, `readiness_timeout`,
`endpoint_derivation_failure`, `transport_failure`, or `protocol_incompatible`),
and a sanitized `message`. The record never contains credentials, tokens, environment
variable values, or paths outside the workspace. A readiness-timeout record
shows that the initial deadline was exceeded; it does not by itself prove that
the session failed to recover later. Persisting the record is best-effort, and
failures to write it are swallowed (the process exit code and stderr line
remain the primary terminal signal).

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

## Pre-initialize `server/discover` probe (Copilot CLI compatibility)

GitHub Copilot CLI `1.0.81-8` (a **prerelease**; the latest stable at the time
of writing is `1.0.80`) sends a JSON-RPC request with method `server/discover`
and request id `0` **before** it sends MCP `initialize`. The MCP specification
requires `initialize` to be the first request, and the rmcp handshake enforces
that strictly: it rejected the probe with `expect initialized request` and
terminated, which the client observed as a broken pipe and which classified as
`transport_failure` (exit `13`).

Since 130-F, the shim opens a narrow **pre-initialize compatibility window**
that tolerates exactly this probe.

### The contract

While the session is waiting for `initialize`:

| Inbound frame | Shim behavior |
|---|---|
| `initialize` | Forwarded to rmcp unchanged; the window closes permanently |
| `server/discover` **with** an id | Answered with JSON-RPC `-32601` (`Method not found`), echoing the request id verbatim and type-preserving; not forwarded; the window stays open |
| `server/discover` **without** an id | Dropped silently — JSON-RPC forbids responding to a notification; the window stays open |
| Anything else | Forwarded to rmcp unchanged, preserving existing ordering and error semantics |

The interception allowlist is **exactly one method**. Unknown id-bearing
methods are deliberately *not* absorbed: they still reach rmcp's strict path
so genuine client ordering bugs remain visible rather than being masked.

`server/discover` is undocumented in the `1.0.81-8` prerelease notes and may be
prerelease-only, so the shim does not attempt to implement it. Answering
`-32601` is the conservative choice and matches the GitHub MCP server, which
returns `method invalid during initialization` for the same probe in the same
Copilot run and which Copilot demonstrably tolerates.

Request id `0` is echoed as the JSON **number** `0`. Zero is a classic
falsy-id serialization hazard: coercing it to `null`, to an absent field, or to
the string `"0"` would leave the client unable to correlate the response.

### Kill switch and rollback

| Variable | Default | Effect |
|---|---|---|
| `ENGRAM_MCP_PREINIT_COMPAT` | enabled | Set to `0` to disable the compatibility window and restore strict rmcp handshake ordering |

Rollback options, cheapest first:

1. **Runtime** — set `ENGRAM_MCP_PREINIT_COMPAT=0` in the MCP client's server
   definition. No redeploy or reinstall is required. With the switch off, a
   pre-initialize probe again produces `transport_failure` (exit `13`).
2. **Source** — the production change is confined to `src/shim/preinit_compat.rs`
   and the transport binding in `src/shim/transport.rs`. A single `git revert`
   of the implementation commit restores the prior behavior. The test files are
   additive and safe to leave in place.

Because the window is armed only before `initialize`, any rollback affects only
the handshake window — never a live session's tool traffic.

### Monitoring plan and post-deploy observation window

The compatibility window is **on by default** and sits in the MCP handshake
path, so a regression here removes MCP availability entirely rather than
degrading a feature. Watch these signals after rollout.

| Signal | Where observed | Baseline | Alert / rollback threshold |
|---|---|---|---|
| Shim exit code `13` (`transport_failure`) on client disconnect | Exit code of `engram shim`; `failure_class` in `<workspace>/.engram/diagnostics/shim-startup-failures.jsonl` | `0` occurrences for probe-then-`initialize` sessions | **Any** exit `13` attributable to a pre-`initialize` probe |
| MCP client initialization failures | Client log (`.copilot/logs/process-*.log`): `failed to initialize MCP`, `os error 232`, `expect initialized request` | `0` occurrences | **Any** occurrence naming the `engram` server |
| Non-JSON-RPC bytes on shim stdout | `tests/contract/shim_stdout_purity_test.rs` in CI; manual stdio runbook step 2 | `0` | **Any** unparseable stdout line |
| `tools/list` catalog served after a probe | `/mcp show engram` or the client's server list | Full catalog | Catalog empty or server listed as failed |

* **Owner**: the operator performing the Copilot upgrade or Engram install.
* **Observation window**: the first **48 hours** of normal Copilot CLI usage
  after installing a build containing this change, plus the first session
  following any Copilot CLI version change (the probe behavior is a
  client-side prerelease trait and can shift between releases).
* **Rollback trigger**: any occurrence in the "Alert / rollback threshold"
  column above. Roll back with `ENGRAM_MCP_PREINIT_COMPAT=0` first (runtime,
  no redeploy) and confirm the signal clears; if it does not, the cause is not
  the compatibility window.
* **Window close**: record the outcome as healthy, degraded, or rolled back.
  A healthy close is the precondition for treating the window as steady-state
  rather than under observation.

### Retiring this compatibility layer


This layer exists for a **prerelease** client defect. At the time it shipped
there was no public `github/copilot-cli` issue tracking the behavior, so it
cannot be retired by watching an upstream ticket. Retire it deliberately when
both of the following hold:

* the Copilot CLI releases in use no longer send `server/discover` before
  `initialize`, and
* the contract tests in `tests/contract/shim_pre_initialize_probe_test.rs` are
  removed in the same change as the production module.

Until then, treat the window as load-bearing for Copilot CLI users on Windows.

### Windows verification runbook

Verify against **both** Copilot CLI channels. Do not change the installed
Copilot version as part of this procedure.

1. Build and install the shim under test, then confirm the running build:

   ```powershell
   engram --version
   ```

2. Confirm the compatibility window is active by driving the exact Copilot
   ordering over stdio:

   ```powershell
   $probe = '{"jsonrpc":"2.0","id":0,"method":"server/discover","params":{}}'
   $init  = '{"jsonrpc":"2.0","id":1,"method":"initialize","params":{"protocolVersion":"2024-11-05","capabilities":{},"clientInfo":{"name":"runbook","version":"1.0"}}}'
   $probe, $init | engram shim --workspace .
   ```

   Expect a first frame carrying `"error":{"code":-32601,...}` with `"id":0`
   as a number, followed by a successful `initialize` result. A closed pipe
   after the first frame means the window is disabled or the binary is stale.

3. Launch Copilot CLI `1.0.81-8` (prerelease) with **only** Engram enabled and
   confirm the MCP connection reports no initialization failure, that
   `/mcp show engram` lists the server as connected, and that `tools/list`
   returns the full catalog.

4. Repeat step 3 against stable Copilot CLI `1.0.80` and confirm the behavior
   is unchanged — the window is a no-op for clients that never send the probe.

5. Disconnect cleanly and confirm the shim exits `0`, not `13`.

6. Confirm the daemon readiness budget is **unchanged**: `ENGRAM_READY_TIMEOUT_MS`
   must have the same default as before this change. Increasing the readiness
   timeout was explicitly rejected as a fix — it masks the symptom without
   addressing the handshake ordering defect.
