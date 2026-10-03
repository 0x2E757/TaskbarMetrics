# TaskbarMetrics.exe

Part of [Building and running](../building.md).

Running it without arguments attaches the component to the current Explorer and exits
once the placement is confirmed. After a successful attach (or code `2`) it also starts
`TaskbarMetrics.History.exe` unless it is running; with `process_monitoring=true` (the
default) it starts it elevated: through the installed task, otherwise with a UAC prompt.
Unless the attach failed, it then starts the watcher, `TaskbarMetrics.exe --watch`, unless
one runs.

The watcher stays in the background with a tray icon (on Windows 11 a new icon starts among
the hidden ones) whose menu has one item, "Close Taskbar Metrics": it closes everything, as
`--stop` does. Every 2 s it checks the Explorer that shows the taskbar:

- a new Explorer (a crash, an update, a restart from the Task Manager) gets the tiles again:
  the watcher runs the launcher, every 5 s for a minute, then once a minute, until it attaches;
- a history recorder or a CPU temperature collector missing on two checks in a row is started
  again, once a minute at most. The collector is brought back only if it ran before, so a
  copy without PawnIO does not ask for it. A start that fails, such as a declined UAC prompt,
  is not tried again until the next attach; the installed copy starts the elevated ones
  through its tasks, without a prompt. The collector counts as running while it published
  within 5 s: Explorer keeps its shared memory open for a while after it ended;
- tiles stopped with `--stop` keep their collectors stopped.

It is started without inherited handles, so a script or the portable copy reading the
launcher's output does not wait for it.

- `--sample` prints three measurements without attaching to Explorer.
- `--stop` closes the program: it queues the removal of the elements on the UI queue, the
  history and CPU temperature collectors exit with them, and the metrics window and the
  watcher close. Running the launcher again resumes work and rereads the configuration.
- `--watch` runs the watcher; the launcher starts it.
- `--sensors` starts the CPU temperature sensor (a UAC prompt unless installed).
- `--autostart` — the autostart mode at Windows sign-in ([startup-and-window.md](../window/startup-and-window.md)).
- `--register-tasks` and `--remove-tasks` — the installer's elevated steps
  ([installer.md](installer.md)): Task Scheduler tasks that start the collectors of a copy in
  Program Files elevated without a UAC prompt. Other copies are refused.
- `--unload` closes the watcher, which would attach to the new Explorer, and the window, then ends Explorer, which Windows restarts by itself, so the
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
