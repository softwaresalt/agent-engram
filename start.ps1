$autoharnessCmd = Get-Command autoharness -ErrorAction SilentlyContinue
$autoharness_home = $null
if ($autoharnessCmd) {
    try {
        $autoharness_home = (& $autoharnessCmd.Source home).Trim()
    } catch {
        Write-Warning "autoharness home lookup failed (non-fatal): $_"
    }
}

$global_agents_src = if ($autoharness_home) {
    Join-Path $autoharness_home ".github\agents"
} else {
    $null
}
$local_agents = ".github\agents"

# Copies global autoharness agents into .github/agents (tracked; workspace-discoverable).
# To keep them gitignored instead, change this to ".github\local-agents" and update
# chat.agentFilesLocations in the workspace settings file to include that path.
if ($global_agents_src -and (Test-Path $global_agents_src)) {
    Get-ChildItem "$global_agents_src\*.agent.md" | ForEach-Object {
        $dest = Join-Path $local_agents $_.Name
        $sourceFile = $_
        $shouldCopy = -not (Test-Path $dest)

        if (-not $shouldCopy) {
            $destFile = Get-Item $dest
            $sourceHash = (Get-FileHash -LiteralPath $sourceFile.FullName -Algorithm SHA256).Hash
            $destHash = (Get-FileHash -LiteralPath $destFile.FullName -Algorithm SHA256).Hash
            $shouldCopy = $sourceHash -ne $destHash -and $sourceFile.LastWriteTimeUtc -gt $destFile.LastWriteTimeUtc

            if ($shouldCopy) {
                Write-Warning "Overwriting local agent '$dest' with newer autoharness copy."
            }
        }

        if ($shouldCopy) { Copy-Item $sourceFile.FullName $dest -Force }
    }
}

function Invoke-EngramCommandWithProgress {
    param(
        [Parameter(Mandatory = $true)]
        [string]$Executable,

        [Parameter(Mandatory = $true)]
        [string]$Subcommand,

        [string[]]$GlobalArguments = @(),

        [string[]]$Arguments = @(),

        [Parameter(Mandatory = $true)]
        [string]$Activity,

        [Parameter(Mandatory = $true)]
        [string]$Status
    )

    $startInfo = [System.Diagnostics.ProcessStartInfo]::new()
    $startInfo.FileName = $Executable
    $startInfo.UseShellExecute = $false
    $startInfo.RedirectStandardOutput = $true
    $startInfo.RedirectStandardError = $true
    $startInfo.CreateNoWindow = $true

    [void]$startInfo.ArgumentList.Add("--format")
    [void]$startInfo.ArgumentList.Add("text")

    foreach ($argument in $GlobalArguments) {
        [void]$startInfo.ArgumentList.Add($argument)
    }

    [void]$startInfo.ArgumentList.Add($Subcommand)

    foreach ($argument in $Arguments) {
        [void]$startInfo.ArgumentList.Add($argument)
    }

    $process = [System.Diagnostics.Process]::new()
    $process.StartInfo = $startInfo

    if (-not $process.Start()) {
        throw "Failed to start engram $Subcommand."
    }

    $stdoutTask = $process.StandardOutput.ReadToEndAsync()
    $stderrTask = $process.StandardError.ReadToEndAsync()
    $startedAt = Get-Date
    $percentComplete = 0

    while (-not $process.WaitForExit(250)) {
        $percentComplete = ($percentComplete + 4) % 100
        $elapsedSeconds = [math]::Floor(((Get-Date) - $startedAt).TotalSeconds)
        Write-Progress -Id 1 -Activity $Activity -Status "$Status ($elapsedSeconds s elapsed)" -PercentComplete $percentComplete
    }

    $process.WaitForExit()
    Write-Progress -Id 1 -Activity $Activity -Completed

    $stdout = $stdoutTask.GetAwaiter().GetResult().TrimEnd()
    $stderr = $stderrTask.GetAwaiter().GetResult().TrimEnd()

    if (-not [string]::IsNullOrWhiteSpace($stdout)) {
        Write-Host $stdout
    }

    if ($process.ExitCode -ne 0) {
        if (-not [string]::IsNullOrWhiteSpace($stderr)) {
            throw $stderr
        }

        if (-not [string]::IsNullOrWhiteSpace($stdout)) {
            throw $stdout
        }

        throw "engram $Subcommand failed with exit code $($process.ExitCode)."
    }

    if (-not [string]::IsNullOrWhiteSpace($stderr)) {
        Write-Warning $stderr
    }
}

$env:COPILOT_HOME = if ($env:COPILOT_HOME) { $env:COPILOT_HOME } else { Join-Path $PSScriptRoot ".copilot" }
$env:ENGRAM_DATA_DIR = if ($env:ENGRAM_DATA_DIR) { $env:ENGRAM_DATA_DIR } else { Join-Path $PSScriptRoot ".engram" }
if (-not $env:GITHUB_TOKEN) {
    $ghCmd = Get-Command gh -ErrorAction SilentlyContinue
    if ($ghCmd) {
        try {
            $ghToken = (& $ghCmd.Source auth token 2>$null).Trim()
            if ($ghToken) {
                $env:GITHUB_TOKEN = $ghToken
            }
        } catch {
            Write-Warning "gh auth token failed (non-fatal): $_"
        }
    }
}
$copilotExe = if ($env:COPILOT_EXE_PATH) {
    $env:COPILOT_EXE_PATH
} elseif ($env:COPILOT_EXE) {
    $env:COPILOT_EXE
} else {
    $copilotCommand = Get-Command "copilot" -ErrorAction SilentlyContinue
    if ($copilotCommand) { $copilotCommand.Source } else { $null }
}

if (-not $copilotExe) {
    throw "Unable to locate Copilot CLI. Set COPILOT_EXE_PATH (or COPILOT_EXE for backward compatibility) or add 'copilot' to PATH."
}

$backlogitCmd = Get-Command backlogit -ErrorAction SilentlyContinue
if ($backlogitCmd) {
    try {
        backlogit sync
    } catch {
        Write-Warning "backlogit sync failed (non-fatal): $_"
    }
}

$engramCmd = Get-Command engram -ErrorAction SilentlyContinue
if ($engramCmd) {
    try {
        & $engramCmd.Source --format text bind
        # Prefer sync for fast steady-state startup; on a fresh branch-local DB it
        # falls back to a full index, so give it the longer indexing timeout.
        Invoke-EngramCommandWithProgress `
            -Executable $engramCmd.Source `
            -Subcommand "sync" `
            -GlobalArguments @("--timeout", "300") `
            -Activity "Synchronizing Engram index" `
            -Status "Syncing branch-local code graph"
    } catch {
        Write-Warning "engram sync failed (non-fatal): $_"
    }
}

$copilotArguments = @()
if (-not ($args -contains "--remote")) {
    $copilotArguments += "--remote"
}
$copilotArguments += $args

& $copilotExe @copilotArguments


# ── Claude Code ─────────────────────────────────────────────────────────────
# Uncomment to run Claude Code with workspace-local state directories.
# CLAUDE_CONFIG_DIR redirects Claude's config and history to the workspace.
# Verify that your installed version of Claude Code supports this env variable.
#
# $env:CLAUDE_CONFIG_DIR = ".\.claude"
# claude

# ── OpenAI Codex / Agents ────────────────────────────────────────────────────
# Uncomment to run Codex with a workspace-local API key file.
#
# $env:OPENAI_API_KEY = (Get-Content .openai-token -Raw).Trim()
# codex
