#$autoharness_home = (autoharness home)
#$global_agents_src = "$autoharness_home\.github\agents"
#$local_agents = ".github\agents"

# Copies global autoharness agents into .github/agents (tracked; workspace-discoverable).
# To keep them gitignored instead, change this to ".github\local-agents" and update
# chat.agentFilesLocations in the workspace settings file to include that path.
#if (Test-Path $global_agents_src) {
#    Get-ChildItem "$global_agents_src\*.agent.md" | ForEach-Object {
#        $dest = Join-Path $local_agents $_.Name
#        $sourceFile = $_
#        $shouldCopy = -not (Test-Path $dest)
#
#        if (-not $shouldCopy) {
#            $destFile = Get-Item $dest
#            $shouldCopy = $sourceFile.LastWriteTimeUtc -gt $destFile.LastWriteTimeUtc
#        }
#
#        if ($shouldCopy) { Copy-Item $sourceFile.FullName $dest }
#    }
#}

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

    Write-Host "$Activity — $Status"

    $engramArguments = @("--format", "text")
    $engramArguments += $GlobalArguments
    $engramArguments += $Subcommand
    $engramArguments += $Arguments

    & $Executable @engramArguments
    $exitCode = $LASTEXITCODE

    if ($exitCode -ne 0) {
        throw "engram $Subcommand failed with exit code $exitCode."
    }
}

if (Test-Path -LiteralPath .env.local) {
  Get-Content -LiteralPath .env.local | ForEach-Object {
    if ($_ -match '^\s*([A-Z_][A-Z0-9_]*)\s*=\s*(.+?)\s*$') {
      Set-Item -Path "env:$($matches[1])" -Value $matches[2]
    }
  }
}

$env:GITHUB_PERSONAL_ACCESS_TOKEN = (gh auth token)
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

# Pre-warm the Engram code graph before launching Copilot. Direct mode
# (engram sync --direct) runs indexing in this process and leaves no daemon
# behind; the shim spawns the daemon later on the first MCP call. Direct mode is
# also the daemon-startup / IPC-timeout escape hatch.
# Reference: docs/configuration.md (Daemonless direct indexing).
$engramCmd = Get-Command engram -ErrorAction SilentlyContinue
if ($engramCmd) {
    try {
        Invoke-EngramCommandWithProgress `
            -Executable $engramCmd.Source `
            -Subcommand "sync" `
            -GlobalArguments @("--timeout", "3000") `
            -Arguments @("--direct") `
            -Activity "Synchronizing Engram index" `
            -Status "Direct pre-warm before Copilot startup"
    } catch {
        Write-Warning "engram direct pre-warm failed; retrying via daemon sync: $_"
        try {
            & $engramCmd.Source --format text bind
            Invoke-EngramCommandWithProgress `
                -Executable $engramCmd.Source `
                -Subcommand "sync" `
                -GlobalArguments @("--timeout", "3000") `
                -Activity "Synchronizing Engram index" `
                -Status "Daemon-backed pre-warm fallback"
        } catch {
            Write-Warning "engram sync failed (non-fatal): $_"
        }
    }
}

$copilotArguments = @()
if (-not ($args -contains "--yolo")) {
    $copilotArguments += "--yolo"
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
