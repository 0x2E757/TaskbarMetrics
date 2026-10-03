# Taskbar Metrics — UI kit (variant A)

Specification for updating the whole UI of the main window. Numeric values are in [`tokens.json`](tokens.json) (next to this file); the parts below cover structure, behavior and rules. If the text and `tokens.json` disagree, `tokens.json` wins. Artboards to check against: [Visual reference](ui-kit/visual-reference.md).

## Parts

0. [Implementation constraints](ui-kit/constraints.md) — Rust, system XAML, token placeholders, accent, chart colors, theme, DIP, icons.
1. Window
   - [Window frame](ui-kit/window-frame.md) — system title bar, navigation pane, content layer, stack Header → Device options → Stats → Chart → Table, chart height, 780 × 960 minimum.
   - [Responsiveness](ui-kit/responsiveness.md) — width classes ≥ 1100 and 760–1099 (< 760 never occurs: minimum width 780).
2. [Navigation](ui-kit/navigation.md) — items, sections, selected/hover, 56 mode.
3. Section content
   - [Header](ui-kit/header.md) — subtitles, ModePill, BackToLive, Kbd.
   - [Stats](ui-kit/stats.md) — blocks per section, hints, wrapping, alerts.
   - [ChartCard](ui-kit/chart-card.md) — legend, InfoButton, legend items.
   - [TableCard](ui-kit/table-card.md) — toolbar, header, columns, list body, footer, scrolling.
   - [TeachingTip "How it’s measured"](ui-kit/teaching-tip.md) — look and texts per section.
4. Chart (own renderer)
   - [Geometry and scales](ui-kit/chart-geometry.md) — plot area, grid, axes, scales, samples.
   - [Series and drawing order](ui-kit/chart-series.md) — layers, series styles, hover dimming, mirrored mode, > 100 °C, updates and performance.
   - [Moment marker and overlays](ui-kit/chart-overlays.md) — marker, no samples, awaiting ETW, no shared memory, empty history.
5. State model and interactions
   - [State model, moment and hover](ui-kit/state-model.md) — modes, history, hover, chart tooltip.
   - [Pins, search and keyboard](ui-kit/pins-and-search.md) — pinning, search, Top 10 / All, persistence.
6. Controls and their states
   - [Buttons, rows, search](ui-kit/controls.md) — common focus rules, Button, Primary, Subtle, PinButton, Row, SearchBox, Segmented.
   - [Form and settings controls](ui-kit/controls-forms.md) — ToggleSwitch, Slider, NumberBox, SettingsCard, Expander, ComboBox.
7. [Empty and special states](ui-kit/special-states.md) — empty history, monitoring off, no administrator rights.
8. Settings
   - [Structure and preview](ui-kit/settings.md) — tabs, preview card, taskbar strip.
   - [Groups](ui-kit/settings-groups.md) — Monitoring, Tile appearance, Alerts, Language, Window appearance, Startup and window.
   - [Alert range, colors, color picker](ui-kit/settings-alerts-and-colors.md).
9. Formatting and strings
   - [Formatting](ui-kit/formatting.md) — locales, precision, time, string length.
   - [Strings](ui-kit/strings.md) — EN string keys.
10. [Accessibility](ui-kit/accessibility.md) — automation names, keyboard, tab order, contrast.
11. [Acceptance](ui-kit/acceptance.md) — checklist.
