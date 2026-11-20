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

if (-not ("EngramPipeCapture" -as [type])) {
  Add-Type -TypeDefinition @"
public sealed class EngramPipeCapture {
  private readonly System.Text.StringBuilder buffer = new System.Text.StringBuilder();
  private readonly System.Threading.Thread worker;
  public EngramPipeCapture(System.IO.TextReader reader) {
    worker = new System.Threading.Thread(() => {
      try {
        // One character at a time: a block Read can wait for more data while
        // already-emitted output sits in the decoder buffer.
        int next;
        while ((next = reader.Read()) >= 0) {
          lock (buffer) { buffer.Append((char)next); }
        }
      } catch (System.Exception) { }
    });
    worker.IsBackground = true;
    worker.Start();
  }
  public bool Join(int milliseconds) { return worker.Join(milliseconds); }
  public string Text { get { lock (buffer) { return buffer.ToString(); } } }
}
"@
}

function Invoke-EngramPreflight {
  # PA-2 launcher contract: `engram preflight <WORKSPACE> --timeout-ms <T>` replaces
  # the legacy sync/bind pre-warm under one budget. The exit status is authoritative;
  # every failure path throws so Copilot is never launched after a failed preflight.
  param(
    [Parameter(Mandatory = $true)]
    [string]$Executable,

    [Parameter(Mandatory = $true)]
    [string]$Workspace,

    [Parameter(Mandatory = $true)]
    [int]$BudgetMs
  )

  # Outer budget minus (2000 ms relay grace + 1000 ms margin).
  $preflightTimeoutMs = $BudgetMs - 3000
  if (-not (Test-Path -LiteralPath $Executable -PathType Leaf)) {
    throw "Engram preflight executable not found: $Executable (build it with 'cargo build')."
  }

  Write-Host "Engram preflight — verifying workspace before Copilot startup"

  $startInfo = New-Object System.Diagnostics.ProcessStartInfo
  $startInfo.FileName = $Executable
  $startInfo.UseShellExecute = $false
  $startInfo.RedirectStandardOutput = $true
  $startInfo.RedirectStandardError = $true
  $startInfo.CreateNoWindow = $true
  # One ArgumentList element per argument: paths with spaces or a trailing
  # backslash are preserved. Never build a joined argument string.
  foreach ($argument in @("preflight", $Workspace, "--timeout-ms", [string]$preflightTimeoutMs)) {
    [void]$startInfo.ArgumentList.Add($argument)
  }

  $process = $null
  $stdout = ""
  $stderr = ""
  $failure = $null
  $nonZeroExit = $false
  try {
    $process = [System.Diagnostics.Process]::Start($startInfo)
    $stdoutCapture = New-Object EngramPipeCapture -ArgumentList $process.StandardOutput
    $stderrCapture = New-Object EngramPipeCapture -ArgumentList $process.StandardError

    if (-not $process.WaitForExit($BudgetMs)) {
      try {
        # Terminate only the child this launcher started ($process, owned by
        # object identity). Descendants it spawned (for example a reusable daemon)
        # are not owned by the launcher and are never terminated.
        $process.Kill()
        $cleanupTimeoutMs = 1000
        if (-not $process.WaitForExit($cleanupTimeoutMs)) {
          Write-Host "Timed-out Engram preflight process did not exit within $cleanupTimeoutMs ms."
        }
      }
      catch {
        Write-Host "Timed-out Engram preflight process cleanup failed: $_"
      }
      $failure = "engram preflight exceeded its $BudgetMs ms budget."
    }

    # Bounded drain: an unowned descendant may hold the pipes open after exit.
    # The captures buffer incrementally, so output already emitted is kept even
    # when a descendant holds a pipe open past the shared drain deadline.
    $drainClock = [System.Diagnostics.Stopwatch]::StartNew()
    [void]$stdoutCapture.Join(2000)
    [void]$stderrCapture.Join([Math]::Max(0, 2000 - [int]$drainClock.ElapsedMilliseconds))
    $stdout = $stdoutCapture.Text
    $stderr = $stderrCapture.Text

    if ($null -eq $failure -and $process.ExitCode -ne 0) {
      $failure = "engram preflight failed with exit code $($process.ExitCode)."
      $nonZeroExit = $true
    }
    if ($null -eq $failure) {
      # Exit 0 is necessary but not sufficient: a Succeeded verdict line must be
      # present and no verdict line may report any other state.
      $succeeded = $false
      foreach ($line in ($stdout -split "`r?`n")) {
        if ($line -match '"state"\s*:\s*"([^"]*)"') {
          if ($matches[1] -eq "Succeeded") { $succeeded = $true }
          else {
            $failure = "engram preflight reported state '$($matches[1])': $line"
            break
          }
        }
      }
      if ($null -eq $failure -and -not $succeeded) {
        $failure = "engram preflight exited 0 without a Succeeded verdict line."
      }
    }
  }
  catch {
    $failure = "engram preflight could not run: $_"
  }
  finally {
    if ($null -ne $process) {
      $process.Dispose()
    }
  }

  if ($stdout) { Write-Host $stdout.TrimEnd() }
  if ($null -ne $failure) {
    if ($stderr) { [Console]::Error.WriteLine($stderr.TrimEnd()) }
    # The typed stage comes from the first non-Succeeded verdict line; when the
    # process exited non-zero, any line carrying a stage is accepted.
    $failedStage = $null
    foreach ($line in ([string]$stdout -split "`r?`n")) {
      if ($line -notmatch '"stage"\s*:\s*"([^"]*)"') { continue }
      $lineStage = $matches[1]
      $isSucceeded = ($line -match '"state"\s*:\s*"Succeeded"')
      if ((-not $isSucceeded) -or $nonZeroExit) {
        if ($lineStage) { $failedStage = $lineStage; break }
      }
    }
    if ($failedStage) {
      Write-Host "engram preflight failed at stage $failedStage."
    }
    throw "$failure Copilot was not launched."
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

  $engramExe = Join-Path $PSScriptRoot "target\debug\engram.exe"
  $preflightBudgetMs = 15000
  if ($env:ENGRAM_PREWARM_TIMEOUT_MS) {
    $configuredBudgetMs = 0
    if (
      -not [int]::TryParse($env:ENGRAM_PREWARM_TIMEOUT_MS, [ref]$configuredBudgetMs) -or
      $configuredBudgetMs -le 3000 -or
      $configuredBudgetMs -gt 30000
    ) {
      throw "Invalid ENGRAM_PREWARM_TIMEOUT_MS; expected 3001..30000 milliseconds."
    }
    $preflightBudgetMs = $configuredBudgetMs
  }
  Invoke-EngramPreflight -Executable $engramExe -Workspace $PSScriptRoot -BudgetMs $preflightBudgetMs

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
