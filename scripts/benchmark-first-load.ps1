[CmdletBinding()]
param(
    [string]$Binary = '',
    [Parameter(Mandatory)]
    [string[]]$File,
    [int]$Runs = 5,
    [int]$TimeoutSeconds = 10
)

$ErrorActionPreference = 'Stop'
if ([string]::IsNullOrWhiteSpace($Binary)) {
    $Binary = Join-Path $PSScriptRoot '..\target\release\widno.exe'
}
$binaryPath = (Resolve-Path -LiteralPath $Binary).Path
$timingPath = Join-Path $env:TEMP 'widno-load-times.csv'
$env:WIDNO_LOAD_TIMING = '1'

$results = foreach ($path in $File) {
    $resolvedPath = (Resolve-Path -LiteralPath $path).Path
    Remove-Item -LiteralPath $timingPath -Force -ErrorAction SilentlyContinue

    for ($run = 1; $run -le $Runs; $run++) {
        $process = Start-Process -FilePath $binaryPath `
            -ArgumentList @("`"$resolvedPath`"") -PassThru
        Wait-Process -Id $process.Id -Timeout $TimeoutSeconds -ErrorAction SilentlyContinue
        if (-not $process.HasExited) {
            Stop-Process -Id $process.Id -Force
        }
    }

    if (-not (Test-Path -LiteralPath $timingPath)) {
        throw "Widno did not produce timing data for $resolvedPath"
    }

    $samples = @(Get-Content -LiteralPath $timingPath | ForEach-Object {
        $columns = $_ -split ','
        [pscustomobject]@{
            Timestamp = [long]$columns[0]
            Milliseconds = [int]$columns[1]
            Kind = $columns[2]
        }
    })
    if ($samples.Count -lt $Runs) {
        throw "Expected $Runs timing samples for $resolvedPath, got $($samples.Count)"
    }

    $values = @($samples.Milliseconds | Sort-Object)
    [pscustomobject]@{
        File = Split-Path -Leaf $resolvedPath
        Kind = $samples[0].Kind
        Samples = $values.Count
        MedianMs = $values[[int][math]::Floor(($values.Count - 1) / 2)]
        MinMs = $values[0]
        MaxMs = $values[$values.Count - 1]
    }
}

$results | Format-Table -AutoSize
Write-Output "Raw samples: $timingPath"
