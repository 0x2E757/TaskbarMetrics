# Chart geometry and scales

[UI kit](../UI-KIT.md) · The chart is drawn by our own renderer.

## Geometry

- Canvas height: 270, or 210 in windows narrower than 1100.
- Plot area: `x0 = pad.left`, `x1 = W − pad.right`, `y0 = pad.top`, `y1 = H − pad.bottom`.
- `pad` is 46 / 44 / 28 / 22 (left, right, top, bottom); below 1100 the sides are 40 / 36. The right padding stays the same without a temperature axis.
- `pad.top` = 28 leaves room for the moment label; it stays constant even when there is no label, so the chart does not jump.
- The X axis always covers the last 300 s (600 samples of 0.5 s). Sample *i* is drawn at `x0 + (i + 0.5)/600 · (x1 − x0)`.
- **Grid:**
  - Horizontal lines at `gridFractions` in `grid`; the zero line in `border`.
  - Vertical lines at every whole minute.
  - Minute labels `HH:MM` (11 `text3`) under the axis, centered on the line. A label is skipped if it is less than 16 from `x0`, less than 56 from `x1`, or less than 44 from the previous label.
  - The right edge is labeled 11/600 `text2`: in live "now", in history the end time of the frozen window `HH:MM:SS`.
- **Left Y axis:** labels 11 `text3`, right-aligned at `x0 − 8`, as whole numbers or with the fewest decimals needed (up to 2). Only the top label carries the unit: `100 %`, `50 MB/s`; memory is labeled in GB (`32 GB`). In mirrored mode both outermost labels carry the unit.
- **Right temperature axis** (CPU/GPU): labels `0`, `50`, `100 °C` in `temp` at `x1 + 8`; the scale is always 0–100.

## Scales

- CPU and GPU: 0–100 %.
- Memory: 0 … the larger of the installed physical memory and the peak, rounded up to a whole 4 GB (30.9 GB → 32 GB; grid at every quarter), at least 4 GB.
- DISK: `max(10, nearest 1-2-5 step ≥ peak)` MB/s. NET: the same with a minimum of 1.
- The peak is taken over the total lines and the pinned processes for the visible 5 min.
- The scale grows immediately and shrinks only when the peak drops out of the window. No animation; redrawn on the next frame.
- Values above the scale stay as they are in numbers, while on the chart they are clipped at the top (this applies to processes too).

## Samples

- Every sample is one point of its series; there is no downsampling.
- `None` (no data) does not turn into 0: the line and the fill break there.
