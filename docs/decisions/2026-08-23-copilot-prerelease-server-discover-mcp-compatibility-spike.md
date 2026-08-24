---
title: "Copilot prerelease server/discover MCP compatibility"
type: spike
date: 2026-08-23
time_box: "2h"
conclusion: "pivot"
confidence: "high"
linked_parent_work_item: null
promoted_to: ["none"]
tags:
  - "mcp"
  - "copilot-cli"
  - "windows"
---

## Goal

Determine whether Engram's Windows MCP failure is caused by Tokio, Engram
daemon startup, workspace registration, or GitHub Copilot CLI 1.0.81-8.

## Success Criteria

* Reproduce the failure through Copilot and through a protocol-level MCP client
* Separate stdio framing behavior from daemon readiness
* Identify an immediate recovery path and a durable compatibility direction

## Scope Constraints

The investigation was read-mostly. It did not terminate the stalled daemon,
replace binaries outside the workspace, or modify production source code.
The ignored workspace-local `.mcp.json` was repaired so Copilot could attempt
the Engram connection.

## Investigation Approach

1. Inspect installed Engram and Copilot versions and workspace MCP registration
2. Exercise Engram directly over MCP stdio without Copilot
3. Measure daemon startup and query its raw named-pipe health endpoint
4. Launch a fresh Copilot client and inspect its MCP handshake log
5. Compare the installed Copilot build with the latest stable release

## Findings

### What Was Discovered

The installed Engram build does not expose an HTTP MCP endpoint at port 7437.
It uses an MCP stdio shim backed by a per-workspace Windows named pipe. The
failed HTTP probe was therefore expected for this build.

The workspace `.mcp.json` did not contain an `engram` server. Copilot confirmed
that only backlogit, context7, tavily, and GitHub were registered. A local
Engram registration was added:

```json
{
  "type": "stdio",
  "command": "engram",
  "args": ["shim"],
  "env": {
    "ENGRAM_WORKSPACE": "${workspaceFolder}"
  },
  "tools": ["*"]
}
```

Direct MCP stdio tests passed. Engram returned a valid `initialize` result,
listed its 20-tool catalog, and completed
`tools/call(get_daemon_status)`. The response reported version `0.2.0`,
protocol 1, PID 24740, and a reachable workspace.

The daemon had a separate cold-start problem. It started at 16:35:42 local
time, remained in `starting` beyond repeated 30-second readiness windows,
consumed about 1.37 GB, and accumulated about 893 CPU-seconds. The 135 MB
Cozo database stopped changing at 16:42:44, and the health report recorded
session indexing at 16:43:12. Startup therefore took about 7.5 minutes before
the daemon became usable. Once ready, direct MCP calls were healthy and the
offline scan was green.

The decisive Copilot failure is in
`.copilot/logs/process-1787528861694-24588.log`:

* Line 105 shows Copilot sending JSON-RPC request `server/discover` with ID 0
  before the MCP `initialize` request
* Engram's rmcp 1.1 state machine rejects that ordering with
  `expect initialized request`
* Line 106 shows the downstream broken-pipe error after Engram exits with
  transport-failure exit code 13

The Tokio type names in the client error describe Copilot's child-process pipe
implementation. Tokio reports the closed pipe; it is not the root cause.

Copilot successfully initializes other MCP servers in the same run. The GitHub
MCP server logs `method invalid during initialization` for `server/discover`
but remains alive and accepts the subsequent standard handshake. Engram's
strict rmcp initialization path exits instead.

The installed Copilot build is `1.0.81-8`, published as a prerelease on
2026-08-23. The latest stable release is `1.0.80`. The public prerelease notes
do not document the `server/discover` behavior, and no matching public issue
was found.

### What Was Tried and Failed

* HTTP connection to `127.0.0.1:7437/mcp`: no listener because this Engram
  build uses stdio plus named-pipe IPC
* Repeated CLI readiness probes: timed out while the daemon performed its
  multi-minute cold start
* A fresh Copilot prompt: Engram exited before initialization because Copilot
  sent `server/discover` first; the model then fell back to the Engram CLI

### Remaining Unknowns

* Whether stable Copilot 1.0.80 omits the pre-initialize `server/discover`
  request was not executed locally
* Which Cozo open or schema-bootstrap operation accounts for most of the
  7.5-minute cold start
* Whether GitHub intends `server/discover` as a prerelease-only extension or a
  permanent client preflight

## Recommendation

**Conclusion**: pivot

**Confidence**: high

Treat this as two independent defects.

For immediate recovery, use the workspace-local Engram registration and move
Copilot from prerelease `1.0.81-8` to stable `1.0.80`, then restart Copilot.
The stable downgrade is the lowest-risk client-side isolation step, but it
still requires an end-to-end verification because stable behavior was not run
in this investigation.

For durable compatibility, Engram should tolerate Copilot's pre-initialize
`server/discover` probe: return JSON-RPC method-not-found for that request and
continue waiting for the standard `initialize` request. Contract coverage must
prove that normal MCP clients remain unchanged, stdout remains JSON-RPC-only,
and a Copilot-style probe no longer terminates the shim.

Track the multi-minute Cozo cold start separately. Increasing the shim timeout
would mask the symptom and would not fix the expensive database open or
bootstrap path.

## Next Steps

* Switch Copilot to stable 1.0.80 and verify Engram initialization after restart
* Add an Engram contract test for `server/discover` before `initialize`
* Implement a narrow compatibility preflight ahead of rmcp initialization
* Profile Cozo open and schema bootstrap for the 135 MB branch database

## References

* `.mcp.json`
* `.copilot/logs/process-1787528861694-24588.log`
* `src/shim/transport.rs`
* `src/db/cozo_backend/mod.rs`
* `docs/decisions/2026-08-21-870b1aff-copilot-mcp-stdio-initialize-investigation.md`
* <https://github.com/github/copilot-cli/releases/tag/v1.0.81-8>
* <https://github.com/github/copilot-cli/releases/tag/v1.0.80>

## Resolution

Implemented in shipment 124-S / feature 130-F. The shim now opens a narrow
pre-`initialize` compatibility window that answers `server/discover` with
JSON-RPC `-32601` while preserving the request id type, and continues waiting
for a standards-compliant `initialize`. The interception allowlist is exactly
that one method; every other frame still reaches rmcp unchanged.

* Implementation: `src/shim/preinit_compat.rs`, bound in `src/shim/transport.rs`
* Kill switch: `ENGRAM_MCP_PREINIT_COMPAT=0` restores strict rmcp ordering
* Operator runbook, rollback, and retirement criteria:
  [`docs/troubleshooting.md`](../troubleshooting.md#pre-initialize-serverdiscover-probe-copilot-cli-compatibility)
* Plan: `docs/exec-plans/2026-08-23-copilot-server-discover-compat-plan.md`

The Cozo cold-start cost recorded in the findings above remains an open,
independently tracked defect and was deliberately excluded from this shipment.
