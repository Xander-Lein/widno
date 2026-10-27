[CmdletBinding()]
param(
    [string]$Iscc = ""
)

$ErrorActionPreference = "Stop"
$projectRoot = Split-Path -Parent $PSScriptRoot
$binary = Join-Path $projectRoot "target\release\widno.exe"
$script = Join-Path $projectRoot "installer\widno.iss"

if (-not (Test-Path $binary)) {
    throw "Release binary not found: $binary. Run cargo build --release --locked first."
}

if ([string]::IsNullOrWhiteSpace($Iscc)) {
    $command = Get-Command iscc.exe -ErrorAction SilentlyContinue
    if ($command) {
        $Iscc = $command.Source
    } else {
        $candidates = @(
            "${env:ProgramFiles(x86)}\Inno Setup 6\ISCC.exe",
            "${env:ProgramFiles}\Inno Setup 6\ISCC.exe",
            "${env:LOCALAPPDATA}\Programs\Inno Setup 6\ISCC.exe"
        ) | Where-Object { $_ -and (Test-Path $_) }
        if (@($candidates).Count -gt 0) {
            $Iscc = @($candidates)[0]
        }
    }
}

if ([string]::IsNullOrWhiteSpace($Iscc) -or -not (Test-Path $Iscc)) {
    throw "Inno Setup 6 compiler (ISCC.exe) was not found. Install it or pass -Iscc C:\path\ISCC.exe."
}

$dist = Join-Path $projectRoot "dist"
New-Item -ItemType Directory -Force -Path $dist | Out-Null
& $Iscc $script
if ($LASTEXITCODE -ne 0) {
    throw "Inno Setup failed with exit code $LASTEXITCODE."
}

Write-Output "Installer created in $dist"
