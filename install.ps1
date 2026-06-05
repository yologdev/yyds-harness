#Requires -Version 5.1
$ErrorActionPreference = "Stop"

$Repo = "yologdev/yyds-harness"
$ArchivePrefix = "yyds-harness"
$InstallDir = Join-Path $env:USERPROFILE ".yoyo\bin"

function Main {
    # Detect architecture (with fallback for older .NET Framework)
    try {
        $Arch = [System.Runtime.InteropServices.RuntimeInformation]::OSArchitecture.ToString()
    } catch {
        $Arch = $env:PROCESSOR_ARCHITECTURE
    }
    switch ($Arch) {
        { $_ -in "X64", "AMD64" } { $Target = "x86_64-pc-windows-msvc" }
        default {
            Write-Host "Unsupported architecture: $Arch. Falling back to cargo install."
            Invoke-CargoFallback
            return
        }
    }

    Write-Host "Detected platform: $Target"

    # Get latest release tag
    try {
        $Release = Invoke-RestMethod -Uri "https://api.github.com/repos/$Repo/releases/latest"
        $Version = $Release.tag_name
    } catch {
        Write-Host "Error: failed to fetch release info from GitHub API."
        Write-Host "You may be rate-limited. Try building from source instead."
        exit 1
    }

    if (-not $Version) {
        Write-Host "Error: could not determine latest release version."
        Write-Host "Try building from source instead."
        exit 1
    }

    Write-Host "Installing Yoyo DS Harness $Version..."

    $Archive = "$ArchivePrefix-$Version-$Target.zip"
    $Url = "https://github.com/$Repo/releases/download/$Version/$Archive"
    $ChecksumUrl = "$Url.sha256"

    # Download to temp directory
    $TmpDir = Join-Path ([System.IO.Path]::GetTempPath()) ([System.IO.Path]::GetRandomFileName())
    New-Item -ItemType Directory -Path $TmpDir -Force | Out-Null

    try {
        Write-Host "Downloading $Url..."
        try {
            Invoke-WebRequest -Uri $Url -OutFile (Join-Path $TmpDir $Archive) -UseBasicParsing
        } catch {
            Write-Host "Error: failed to download $Archive"
            Write-Host "The release may not exist yet. Try building from source instead."
            exit 1
        }

        # Download checksum (optional)
        $ChecksumFile = Join-Path $TmpDir "$Archive.sha256"
        $ChecksumAvailable = $false
        try {
            Invoke-WebRequest -Uri $ChecksumUrl -OutFile $ChecksumFile -UseBasicParsing
            $ChecksumAvailable = $true
        } catch {
            Write-Host "Warning: checksum file not available, skipping verification."
        }

        # Verify checksum (if downloaded, verification MUST pass)
        if ($ChecksumAvailable) {
            $ExpectedLine = Get-Content $ChecksumFile -Raw
            if (-not $ExpectedLine -or $ExpectedLine.Trim().Length -eq 0) {
                Write-Host "Error: checksum file is empty."
                exit 1
            }
            $ExpectedHash = ($ExpectedLine -split '\s+')[0].Trim().ToLower()
            $ActualHash = (Get-FileHash -Algorithm SHA256 (Join-Path $TmpDir $Archive)).Hash.ToLower()
            if ($ExpectedHash -ne $ActualHash) {
                Write-Host "Error: checksum verification failed. The download may be corrupted."
                Write-Host "Expected: $ExpectedHash"
                Write-Host "Actual:   $ActualHash"
                exit 1
            }
            Write-Host "Checksum verified."
        }

        # Extract
        try {
            Expand-Archive -Path (Join-Path $TmpDir $Archive) -DestinationPath $TmpDir -Force
        } catch {
            Write-Host "Error: failed to extract $Archive. The download may be corrupted."
            Write-Host "Try building from source instead."
            exit 1
        }

        # Find the binaries
        $PrimaryBinary = Get-ChildItem -Path $TmpDir -Filter "yyds.exe" -Recurse | Select-Object -First 1
        $CompatBinary = Get-ChildItem -Path $TmpDir -Filter "yoyo.exe" -Recurse | Select-Object -First 1
        if (-not $PrimaryBinary -or -not $CompatBinary) {
            Write-Host "Error: binaries 'yyds.exe' and 'yoyo.exe' not found in archive."
            Write-Host "Please report this: https://github.com/$Repo/issues"
            exit 1
        }

        # Install
        New-Item -ItemType Directory -Path $InstallDir -Force | Out-Null
        try {
            Copy-Item -Path $PrimaryBinary.FullName -Destination (Join-Path $InstallDir "yyds.exe") -Force
            Copy-Item -Path $CompatBinary.FullName -Destination (Join-Path $InstallDir "yoyo.exe") -Force
        } catch {
            Write-Host "Error: could not install Yoyo DS Harness binaries to $InstallDir"
            Write-Host "If yoyo is currently running, close it and try again."
            exit 1
        }

        Write-Host "Installed yyds to $InstallDir\yyds.exe"
        Write-Host "Installed yoyo compatibility alias to $InstallDir\yoyo.exe"

        # Check PATH
        $UserPath = [Environment]::GetEnvironmentVariable("PATH", "User")
        if (-not $UserPath) { $UserPath = "" }
        if ($UserPath -split ';' -notcontains $InstallDir) {
            try {
                $NewPath = if ($UserPath) { "$InstallDir;$UserPath" } else { $InstallDir }
                [Environment]::SetEnvironmentVariable("PATH", $NewPath, "User")
                $env:PATH = "$InstallDir;$env:PATH"
                Write-Host "Added $InstallDir to your PATH."
                Write-Host "Restart your terminal for the change to take effect."
            } catch {
                Write-Host ""
                Write-Host "Add yoyo to your PATH manually:"
                Write-Host "  `$env:PATH = `"$InstallDir;`$env:PATH`""
                Write-Host ""
            }
        }

        Write-Host "Run 'yyds --help' to get started."
    } finally {
        Remove-Item -Path $TmpDir -Recurse -Force -ErrorAction SilentlyContinue
    }
}

function Invoke-CargoFallback {
    if (Get-Command cargo -ErrorAction SilentlyContinue) {
        Write-Host "Building from source requires the sibling yoagent-state checkout until it is published."
        Write-Host "Clone $Repo and ../yoagent-state, then run: cargo install --path ."
        exit 1
    } else {
        Write-Host "Error: cargo is not installed. Install Rust first: https://rustup.rs"
        exit 1
    }
}

Main
