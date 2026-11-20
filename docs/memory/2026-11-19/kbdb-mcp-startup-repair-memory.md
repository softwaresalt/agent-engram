---
title: kbdb MCP startup repair
date: 2026-11-19
status: complete
---

## Outcome

The kbdb MCP server failed during Copilot CLI discovery because Copilot sent
`server/discover` before the standard `initialize` request. kbdb 0.3.1 uses
rmcp, which requires `initialize` as the first request and exited with code 2.

The workspace already contained a discovery compatibility shim, which answers
`server/discover` locally and
then forwards the MCP session to kbdb. The ignored `.mcp.json` registration
bypassed that shim. The shim was subsequently renamed to
`.copilot\kbdb-mcp-shim.cjs`.

## Changes

* Updated the local `kbdb` MCP command to launch
  `.copilot\kbdb-mcp-shim.cjs` with the workspace-local Bun runtime
* Kept kbdb arguments as `serve --read-only`
* Preserved the existing kbdb binary, databases, and index
* Left the existing `start.ps1`, `.backlogit/runtime/`, `.bin/`, and `tools/`
  working-tree changes untouched

## Verification

* Direct kbdb initialize handshake succeeded when run without the incompatible
  discovery probe
* The shim answered `server/discover` and forwarded a successful `initialize`
  response
* A fresh Copilot CLI prompt loaded kbdb through the updated MCP registration
  and called `get_status` successfully
* kbdb reported 7 sources, 68,714 chunks, schema version 4, and idle auto-sync

## Safety Record

* Mode: investigate-first
* Proposed action: change only the ignored kbdb MCP launch wiring
* Action risk: moderate
* Approval requirement: not required for the non-destructive local config edit
* Action result: applied
* Rollback: restore the previous direct kbdb command in `.mcp.json`
