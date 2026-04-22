#!/usr/bin/env pwsh
<#
.SYNOPSIS
    Register or remove a weekly Windows Scheduled Task that prunes build artifacts.

.DESCRIPTION
    Creates a per-user scheduled task that runs the workspace cleanup script on
    a weekly schedule. The task removes `target/` and `target-*` directories and
    leaves `.copilot/`, `.engram/`, and other workspace data untouched.

.PARAMETER RepoRoot
    Repository root. Defaults to the parent of this script directory.

.PARAMETER TaskName
    Name of the Windows Scheduled Task.

.PARAMETER DayOfWeek
    Day of week for the recurring cleanup run.

.PARAMETER At
    Start time for the recurring cleanup run.

.PARAMETER Unregister
    Remove the scheduled task instead of creating or updating it.

.EXAMPLE
    .\scripts\register-build-prune-task.ps1

.EXAMPLE
    .\scripts\register-build-prune-task.ps1 -Unregister
    #>

[CmdletBinding(SupportsShouldProcess = $true, ConfirmImpact = 'Medium')]
param(
    [string]$RepoRoot = (Join-Path $PSScriptRoot ".."),
    [string]$TaskName = "agent-engram-prune-build-artifacts",
    [ValidateSet('Sunday', 'Monday', 'Tuesday', 'Wednesday', 'Thursday', 'Friday', 'Saturday')]
    [string]$DayOfWeek = 'Sunday',
    [string]$At = '3:00 AM',
    [switch]$Unregister
)

$ErrorActionPreference = "Stop"
$RepoRoot = (Resolve-Path $RepoRoot).Path
$CleanupScript = Join-Path $RepoRoot "scripts\prune-build-artifacts.ps1"

if (-not (Test-Path $CleanupScript)) {
    throw "Cleanup script not found: $CleanupScript"
}

if ($Unregister) {
    try {
        Get-ScheduledTask -TaskName $TaskName -ErrorAction Stop | Out-Null
    } catch {
        Write-Host "Scheduled task '$TaskName' is not registered." -ForegroundColor Yellow
        return
    }

    if ($PSCmdlet.ShouldProcess($TaskName, "Unregister scheduled build cleanup task")) {
        Unregister-ScheduledTask -TaskName $TaskName -Confirm:$false
        Write-Host "Removed scheduled task '$TaskName'." -ForegroundColor Green
    }
    return
}

$PowerShellExe = (Get-Command pwsh.exe -ErrorAction SilentlyContinue).Source
if (-not $PowerShellExe) {
    $PowerShellExe = (Get-Command powershell.exe -ErrorAction Stop).Source
}

$UserId = if ($env:USERDOMAIN) {
    "$($env:USERDOMAIN)\$($env:USERNAME)"
} else {
    $env:USERNAME
}

$ActionArgs = @(
    '-NoProfile',
    '-ExecutionPolicy', 'Bypass',
    '-File', ('"{0}"' -f $CleanupScript),
    '-RepoRoot', ('"{0}"' -f $RepoRoot),
    '-PurgePrimaryTarget',
    '-IgnoreAge'
) -join ' '

$Action = New-ScheduledTaskAction -Execute $PowerShellExe -Argument $ActionArgs
$Trigger = New-ScheduledTaskTrigger -Weekly -DaysOfWeek $DayOfWeek -At $At
$Settings = New-ScheduledTaskSettingsSet `
    -StartWhenAvailable `
    -RunOnlyIfIdle `
    -IdleDuration (New-TimeSpan -Minutes 10) `
    -IdleWaitTimeout (New-TimeSpan -Hours 2) `
    -ExecutionTimeLimit (New-TimeSpan -Hours 4) `
    -MultipleInstances IgnoreNew
$Principal = New-ScheduledTaskPrincipal -UserId $UserId -LogonType Interactive -RunLevel Limited
$Description = 'Weekly cleanup for agent-engram build artifacts (target/ and target-* directories only).'

if ($PSCmdlet.ShouldProcess($TaskName, "Register scheduled build cleanup task")) {
    Register-ScheduledTask `
        -TaskName $TaskName `
        -Action $Action `
        -Trigger $Trigger `
        -Principal $Principal `
        -Settings $Settings `
        -Description $Description `
        -Force | Out-Null

    Write-Host "Registered scheduled task '$TaskName'." -ForegroundColor Green
    Write-Host "Schedule : Weekly on $DayOfWeek at $At" -ForegroundColor Gray
    Write-Host "Action   : $PowerShellExe $ActionArgs" -ForegroundColor Gray
}
