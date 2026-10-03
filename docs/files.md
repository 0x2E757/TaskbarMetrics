# Data files

The program writes to `%LOCALAPPDATA%\Taskbar Metrics`, the same folder for an installed and a
portable copy: the binaries may sit where the user cannot write, such as Program Files.

| File | What it stores |
|---|---|
| `taskbar-metrics.conf` | [Configuration](configuration.md) |
| `taskbar-metrics.appearance` | Tile style and palettes applied to the taskbar; Explorer rereads it on a sampling tick at most once a second |
| `taskbar-metrics.editor` | Tile style draft in the editor |
| `taskbar-metrics.colors` | Light and dark theme palette draft |
| `taskbar-metrics.chart` | Window chart line width |
| `taskbar-metrics.theme` | Window theme |
| `taskbar-metrics.language` | Window language |
| `taskbar-metrics.window` | Window position and the flag for resetting it on close |
| `taskbar-metrics.pins` | Pinned processes |
| `taskbar-metrics.watch` | Devices shown in the open window; the collector honors the file if it is no older than 5 s |
| `taskbar-metrics.log` | DLL diagnostics, recreated on start |
| `TaskbarMetrics.Window.error.log` | The error the window exited with |

The elevated collectors only read this folder. Their own output goes beside their EXE:

| File | What it stores |
|---|---|
| `TaskbarMetrics.History.error.log`, `TaskbarMetrics.Sensors.error.log` | The error the process exited with |
| `TaskbarMetrics.Sensors.sample.log` | Readings of `TaskbarMetrics.Sensors.exe --sample` |
| `TaskbarMetrics.History.probe.txt` | Result of `TaskbarMetrics.History.exe --probe` |

Checks (`--verify-*`) and the window demo mode (`--demo`) do not write user settings.
History is never persisted to disk.
