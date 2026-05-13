# install.ps1 — one-liner installer for engram (Windows)
# Usage: irm https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.ps1 | iex
$ErrorActionPreference = 'Stop'

$Repo = 'softwaresalt/agent-engram'
$InstallDir = if ($env:ENGRAM_INSTALL_DIR) { $env:ENGRAM_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\engram' }

function Main {
    Detect-Platform
    Fetch-LatestTag
    Download-Archive
    Verify-Archive
    Extract-Binary
    Update-Path
    Print-Success
}

function Detect-Platform {
    if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') {
        Write-Error "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE. Only x86_64 (AMD64) is supported."
        exit 1
    }
    $script:Target = 'x86_64-pc-windows-msvc'
    $script:Ext = 'zip'
    Write-Host "Detected platform: Windows x86_64 ($script:Target)"
}

function Fetch-LatestTag {
    Write-Host 'Fetching latest release...'
    try {
        $release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest" -Headers @{ 'User-Agent' = 'engram-installer' }
        $script:Tag = $release.tag_name
    }
    catch {
        Write-Error "Failed to fetch latest release. Check your network connection and try again."
        exit 1
    }

    if (-not $script:Tag) {
        Write-Error 'Could not determine latest release tag.'
        exit 1
    }
    Write-Host "Latest release: $script:Tag"
}

function Download-Archive {
    $script:ArchiveName = "engram-$script:Tag-$script:Target.$script:Ext"
    $url = "https://github.com/$Repo/releases/download/$script:Tag/$script:ArchiveName"
    $script:TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) "engram-install-$(Get-Random)"
    New-Item -ItemType Directory -Path $script:TmpDir -Force | Out-Null
    $script:ArchivePath = Join-Path $script:TmpDir $script:ArchiveName

    Write-Host "Downloading $script:ArchiveName..."
    try {
        Invoke-WebRequest -Uri $url -OutFile $script:ArchivePath -UseBasicParsing
    }
    catch {
        Write-Error "Download failed. URL: $url"
        Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue
        exit 1
    }
}

function Verify-Archive {
    # Validate the file exists and is non-empty
    if (-not (Test-Path $script:ArchivePath)) {
        Write-Error 'Downloaded file not found.'
        Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue
        exit 1
    }

    $fileSize = (Get-Item $script:ArchivePath).Length
    if ($fileSize -lt 1024) {
        Write-Error "Downloaded file is too small ($fileSize bytes) — likely not a valid archive."
        Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue
        exit 1
    }

    # Verify the zip can be opened
    try {
        $zip = [System.IO.Compression.ZipFile]::OpenRead($script:ArchivePath)
        $entryCount = $zip.Entries.Count
        $zip.Dispose()
        if ($entryCount -eq 0) {
            Write-Error 'Archive contains no entries.'
            Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue
            exit 1
        }
    }
    catch {
        Write-Error "Archive integrity check failed: $_"
        Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue
        exit 1
    }

    Write-Host 'Archive verified.'
}

function Extract-Binary {
    $extractDir = Join-Path $script:TmpDir 'extracted'
    Add-Type -AssemblyName System.IO.Compression.FileSystem
    [System.IO.Compression.ZipFile]::ExtractToDirectory($script:ArchivePath, $extractDir)

    $binary = Join-Path $extractDir 'engram.exe'
    if (-not (Test-Path $binary)) {
        Write-Error 'engram.exe not found in archive.'
        Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue
        exit 1
    }

    New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
    Copy-Item $binary (Join-Path $InstallDir 'engram.exe') -Force
    Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue

    Write-Host "Installed engram to $InstallDir\engram.exe"
}

function Update-Path {
    $userPath = [Environment]::GetEnvironmentVariable('PATH', 'User')
    if ($userPath -split ';' | Where-Object { $_ -eq $InstallDir }) {
        return
    }

    [Environment]::SetEnvironmentVariable('PATH', "$InstallDir;$userPath", 'User')
    $env:PATH = "$InstallDir;$env:PATH"
    Write-Host "Added $InstallDir to user PATH."
}

function Print-Success {
    Write-Host ''
    Write-Host "engram $script:Tag installed successfully!" -ForegroundColor Green
    Write-Host ''
    Write-Host 'Next steps:'
    Write-Host '  engram install    # Initialize a workspace'
    Write-Host '  engram sync       # Build the first index'
    Write-Host ''
}

Main
