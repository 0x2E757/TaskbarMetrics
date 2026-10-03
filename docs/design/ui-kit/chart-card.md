# ChartCard

[UI kit](../UI-KIT.md)

- A card. The legend is at the top: padding 10 16 0, text 12 `text2`, marker 6 from its label, gap 16, 6 between lines.
- Right after the "Show on taskbar" and "Always monitor" checkboxes (20 apart) comes the **InfoButton**, so it takes no width from the stats and the legend: height 26, padding 0 8, icon `info` 14 + "How it’s measured" 12 `text2`, 6 apart. While the pointer is over it or it has keyboard focus, a card is shown below it (see [TeachingTip](teaching-tip.md)). The button has no hover or pressed states of its own, only the system keyboard focus rectangle.
- Then the chart itself, padding 4 16 8.

**Legend items** (marker + label; the kind is conveyed by the marker shape, not only by color):

| Kind | Marker | When shown |
|---|---|---|
| Total line | 16×2 line in `total` | always: "Total load" (CPU), "Busiest engine" (GPU), "RAM in use", "Disk total (PDH)", "Total (PDH)" (NET); "Total" below 1100 |
| Temperature | 16×2 line, dashed 3/2, in `temp` | CPU, GPU: "Temperature" ("Temp." below 1100) |
| Pinned process | 14×10 rectangle, radius 2, fill in the color at α 0.3, top edge 2 px | one per pinned process, while process monitoring is on |
| Pinned, but no data at the moment | 14×10 rectangle, radius 1.25, dashed 1.5 | in history, if there is no value at the moment |
| Moment | vertical 2×12 bar in `ink` | in history, in windows ≥ 1100 |
| No samples / awaiting ETW / no shared memory | hatched 14×10 rectangle, radius 2, border `border` | when the visible span has a gap of 2+ samples, in DISK/NET live with process monitoring on, or (RAM) when the window was closed |
