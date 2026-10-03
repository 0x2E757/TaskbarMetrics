# Taskbar Metrics

CPU, GPU, RAM, DISK and NET metrics on the left side of the stock **Windows 11 x64** taskbar.
Rust, the standard library and system Windows DLLs. No third-party crates, services,
Windhawk, C/C++, .NET or COM registration in the registry.

- **Taskbar tiles** with numbers and charts; every drive (mapped SMB shares included), network adapter and
  graphics card can have its own tile. Reorder them by dragging.
- **History window**: 5-minute charts, picking a moment on the chart and the processes that
  used resources at exactly that time; process pinning, search, light and dark themes,
  English and Russian.
- **Background history** is recorded even while the window is closed; DISK/NET process history comes from ETW.
- **Temperature** of the GPU via WDDM and of the CPU via the official PawnIO driver.
- **Settings** for tiles, colors and alerts right in the window, autostart when signing in to Windows.

## Quick start

Install with `TaskbarMetrics-Setup-<version>.exe`, or run `TaskbarMetrics-Portable-<version>.exe`
without installing ([how they differ](docs/building/installer.md)). To build from source,
you need Rust with the `x86_64-pc-windows-msvc` target, the MSVC linker and the Windows SDK.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\build.ps1
.\target\package\TaskbarMetrics.exe            # attach the tiles to Explorer
.\target\package\TaskbarMetrics.Window.exe     # open the history window
.\target\package\TaskbarMetrics.exe --stop     # close the program, as the tray icon does
```

Administrator rights are needed only for DISK/NET process history (ETW) and the CPU
temperature. Setup asks once; the portable and a source build ask at each start.

## Documentation

| Page | About |
|---|---|
| [History window](docs/window.md) | Charts, process table, pins, window settings |
| [Taskbar tiles](docs/taskbar.md) | Placement, appearance, charts, tile editor |
| [Metrics](docs/metrics.md) | Where each number comes from |
| [Configuration](docs/configuration.md) | `taskbar-metrics.conf` and the other data files |
| [Background history](docs/history.md) | Collector, CSV export, limitations |
| [CPU temperature](docs/cpu-temperature.md) | PawnIO, supported processors |
| [Building and running](docs/building.md) | Package, arguments, exit codes, checks |
| [Architecture](docs/architecture.md) | Components, adding a metric, integration risks |
| [Contributing](CONTRIBUTING.md) | Code rules, commit types and scopes |
| [UI kit](docs/design/UI-KIT.md) | Window specification and [tokens](docs/design/tokens.json) |

## Limitations

The integration uses XAML Diagnostics and Explorer's internal tree — this is not a public
API, and a Windows update may require changes to the platform adapter. Replacing the DLL
requires restarting Explorer. Windows 10, Windows Server, ARM64 and third-party taskbar
replacements are not supported. See [Architecture](docs/architecture.md) for details.

## License

[MIT](LICENSE). The PawnIO modules in `vendor/pawnio` are distributed under
LGPL-2.1-or-later — see [vendor/pawnio/README.md](vendor/pawnio/README.md). Security issues:
[SECURITY.md](SECURITY.md).
