# install.ps1 — one-liner installer for engram (Windows)
# Usage: irm https://raw.githubusercontent.com/softwaresalt/agent-engram/main/scripts/install.ps1 | iex

$Repo = 'softwaresalt/agent-engram'
$InstallDir = if ($env:ENGRAM_INSTALL_DIR) { $env:ENGRAM_INSTALL_DIR } else { Join-Path $env:LOCALAPPDATA 'Programs\engram' }

function Cleanup { if ($script:TmpDir -and (Test-Path $script:TmpDir)) { Remove-Item -Recurse -Force $script:TmpDir -ErrorAction SilentlyContinue } }

function Abort([string]$Message) { Write-Host "Error: $Message" -ForegroundColor Red; Cleanup; exit 1 }

function Main {
    Detect-Platform
    Fetch-LatestTag
    Download-Archive
    Verify-Checksum
    Verify-Archive
    Extract-Binary
    Update-Path
    Print-Success
}

function Detect-Platform {
    if ($env:PROCESSOR_ARCHITECTURE -ne 'AMD64') {
        Abort "Unsupported architecture: $env:PROCESSOR_ARCHITECTURE. Only x86_64 (AMD64) is supported."
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
        Abort "Failed to fetch latest release. Check your network connection and try again."
    }

    if (-not $script:Tag) {
        Abort 'Could not determine latest release tag.'
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
        Abort "Download failed. URL: $url"
    }

    # Also download the SHA256 checksum sidecar if available
    $script:ChecksumUrl = "$url.sha256"
    $script:ChecksumPath = "$script:ArchivePath.sha256"
    try {
        Invoke-WebRequest -Uri $script:ChecksumUrl -OutFile $script:ChecksumPath -UseBasicParsing -ErrorAction Stop
    }
    catch {
        $script:ChecksumPath = $null
        Write-Host 'SHA256 checksum not available for this release — skipping verification.'
    }
}

function Verify-Checksum {
    if (-not $script:ChecksumPath -or -not (Test-Path $script:ChecksumPath)) { return }

    $expected = (Get-Content $script:ChecksumPath -Raw).Trim().Split(' ')[0].ToLower()
    $actual = (Get-FileHash -Path $script:ArchivePath -Algorithm SHA256).Hash.ToLower()

    if ($expected -ne $actual) {
        Abort "SHA256 checksum mismatch. Expected: $expected, got: $actual"
    }
    Write-Host 'SHA256 checksum verified.'
}

function Verify-Archive {
    if (-not (Test-Path $script:ArchivePath)) {
        Abort 'Downloaded file not found.'
    }

    $fileSize = (Get-Item $script:ArchivePath).Length
    if ($fileSize -lt 1024) {
        Abort "Downloaded file is too small ($fileSize bytes) — likely not a valid archive."
    }

    try {
        Add-Type -AssemblyName System.IO.Compression.FileSystem
        $zip = [System.IO.Compression.ZipFile]::OpenRead($script:ArchivePath)
        $entryCount = $zip.Entries.Count
        $zip.Dispose()
        if ($entryCount -eq 0) {
            Abort 'Archive contains no entries.'
        }
    }
    catch {
        Abort "Archive integrity check failed: $_"
    }

    Write-Host 'Archive verified.'
}

function Extract-Binary {
    $extractDir = Join-Path $script:TmpDir 'extracted'
    [System.IO.Compression.ZipFile]::ExtractToDirectory($script:ArchivePath, $extractDir)

    $binary = Join-Path $extractDir 'engram.exe'
    if (-not (Test-Path $binary)) {
        Abort 'engram.exe not found in archive.'
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
