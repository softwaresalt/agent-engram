#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter()]
    [string[]]$BaselinePaths,

    [Parameter()]
    [string[]]$EngramPaths,

    [Parameter()]
    [ValidateSet("table", "json")]
    [string]$OutputFormat = "table",

    [Parameter()]
    [switch]$Validate
)

$ErrorActionPreference = "Stop"

function Get-Median {
    param(
        [double[]]$Values
    )

    if ($Values.Count -eq 0) {
        return 0.0
    }

    $sorted = $Values | Sort-Object
    $middle = [math]::Floor($sorted.Count / 2)
    if ($sorted.Count % 2 -eq 1) {
        return [double]$sorted[$middle]
    }

    return ([double]$sorted[$middle - 1] + [double]$sorted[$middle]) / 2.0
}

function Get-ConfidenceLevel {
    param(
        [Parameter(Mandatory)]
        [int]$SampleCount
    )

    if ($SampleCount -lt 10) {
        return "low"
    }
    if ($SampleCount -le 50) {
        return "medium"
    }

    return "high"
}

function Get-PurposeForEngramRecord {
    param(
        [Parameter(Mandatory)]
        [object]$Record
    )

    switch ($Record.tool_name) {
        "list_symbols" { return "point_lookup" }
        "impact_analysis" { return "impact_trace" }
        "unified_search" { return "broad_exploration" }
        "map_code" {
            if ([int]$Record.results_returned -le 1) {
                return "point_lookup"
            }
            return "neighborhood"
        }
        default { return "implementation" }
    }
}

function Get-JsonLines {
    param(
        [Parameter(Mandatory)]
        [string[]]$Paths
    )

    $records = @()
    foreach ($path in $Paths) {
        foreach ($line in Get-Content -LiteralPath $path) {
            if ([string]::IsNullOrWhiteSpace($line)) {
                continue
            }
            $records += ($line | ConvertFrom-Json)
        }
    }

    return $records
}

function Build-CalibrationReport {
    param(
        [Parameter(Mandatory)]
        [object[]]$BaselineRecords,

        [Parameter(Mandatory)]
        [object[]]$EngramRecords
    )

    $purposes = @("point_lookup", "neighborhood", "impact_trace", "broad_exploration", "implementation")
    $report = @()

    foreach ($purpose in $purposes) {
        $baseline = @($BaselineRecords | Where-Object { $_.turn_purpose -eq $purpose })
        $engram = @($EngramRecords | Where-Object { (Get-PurposeForEngramRecord -Record $_) -eq $purpose })

        $baselineMedian = Get-Median -Values @($baseline | ForEach-Object { [double]$_.estimated_tokens })
        $engramMedian = Get-Median -Values @($engram | ForEach-Object { [double]$_.estimated_tokens })
        $multiplier = if ($engramMedian -eq 0) { 0.0 } else { $baselineMedian / $engramMedian }
        $sampleCount = [math]::Min($baseline.Count, $engram.Count)

        $report += [ordered]@{
            purpose          = $purpose
            baseline_median  = [math]::Round($baselineMedian, 2)
            engram_median    = [math]::Round($engramMedian, 2)
            multiplier       = [math]::Round($multiplier, 2)
            baseline_samples = $baseline.Count
            engram_samples   = $engram.Count
            confidence       = Get-ConfidenceLevel -SampleCount $sampleCount
        }
    }

    return $report
}

if ($Validate) {
    $baselineFixture = @(
        [pscustomobject]@{ turn_purpose = "point_lookup"; estimated_tokens = 400 },
        [pscustomobject]@{ turn_purpose = "point_lookup"; estimated_tokens = 800 },
        [pscustomobject]@{ turn_purpose = "impact_trace"; estimated_tokens = 2000 }
    )
    $engramFixture = @(
        [pscustomobject]@{ tool_name = "list_symbols"; estimated_tokens = 100; results_returned = 1 },
        [pscustomobject]@{ tool_name = "list_symbols"; estimated_tokens = 200; results_returned = 1 },
        [pscustomobject]@{ tool_name = "impact_analysis"; estimated_tokens = 500; results_returned = 8 }
    )
    $report = Build-CalibrationReport -BaselineRecords $baselineFixture -EngramRecords $engramFixture
    if (($report | Where-Object purpose -eq "point_lookup").multiplier -le 0) {
        throw "Expected point_lookup multiplier to be greater than zero."
    }
    if (($report | Where-Object purpose -eq "impact_trace").confidence -ne "low") {
        throw "Confidence threshold validation failed."
    }

    Write-Host "Compare-Metrics validation passed."
    return
}

if (-not $BaselinePaths -or -not $EngramPaths) {
    throw "BaselinePaths and EngramPaths are required unless -Validate is specified."
}

$baselineRecords = Get-JsonLines -Paths $BaselinePaths
$engramRecords = Get-JsonLines -Paths $EngramPaths
$report = Build-CalibrationReport -BaselineRecords $baselineRecords -EngramRecords $engramRecords

if ($OutputFormat -eq "json") {
    $report | ConvertTo-Json -Depth 5
}
else {
    $report | Format-Table -AutoSize
}
