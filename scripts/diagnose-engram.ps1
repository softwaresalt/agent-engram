#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Read-only Engram customer-box diagnostics report.

.DESCRIPTION
    Resolves the target workspace, then reports:
      - the resolved engram executable
      - engram version
      - daemon status
      - workspace status
      - health
      - a live unified search for "workspace binding"
      - the last 20 lines of .engram\diagnostics\shim-startup-failures.jsonl
        (or a note that the file is absent)

    All engram subcommands are invoked with an explicit --workspace path and
    --format text. This script is diagnostic-only: it makes no mutating calls
    (no bind, sync, index, flush, install, etc.). It does not hide command
    failures -- every diagnostic runs even if an earlier one failed, so the
    operator always gets a full report. The script exits nonzero if any
    engram diagnostic command failed.

.PARAMETER Workspace
    Path to the workspace to diagnose. Defaults to the current directory.

.EXAMPLE
    .\scripts\diagnose-engram.ps1

.EXAMPLE
    .\scripts\diagnose-engram.ps1 -Workspace C:\path\to\workspace
#>

[CmdletBinding()]
param(
    [string]$Workspace = (Get-Location).Path
)

Set-StrictMode -Version Latest
$ErrorActionPreference = "Stop"
# Native (external) command failures must not throw terminating errors here --
# every diagnostic below needs to run regardless of earlier failures.
$PSNativeCommandUseErrorActionPreference = $false

$script:HadFailure = $false

# ── Resolve workspace ───────────────────────────────────────────────────────

try {
    $ResolvedWorkspace = (Resolve-Path -LiteralPath $Workspace).Path
} catch {
    Write-Error "Cannot resolve workspace path '$Workspace': $($_.Exception.Message)"
    exit 1
}

$EngramCmd = Get-Command engram -ErrorAction SilentlyContinue

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Engram customer-box diagnostics" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Workspace : $ResolvedWorkspace" -ForegroundColor Gray
Write-Host "  Engram    : $(if ($EngramCmd) { $EngramCmd.Source } else { '<not found on PATH>' })" -ForegroundColor Gray

function Invoke-EngramDiagnostic {
    param(
        [Parameter(Mandatory)][string]$Title,
        [Parameter(Mandatory)][string[]]$Arguments
    )

    Write-Host ""
    Write-Host "-- $Title ----------------------------------" -ForegroundColor Yellow

    if (-not $EngramCmd) {
        Write-Host "  SKIPPED: engram executable not found on PATH." -ForegroundColor Red
        $script:HadFailure = $true
        return
    }

    & $EngramCmd.Source @Arguments 2>&1 | ForEach-Object { Write-Host "  $_" }

    if ($LASTEXITCODE -ne 0) {
        Write-Host "  FAILED (exit code $LASTEXITCODE)" -ForegroundColor Red
        $script:HadFailure = $true
    }
}

# -- engram version (does not take --workspace) -----------------------------

Write-Host ""
Write-Host "-- engram version ------------------------------" -ForegroundColor Yellow
if ($EngramCmd) {
    & $EngramCmd.Source --version 2>&1 | ForEach-Object { Write-Host "  $_" }
    if ($LASTEXITCODE -ne 0) {
        Write-Host "  FAILED (exit code $LASTEXITCODE)" -ForegroundColor Red
        $script:HadFailure = $true
    }
} else {
    Write-Host "  SKIPPED: engram executable not found on PATH." -ForegroundColor Red
    $script:HadFailure = $true
}

# -- Live diagnostics (all read-only) ---------------------------------------

Invoke-EngramDiagnostic -Title "daemon status" -Arguments @(
    "daemon-status", "--workspace", $ResolvedWorkspace, "--format", "text"
)

Invoke-EngramDiagnostic -Title "workspace status" -Arguments @(
    "workspace-status", "--workspace", $ResolvedWorkspace, "--format", "text"
)

Invoke-EngramDiagnostic -Title "health" -Arguments @(
    "health", "--workspace", $ResolvedWorkspace, "--format", "text"
)

Invoke-EngramDiagnostic -Title "search: 'workspace binding'" -Arguments @(
    "search", "workspace binding", "--workspace", $ResolvedWorkspace, "--format", "text"
)

# -- Shim startup failures diagnostics log (read-only tail) -----------------

Write-Host ""
Write-Host "-- shim-startup-failures.jsonl (last 20 lines) --" -ForegroundColor Yellow

$DiagnosticsLog = Join-Path $ResolvedWorkspace ".engram\diagnostics\shim-startup-failures.jsonl"

if (Test-Path -LiteralPath $DiagnosticsLog) {
    Get-Content -LiteralPath $DiagnosticsLog -Tail 20 | ForEach-Object { Write-Host "  $_" }
} else {
    Write-Host "  Absent: $DiagnosticsLog" -ForegroundColor Gray
}

# -- Summary ---------------------------------------------------------------

Write-Host ""
Write-Host "============================================================" -ForegroundColor Cyan
if ($script:HadFailure) {
    Write-Host "  One or more Engram diagnostics FAILED. See details above." -ForegroundColor Red
    Write-Host "============================================================" -ForegroundColor Cyan
    exit 1
} else {
    Write-Host "  All Engram diagnostics completed successfully." -ForegroundColor Green
    Write-Host "============================================================" -ForegroundColor Cyan
    exit 0
}
