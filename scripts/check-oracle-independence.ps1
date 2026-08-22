<#
.SYNOPSIS
    Independence guard for the agent-visible MCP catalog oracle (Feature 127-F).
.DESCRIPTION
    Enforces the oracle independence invariant mechanically, so a future
    refactor cannot quietly reconnect the oracle to the production derivation
    path:

      1. Forbidden-import scan — the oracle test and its capture helper (the
         Rust sources that could `use` the production module) must NOT
         reference the production catalog module or its enumeration function.
         The forbidden tokens are 'tools_catalog' and 'all_tools'. The
         human-authored JSON fixture is data, not code: it may name the source
         contract in its policy note, and its independence is enforced by the
         regeneration scan plus its header, not this token scan.

      2. Fixture-regeneration scan — no build script, test, CI step, or helper
         script may write the fixture file. Any line under build.rs,
         .github/workflows, scripts, or tests that names the fixture AND uses a
         write verb is a violation.

    Exit code 0 means independent; exit code 1 means a violation was detected
    (each violation is printed to stderr). The two scenarios are demonstrable
    by pointing -Root at a throwaway tree that contains a violating copy.
.PARAMETER Root
    Repository root to scan. Defaults to the parent of this script's directory.
#>
param(
    [string]$Root = (Split-Path -Parent (Split-Path -Parent $MyInvocation.MyCommand.Path))
)

Set-StrictMode -Version Latest
$ErrorActionPreference = 'Stop'

$forbiddenTokens = @('tools_catalog', 'all_tools')
$fixtureName = 'mcp_tool_catalog.expected.json'
$writeVerbs = @(
    'fs::write', 'write_all', 'File::create', 'to_writer', 'to_writer_pretty',
    'Out-File', 'Set-Content', 'Add-Content', 'tee', '>'
)

$selfPath = $MyInvocation.MyCommand.Path
$violations = @()

# 1. Forbidden-import / forbidden-token scan of the two Rust oracle artifacts.
$scanFiles = @(
    (Join-Path $Root 'tests/contract/mcp_catalog_oracle_test.rs'),
    (Join-Path $Root 'tests/helpers/mcp_catalog_capture.rs')
)
foreach ($file in $scanFiles) {
    if (-not (Test-Path -LiteralPath $file)) { continue }
    $lines = @(Get-Content -LiteralPath $file)
    for ($i = 0; $i -lt $lines.Count; $i++) {
        foreach ($token in $forbiddenTokens) {
            if ($lines[$i].Contains($token)) {
                $violations += "FORBIDDEN-IMPORT: ${file}:$($i + 1) references '$token'"
            }
        }
    }
}

# 2. Fixture-regeneration scan of build scripts, CI, helper scripts, and tests.
$regenRoots = @(
    (Join-Path $Root 'build.rs'),
    (Join-Path $Root '.github/workflows'),
    (Join-Path $Root 'scripts'),
    (Join-Path $Root 'tests')
)
foreach ($regenRoot in $regenRoots) {
    if (-not (Test-Path -LiteralPath $regenRoot)) { continue }
    $item = Get-Item -LiteralPath $regenRoot
    $candidates = if ($item.PSIsContainer) {
        Get-ChildItem -LiteralPath $regenRoot -Recurse -File
    } else {
        @($item)
    }
    foreach ($candidate in $candidates) {
        if ($candidate.FullName -eq $selfPath) { continue }
        $lines = @(Get-Content -LiteralPath $candidate.FullName)
        for ($i = 0; $i -lt $lines.Count; $i++) {
            $line = $lines[$i]
            if (-not $line.Contains($fixtureName)) { continue }
            foreach ($verb in $writeVerbs) {
                if ($line.Contains($verb)) {
                    $violations += "FIXTURE-REGENERATION: $($candidate.FullName):$($i + 1) writes '$fixtureName' via '$verb'"
                    break
                }
            }
        }
    }
}

if ($violations.Count -gt 0) {
    [Console]::Error.WriteLine('Oracle independence guard: FAIL')
    foreach ($violation in $violations) {
        [Console]::Error.WriteLine("  $violation")
    }
    exit 1
}

Write-Output 'Oracle independence guard: PASS (no forbidden imports, no fixture regeneration).'
exit 0
