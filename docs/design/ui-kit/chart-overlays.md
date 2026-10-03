# Chart moment marker and overlays

[UI kit](../UI-KIT.md)

## Moment marker

- A vertical `ink` 1.25 line from `y0 − 4` to `y1`.
- Above the plot (26 above `y0`), a label centered on the line: height 20, padding 0 7, radius 4, background `ink`, text `card`, 11/600, icon `clock` 11, time `HH:MM:SS.d`.
- At the intersections with each visible series (temperature on its smoothed line), dots r 3.25: fill `card`, 2 stroke in the series color. If there is no value, there is no dot.
- The label position is confined to the plot: near the edges the label shifts inward while the line stays in place.

## Overlays

Vertical 1 px lines (the time grid, dashed boundaries) are placed in the middle of a physical pixel, taking the window DPI into account: the chart moves 1.4 px per sample, and a line at a fractional x would flicker between crisp and smeared over 2 px. Dashed boundaries use dashes 3/3. Chips: 11 `text2`, background `card`, border `border`, radius 4.

- **No samples** (collection was interrupted, or the device was not recorded):
  - Spans of at least 2 missing samples get 135° hatching, 1.5 px lines in `hatch`, step 6, over the full plot height.
  - From 8 missing samples (4 s), the chip "no samples · 21 s" at the bottom center of the span.
- **Awaiting ETW** (DISK/NET, live only):
  - Hatching (step 5) of width `max(4 px, 4 samples)` at the right edge with a dashed left boundary in `text3`. Process series are not drawn in this zone.
  - At the top right, 10 inside the plot, a chip: `clock` 12 + "per-process · ETW ~2 s".
- **No shared memory** (RAM: the window was closed, the shared pages of processes were not scanned):
  - Hatching with step 5 over the frames where processes have no shared memory; dashed `text3` boundaries on both sides of the span, except at the plot edges.
  - At the top center of the span, the chip "shared memory not measured", if the span is 40 px wider than the text.
  - The legend item "Window closed: no shared memory".
- **Empty history** (less than 5 min collected):
  - Left of the first sample, a dashed vertical line in `text3`.
  - Centered in the empty area: icon `history` 20 `text3`, "Collecting history" (13/600), "Collected 0:42 of 5:00. Left of the data is not zero but missing samples." (12 `text2`).
  - If the empty area is narrower than 200 px, the text is not shown.
