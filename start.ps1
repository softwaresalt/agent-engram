$autoharness_home = (autoharness home)
$global_agents_src = "$autoharness_home\.github\agents"
$local_agents = ".github\agents"

# Copies global autoharness agents into .github/agents (tracked; workspace-discoverable).
# To keep them gitignored instead, change this to ".github\local-agents" and update
# chat.agentFilesLocations in the workspace settings file to include that path.
if (Test-Path $global_agents_src) {
    Get-ChildItem "$global_agents_src\*.agent.md" | ForEach-Object {
        $dest = Join-Path $local_agents $_.Name
        $sourceFile = $_
        $shouldCopy = -not (Test-Path $dest)

        if (-not $shouldCopy) {
            $destFile = Get-Item $dest
            $shouldCopy = $sourceFile.LastWriteTimeUtc -gt $destFile.LastWriteTimeUtc
        }

        if ($shouldCopy) { Copy-Item $sourceFile.FullName $dest }
    }
}

$env:COPILOT_HOME = if ($env:COPILOT_HOME) { $env:COPILOT_HOME } else { Join-Path $PSScriptRoot ".copilot" }
$env:ENGRAM_DATA_DIR = if ($env:ENGRAM_DATA_DIR) { $env:ENGRAM_DATA_DIR } else { Join-Path $PSScriptRoot ".engram" }
$env:GITHUB_TOKEN = (gh auth token)
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
backlogit sync index

& $copilotExe @args


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
