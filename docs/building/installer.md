# Installer and portable

Part of [Building and running](../building.md).

```powershell
winget install JRSoftware.InnoSetup --scope user
powershell -NoProfile -ExecutionPolicy Bypass -File tools\release.ps1
```

`release.ps1` runs [`build.ps1`](../building.md) into `target\dist\package`, so a copy attached
from `target\package` keeps running, and writes to `target\dist`:

| File | What it is |
|---|---|
| `TaskbarMetrics-Setup-<version>.exe` | The installer, from `installer\taskbar-metrics.iss` |
| `TaskbarMetrics-Portable-<version>.exe` | The whole package in one executable |
| `SHA256SUMS.txt` | Checksums of both |

The version is the one in `Cargo.toml`.

On GitHub, `.github/workflows/release.yml` does the same for a pushed tag `v<version>` that
matches `Cargo.toml` and attaches the files to a draft release; `ci.yml` runs the formatting
check, Clippy and the tests on every push to `main` and every pull request.

## Setup

Asks for administrator rights once and, in English or Russian:

- installs into `Program Files\Taskbar Metrics` (no folder choice);
- registers the Task Scheduler tasks `\Taskbar Metrics\History` and `\Taskbar Metrics\Sensors`
  (`TaskbarMetrics.exe --register-tasks`). Each starts its collector as `--serve <Explorer PID>`
  with the highest rights of the user who runs it, in that user's session. Users may read and
  run them, only administrators change them. The launcher runs a task only when it starts the
  launcher's own copy, so the collectors of an installed copy start without a UAC prompt; a
  portable or development copy still asks;
- "Start with Windows" writes the same Run value as the window's switch;
- "Install the PawnIO driver", offered while PawnIO is missing, downloads the official
  PawnIO 2.2.0 installer from GitHub, checks its SHA-256 and runs it silently. Its terms do
  not cover bundling, so it is not carried inside;
- starts the program with `--autostart`.

An update first runs the installed `TaskbarMetrics.exe --unload`: the tray icon and the window close and
Explorer restarts, which frees the DLL. Uninstalling unloads the same way, removes the tasks
and the Run value when it starts this copy, and keeps the [data folder](../files.md) and PawnIO,
which other programs may use.

Without code signing, SmartScreen warns about both executables.

## Portable

Carries the package files (`src/bin/taskbar-metrics-portable.rs`, built with
`--features portable`) and writes them to `%LOCALAPPDATA%\Taskbar Metrics\bin`, then runs the
launcher there with the same arguments. Files that already match stay untouched. When a newer
portable finds its files in use, it unloads like Setup does, replaces them and retries
the attach for 30 s while the taskbar restarts. The Run value points into `bin`, so it survives
the portable file moving. The collectors ask for UAC, as nothing is installed.
