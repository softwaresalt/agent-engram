#Requires -Version 7.0
# Runtime registration is handled by the setup commands.
# This script only launches Copilot CLI with workspace-local state.
#
# Auto-MergeInstall / Auto-Tune are GLOBAL agents provided by the autoharness
# marketplace plugin. They are the versions used when upgrading autoharness and
# are intentionally NOT copied into this workspace's local .copilot — a stale
# local copy would shadow the global agent during an upgrade. Upgrade them
# globally with `copilot plugin install autoharness@autoharness`; do not run
# `setup-copilot-cli` here (COPILOT_HOME is redirected to a workspace-local dir).

$envLocalPath = Join-Path $PSScriptRoot ".env.local"
if (Test-Path -LiteralPath $envLocalPath -PathType Leaf) {
  Get-Content -LiteralPath $envLocalPath | ForEach-Object {
    if ($_ -match '^\s*([A-Z_][A-Z0-9_]*)\s*=\s*(.*?)\s*$') {
      $name = $matches[1]
      if ($null -eq [Environment]::GetEnvironmentVariable($name, "Process")) {
        $value = $matches[2]
        if ($value.Length -ge 2) {
          $firstChar = $value[0]
          $lastChar = $value[$value.Length - 1]
          if ((($firstChar -eq '"') -or ($firstChar -eq "'")) -and ($lastChar -eq $firstChar)) {
            $value = $value.Substring(1, $value.Length - 2)
          }
        }
        [Environment]::SetEnvironmentVariable($name, $value, "Process")
      }
    }
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
    }
    catch {
      Write-Warning "gh auth token failed (non-fatal): $_"
    }
  }
}
$copilotExe = if ($env:COPILOT_EXE_PATH) {
  $env:COPILOT_EXE_PATH
}
elseif ($env:COPILOT_EXE) {
  $env:COPILOT_EXE
}
else {
  $copilotCommand = Get-Command "copilot" -ErrorAction SilentlyContinue
  if ($copilotCommand) { $copilotCommand.Source } else { $null }
}

if (-not $copilotExe) {
  throw "Unable to locate Copilot CLI. Set COPILOT_EXE_PATH (or COPILOT_EXE for backward compatibility) or add 'copilot' to PATH."
}

# Index preparation is explicit maintenance, never a prerequisite for a session.
Push-Location -LiteralPath $PSScriptRoot
try {
  $backlogitCmd = Get-Command backlogit -ErrorAction SilentlyContinue
  if ($backlogitCmd) {
    try {
      backlogit sync
      if ($LASTEXITCODE -ne 0) {
        Write-Warning "backlogit sync exited with code $LASTEXITCODE (non-fatal); index may be stale."
      }
    }
    catch {
      Write-Warning "backlogit sync failed (non-fatal): $_"
    }
  }

  # --remote remains opt-in; forward only the caller's arguments.
  $copilotArguments = @($args)
  $LASTEXITCODE = 0
  & $copilotExe @copilotArguments
  $copilotExitCode = $LASTEXITCODE
}
finally {
  Pop-Location
}
exit $copilotExitCode
