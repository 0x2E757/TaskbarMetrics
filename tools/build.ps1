param([switch]$SkipTests, [string]$Destination)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
Push-Location $projectRoot
try {
    if (-not $SkipTests) {
        & cargo fmt --check
        if ($LASTEXITCODE -ne 0) { throw 'Formatting check failed' }
        & cargo test --offline
        if ($LASTEXITCODE -ne 0) { throw 'Tests failed' }
        & cargo clippy --offline --all-targets -- -D warnings
        if ($LASTEXITCODE -ne 0) { throw 'Clippy failed' }
    }
    & cargo build --release --offline
    if ($LASTEXITCODE -ne 0) { throw 'Release build failed' }
    & (Join-Path $PSScriptRoot 'package-sensors.ps1') -Destination (Join-Path $projectRoot 'target\release')
    $destination = if ($Destination) { $Destination } else { Join-Path $projectRoot 'target\package' }
    New-Item -ItemType Directory -Force -Path $destination | Out-Null
    . (Join-Path $PSScriptRoot 'package-names.ps1')
    # The collector is copied by package-sensors.ps1 below.
    foreach ($name in $PackageNames.Keys | Where-Object { $_ -ne 'metrics-sensors.exe' }) {
        Copy-Item -LiteralPath (Join-Path $projectRoot "target\release\$name") -Destination (Join-Path $destination $PackageNames[$name])
    }
    Copy-Item -LiteralPath (Join-Path $projectRoot 'README.md') -Destination $destination
    & (Join-Path $PSScriptRoot 'package-sensors.ps1') -Destination $destination
    Write-Output "Package: $destination"
} finally {
    Pop-Location
}
