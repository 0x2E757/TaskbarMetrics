# History window

Click a taskbar tile or run `TaskbarMetrics.Window.exe` (no arguments means CPU;
`--monitor gpu`, `ram`, `disk`, `net`, `disk@D:`…). It is a separate process using system XAML,
with no third-party libraries and no WebView. Clicking a tile again raises the already open window and
switches the section.

## Chart and table

Up to 5 minutes of history in 500 ms steps, time selection, the process table with search,
pinned processes (`taskbar-metrics.pins`) and the hover highlight:
[window/chart-and-table.md](window/chart-and-table.md).

## Devices in the menu

How graphics cards and drives are numbered and named, drive types and letters, the RAM modules
subtitle, network adapters, and the "Show on taskbar" / "Always monitor" check boxes:
[window/devices-in-the-menu.md](window/devices-in-the-menu.md).

## Sections

What the chart and the table rows of each section mean:

- GPU and RAM: [window/sections-gpu-and-ram.md](window/sections-gpu-and-ram.md);
- DISK and NET, including the "All time" column:
  [window/sections-disk-and-net.md](window/sections-disk-and-net.md).

## Settings

The settings page with its "General" tab (background monitoring, window theme, chart line
width, display language) and "Taskbar" tab (tile editor, colors, alerts, value simulation):
[window/settings.md](window/settings.md).
Autostart at sign-in (`--autostart`) and the window size and position reset:
[window/startup-and-window.md](window/startup-and-window.md).

## Updates and rendering

How the window receives history and redraws only what changed:
[window/updates-and-rendering.md](window/updates-and-rendering.md). Closing the window does not
stop the background history.

## Checks and demo

`--verify-monitor` and the synthetic history `--demo` with `--theme` and `--language`:
[window/checks-and-demo.md](window/checks-and-demo.md).
