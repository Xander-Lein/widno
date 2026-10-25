[CmdletBinding(SupportsShouldProcess)]
param(
    [Parameter(Mandatory)]
    [string]$Binary
)

$resolvedBinary = (Resolve-Path $Binary).Path
$classesRoot = 'HKCU:\Software\Classes'
$progId = 'Widno.ImageFile'
$extensions = @(
    '.jpg', '.jpeg', '.png', '.gif', '.bmp', '.tif', '.tiff', '.webp',
    '.zip', '.rar', '.7z'
)

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

foreach ($extension in $extensions) {
    $extensionKey = Join-Path $classesRoot $extension
    if ($PSCmdlet.ShouldProcess($extensionKey, "Associate $extension with Widno")) {
        New-Item -Path $extensionKey -Force | Out-Null
        Set-DefaultRegistryValue $extensionKey $progId "Associate $extension with Widno"
        $openWithProgids = Join-Path $extensionKey 'OpenWithProgids'
        New-Item -Path $openWithProgids -Force | Out-Null
        New-ItemProperty -Path $openWithProgids -Name $progId -Value '' -PropertyType String -Force | Out-Null
    }
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

Write-Output "Widno associations registered for $($extensions.Count) extensions."
Write-Output 'Restart Explorer or sign out/in if Windows still shows a cached icon.'
