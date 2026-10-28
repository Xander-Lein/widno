[CmdletBinding()]
param(
    [Parameter(Mandatory)]
    [string]$Binary,
    [string[]]$Extension = @(
        '.jpg', '.jpeg', '.png', '.gif', '.bmp', '.tif', '.tiff', '.webp', '.svg', '.ico', '.cur',
        '.dib', '.j2c', '.j2k', '.jp2', '.pbm', '.pcx', '.pgm', '.ppm', '.tga',
        '.arw', '.cr2', '.cr3', '.crw', '.dng', '.mrw', '.nef', '.nrw', '.orf', '.pef',
        '.raf', '.rwl', '.rw2', '.sr2', '.srf', '.srw', '.x3f',
        '.zip', '.rar', '.7z', '.cbz', '.cbr', '.cb7'
    )
)

$ErrorActionPreference = 'Stop'
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$classesRoot = 'HKCU:\Software\Classes'
$expectedCommand = "`"$binaryPath`" `"%1`""
$failures = [System.Collections.Generic.List[string]]::new()

function Get-DefaultValue([string]$Path) {
    (Get-ItemProperty -Path $Path -Name '(default)' -ErrorAction SilentlyContinue).'(default)'
}

foreach ($extensionName in $Extension) {
    $root = Join-Path $classesRoot "SystemFileAssociations\$extensionName\shell\Open with Widno"
    $command = Get-DefaultValue (Join-Path $root 'command')
    $icon = (Get-ItemProperty -Path $root -Name 'Icon' `
        -ErrorAction SilentlyContinue).Icon
    if (-not (Test-Path $root)) {
        $failures.Add("${extensionName}: Open with Widno is missing")
    }
    if ($command -ne $expectedCommand) {
        $failures.Add("${extensionName}: command is incorrect")
    }
    if ($icon -ne "`"$binaryPath`",0") {
        $failures.Add("${extensionName}: icon is incorrect")
    }

    $oldVerb = Join-Path $root '..\Browse with Widno'
    if (Test-Path $oldVerb) {
        $failures.Add("${extensionName}: old Browse with Widno remains")
    }
}

$folderRoot = Join-Path $classesRoot 'Directory\shell\Open with Widno'
if (-not (Test-Path $folderRoot)) {
    $failures.Add('Directory: Open with Widno is missing')
} elseif ((Get-DefaultValue (Join-Path $folderRoot 'command')) -ne $expectedCommand) {
    $failures.Add('Directory: command is incorrect')
}

foreach ($legacyKey in @(
        (Join-Path $classesRoot 'Widno.ImageFile'),
        (Join-Path $classesRoot 'Applications\widno.exe')
    )) {
    if (Test-Path $legacyKey) {
        $failures.Add("Legacy registration remains: $legacyKey")
    }
}

foreach ($extensionName in $Extension) {
    $openWith = Join-Path $classesRoot "$extensionName\OpenWithProgids"
    $legacy = Get-ItemProperty -Path $openWith -Name 'Widno.ImageFile' `
        -ErrorAction SilentlyContinue
    if ($null -ne $legacy) {
        $failures.Add("${extensionName}: Widno.ImageFile is still in OpenWithProgids")
    }
    $userChoice = Join-Path $classesRoot "..\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$extensionName\UserChoice"
    if (Test-Path $userChoice) {
        Write-Verbose "$extensionName has an existing UserChoice; it was not modified."
    }
}

if ($failures.Count -gt 0) {
    throw ($failures -join [Environment]::NewLine)
}

Write-Output "Association smoke passed for $($Extension.Count) extensions and folders."
Write-Output 'No Widno default-association keys were found.'
