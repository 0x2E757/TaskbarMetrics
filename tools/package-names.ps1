# Cargo's name of each packaged file, then the name it is packaged under. The same
# list as Executables::PACKAGED in src/platform/executables.rs; a test compares them.
$PackageNames = [ordered]@{
    'taskbar-metrics.exe' = 'TaskbarMetrics.exe'
    'metrics-window.exe' = 'TaskbarMetrics.Window.exe'
    'metrics-history.exe' = 'TaskbarMetrics.History.exe'
    'metrics-sensors.exe' = 'TaskbarMetrics.Sensors.exe'
    'taskbar_metrics_host.dll' = 'TaskbarMetrics.Host.dll'
}
