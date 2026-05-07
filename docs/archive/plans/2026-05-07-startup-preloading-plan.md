---
title: "Startup Script Engram Pre-Loading"
description: "Add engram sync call to start.ps1 before Copilot launch"
source_deliberation: "docs/decisions/2026-05-07-startup-preloading-deliberation.md"
source_stash: "B59D87CA"
requires_plan_hardening: no
---

## Objective

Add an engram sync step to `start.ps1` that pre-loads the workspace database
before Copilot launches, eliminating MCP timeout on initial binding.

## Implementation Units

### Unit 1: Add engram sync to start.ps1

**Files modified**: `start.ps1`

**Change**: Insert an engram sync block between the backlogit sync block and the
`& $copilotExe --remote` line. Follow the identical pattern:

```powershell
$engramCmd = Get-Command engram -ErrorAction SilentlyContinue
if ($engramCmd) {
    try {
        engram sync --workspace . --quiet
    } catch {
        Write-Warning "engram sync failed (non-fatal): $_"
    }
}
```

**Rationale**:
- `Get-Command` check: skips cleanly if engram not on PATH
- `--workspace .`: explicit workspace targeting (cwd = repo root since start.ps1 is run from there)
- `--quiet`: suppresses success output, only errors print
- `try/catch` + `Write-Warning`: non-fatal; Copilot launches regardless
- Placement after backlogit sync: ensures backlogit state is fresh for engram to index

### Unit 2: Contract test for start.ps1 integration

**Files modified**: `tests/integration/startup_script_test.rs` (new file)

**Change**: Add a lightweight integration test that validates:
1. The start.ps1 script parses without syntax errors (PowerShell `-File` with `-WhatIf` or AST parse)
2. The engram sync section exists and follows the expected pattern

Actually, PowerShell script testing is not part of the Rust test suite. This unit
is **out of scope** — start.ps1 is a user-facing script, not a compiled artifact.
The acceptance criterion is manual verification that the script runs correctly.

## Acceptance Criteria

- [ ] `start.ps1` contains an engram sync block before the copilot launch line
- [ ] The `--remote` flag on the copilot command is preserved
- [ ] Running `start.ps1` with engram on PATH executes sync without error
- [ ] Running `start.ps1` without engram on PATH skips gracefully (no error)
- [ ] Running `start.ps1` when engram sync fails still launches Copilot

## Dependency Order

```text
Single task — no dependencies.
```

## Constitution Check

- **I. Safety-First Rust**: N/A — this is a PowerShell script change, not Rust code
- **II. Test-First**: N/A — script changes don't have Rust test harnesses
- **IV. CLI Containment**: Respected — script operates in cwd
- **VII. Destructive Approval**: No destructive commands added

## Plan Review

**Gate Decision: PASS**

Reviewed by consolidated persona assessment (lightweight plan, no Rust code).

### Findings

None. Plan follows the established `start.ps1` pattern verbatim. No architectural,
security, scope, or compliance concerns.

### Rationale

- Single-file script change with no compiled artifacts
- Identical error-handling pattern to existing backlogit sync block
- No new dependencies introduced
- Non-fatal by design — cannot break Copilot launch
- `--remote` flag preserved as requested

<!-- plan-review-attempt: 1 -->
<!-- plan-review-verdict: PASS -->
