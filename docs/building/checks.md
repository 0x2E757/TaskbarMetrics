# Checks

Part of [Building and running](../building.md).

```powershell
cargo fmt --check
cargo test --offline
cargo clippy --offline --all-targets -- -D warnings
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-abi.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools\verify-markdown.ps1
.\target\package\TaskbarMetrics.Window.exe --verify-monitor
.\target\package\TaskbarMetrics.Window.exe --verify-settings
.\target\package\TaskbarMetrics.Window.exe --verify-tiles
```

The tests cover composition with a new metric, error isolation, GPU aggregation,
configuration, placement, COM identity/lifetime and a real Windows DispatcherQueue.
`verify-abi.ps1` checks GUIDs and the interface slots in use against the installed SDK;
the path of another version can be passed via `-Sdk`. The x64 SDK offsets of the ETW and
`NtQuerySystemInformation` structures are written next to the code that uses them.
`verify-markdown.ps1` keeps Markdown files short: 5000 characters for the root `AGENTS.md`,
10 000 for the root `README.md`, 3000 for any other. The window checks
are described in [window/checks-and-demo.md](../window/checks-and-demo.md) and
[taskbar/checks.md](../taskbar/checks.md).

Running on multiple monitors, theme/DPI changes and different Windows builds require
separate manual checking; the automated tests do not replace it.
