[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)]
    [string]$Binary,
    [switch]$Unregister
)

$ErrorActionPreference = 'Stop'
$classesRoot = 'HKCU:\Software\Classes'
$verbName = 'Open with Widno'
$resolvedBinary = $null
if (-not $Unregister) {
    $resolvedBinary = (Resolve-Path -LiteralPath $Binary).Path
}
$iconValue = if ($resolvedBinary) { "`"$resolvedBinary`",0" } else { '' }
$commandValue = if ($resolvedBinary) { "`"$resolvedBinary`" `"%1`"" } else { '' }

$imageExtensions = @(
    '.jpg', '.jpeg', '.png', '.gif', '.bmp', '.tif', '.tiff', '.webp',
    '.svg', '.ico', '.cur', '.dib', '.j2c', '.j2k', '.jp2', '.pbm',
    '.pcx', '.pgm', '.ppm', '.tga', '.arw', '.cr2', '.cr3', '.crw',
    '.dng', '.mrw', '.nef', '.nrw', '.orf', '.pef', '.raf', '.rwl',
    '.rw2', '.sr2', '.srf', '.srw', '.x3f'
)
$archiveExtensions = @('.zip', '.rar', '.7z', '.cbz', '.cbr', '.cb7')
$contextExtensions = $imageExtensions + $archiveExtensions

function Set-RegistryValue([string]$Path, [string]$Name, [string]$Value) {
    if ($PSCmdlet.ShouldProcess($Path, "Set $Name")) {
        New-Item -Path $Path -Force | Out-Null
        if ($Name -eq '(default)') {
            Set-Item -Path $Path -Value $Value -Force
        } else {
            New-ItemProperty -Path $Path -Name $Name -Value $Value `
                -PropertyType String -Force | Out-Null
        }
    }
}

function Add-OpenWithVerb([string]$Root) {
    $verb = Join-Path $Root "shell\$verbName"
    Set-RegistryValue $verb '(default)' $verbName
    Set-RegistryValue $verb 'Icon' $iconValue
    Set-RegistryValue (Join-Path $verb 'command') '(default)' $commandValue
}

function Remove-OldWidnoRegistration([string]$Extension) {
    $extensionKey = Join-Path $classesRoot $Extension
    $oldVerb = Join-Path $extensionKey 'shell\Browse with Widno'
    Remove-Item -LiteralPath $oldVerb -Recurse -Force -ErrorAction SilentlyContinue
    $systemAssociationKey = Join-Path $classesRoot "SystemFileAssociations\$Extension"
    Remove-Item -LiteralPath (Join-Path $systemAssociationKey 'shell\Browse with Widno') `
        -Recurse -Force -ErrorAction SilentlyContinue

    $oldOpenWith = Join-Path $extensionKey 'OpenWithProgids'
    Remove-ItemProperty -LiteralPath $oldOpenWith -Name 'Widno.ImageFile' `
        -Force -ErrorAction SilentlyContinue

    $default = (Get-ItemProperty -LiteralPath $extensionKey -Name '(default)' `
        -ErrorAction SilentlyContinue).'(default)'
    if ($default -eq 'Widno.ImageFile') {
        Remove-ItemProperty -LiteralPath $extensionKey -Name '(default)' `
            -Force -ErrorAction SilentlyContinue
    }
}

if ($Unregister) {
    foreach ($extension in $contextExtensions) {
        $extensionKey = Join-Path $classesRoot "SystemFileAssociations\$extension"
        Remove-Item -LiteralPath (Join-Path $extensionKey "shell\$verbName") `
            -Recurse -Force -ErrorAction SilentlyContinue
        Remove-OldWidnoRegistration $extension
    }
    $directoryShell = Join-Path $classesRoot 'Directory\shell'
    Remove-Item -LiteralPath (Join-Path $directoryShell $verbName) `
        -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $classesRoot 'Widno.ImageFile') `
        -Recurse -Force -ErrorAction SilentlyContinue
    Remove-Item -LiteralPath (Join-Path $classesRoot 'Applications\widno.exe') `
        -Recurse -Force -ErrorAction SilentlyContinue
    Write-Output 'Widno context-menu entries removed.'
    exit 0
}

foreach ($extension in $contextExtensions) {
    Remove-OldWidnoRegistration $extension
    Add-OpenWithVerb (Join-Path $classesRoot "SystemFileAssociations\$extension")
}

$directoryShell = Join-Path $classesRoot 'Directory\shell'
$oldFolderVerb = Join-Path $directoryShell 'Browse with Widno'
Remove-Item -LiteralPath $oldFolderVerb -Recurse -Force -ErrorAction SilentlyContinue
Add-OpenWithVerb $directoryShell

Remove-Item -LiteralPath (Join-Path $classesRoot 'Widno.ImageFile') `
    -Recurse -Force -ErrorAction SilentlyContinue
Remove-Item -LiteralPath (Join-Path $classesRoot 'Applications\widno.exe') `
    -Recurse -Force -ErrorAction SilentlyContinue

if ($PSCmdlet.ShouldProcess('Explorer', 'Notify Windows about changed context menu')) {
    Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class WidnoShellNotify {
    [DllImport("shell32.dll", CharSet = CharSet.Unicode)]
    public static extern void SHChangeNotify(uint wEventId, uint uFlags, IntPtr dwItem1, IntPtr dwItem2);
}
'@
    [WidnoShellNotify]::SHChangeNotify(0x08000000, 0, [IntPtr]::Zero, [IntPtr]::Zero)
}

Write-Output "Open with Widno registered for $($contextExtensions.Count) extensions and folders."
Write-Output 'Default Windows applications and UserChoice associations were not changed.'
