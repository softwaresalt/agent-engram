#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Run the engram daemon locally against the current repository workspace.

.DESCRIPTION
    Builds the engram release binary, spawns the daemon with debug-level
    structured logging, waits for IPC readiness, and streams daemon log
    output to the terminal. Press Ctrl-C to stop — the daemon is killed
    and the lockfile is cleaned up automatically.

.PARAMETER Workspace
    Path to the Git workspace to bind. Defaults to the repository root
    (the directory containing this script's parent).

.PARAMETER LogLevel
    RUST_LOG filter string. Defaults to "engram=debug" for full daemon
    tracing. Use "engram=info" for quieter output or "engram=trace" for
    maximum verbosity.

.PARAMETER SkipBuild
    Skip the cargo build step (use an existing binary in target/release/).

.EXAMPLE
    .\scripts\run-local.ps1
    .\scripts\run-local.ps1 -LogLevel "engram=trace"
    .\scripts\run-local.ps1 -SkipBuild
#>

[CmdletBinding()]
param(
    [string]$Workspace,
    [string]$LogLevel = "engram=debug",
    [switch]$SkipBuild
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path (Join-Path $PSScriptRoot "..")).Path
$Binary = Join-Path $RepoRoot "target\release\engram.exe"

# ── Resolve workspace ──────────────────────────────────────────────────────────

if (-not $Workspace) {
    $Workspace = $RepoRoot
}

$Workspace = (Resolve-Path $Workspace).Path

if (-not (Test-Path (Join-Path $Workspace ".git"))) {
    Write-Error "Not a Git workspace: $Workspace (missing .git directory)"
    exit 1
}

Write-Host "╔══════════════════════════════════════════════════════════╗" -ForegroundColor Cyan
Write-Host "║           engram daemon — local runner                  ║" -ForegroundColor Cyan
Write-Host "╚══════════════════════════════════════════════════════════╝" -ForegroundColor Cyan
Write-Host ""
Write-Host "  Workspace : $Workspace" -ForegroundColor Gray
Write-Host "  Log level : $LogLevel" -ForegroundColor Gray
Write-Host ""

# ── Build ──────────────────────────────────────────────────────────────────────

if (-not $SkipBuild) {
    Write-Host "[1/3] Building release binary..." -ForegroundColor Yellow
    Push-Location $RepoRoot
    try {
        cargo build --release 2>&1 | ForEach-Object { Write-Host "  $_" -ForegroundColor DarkGray }
        if ($LASTEXITCODE -ne 0) {
            Write-Error "cargo build failed (exit code $LASTEXITCODE)"
            exit 1
        }
    }
    finally {
        Pop-Location
    }
    Write-Host "  Build complete: $Binary" -ForegroundColor Green
} else {
    if (-not (Test-Path $Binary)) {
        Write-Error "Binary not found at $Binary — run without -SkipBuild first"
        exit 1
    }
    Write-Host "[1/3] Skipping build (using existing binary)" -ForegroundColor DarkYellow
}

Write-Host ""

# ── Spawn daemon ───────────────────────────────────────────────────────────────

Write-Host "[2/3] Spawning daemon..." -ForegroundColor Yellow

$env:RUST_LOG = $LogLevel
$env:ENGRAM_LOG_FORMAT = "pretty"

$DaemonProcess = Start-Process -FilePath $Binary `
    -ArgumentList "daemon", "--workspace", $Workspace `
    -NoNewWindow `
    -PassThru `
    -RedirectStandardError (Join-Path $Workspace ".engram\logs\daemon-stderr.log")

$DaemonPid = $DaemonProcess.Id
Write-Host "  Daemon PID: $DaemonPid" -ForegroundColor Green

# ── Wait for IPC readiness ─────────────────────────────────────────────────────

Write-Host "[3/3] Waiting for IPC readiness..." -ForegroundColor Yellow

# Compute the named-pipe endpoint (matches daemon's SHA-256 prefix logic)
$Sha256 = [System.Security.Cryptography.SHA256]::Create()
$HashBytes = $Sha256.ComputeHash([System.Text.Encoding]::UTF8.GetBytes($Workspace))
$Prefix = ($HashBytes[0..7] | ForEach-Object { $_.ToString("x2") }) -join ""
$PipeName = "\\.\pipe\engram-$Prefix"

Write-Host "  IPC endpoint: $PipeName" -ForegroundColor Gray

$MaxAttempts = 30
$Delay = 100  # ms
$Ready = $false

for ($i = 1; $i -le $MaxAttempts; $i++) {
    Start-Sleep -Milliseconds $Delay

    # Test named pipe existence
    try {
        $PipeExists = Test-Path $PipeName -ErrorAction SilentlyContinue
        if ($PipeExists) {
            $Ready = $true
            break
        }
    } catch {
        # Pipe not ready yet
    }

    $Delay = [Math]::Min($Delay * 2, 2000)
}

if (-not $Ready) {
    Write-Host "  WARNING: Could not confirm IPC readiness after $MaxAttempts attempts" -ForegroundColor Red
    Write-Host "  The daemon may still be starting — check logs below" -ForegroundColor Red
} else {
    Write-Host "  Daemon ready! (attempt $i)" -ForegroundColor Green
}

Write-Host ""
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host "  Daemon is running. Press Ctrl-C to stop."               -ForegroundColor Cyan
Write-Host "  Log file: $Workspace\.engram\logs\daemon-stderr.log"    -ForegroundColor Gray
Write-Host "════════════════════════════════════════════════════════════" -ForegroundColor Cyan
Write-Host ""

# ── Tail logs until Ctrl-C ─────────────────────────────────────────────────────

$LogFile = Join-Path $Workspace ".engram\logs\daemon-stderr.log"

try {
    # Wait briefly for log file to appear
    $LogWait = 0
    while (-not (Test-Path $LogFile) -and $LogWait -lt 5) {
        Start-Sleep -Seconds 1
        $LogWait++
    }

    if (Test-Path $LogFile) {
        # Tail the log file, following new content
        Get-Content -Path $LogFile -Wait -Tail 50
    } else {
        Write-Host "Log file not found — daemon may write to stderr directly." -ForegroundColor Yellow
        Write-Host "Waiting for daemon to exit..." -ForegroundColor Gray
        $DaemonProcess.WaitForExit()
    }
}
finally {
    # ── Cleanup ────────────────────────────────────────────────────────────────
    Write-Host ""
    Write-Host "Stopping daemon (PID $DaemonPid)..." -ForegroundColor Yellow

    if (-not $DaemonProcess.HasExited) {
        Stop-Process -Id $DaemonPid -Force -ErrorAction SilentlyContinue
        $DaemonProcess.WaitForExit(5000) | Out-Null
    }

    # Clean up lockfile if it exists
    $LockFile = Join-Path $Workspace ".engram\run\daemon.lock"
    if (Test-Path $LockFile) {
        Remove-Item $LockFile -Force -ErrorAction SilentlyContinue
        Write-Host "  Removed stale lockfile" -ForegroundColor Gray
    }

    Write-Host "  Daemon stopped." -ForegroundColor Green
}
