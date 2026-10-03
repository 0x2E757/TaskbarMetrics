param([Parameter(Mandatory=$true)][string]$Destination)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'package-names.ps1')
New-Item -ItemType Directory -Force -Path $Destination | Out-Null
$release = Join-Path $projectRoot 'target\release'
# Beside the Cargo build only the PawnIO module is added; a package gets the
# collector under its packaged name.
if ([IO.Path]::GetFullPath($Destination).TrimEnd('\') -ne [IO.Path]::GetFullPath($release).TrimEnd('\')) {
    $sourceExe = Join-Path $release 'metrics-sensors.exe'
    $targetExe = Join-Path $Destination $PackageNames['metrics-sensors.exe']
    # After Explorer exits, the collector's lifetime check can take one second.
    for ($attempt = 0; ; $attempt++) {
        try { Copy-Item -LiteralPath $sourceExe -Destination $targetExe; break }
        catch {
            if ($attempt -ge 20) { throw }
            Start-Sleep -Milliseconds 100
        }
    }
}
$moduleDestination = Join-Path $Destination 'pawnio'
New-Item -ItemType Directory -Force -Path $moduleDestination | Out-Null
Get-ChildItem -LiteralPath (Join-Path $projectRoot 'vendor\pawnio') | Copy-Item -Destination $moduleDestination -Recurse -Force
