# Tile editor

Part of [Taskbar tiles](../taskbar.md). Other settings: [window/settings.md](../window/settings.md).

The editor is built into the window settings; it is opened on its own by
`TaskbarMetrics.Window.exe --editor` (or `cargo run --release --offline --bin metrics-window -- --editor`).
It runs on the system UWP XAML (XAML Islands) without additional crates, the Windows App SDK
or WebView2. The markup is embedded in the EXE: `src/platform/metrics_window/settings.xaml`, in the window —
`src/platform/metrics_window/monitor/settings.xaml`.

- CPU/GPU/RAM/NET/DISK previews in the light and dark themes use the same
  `metric_tile.xaml` template, point updates and Composition clip as on the taskbar. The earlier
  chart points are a fixed simulated wave; the number and the last point come from the preview
  value fields. In the window, "Live values" fills those fields from the newest recorded sample
  and locks them; "Test" makes them editable again.
- Fields and sliders change the CPU/GPU width (the other tiles follow with their own offset),
  spacing, radius, three text sizes, line width, fill opacity and the X/Y
  offset of the numbers. The height stays 36 px.
- The left part of the chart has separate settings: the geometry start from the left edge of the
  tile, the fade start relative to the right edge of the text, the fade length and
  the opacity of the left part. A length of 0 means a hard edge, an opacity of 100%
  removes the fade.
- Colors are set with HEX fields (`#RRGGBB` and `#AARRGGBB`) separately for the light and dark themes:
  tile background, header, main/hot number, temperature, download arrow, upload
  text, main line, fill, temperature and upload lines, hover, pressed and the background behind
  the tiles. Invalid input keeps the last valid color. Next to each field is a
  swatch on a checkerboard background: the button opens the app's own color picker in a Flyout:
  a saturation/value area, hue and alpha bars, HEX and alpha % fields, 8 presets and a
  before/after swatch. The picker starts from the field's color; "Done" writes the color into
  the field and closes the picker, "Cancel" closes it without changes.

The style draft is saved automatically to `taskbar-metrics.editor`, the palettes to
`taskbar-metrics.colors`; on startup the valid saved values are restored, except
"Dashed temperature line", which starts unchecked.
The controller checks the parameters every 16 ms on its own (at most every 25 ms in the window, while the settings are open) and
rebuilds only the preview on a change;
the fields are not recreated, focus is kept. After layout the fade of the
charts under the text is recalculated separately. "Apply to taskbar" exists only in the window's
settings: it saves the geometry, typography, dashed line, palettes and thresholds to
`taskbar-metrics.appearance`, and the taskbar picks them up within a second without restarting
Explorer.
