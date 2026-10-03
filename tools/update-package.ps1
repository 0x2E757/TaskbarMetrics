param([Parameter(Mandatory=$true)][int]$ExplorerPid)
$ErrorActionPreference = 'Stop'
$projectRoot = Split-Path -Parent $PSScriptRoot
. (Join-Path $PSScriptRoot 'package-names.ps1')
$destination = Join-Path $projectRoot 'target\package'
$shellPath = Join-Path $env:WINDIR 'explorer.exe'
$shellProcess = Get-Process -Id $ExplorerPid -ErrorAction Stop
if ($shellProcess.ProcessName -ne 'explorer' -or $shellProcess.Path -ne $shellPath) {
    throw 'The specified process is not Windows Explorer.'
}
$currentSession = (Get-Process -Id $PID).SessionId
if ($shellProcess.SessionId -ne $currentSession) { throw 'Explorer belongs to a different session.' }
foreach ($file in $PackageNames.Keys) {
    if (-not (Test-Path -LiteralPath (Join-Path $projectRoot "target\release\$file"))) {
        throw "Missing release artifact: $file"
    }
}
Get-Process TaskbarMetrics.Window -ErrorAction SilentlyContinue | Where-Object Path -eq (Join-Path $destination 'TaskbarMetrics.Window.exe') | ForEach-Object {
    $_.CloseMainWindow() | Out-Null
    if (-not $_.WaitForExit(3000)) { throw 'Close the metrics window before updating its executable.' }
}
# The CPU temperature collector exits with its Explorer; it is started again below.
# It runs elevated, so its path is not readable from here: the name has to do.
$sensorsRunning = $null -ne (Get-Process TaskbarMetrics.Sensors -ErrorAction SilentlyContinue)
Stop-Process -InputObject $shellProcess -ErrorAction Stop
$shellProcess.WaitForExit(10000) | Out-Null
New-Item -ItemType Directory -Force -Path $destination | Out-Null
# The collector is copied by package-sensors.ps1 below.
foreach ($file in $PackageNames.Keys | Where-Object { $_ -ne 'metrics-sensors.exe' }) {
    for ($attempt = 0; ; $attempt++) {
        try { Copy-Item -LiteralPath (Join-Path $projectRoot "target\release\$file") -Destination (Join-Path $destination $PackageNames[$file]); break }
        catch { if ($attempt -ge 30) { throw }; Start-Sleep -Milliseconds 100 }
    }
}
Copy-Item -LiteralPath (Join-Path $projectRoot 'README.md') -Destination $destination
& (Join-Path $PSScriptRoot 'package-sensors.ps1') -Destination $destination
# Explorer can start before Shell_TrayWnd/XAML is ready. A fixed sleep was not
# sufficient: the script returned ERROR_NOT_FOUND and left the panel detached.
$appExit = 1
for ($attempt = 0; $attempt -lt 15; $attempt++) {
    $replacement = Get-Process explorer -ErrorAction SilentlyContinue | Where-Object { $_.SessionId -eq $currentSession }
    if ($null -eq $replacement) { Start-Process -FilePath $shellPath -WindowStyle Hidden }
    Start-Sleep -Seconds 2
    & (Join-Path $destination 'TaskbarMetrics.exe')
    $appExit = $LASTEXITCODE
    if ($appExit -eq 0 -or $appExit -eq 2) { break }
}
if ($sensorsRunning -and ($appExit -eq 0 -or $appExit -eq 2)) {
    & (Join-Path $destination 'TaskbarMetrics.exe') --sensors
}
$logPath = Join-Path $env:LOCALAPPDATA 'Taskbar Metrics\taskbar-metrics.log'
if (Test-Path -LiteralPath $logPath) { Get-Content -LiteralPath $logPath -Tail 30 }
exit $appExit
