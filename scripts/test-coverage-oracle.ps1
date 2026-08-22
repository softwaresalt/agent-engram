#!/usr/bin/env pwsh
# Canonical dev-test coverage oracle (Feature 126-F, stash C2413934).
#
# Computes the required test-target set for a diff from the declared manifest
# (.cargo/test-coverage-manifest.toml), compares it against the selected set,
# and reports required / selected / omitted. The gate passes only when
# `omitted == 0` and no source surface is unmapped. Runnable standalone so a
# reviewer can reproduce the numbers without running the suite.
#
# Modes:
#   report        required vs selected; FAIL if omitted > 0 or a src surface is unmapped
#   select        print the required target names (TARGET=<name>) for the diff
#   completeness  FAIL if any [[test]] target is unmapped by a src/crates surface,
#                 or any top-level module under src/ is not a declared surface
#   run           execute the required set in concurrency-bounded batches (U4);
#                 --dry-run reports the bounded plan without spawning cargo
#
# CLI (identical flags to scripts/test-coverage-oracle.sh):
#   --mode <report|select|completeness|run>
#   --changed <comma-separated paths>
#   --selected <comma-separated target names>   (report mode; default = required)
#   --dry-run                                    (run mode)
#   --repo-root <path>  --manifest <path>  --cargo-toml <path>

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

# ── Argument parsing (manual, to mirror the shell script exactly) ────────────
$Mode = 'report'
$ChangedRaw = ''
$SelectedRaw = ''
$SelectedProvided = $false
$DryRun = $false
$RepoRoot = ''
$ManifestPath = ''
$CargoTomlPath = ''

for ($i = 0; $i -lt $args.Count; $i++) {
    switch ($args[$i]) {
        '--mode' { $i++; $Mode = [string]$args[$i] }
        '--changed' { $i++; $ChangedRaw = [string]$args[$i] }
        '--selected' { $i++; $SelectedRaw = [string]$args[$i]; $SelectedProvided = $true }
        '--dry-run' { $DryRun = $true }
        '--repo-root' { $i++; $RepoRoot = [string]$args[$i] }
        '--manifest' { $i++; $ManifestPath = [string]$args[$i] }
        '--cargo-toml' { $i++; $CargoTomlPath = [string]$args[$i] }
        default { Write-Error "unknown argument: $($args[$i])"; exit 2 }
    }
}

if ([string]::IsNullOrEmpty($RepoRoot)) {
    $RepoRoot = Split-Path -Parent (Split-Path -Parent $PSCommandPath)
}
if ([string]::IsNullOrEmpty($ManifestPath)) {
    $ManifestPath = Join-Path $RepoRoot '.cargo/test-coverage-manifest.toml'
}
if ([string]::IsNullOrEmpty($CargoTomlPath)) {
    $CargoTomlPath = Join-Path $RepoRoot 'Cargo.toml'
}

function Normalize-Path([string]$p) { return ($p -replace '\\', '/').Trim() }

# ── Load declared [[test]] targets (name -> path) ────────────────────────────
$cargoText = (Get-Content -Raw -Path $CargoTomlPath) -replace "`r`n", "`n"
$targetNames = New-Object System.Collections.Generic.List[string]
$targetPathByName = @{}
$rx = [regex]'(?ms)^\[\[test\]\]\s*\n\s*name\s*=\s*"([^"]+)"\s*\n\s*path\s*=\s*"([^"]+)"'
foreach ($m in $rx.Matches($cargoText)) {
    $n = $m.Groups[1].Value
    $targetNames.Add($n) | Out-Null
    $targetPathByName[$n] = (Normalize-Path $m.Groups[2].Value)
}

# ── Load manifest settings and surfaces ──────────────────────────────────────
$manifestText = (Get-Content -Raw -Path $ManifestPath) -replace "`r`n", "`n"

$maxConcurrent = 8
$testThreads = 4
$mSet = [regex]::Match($manifestText, '(?m)^\s*max_concurrent_test_binaries\s*=\s*(\d+)')
if ($mSet.Success) { $maxConcurrent = [int]$mSet.Groups[1].Value }
$tSet = [regex]::Match($manifestText, '(?m)^\s*test_threads\s*=\s*(\d+)')
if ($tSet.Success) { $testThreads = [int]$tSet.Groups[1].Value }
# Environment overrides (set by cargo [env] when invoked via `cargo dev-test`).
$envMax = 0
if ($env:ENGRAM_DEVTEST_MAX_BINARIES -and [int]::TryParse($env:ENGRAM_DEVTEST_MAX_BINARIES, [ref]$envMax)) { $maxConcurrent = $envMax }
$envThreads = 0
if ($env:ENGRAM_DEVTEST_TEST_THREADS -and [int]::TryParse($env:ENGRAM_DEVTEST_TEST_THREADS, [ref]$envThreads)) { $testThreads = $envThreads }

$surfaces = New-Object System.Collections.Generic.List[object]
$blockRx = [regex]'(?ms)^\[\[surface\]\]\s*\n(.*?)(?=^\[\[|\Z)'
foreach ($b in $blockRx.Matches($manifestText)) {
    $body = $b.Groups[1].Value
    $pm = [regex]::Match($body, '(?m)^\s*path\s*=\s*"([^"]+)"')
    if (-not $pm.Success) { continue }
    $spath = Normalize-Path $pm.Groups[1].Value
    $tm = [regex]::Match($body, '(?ms)targets\s*=\s*\[(.*?)\]')
    $globs = New-Object System.Collections.Generic.List[string]
    if ($tm.Success) {
        foreach ($qm in [regex]::Matches($tm.Groups[1].Value, '"([^"]+)"')) {
            $globs.Add($qm.Groups[1].Value) | Out-Null
        }
    }
    $surfaces.Add([pscustomobject]@{ Path = $spath; Globs = $globs }) | Out-Null
}

function Expand-Glob([string]$glob) {
    $out = New-Object System.Collections.Generic.List[string]
    if ($glob -eq '*') {
        foreach ($n in $targetNames) { $out.Add($n) | Out-Null }
    }
    elseif ($glob.EndsWith('*')) {
        $prefix = $glob.Substring(0, $glob.Length - 1)
        foreach ($n in $targetNames) { if ($n.StartsWith($prefix)) { $out.Add($n) | Out-Null } }
    }
    else {
        if ($targetNames.Contains($glob)) { $out.Add($glob) | Out-Null }
    }
    return $out
}

# ── Resolve required targets + unmapped source surfaces for a changed set ────
function Resolve-Diff([string[]]$changed) {
    $required = New-Object System.Collections.Generic.HashSet[string]
    $unmapped = New-Object System.Collections.Generic.List[string]
    foreach ($rawf in $changed) {
        $f = Normalize-Path $rawf
        if ([string]::IsNullOrEmpty($f)) { continue }
        $matched = $false
        # Self-coverage: a changed declared test file requires its own target.
        foreach ($n in $targetNames) {
            if ($targetPathByName[$n] -eq $f) { [void]$required.Add($n); $matched = $true }
        }
        # Additive surface matching (prefix).
        foreach ($s in $surfaces) {
            if ($f -eq $s.Path -or $f.StartsWith($s.Path)) {
                foreach ($g in $s.Globs) { foreach ($t in (Expand-Glob $g)) { [void]$required.Add($t) } }
                $matched = $true
            }
        }
        if (-not $matched) {
            if ($f.StartsWith('src/')) { $unmapped.Add($f) | Out-Null }
        }
    }
    return [pscustomobject]@{ Required = $required; Unmapped = $unmapped }
}

function Get-GitChanged() {
    $set = New-Object System.Collections.Generic.HashSet[string]
    foreach ($cmd in @(@('diff', '--name-only'), @('diff', '--name-only', '--cached'), @('diff', '--name-only', 'origin/main...HEAD'))) {
        try {
            $out = & git -C $RepoRoot @cmd 2>$null
            foreach ($l in $out) { if (-not [string]::IsNullOrWhiteSpace($l)) { [void]$set.Add((Normalize-Path $l)) } }
        }
        catch { }
    }
    return @($set)
}

# ── Compute the changed set ──────────────────────────────────────────────────
if ([string]::IsNullOrEmpty($ChangedRaw)) {
    if ($Mode -eq 'completeness') { $changedFiles = @() }
    else { $changedFiles = Get-GitChanged }
}
else {
    $changedFiles = $ChangedRaw.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' }
}

switch ($Mode) {
    'completeness' {
        # Target coverage against src/ and crates/ surfaces only.
        $srcSurfaces = @($surfaces | Where-Object { $_.Path.StartsWith('src/') -or $_.Path.StartsWith('crates/') })
        $mappedTargets = New-Object System.Collections.Generic.HashSet[string]
        foreach ($s in $srcSurfaces) { foreach ($g in $s.Globs) { foreach ($t in (Expand-Glob $g)) { [void]$mappedTargets.Add($t) } } }
        $unmappedTargets = @($targetNames | Where-Object { -not $mappedTargets.Contains($_) } | Sort-Object)

        # Module coverage: every top-level entry under src/ must be a declared surface.
        $srcDir = Join-Path $RepoRoot 'src'
        $modules = New-Object System.Collections.Generic.List[string]
        foreach ($e in Get-ChildItem -LiteralPath $srcDir -Force) {
            if ($e.PSIsContainer) { $modules.Add("src/$($e.Name)") | Out-Null }
            elseif ($e.Extension -eq '.rs') { $modules.Add("src/$($e.Name)") | Out-Null }
        }
        $unmappedModules = New-Object System.Collections.Generic.List[string]
        foreach ($mod in $modules) {
            $covered = $false
            foreach ($s in $surfaces) { if ($s.Path -eq $mod -or $s.Path.StartsWith("$mod/") -or $s.Path -eq "$mod/") { $covered = $true; break } }
            if (-not $covered) { $unmappedModules.Add($mod) | Out-Null }
        }

        $status = if ($unmappedTargets.Count -eq 0 -and $unmappedModules.Count -eq 0) { 'PASS' } else { 'FAIL' }
        Write-Output 'MODE=completeness'
        Write-Output "TARGET_COUNT=$($targetNames.Count)"
        Write-Output "MODULE_COUNT=$($modules.Count)"
        Write-Output "UNMAPPED_TARGETS_COUNT=$($unmappedTargets.Count)"
        Write-Output "UNMAPPED_TARGETS=$([string]::Join(',', $unmappedTargets))"
        Write-Output "UNMAPPED_MODULES_COUNT=$($unmappedModules.Count)"
        Write-Output "UNMAPPED_MODULES=$([string]::Join(',', $unmappedModules))"
        Write-Output "STATUS=$status"
        if ($status -eq 'PASS') { exit 0 } else { exit 1 }
    }
    'select' {
        $r = Resolve-Diff $changedFiles
        $req = @(@($r.Required) | Sort-Object)
        Write-Output 'MODE=select'
        Write-Output "REQUIRED_COUNT=$($req.Count)"
        foreach ($t in $req) { Write-Output "TARGET=$t" }
        if ($r.Unmapped.Count -gt 0) {
            Write-Output "UNMAPPED_COUNT=$($r.Unmapped.Count)"
            foreach ($u in $r.Unmapped) { Write-Output "UNMAPPED=$u" }
            Write-Output 'STATUS=FAIL'
            exit 1
        }
        Write-Output 'STATUS=PASS'
        exit 0
    }
    'report' {
        $r = Resolve-Diff $changedFiles
        $req = @(@($r.Required) | Sort-Object)
        if ($SelectedProvided) {
            $sel = @($SelectedRaw.Split(',') | ForEach-Object { $_.Trim() } | Where-Object { $_ -ne '' })
        }
        else {
            $sel = $req
        }
        $selSet = New-Object System.Collections.Generic.HashSet[string]
        foreach ($s in $sel) { [void]$selSet.Add($s) }
        $omitted = @($req | Where-Object { -not $selSet.Contains($_) } | Sort-Object)
        $status = if ($omitted.Count -eq 0 -and $r.Unmapped.Count -eq 0) { 'PASS' } else { 'FAIL' }
        Write-Output 'MODE=report'
        Write-Output "REQUIRED_COUNT=$($req.Count)"
        Write-Output "SELECTED_COUNT=$($selSet.Count)"
        Write-Output "OMITTED_COUNT=$($omitted.Count)"
        Write-Output "OMITTED=$([string]::Join(',', $omitted))"
        Write-Output "UNMAPPED_COUNT=$($r.Unmapped.Count)"
        Write-Output "UNMAPPED=$([string]::Join(',', @($r.Unmapped)))"
        Write-Output "STATUS=$status"
        if ($status -eq 'PASS') { exit 0 } else { exit 1 }
    }
    'run' {
        $r = Resolve-Diff $changedFiles
        $req = @(@($r.Required) | Sort-Object)
        $cap = [Math]::Max(1, $maxConcurrent)
        $peak = [Math]::Min($req.Count, $cap)
        if ($req.Count -eq 0) { $peak = 0 }
        $batchCount = if ($req.Count -eq 0) { 0 } else { [int][Math]::Ceiling($req.Count / [double]$cap) }
        if ($r.Unmapped.Count -gt 0) {
            Write-Output 'MODE=run'
            Write-Output "REQUIRED_COUNT=$($req.Count)"
            Write-Output "UNMAPPED_COUNT=$($r.Unmapped.Count)"
            Write-Output "UNMAPPED=$([string]::Join(',', @($r.Unmapped)))"
            Write-Output 'STATUS=FAIL'
            exit 1
        }
        Write-Output 'MODE=run'
        Write-Output "REQUIRED_COUNT=$($req.Count)"
        Write-Output "MAX_CONCURRENT_CAP=$cap"
        Write-Output "TEST_THREADS=$testThreads"
        Write-Output "PEAK_CONCURRENT=$peak"
        Write-Output "BATCH_COUNT=$batchCount"
        if ($DryRun) {
            Write-Output 'DRY_RUN=1'
            Write-Output 'STATUS=PASS'
            exit 0
        }
        # Real bounded execution: run each target as its own binary, at most
        # $cap concurrently, so the process budget stays bounded.
        $observedPeak = 0
        $failed = 0
        $pending = [System.Collections.Generic.Queue[string]]::new()
        foreach ($t in $req) { $pending.Enqueue($t) }
        $running = @()
        while ($pending.Count -gt 0 -or $running.Count -gt 0) {
            while ($running.Count -lt $cap -and $pending.Count -gt 0) {
                $name = $pending.Dequeue()
                $job = Start-Job -ScriptBlock {
                    param($root, $tname, $threads)
                    Set-Location $root
                    & cargo test --test $tname -- --test-threads=$threads 2>&1 | Out-Null
                    return $LASTEXITCODE
                } -ArgumentList $RepoRoot, $name, $testThreads
                $running += $job
            }
            if ($running.Count -gt $observedPeak) { $observedPeak = $running.Count }
            $done = Wait-Job -Job $running -Any
            $code = Receive-Job -Job $done
            if ($code -ne 0) { $failed++ }
            Remove-Job -Job $done | Out-Null
            $running = @($running | Where-Object { $_.Id -ne $done.Id })
        }
        Write-Output "OBSERVED_PEAK_CONCURRENT=$observedPeak"
        Write-Output "FAILED_TARGETS=$failed"
        if ($failed -eq 0) { Write-Output 'STATUS=PASS'; exit 0 } else { Write-Output 'STATUS=FAIL'; exit 1 }
    }
    default { Write-Error "unknown mode: $Mode"; exit 2 }
}
