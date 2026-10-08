# Chart series and drawing order

[UI kit](../UI-KIT.md)

Order (bottom to top):
1. Grid and axes.
2. Fill of the total line.
3. Fills of pinned processes (in pinning order).
4. Total line.
5. Lines of pinned processes.
6. Temperature.
7. [Overlays](chart-overlays.md#overlays) and the direction chips.
8. Hovered process (fill + line), on a separate layer.
9. [Moment marker](chart-overlays.md#moment-marker) and its dots, on the same layer.

| Series | Line | Fill |
|---|---|---|
| Total | `total`, width *w* | `total` α `totalFill`, from zero |
| Pinned | `pins[k]`, *w* × 1.25/1.5 | `pins[k]` α `pinFill`, **each from zero** (not stacked) |
| Temperature | `temp`, 1.25, dashed 4/3 anchored to time, 2.5 s moving average (otherwise 1 °C fluctuations break the dashes) | — |
| Hover | `ink`, *w* × 1.75/1.5 | `ink` α `hoverFill` |

- *w* is the "Chart line width" setting: 1–4 in steps of 0.25, default 1.
- **Dimming on hover:** while a process is hovered, layers 2–6 together are drawn at opacity 0.6.
- **Gaps:** a single missing frame (unhatched) is bridged: the two samples are joined by a line in the series style and the fill goes on under it, both at 50 % of their opacity; the temperature dashes run on across it at 50 % too. The line and the fill break at every `None` and at 2 or more missing frames; neighboring points are not joined across them. A process that started later begins at its first sample; one that exited ends at its last.
- **Zero:** when a process exists and consumes 0, a line is drawn along the axis.
- **Mirrored mode (DISK/NET):**
  - The middle of the plot is zero (a line in `border`).
  - Read/receive go up, write/send go down, on a shared scale.
  - The fill of the lower half uses the same α ×0.85.
  - Every series, including hover, has both halves in the same color.
  - At the left edge inside the plot, chips 11/600 `text2` on `card` α 0.9, radius 4: at the top `↓ Read` / `↓ Receive`, at the bottom `↑ Write` / `↑ Send`.
- **Temperature > 100 °C:**
  - The line runs into the top of the plot.
  - Over each span where the value is above 100, a strip of thickness 3 in `temp` with rounded ends runs along the top edge.
  - Over the middle of the span with the highest peak, one label "↑ above scale, peak 106 °C": 11/600 `temp`, background `card`, border `temp` α 0.5.

## Updates and performance

- The buffer is a ring of 600 samples, collected in the background independently of the window.
- The chart is redrawn on every new sample (2×/s). The numbers (stats, navigation, table) are refreshed 1×/s from a single sample: the newest one, or in DISK/NET live the one 4 samples (2 s) back, for ETW.
- In history the axis is frozen, but the buffer keeps being written. While the window is hidden, the UI is not drawn; collection continues.
- Rendering: the base chart is rebuilt only on a new sample, a resize, a change of series or settings; hover and the marker are drawn on a separate lightweight layer.
