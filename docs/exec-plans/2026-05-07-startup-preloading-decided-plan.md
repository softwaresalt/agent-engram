---
title: "Startup Script Engram Pre-Loading (Decided)"
decision_date: 2026-05-07
review_verdict: PASS
---

## Decision

Add an engram sync block to `start.ps1` that pre-loads the workspace database before Copilot launches, eliminating MCP timeout on initial binding.

## Implementation

Insert engram sync block between the backlogit sync block and the `& $copilotExe --remote` line:

```powershell
$engramCmd = Get-Command engram -ErrorAction SilentlyContinue
if ($engramCmd) {
    engram sync --workspace $PSScriptRoot --quiet
}
```

## Key Constraints

- **Non-fatal**: sync failures do not prevent Copilot launch (no try/catch; errors ignored)
- **Workspace binding**: Use `$PSScriptRoot` for explicit workspace targeting
- **Quiet mode**: `--quiet` suppresses success output
- **Placement**: After backlogit sync, before copilot launch
- **Preserve flags**: `--remote @args` on copilot line remains unchanged

## Implementation Differences from Plan

- **Error handling**: Plan proposed try/catch; implementation uses silent failure (engram not on PATH returns no error)
- **Workspace argument**: Plan proposed `--workspace .`; implementation uses `$PSScriptRoot` for clarity

## Review Summary

**Verdict: PASS**

Single-file script change. Lightweight plan, identical pattern to existing backlogit sync block. No architectural, security, scope, or compliance concerns. No new dependencies introduced. Non-fatal by design — cannot break Copilot launch.
