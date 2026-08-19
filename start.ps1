# Runtime registration is handled by the setup commands.
# This script only launches Copilot CLI with workspace-local state.
#
# Auto-MergeInstall / Auto-Tune are GLOBAL agents provided by the autoharness
# marketplace plugin. They are the versions used when upgrading autoharness and
# are intentionally NOT copied into this workspace's local .copilot — a stale
# local copy would shadow the global agent during an upgrade. Upgrade them
# globally with `copilot plugin install autoharness@autoharness`; do not run
# `setup-copilot-cli` here (COPILOT_HOME is redirected to a workspace-local dir).

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
    [string]$Status,

    [Parameter(Mandatory = $true)]
    [DateTimeOffset]$Deadline
  )

  Write-Host "$Activity — $Status"

  $engramArguments = @("--format", "text")
  $engramArguments += $GlobalArguments
  $engramArguments += $Subcommand
  $engramArguments += $Arguments

  $remainingMs = [Math]::Floor(($Deadline - [DateTimeOffset]::UtcNow).TotalMilliseconds)
  if ($remainingMs -le 0) {
    throw "Engram pre-warm exceeded its shared wall-clock budget before $Subcommand could start."
  }

  $process = $null
  try {
    $processExecutable = $Executable
    $processArguments = $engramArguments
    if ([IO.Path]::GetExtension($Executable) -ieq ".ps1") {
      $pwshCommand = Get-Command pwsh -ErrorAction Stop
      $processExecutable = $pwshCommand.Source
      $processArguments = @("-NoProfile", "-NonInteractive", "-File", $Executable)
      $processArguments += $engramArguments
    }

    $process = Start-Process `
      -FilePath $processExecutable `
      -ArgumentList $processArguments `
      -NoNewWindow `
      -PassThru

    if (-not $process.WaitForExit([int]$remainingMs)) {
      try {
        # Terminate only the command process started above. A daemon-backed
        # command may auto-spawn a reusable daemon that outlives this pre-warm;
        # the launcher does not own that descendant and must not kill it.
        $process.Kill()
        $process.WaitForExit()
      }
      catch {
        Write-Warning "Timed-out Engram pre-warm process cleanup failed: $_"
      }
      throw "engram $Subcommand exceeded the shared Engram pre-warm wall-clock budget."
    }

    if ($process.ExitCode -ne 0) {
      throw "engram $Subcommand failed with exit code $($process.ExitCode)."
    }
  }
  finally {
    if ($null -ne $process) {
      $process.Dispose()
    }
  }
}

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

# Anchor workspace operations (backlogit, engram, Copilot) to the repo dir for the
# duration of the launched commands, then restore the caller's location. The current
# location is runspace state (not script-scoped), so Push/Pop in a finally keeps the
# caller's PowerShell session location unchanged even on failure.
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

  $engramCmd = Get-Command engram -ErrorAction SilentlyContinue
  if ($engramCmd) {
    $prewarmTimeoutMs = 15000
    if ($env:ENGRAM_PREWARM_TIMEOUT_MS) {
      $configuredTimeoutMs = 0
      if (
        [int]::TryParse($env:ENGRAM_PREWARM_TIMEOUT_MS, [ref]$configuredTimeoutMs) -and
        $configuredTimeoutMs -gt 0 -and
        $configuredTimeoutMs -le 30000
      ) {
        $prewarmTimeoutMs = $configuredTimeoutMs
      }
      else {
        Write-Warning "Ignoring invalid ENGRAM_PREWARM_TIMEOUT_MS; expected 1..30000 milliseconds."
      }
    }
    $prewarmDeadline = [DateTimeOffset]::UtcNow.AddMilliseconds($prewarmTimeoutMs)

    try {
      Invoke-EngramCommandWithProgress `
        -Executable $engramCmd.Source `
        -Subcommand "sync" `
        -GlobalArguments @("--timeout", "300") `
        -Arguments @("--direct") `
        -Activity "Synchronizing Engram index" `
        -Status "Direct pre-warm before Copilot startup" `
        -Deadline $prewarmDeadline
    }
    catch {
      Write-Warning "engram direct pre-warm failed; retrying via daemon sync: $_"
      try {
        Invoke-EngramCommandWithProgress `
          -Executable $engramCmd.Source `
          -Subcommand "bind" `
          -Activity "Binding Engram workspace" `
          -Status "Daemon-backed pre-warm fallback" `
          -Deadline $prewarmDeadline
        Invoke-EngramCommandWithProgress `
          -Executable $engramCmd.Source `
          -Subcommand "sync" `
          -GlobalArguments @("--timeout", "300") `
          -Activity "Synchronizing Engram index" `
          -Status "Daemon-backed pre-warm fallback" `
          -Deadline $prewarmDeadline
      }
      catch {
        Write-Warning "engram sync failed (non-fatal): $_"
      }
    }
  }

  # Forward caller arguments as-is. --remote (Copilot remote control, which streams
  # session output to GitHub and permits steering from authenticated remote devices)
  # is opt-in: it is forwarded only when the caller explicitly supplies it, and is
  # never force-enabled by the launcher.
  $copilotArguments = @($args)

  & $copilotExe @copilotArguments
}
finally {
  Pop-Location
}
