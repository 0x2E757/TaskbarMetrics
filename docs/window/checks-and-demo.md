# Checks and demo

Part of [History window](../window.md).

`--verify-monitor` checks all tabs, time selection, the process series, settings and
both localizations on test snapshots, without writing user settings.

To compare against the [design/UI-KIT.md](../design/UI-KIT.md) specification, the window can be opened on
a synthetic history: `TaskbarMetrics.Window.exe --monitor cpu --demo history`
(scenarios `history`, `live`, `gpu`, `empty`, `off`, `settings`; theme —
`--theme dark|light`, language — `--language en|ru`). In this mode the window still opens the
collector's `.Live` pipe, but ignores what it receives and shows only the synthetic history.
Pins, theme, chart line, language, window placement, the device check boxes, the startup
options and the monitoring switch are not saved. One exception: "Apply to taskbar" on the
settings page (`--demo settings`) does write `taskbar-metrics.appearance`, and the taskbar
picks it up.
