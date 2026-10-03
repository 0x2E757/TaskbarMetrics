# Settings

Part of [History window](../window.md). "Startup and window":
[startup-and-window.md](startup-and-window.md).

Settings is a page of the window: the "Settings" item of the navigation panel shows it in
place of the section's chart and table; opened from a section, it starts on its first tab.
It has two tabs:

- "General": "Startup and window", "Taskbar tiles" (the monitors that show them), "Monitoring"
  (the background process history switch),
  "Window appearance" (theme, chart line width) and "Language".
- "Taskbar": the tile editor with its preview and value simulation, colors and alerts.

The "Apply to taskbar" button saves
`taskbar-metrics.appearance`; Explorer rereads it once a second and keeps the
history when the appearance changes. Test values do not replace the real metrics. The
monitoring switch itself takes effect immediately. More about the editor is in
[tile-editor.md](../taskbar/tile-editor.md).

- "Window appearance → Window theme": "System" (the default; follows the Windows app
  theme and changes with it), "Light" or "Dark". Applied immediately, stored
  in `taskbar-metrics.theme`; checks and demo do not read it (`--theme` still
  sets the theme for snapshots).
- "Window appearance → Chart line width": 1–4 px in 0.25 steps, 1 by default. Changes the
  total load line; the lines of pinned processes and of the process under the cursor keep their
  proportion to it (×1.25/1.5 and ×1.75/1.5). Stored in `taskbar-metrics.chart`.
- "Startup and window": sign-in autostart and the window size and position reset, see
  [startup-and-window.md](startup-and-window.md).
- "Taskbar tiles → Monitors": a check box for each connected monitor, named by its model, size
  and "main" for the main one. A click is saved to `monitors` right away
  ([configuration.md](../configuration.md)) and the taskbars follow within a second. With every
  box unchecked no taskbar shows the tiles, and the hint under the boxes says so. When none of
  the chosen monitors is connected, the main monitor shows the tiles. The list is read when the
  page opens.
- English is the base language. Until a language is chosen, the window opens in Russian if
  Russian is the Windows display language or one of the keyboard layouts. The language is changed
  in "Settings → General → Language → Display language" without restarting the process. The choice is stored in
  `taskbar-metrics.language`. All window and settings texts are in English with their Russian
  translations in one catalog, [`locale.rs`](../../src/platform/metrics_window/monitor/locale.rs);
  the standalone tile editor (`--editor`) is English only. The texts are separate from the
  history model and chart drawing.
