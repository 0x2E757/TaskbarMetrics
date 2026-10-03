# Implementation constraints

[UI kit](../UI-KIT.md)

- Rust without third-party crates, system XAML (`Windows.UI.Xaml`), charts drawn by our own renderer.
- WinUI 2 (`Microsoft.UI.Xaml`) is an external dependency, so its resources (`CardBackgroundFillColorDefaultBrush` and the like) are not available. Colors are not theme resources either: the XAML templates carry `$token$` placeholders that are replaced with the token's value for the current theme (`$card$` → `#FFFFFF` / `#28292C`).
- **Accent** is the fixed `accent` token (`#005FB8` / `#60CDFF`, the Windows defaults); the system accent color is not read.
- **Chart colors** (`total`, `pins`, `temp`, `ink`) are fixed. Otherwise the pinned-process palette could coincide with the color of the total line.
- **Theme.** Follows the Windows app theme (`UISettings` background color, checked every second) unless the settings choose Light or Dark. It switches on the fly, without a restart: the window markup is rebuilt.
- **Sizes** are in DIP. Line widths and font sizes scale with DPI only, never with the window size.
- **Icons.** Our own, 16×16, as `Path` (data in [`tokens.json`](../tokens.json) → `icons.paths`). Stroke `currentColor` 1.3. Stroke 1.6: the moment label `clock` 11, the "above scale" arrow 11, the direction-chip arrows 12 and the 16 px stats `alert`; the other icons ≤ 12 px (search clear `x`, sort chevron, stats `info`, ETW chip `clock`) keep 1.3. Elements with `fill: true` are filled, not stroked. Navigation icons (devices and the settings gear) use a 1 px stroke on half pixels (`x.5`) and fills on whole pixels, so the lines are crisp at 100 %. The network adapter is `wifi` (arcs and a dot) or `ethernet` (a monitor with a cable), by adapter type, as in Segoe Fluent. The logo is a blue square with a thin white pulse line; the large window icon is drawn at 24 px at 96 DPI, the size of a taskbar icon.
