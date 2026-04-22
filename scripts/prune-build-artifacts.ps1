#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Remove generated build artifact directories from the workspace.

.DESCRIPTION
    Deletes temporary Cargo build directories that match `target-*` and,
    when requested, the primary `target/` directory. This script is intended
    for manual cleanup and scheduled maintenance.

    The script only removes build artifact directories. It does not touch
    `.copilot/`, `.engram/`, logs, or source files.

.PARAMETER RepoRoot
    Repository root to scan. Defaults to the parent of this script directory.

.PARAMETER MaxAgeDays
    Minimum directory age in days before it is eligible for deletion. Ignored
    when `-IgnoreAge` is supplied.

.PARAMETER PurgePrimaryTarget
    Include the primary `target/` directory in the cleanup candidate list.

.PARAMETER IgnoreAge
    Delete matching directories regardless of last write time.

.EXAMPLE
    .\scripts\prune-build-artifacts.ps1 -PurgePrimaryTarget -IgnoreAge -WhatIf
    #>

[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param(
    [string]$RepoRoot = (Join-Path $PSScriptRoot ".."),
    [int]$MaxAgeDays = 7,
    [switch]$PurgePrimaryTarget,
    [switch]$IgnoreAge
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path $RepoRoot).Path
$Cutoff = (Get-Date).AddDays(-$MaxAgeDays)
$Candidates = [System.Collections.Generic.List[System.IO.DirectoryInfo]]::new()

if ($PurgePrimaryTarget) {
    $PrimaryTarget = Join-Path $RepoRoot "target"
    if (Test-Path $PrimaryTarget) {
        $Candidates.Add((Get-Item $PrimaryTarget))
    }
}

Get-ChildItem -Path $RepoRoot -Directory -Force |
    Where-Object { $_.Name -like 'target-*' } |
    ForEach-Object { $Candidates.Add($_) }

$Targets = $Candidates | Sort-Object FullName -Unique

if (-not $Targets) {
    Write-Host "No build artifact directories matched the cleanup rules." -ForegroundColor Yellow
    return
}

$EligibleCount = 0
$DeletedCount = 0

foreach ($Target in $Targets) {
    $IsEligible = $IgnoreAge -or $Target.LastWriteTime -lt $Cutoff
    if (-not $IsEligible) {
        Write-Host "Skipping recent directory: $($Target.FullName)" -ForegroundColor DarkYellow
        continue
    }

    $EligibleCount++
    if ($PSCmdlet.ShouldProcess($Target.FullName, "Remove build artifact directory")) {
        Remove-Item -LiteralPath $Target.FullName -Recurse -Force
        $DeletedCount++
        Write-Host "Removed: $($Target.FullName)" -ForegroundColor Green
    }
}

if ($IgnoreAge) {
    Write-Host "Age filter disabled; all matching build directories were eligible." -ForegroundColor Gray
} else {
    Write-Host "Age cutoff: directories older than $MaxAgeDays day(s) were eligible." -ForegroundColor Gray
}

Write-Host "Eligible directories: $EligibleCount" -ForegroundColor Gray
Write-Host "Deleted directories : $DeletedCount" -ForegroundColor Gray
