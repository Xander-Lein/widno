[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)]
    [string]$Binary
)

$resolvedBinary = (Resolve-Path $Binary).Path
$classesRoot = 'HKCU:\Software\Classes'
$progId = 'Widno.ImageFile'
$imageExtensions = @(
    '.jpg', '.jpeg', '.png', '.gif', '.bmp', '.tif', '.tiff', '.webp',
    '.arw', '.cr2', '.cr3', '.crw', '.dng', '.mrw', '.nef', '.nrw',
    '.orf', '.pef', '.raf', '.rwl', '.rw2', '.sr2', '.srf', '.srw', '.x3f'
)
$archiveExtensions = @('.zip', '.rar', '.7z')
$browseExtensions = $imageExtensions + $archiveExtensions

$progIdKey = Join-Path $classesRoot $progId
$iconKey = Join-Path $progIdKey 'DefaultIcon'
$commandKey = Join-Path $progIdKey 'shell\open\command'
$applicationKey = Join-Path $classesRoot 'Applications\widno.exe'
$openWithKey = Join-Path $classesRoot 'Applications\widno.exe\shell\open\command'
$applicationIconKey = Join-Path $classesRoot 'Applications\widno.exe\DefaultIcon'

foreach ($key in @($progIdKey, $iconKey, $commandKey, $applicationKey, $openWithKey, $applicationIconKey)) {
    if ($PSCmdlet.ShouldProcess($key, 'Create Widno file association key')) {
        New-Item -Path $key -Force | Out-Null
    }
}

function Set-DefaultRegistryValue([string]$Path, [string]$Value, [string]$Description) {
    if ($PSCmdlet.ShouldProcess($Path, $Description)) {
        Set-Item -Path $Path -Value $Value -Force
    }
}

if ($PSCmdlet.ShouldProcess($iconKey, 'Set Widno icon')) {
    Set-DefaultRegistryValue $iconKey "`"$resolvedBinary`",0" 'Set Widno default icon'
}
if ($PSCmdlet.ShouldProcess($commandKey, 'Set Widno open command')) {
    Set-DefaultRegistryValue $commandKey "`"$resolvedBinary`" `"%1`"" 'Set Widno open command'
}
Set-DefaultRegistryValue $applicationIconKey "`"$resolvedBinary`",0" 'Set widno.exe icon'
Set-DefaultRegistryValue $openWithKey "`"$resolvedBinary`" `"%1`"" 'Set widno.exe open command'

if ($PSCmdlet.ShouldProcess($applicationKey, 'Register widno.exe in Open With')) {
    New-ItemProperty -Path $applicationKey -Name 'FriendlyAppName' -Value 'widno' -PropertyType String -Force | Out-Null
}

function Set-DefaultIfUnclaimed([string]$Extension) {
    $extensionKey = Join-Path $classesRoot $Extension
    $userChoiceKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$Extension\UserChoice"
    New-Item -Path $extensionKey -Force | Out-Null
    $current = (Get-ItemProperty -Path $extensionKey -Name '(default)' -ErrorAction SilentlyContinue).'(default)'
    if (-not (Test-Path $userChoiceKey) -and [string]::IsNullOrWhiteSpace($current)) {
        Set-DefaultRegistryValue $extensionKey $progId "Set Widno default for $Extension"
    }
}

function Add-BrowseCommand([string]$Extension) {
    $extensionKey = Join-Path $classesRoot $extension
    $systemAssociationKey = Join-Path $classesRoot "SystemFileAssociations\$Extension"
    if ($PSCmdlet.ShouldProcess($extensionKey, "Associate $extension with Widno")) {
        New-Item -Path $extensionKey -Force | Out-Null
        New-Item -Path $systemAssociationKey -Force | Out-Null
        $openWithProgids = Join-Path $extensionKey 'OpenWithProgids'
        New-Item -Path $openWithProgids -Force | Out-Null
        New-ItemProperty -Path $openWithProgids -Name $progId -Value '' -PropertyType String -Force | Out-Null
        foreach ($verbRoot in @($extensionKey, $systemAssociationKey)) {
            $verbKey = Join-Path $verbRoot 'shell\Browse with Widno'
            $browseCommandKey = Join-Path $verbKey 'command'
            New-Item -Path $browseCommandKey -Force | Out-Null
            Set-DefaultRegistryValue $verbKey 'Browse with Widno' 'Set Browse with Widno label'
            Set-DefaultRegistryValue $browseCommandKey "`"$resolvedBinary`" `"%1`"" 'Set Browse with Widno command'
        }
    }
}

function Remove-ArchiveDefaultIfWidno([string]$Extension) {
    $extensionKey = Join-Path $classesRoot $Extension
    $userChoiceKey = "HKCU:\Software\Microsoft\Windows\CurrentVersion\Explorer\FileExts\$Extension\UserChoice"
    $current = (Get-ItemProperty -Path $extensionKey -Name '(default)' -ErrorAction SilentlyContinue).'(default)'
    if (-not (Test-Path $userChoiceKey) -and $current -eq $progId) {
        Remove-ItemProperty -Path $extensionKey -Name '(default)' -Force -ErrorAction SilentlyContinue
    }
}

foreach ($extension in $imageExtensions) {
    Set-DefaultIfUnclaimed $extension
}
foreach ($extension in $browseExtensions) {
    Add-BrowseCommand $extension
}
foreach ($extension in $archiveExtensions) {
    Remove-ArchiveDefaultIfWidno $extension
}

if ($PSCmdlet.ShouldProcess('Explorer', 'Notify Windows about changed file associations')) {
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

Write-Output "Widno associations registered for $($browseExtensions.Count) extensions."
Write-Output 'Existing Windows UserChoice associations were preserved.'
Write-Output 'Restart Explorer or sign out/in if Windows still shows a cached icon.'
