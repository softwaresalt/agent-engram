#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Read-only Engram customer-box diagnostics report.

.DESCRIPTION
    Resolves the target workspace, then reports:
      - the resolved engram executable
      - engram version
      - a no-auto-spawn daemon presence probe (reads .engram\run\engram.pid
        directly; never shells out to `engram` to make this determination)
      - daemon status, workspace status, health, and a live unified search
        for "workspace binding" -- ONLY when the presence probe found a
        running daemon, since each of these commands would otherwise
        auto-spawn one (see the "read-only contract" note below)
      - the last 20 lines of .engram\diagnostics\shim-startup-failures.jsonl
        (or a note that the file is absent)

    All engram subcommands are invoked with an explicit --workspace path and
    --format text. This script is diagnostic-only: it makes no mutating calls
    (no bind, sync, index, flush, install, etc.). Every CLI parity command
    routes through `run_tool`, which auto-spawns the daemon when it is not
    already running (src/cli/runner.rs `ensure_daemon_running`) -- an
    auto-spawn can sync/watch and mutate workspace state, so this script
    determines daemon presence itself (via the PID file) before invoking any
    daemon-backed command, and skips those commands entirely when no daemon
    is running rather than starting one. It does not hide command failures --
    every diagnostic runs even if an earlier one failed, so the operator
    always gets a full report. The script exits nonzero if any engram
    diagnostic command failed, or if a daemon-backed diagnostic was skipped
    because no daemon was running.

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

# ── No-auto-spawn daemon presence probe ─────────────────────────────────────
#
# Every CLI parity command (daemon-status, workspace-status, health, search)
# routes through `run_tool`, which calls `ensure_daemon_running` and
# auto-spawns the daemon when it is not already running (src/cli/runner.rs).
# On a live managed daemon, that startup can sync/watch and mutate workspace
# state -- contradicting this script's read-only, customer-box contract. To
# stay non-mutating, daemon presence must be determined WITHOUT invoking the
# `engram` CLI at all.
#
# `.engram\run\engram.pid` is the same PID-file record the daemon lock
# maintains (src/daemon/lockfile.rs) and the shim's own liveness check reads
# (src/shim/pidfile.rs) to decide whether an existing daemon can be reused
# instead of spawned. Reading it directly -- and verifying the recorded PID
# still refers to a live process -- gives an accurate presence signal without
# ever shelling out to `engram`, so it cannot trigger auto-spawn.
function Test-EngramDaemonRunning {
    param(
        [Parameter(Mandatory)][string]$Workspace
    )

    $PidFilePath = Join-Path $Workspace ".engram\run\engram.pid"

    if (-not (Test-Path -LiteralPath $PidFilePath -PathType Leaf)) {
        return $false
    }

    # Any failure to read or parse the PID file is treated as "no daemon" --
    # never as "daemon running" -- so a probe failure can only ever cause the
    # (safe) skip path below, not a false claim that probes were run.
    try {
        $Raw = (Get-Content -LiteralPath $PidFilePath -Raw -ErrorAction Stop).Trim()
    } catch {
        return $false
    }

    if ([string]::IsNullOrWhiteSpace($Raw)) {
        return $false
    }

    $RecordedPid = 0
    $Parsed = $null
    try {
        $Parsed = $Raw | ConvertFrom-Json -ErrorAction Stop
    } catch {
        $Parsed = $null
    }

    if ($null -ne $Parsed -and (Get-Member -InputObject $Parsed -Name "pid" -ErrorAction SilentlyContinue)) {
        if (-not [int]::TryParse([string]$Parsed.pid, [ref]$RecordedPid)) {
            # Structured JSON present but "pid" is non-integer or out of range --
            # malformed metadata takes the safe skip path rather than throwing.
            return $false
        }
    } elseif (-not [int]::TryParse($Raw, [ref]$RecordedPid)) {
        # Neither structured JSON nor the legacy bare-numeric PID file format.
        return $false
    }

    if ($RecordedPid -le 0) {
        return $false
    }

    return $null -ne (Get-Process -Id $RecordedPid -ErrorAction SilentlyContinue)
}

$DaemonRunning = Test-EngramDaemonRunning -Workspace $ResolvedWorkspace

Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Engram customer-box diagnostics" -ForegroundColor Cyan
Write-Host "============================================================" -ForegroundColor Cyan
Write-Host "  Workspace : $ResolvedWorkspace" -ForegroundColor Gray
Write-Host "  Engram    : $(if ($EngramCmd) { $EngramCmd.Source } else { '<not found on PATH>' })" -ForegroundColor Gray
Write-Host "  Daemon    : $(if ($DaemonRunning) { 'running (no-auto-spawn probe found a live PID)' } else { 'not running (no-auto-spawn probe found no live PID)' })" -ForegroundColor Gray

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

    if (-not $DaemonRunning) {
        Write-Host "  SKIPPED: no running daemon detected for this workspace (no-auto-spawn probe found no live PID in .engram\run\engram.pid)." -ForegroundColor Red
        Write-Host "  This diagnostic is read-only and will not start the daemon. Start it normally (e.g. via an MCP client), then re-run this script." -ForegroundColor Red
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
    # With $ErrorActionPreference = "Stop", an unreadable or concurrently
    # removed log (e.g. permission denied, or a race with log rotation) would
    # otherwise raise a terminating error here and abort the script before
    # the summary below ever prints -- contradicting the "full report even on
    # partial failure" contract described at the top of this script. Catch
    # only this tail-read failure, report it explicitly, and fall through to
    # the summary like every other diagnostic failure in this script.
    try {
        Get-Content -LiteralPath $DiagnosticsLog -Tail 20 -ErrorAction Stop | ForEach-Object { Write-Host "  $_" }
    } catch {
        Write-Host "  FAILED to read $DiagnosticsLog`: $($_.Exception.Message)" -ForegroundColor Red
        $script:HadFailure = $true
    }
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
