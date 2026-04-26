$autoharness_home = (autoharness home)
$global_agents_src = "$autoharness_home\.github\agents"
$local_agents = ".github\local-agents"
$local_copilot = ".\.copilot"

# Inject global agents into .github/local-agents (non-destructive — skip if already present).
# This keeps generated copies out of the tracked .github/agents directory.
if (Test-Path $global_agents_src) {
    New-Item -ItemType Directory -Path $local_agents -Force | Out-Null
    Get-ChildItem "$global_agents_src\*.agent.md" | ForEach-Object {
        $dest = Join-Path $local_agents $_.Name
        if (-not (Test-Path $dest)) { Copy-Item $_.FullName $dest }
    }
}

if (-not $env:COPILOT_HOME) {
    $env:COPILOT_HOME = $local_copilot
}
$env:ENGRAM_DATA_DIR = ".\.engram"   # Uncomment when the agent-engram capability pack is active
$env:GITHUB_TOKEN = (gh auth token)
$copilotExe = if ($env:COPILOT_EXE) {
    $env:COPILOT_EXE
} else {
    $cmd = Get-Command "copilot" -ErrorAction SilentlyContinue
    if ($cmd) { $cmd.Source }
    else { (Get-Command "copilot.exe" -ErrorAction SilentlyContinue).Source }
}

if (-not $copilotExe) {
    throw "Unable to locate copilot or copilot.exe. Set COPILOT_EXE or add Copilot to PATH."
}

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
