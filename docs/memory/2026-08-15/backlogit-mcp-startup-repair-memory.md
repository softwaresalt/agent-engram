---
title: Backlogit MCP startup repair
date: 2026-08-15
status: complete
---

## Outcome

Backlogit v1.9.0 loaded the workspace but spent several minutes in its
startup migration before serving MCP requests or CLI output. The local MCP
registration now uses an exact build of upstream merge commit
`1235bcd80879fc59b4632e4b3eadfaf2d746cd9c`, which contains PR #361's
canonical artifact index startup fix.

The configured server completed MCP initialization, advertised 58 tools, and
included `backlogit_list_shipments`, `backlogit_fetch_stash`, and
`backlogit_query_sql`. Patched CLI reads completed in under 300 milliseconds.

## Files and Artifacts

* Modified ignored local configuration: `.mcp.json`
* Built ignored session binary:
  `.copilot/session-state/9ec2cd49-aa90-4186-9c44-d92bca7e5afc/files/backlogit-patched.exe`
* Retained the exact upstream source checkout beside the binary for provenance

## Decisions

* Used the merged upstream fix instead of modifying backlog data or terminating
  MCP processes owned by other live Copilot sessions
* Kept the override local and ignored rather than installing an unreleased
  binary system-wide
* Added `--no-update-check` to the MCP launch arguments to avoid unnecessary
  startup network work

## Failed Approaches

* The stock v1.9.0 MCP handshake exceeded 30 seconds without responding
* Stock `list` and `doctor` commands eventually completed but took several
  minutes
* SQLite integrity was not the cause: `PRAGMA quick_check` returned `ok`

## Next Step

Restart the MCP client so it reloads `.mcp.json`. Replace the local override
with the normal `backlogit mcp` registration after an official release includes
PR #361.
