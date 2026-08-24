---
title: "Engram Copilot MCP diagnostic memory"
date: 2026-08-23
type: memory
---

## Outcome

Diagnosed the current Engram MCP failure as a Copilot CLI prerelease
compatibility issue plus an independent Engram cold-start delay.

## Files Modified

* `.mcp.json` - added the missing local Engram stdio registration; this file is
  intentionally ignored by Git
* `docs/decisions/2026-08-23-copilot-prerelease-server-discover-mcp-compatibility-spike.md`
  - recorded investigation evidence and recommendations
* `docs/memory/2026-08-23/engram-copilot-mcp-diagnostic-memory.md` - recorded
  session continuity

## Decisions

* Do not attribute the broken pipe to Tokio; it is downstream of shim exit
* Treat Copilot `server/discover` before `initialize` as the immediate
  compatibility trigger
* Keep the roughly 7.5-minute Cozo cold start as a separate performance and
  readiness defect
* Prefer stable Copilot 1.0.80 as the immediate isolation step
* Prefer a narrow Engram pre-initialize compatibility handler as the durable
  source fix

## Failed Approaches

* Port 7437 HTTP probe did not apply to the installed stdio/IPC architecture
* Three 30-second daemon readiness probes expired before Cozo startup completed
* A fresh Copilot prompt could not use Engram MCP because the client sent
  `server/discover` before `initialize`

## Open Questions

* Stable Copilot 1.0.80 behavior has not been run locally
* The exact expensive Cozo schema or database-open stage remains unprofiled

## Next Steps

* Restart on Copilot stable 1.0.80 and verify MCP discovery
* Plan and ship Engram compatibility coverage for pre-initialize
  `server/discover`
* Profile Cozo startup independently
