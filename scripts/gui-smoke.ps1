param(
    [string]$Binary = (Join-Path $PSScriptRoot '..\target\release\widno.exe'),
    [string]$Fixture = (Join-Path $PSScriptRoot '..\tests\fixtures\images\fixture.png')
)

$ErrorActionPreference = 'Stop'
Add-Type -AssemblyName System.Windows.Forms

Add-Type @'
using System;
using System.Runtime.InteropServices;
public static class WidnoNative {
    [StructLayout(LayoutKind.Sequential)]
    public struct RECT { public int Left, Top, Right, Bottom; }
    [DllImport("user32.dll")]
    public static extern bool GetWindowRect(IntPtr hWnd, out RECT rect);
    [DllImport("user32.dll")]
    public static extern bool EnumWindows(EnumWindowsProc callback, IntPtr extra);
    [DllImport("user32.dll")]
    public static extern uint GetWindowThreadProcessId(IntPtr hWnd, out uint processId);
    public delegate bool EnumWindowsProc(IntPtr hWnd, IntPtr extra);
}
'@

$process = Start-Process -FilePath (Resolve-Path $Binary) `
    -ArgumentList (Resolve-Path $Fixture) -PassThru
try {
    $deadline = (Get-Date).AddSeconds(10)
    do {
        Start-Sleep -Milliseconds 200
        $process.Refresh()
    } while ($process.MainWindowHandle -eq 0 -and (Get-Date) -lt $deadline)

    if ($process.MainWindowHandle -eq 0) {
        throw 'GUI window was not created within 10 seconds'
    }
    $windows = [System.Collections.Generic.List[object]]::new()
    [WidnoNative]::EnumWindows({
        param($hWnd, $extra)
        $owner = 0
        [WidnoNative]::GetWindowThreadProcessId($hWnd, [ref]$owner) | Out-Null
        if ($owner -eq $process.Id) {
            $candidate = New-Object WidnoNative+RECT
            if ([WidnoNative]::GetWindowRect($hWnd, [ref]$candidate)) {
                $windows.Add([pscustomobject]@{ Handle = $hWnd; Rect = $candidate })
            }
        }
        return $true
    }, [IntPtr]::Zero) | Out-Null
    if ($windows.Count -eq 0) {
        throw 'No top-level GUI window was found'
    }
    $window = $windows | Sort-Object `
        @{ Expression = { ($_.Rect.Right - $_.Rect.Left) * ($_.Rect.Bottom - $_.Rect.Top) } } `
        -Descending | Select-Object -First 1
    $rect = $window.Rect
    $screen = [System.Windows.Forms.Screen]::PrimaryScreen.Bounds
    $width = $rect.Right - $rect.Left
    $height = $rect.Bottom - $rect.Top
    if ($width -lt ($screen.Width - 2) -or $height -lt ($screen.Height - 2)) {
        throw "Window is not fullscreen: ${width}x${height}, expected $($screen.Width)x$($screen.Height)"
    }

    Write-Output "GUI smoke passed: window handle=$($process.MainWindowHandle), size=${width}x${height}"
}
finally {
    if (-not $process.HasExited) {
        Stop-Process -Id $process.Id -Force
    }
}
