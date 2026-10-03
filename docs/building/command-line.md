# TaskbarMetrics.exe

Part of [Building and running](../building.md).

Running it without arguments attaches the component to the current Explorer and exits
once the placement is confirmed. There is no separate window or tray icon. After a
successful attach (or code `2`) it also starts `TaskbarMetrics.History.exe` unless it is
running; with `process_monitoring=true` (the default) it starts it elevated: through the
installed task, otherwise with a UAC prompt.

- `--sample` prints three measurements without attaching to Explorer.
- `--stop` stops collection and queues the removal of the elements on the UI queue; the
  history and CPU temperature collectors exit too. Running it again
  resumes work and rereads the configuration.
- `--sensors` starts the CPU temperature sensor (a UAC prompt unless installed).
- `--autostart` — the autostart mode at Windows sign-in ([startup-and-window.md](../window/startup-and-window.md)).
- `--register-tasks` and `--remove-tasks` — the installer's elevated steps
  ([installer.md](installer.md)): Task Scheduler tasks that start the collectors of a copy in
  Program Files elevated without a UAC prompt. Other copies are refused.
- `--unload` closes the window and ends Explorer, which Windows restarts by itself, so the
  DLL and executables can be replaced or removed.
- `--help` prints the usage. Any other argument, or more than one, prints it with code `1`.

Run it as a normal user in the same session and at the same integrity level as
Explorer. Administrator rights are not required.

Exit codes: `0` — success; `1` — error; `2` — already running or another
attach is in progress; `3` — display not confirmed within 15 seconds. With code `3`
the session may keep running; it can be stopped with `--stop`.

Diagnostics go to `OutputDebugString` with the `[TaskbarMetrics]` prefix and, if the directory
is writable, to `taskbar-metrics.log` in the [data folder](../files.md); `tools\update-package.ps1` prints its last 30 lines after
redeploying.
