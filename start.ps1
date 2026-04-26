$autoharness_home = (autoharness home)
$global_agents_src = "$autoharness_home\.github\agents"
$workspace_agents = ".github\agents"

# Inject global agents into .github/agents (non-destructive — skip if already present)
# This keeps workspace-visible agents in the standard repository agent directory.
if (Test-Path $global_agents_src) {
    New-Item -ItemType Directory -Path $workspace_agents -Force | Out-Null
    Get-ChildItem "$global_agents_src\*.agent.md" | ForEach-Object {
        $dest = Join-Path $workspace_agents $_.Name
        if (-not (Test-Path $dest)) { Copy-Item $_.FullName $dest }
    }
}

$env:COPILOT_HOME = if ($env:COPILOT_HOME) { $env:COPILOT_HOME } else { "" }
$env:ENGRAM_DATA_DIR = ".\.engram"   # Uncomment when the agent-engram capability pack is active
$env:GITHUB_TOKEN = (gh auth token)
$copilotExe = if ($env:COPILOT_EXE) {
    $env:COPILOT_EXE
} else {
    (Get-Command "copilot.exe" -ErrorAction SilentlyContinue).Source
}

if (-not $copilotExe) {
    throw "Unable to locate copilot.exe. Set COPILOT_EXE or add copilot.exe to PATH."
}

& $copilotExe


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
