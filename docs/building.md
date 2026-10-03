# Building and running

You need Rust with the `x86_64-pc-windows-msvc` target, the MSVC linker and the Windows SDK.

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\build.ps1
.\target\package\TaskbarMetrics.exe --sample
.\target\package\TaskbarMetrics.exe
.\target\package\TaskbarMetrics.exe --stop
```

`build.ps1` runs `cargo fmt --check`, `cargo test` and `cargo clippy` (skip them with
`-SkipTests`; the other [checks](building/checks.md) are run by hand), builds release and puts
the EXEs, DLL, `README.md` and the `pawnio` modules into `target/package`.

- [Installer and portable](building/installer.md): the release files and how they install
- [TaskbarMetrics.exe](building/command-line.md): arguments, exit codes, diagnostics
- [Icon and description](building/icon-and-description.md) embedded into every exe
- [Checks](building/checks.md) to run before a commit

## Package

Cargo names the files its own way (`taskbar-metrics.exe`, `metrics-window.exe`,
`taskbar_metrics_host.dll`…: dots are not allowed in its names). The files get their package names
when copied according to the list in `tools\package-names.ps1`; the same list in code is
`src/platform/executables.rs`, and a test checks that they match.

| File | Purpose |
|---|---|
| `TaskbarMetrics.exe` | Attaches the tiles to Explorer, stops them, starts the collectors |
| `TaskbarMetrics.Host.dll` | The tiles inside Explorer |
| `TaskbarMetrics.Window.exe` | The history window and settings |
| `TaskbarMetrics.History.exe` | Background history ([history.md](history.md)) |
| `TaskbarMetrics.Sensors.exe` | CPU temperature ([cpu-temperature.md](cpu-temperature.md)) |

The programs find each other by their package names and must sit side by side, so
they have to be run from `target\package`.

## Replacing the DLL

The DLL stays pinned until Explorer exits, so that callbacks do not call
unloaded code. Stop/resume is supported, but replacing the DLL binary
requires Explorer to exit or the user to sign out. Only `TaskbarMetrics.exe --unload`, which
Setup and the portable executable run before replacing files, restarts the shell; after Explorer
restarts, the EXE has to be run again. Autostart (window settings) attaches only at sign-in, not
after an Explorer restart.

For development, `tools\update-package.ps1 -ExplorerPid <PID>` restarts Explorer,
updates `target\package`, attaches the tiles again and, if the temperature sensor was running,
starts it again.
