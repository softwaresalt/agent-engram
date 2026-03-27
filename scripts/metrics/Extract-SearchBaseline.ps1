#!/usr/bin/env pwsh
[CmdletBinding()]
param(
    [Parameter()]
    [string]$Repository,

    [Parameter()]
    [datetime]$Since,

    [Parameter()]
    [string]$OutputPath,

    [Parameter()]
    [switch]$Validate
)

$ErrorActionPreference = "Stop"

function Get-SanitizedBranchName {
    param(
        [Parameter(Mandatory)]
        [string]$BranchName
    )

    return ($BranchName -replace '/', '__')
}

function Get-PrimaryLanguage {
    param(
        [Parameter(Mandatory)]
        [string]$WorkingDirectory
    )

    $extensions = @{}
    Get-ChildItem -Path $WorkingDirectory -Recurse -File -ErrorAction SilentlyContinue |
        ForEach-Object {
            $extension = $_.Extension.ToLowerInvariant()
            if ([string]::IsNullOrWhiteSpace($extension)) {
                return
            }

            if (-not $extensions.ContainsKey($extension)) {
                $extensions[$extension] = 0
            }
            $extensions[$extension] += 1
        }

    $topExtension = $extensions.GetEnumerator() |
        Sort-Object -Property Value -Descending |
        Select-Object -First 1

    if (-not $topExtension) {
        return "unknown"
    }

    switch ($topExtension.Key) {
        ".rs" { "rust" }
        ".ts" { "typescript" }
        ".tsx" { "typescript" }
        ".js" { "javascript" }
        ".py" { "python" }
        ".go" { "go" }
        ".cs" { "csharp" }
        default { $topExtension.Key.TrimStart('.') }
    }
}

function Get-CodebaseContext {
    param(
        [Parameter(Mandatory)]
        [string]$WorkingDirectory
    )

    if (-not (Test-Path -LiteralPath $WorkingDirectory)) {
        return @{
            language     = "unknown"
            total_files  = $null
            total_loc    = $null
            size_bracket = "unknown"
        }
    }

    $files = Get-ChildItem -Path $WorkingDirectory -Recurse -File -ErrorAction SilentlyContinue
    $totalFiles = @($files).Count
    $totalLoc = 0

    foreach ($file in $files) {
        try {
            $totalLoc += @(Get-Content -LiteralPath $file.FullName -ErrorAction Stop).Count
        }
        catch {
            continue
        }
    }

    $sizeBracket = if ($totalLoc -lt 10000) {
        "small"
    }
    elseif ($totalLoc -le 100000) {
        "medium"
    }
    else {
        "large"
    }

    return @{
        language     = Get-PrimaryLanguage -WorkingDirectory $WorkingDirectory
        total_files  = $totalFiles
        total_loc    = $totalLoc
        size_bracket = $sizeBracket
    }
}

function Convert-BaselineRowsToRecords {
    param(
        [Parameter(Mandatory)]
        [object[]]$Rows
    )

    $records = @()
    $sequenceIndex = 0

    foreach ($row in $Rows) {
        if ([string]::IsNullOrWhiteSpace($row.assistant_response)) {
            continue
        }

        $matches = [regex]::Matches(
            $row.assistant_response,
            '(?im)functions\.(rg|glob|view)|\b(rg|glob|view)\b'
        )

        if ($matches.Count -eq 0) {
            continue
        }

        $toolCalls = @()
        foreach ($match in $matches) {
            $toolName = if ($match.Groups[1].Success) {
                $match.Groups[1].Value
            }
            else {
                $match.Groups[2].Value
            }
            $toolCalls += @{
                tool            = $toolName
                params_summary  = "derived from assistant response"
                response_tokens = [math]::Max([int][math]::Ceiling($row.assistant_response.Length / 12), 1)
            }
        }

        $totalTokens = ($toolCalls | Measure-Object -Property response_tokens -Sum).Sum
        $toolsUsed = $toolCalls.tool
        $purpose = if ($toolsUsed -contains "view") {
            "neighborhood"
        }
        elseif ($toolsUsed -contains "glob") {
            "broad_exploration"
        }
        elseif ($toolsUsed.Count -ge 3) {
            "impact_trace"
        }
        else {
            "point_lookup"
        }

        $branchName = if ([string]::IsNullOrWhiteSpace($row.branch)) {
            "unknown"
        }
        else {
            Get-SanitizedBranchName -BranchName $row.branch
        }

        $codebaseContext = Get-CodebaseContext -WorkingDirectory $row.cwd
        $sequenceIndex += 1
        $timestamp = if ([string]::IsNullOrWhiteSpace($row.timestamp)) {
            [datetime]::UtcNow.ToString("o")
        }
        else {
            [datetime]::Parse($row.timestamp).ToUniversalTime().ToString("o")
        }

        $records += [ordered]@{
            sequence_id       = "{0:D6}" -f $sequenceIndex
            timestamp         = $timestamp
            tool_name         = "file_search_sequence"
            response_bytes    = [uint64]($totalTokens * 4)
            estimated_tokens  = [uint64]$totalTokens
            symbols_returned  = [uint32]0
            results_returned  = [uint32]$toolCalls.Count
            branch            = $branchName
            connection_id     = $null
            turn_purpose      = $purpose
            tools_used        = $toolCalls
            total_search_tokens = [uint64]$totalTokens
            total_tool_calls  = [uint32]$toolCalls.Count
            files_touched     = @()
            codebase_context  = $codebaseContext
            engram_tool       = $null
            repository        = $row.repository
        }
    }

    return @($records)
}

function Assert-RecordShape {
    param(
        [Parameter(Mandatory)]
        [object[]]$Records
    )

    foreach ($record in $Records) {
        foreach ($key in @(
                "sequence_id",
                "timestamp",
                "tool_name",
                "response_bytes",
                "estimated_tokens",
                "symbols_returned",
                "results_returned",
                "branch",
                "turn_purpose",
                "tools_used",
                "total_search_tokens",
                "total_tool_calls",
                "files_touched",
                "codebase_context"
            )) {
            if (-not $record.Contains($key)) {
                throw "Baseline record is missing required field '$key'."
            }
        }
    }
}

if ($Validate) {
    $validationRoot = Join-Path $PSScriptRoot "_validation_fixture"
    try {
        if (-not (Test-Path -LiteralPath $validationRoot)) {
            New-Item -ItemType Directory -Path $validationRoot | Out-Null
            Set-Content -LiteralPath (Join-Path $validationRoot "sample.rs") -Value "fn main() {}"
        }

        $fixture = @(
            @{
                repository         = "softwaresalt/agent-engram"
                branch             = "feature/demo"
                cwd                = $validationRoot
                assistant_response = 'functions.rg => list matches' + [Environment]::NewLine + 'functions.view => read file'
                timestamp          = "2026-03-27T12:00:00Z"
            }
        )

        $records = @(Convert-BaselineRowsToRecords -Rows $fixture)
        Assert-RecordShape -Records $records
        if ($records.Count -ne 1) {
            throw "Expected one validation record but found $($records.Count)."
        }
        if ($records[0].branch -ne "feature__demo") {
            throw "Branch sanitization validation failed."
        }
    }
    finally {
        if (Test-Path -LiteralPath $validationRoot) {
            Remove-Item -LiteralPath $validationRoot -Recurse -Force
        }
    }

    Write-Host "Extract-SearchBaseline validation passed."
    return
}

$candidatePaths = @()
if ($env:COPILOT_CLI_SESSION_STORE) {
    $candidatePaths += $env:COPILOT_CLI_SESSION_STORE
}
if ($env:LOCALAPPDATA) {
    $candidatePaths += Join-Path $env:LOCALAPPDATA "GitHubCopilot\session_store.db"
    $candidatePaths += Join-Path $env:LOCALAPPDATA "github-copilot-cli\session_store.db"
}

$databasePath = $candidatePaths |
    Where-Object { $_ -and (Test-Path -LiteralPath $_) } |
    Select-Object -First 1

if (-not $databasePath) {
    throw "Could not locate the Copilot CLI session store. Set COPILOT_CLI_SESSION_STORE to the database path."
}

$sinceText = if ($PSBoundParameters.ContainsKey("Since")) {
    $Since.ToUniversalTime().ToString("o")
}
else {
    ""
}

$pythonScript = @'
import json
import sqlite3
import sys

database_path, repository, since_text = sys.argv[1], sys.argv[2], sys.argv[3]
query = """
SELECT s.repository, s.branch, s.cwd, t.assistant_response, t.timestamp
FROM sessions s
JOIN turns t ON t.session_id = s.id
WHERE (? = '' OR s.repository = ?)
  AND (? = '' OR t.timestamp >= ?)
ORDER BY t.timestamp ASC
"""

connection = sqlite3.connect(f"file:{database_path}?mode=ro", uri=True)
connection.row_factory = sqlite3.Row
rows = [dict(row) for row in connection.execute(query, (repository, repository, since_text, since_text))]
print(json.dumps(rows))
'@

$rowsJson = $pythonScript | & python - $databasePath ($Repository ?? "") $sinceText
if ($LASTEXITCODE -ne 0) {
    throw "Python failed while reading the session store."
}

$rows = $rowsJson | ConvertFrom-Json
if ($null -eq $rows) {
    $records = @()
}
else {
    $records = @(Convert-BaselineRowsToRecords -Rows @($rows))
}
Assert-RecordShape -Records $records

$jsonLines = $records | ForEach-Object { $_ | ConvertTo-Json -Depth 8 -Compress }
if ($OutputPath) {
    $jsonLines | Set-Content -LiteralPath $OutputPath
}
else {
    $jsonLines
}
