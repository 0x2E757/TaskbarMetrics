# Startup and window

Part of [Settings](settings.md) of the [History window](../window.md).

- "Start when signing in to Windows" writes the `Taskbar Metrics` value to
  `HKCU\Software\Microsoft\Windows\CurrentVersion\Run` (without administrator rights).
  It runs `TaskbarMetrics.exe --autostart`: attaches the tiles and starts the history
  recorder, as a manual start does (on an error, for example while the taskbar is not ready
  yet, it retries every 5 s for a minute). Only after a successful attach does it start the CPU
  temperature sensor, if it is installed. The window does not open. No console appears:
  the `TaskbarMetrics.exe` manifest sets `consoleAllocationPolicy=detached` (Windows 11
  24H2+; in a terminal the output works as usual, on older builds a console flashes). An
  installed copy starts the elevated collectors without prompts ([installer.md](../building/installer.md));
  for any other copy with UAC on, sign-in can bring up two prompts: the history recorder runs
  elevated while `process_monitoring` is on (the default), then the sensor. A value left by a copy from another folder
  or by an earlier version is shown as off and replaced when turned on.
- "Reset the window size and position when it closes" is on by default: a closed
  window opens in the middle of the main monitor's work area at 1280 × 1024, or smaller
  if the work area is smaller. Unchecked, the window opens where it was closed, including
  maximized; a frame that is on no monitor any more opens centered. The window cannot be
  made smaller than 780 × 960 (or the monitor's work area, if that is smaller). The flag
  and position are stored in `taskbar-metrics.window`. Checks and demo always open centered
  and never write the file, but the settings page reads it to show the check box.
