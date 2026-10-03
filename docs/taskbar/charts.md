# Charts

Part of [Taskbar tiles](../taskbar.md).

The chart draws a sample every 2 px from the right edge, as many recent samples as
fit in the tile width: a fill at 30 % opacity and a 1 DIP line by default (both set in the
[tile editor](tile-editor.md)).
A gradient hides the chart under the text, taking the measured width of the block into account. Only
the text and point collections are updated; the XAML is not recreated on every sample.

For DISK and NET each direction has its own scale based on the peak of the samples visible on the
tile (about 55 at a width of 108 px), with a lower bound of 1 MB/s. Zero samples are drawn below
the visible area, so an idle direction shows no line at the bottom edge. The second line (↑ write
or send) is orange by default.

The CPU/GPU temperature line (red by default: `#99FF0000` light, `#B3FF4B4B` dark) has a fixed
0–100 °C scale, like the temperature
axis in the window. The line snaps to whole pixel rows: 0–100 °C map to
34 chart rows (a step of ≈3.03 °C), otherwise a 1 px line blurs across two rows. While over
the last 3 s the temperature stays within two adjacent rows, the line does not jump: it keeps
its row if that row is among them, or takes the row of the average.

The temperature line can be made dashed (3 px dash, 2 px gap): settings,
"Tile charts" → "Dashed temperature line". The dashes are tied to the samples and
move left together with the chart: when an old sample leaves past the edge, `StrokeDashOffset`
grows by the length of the departed segment, otherwise the pattern would restart from the left edge.
The dashed line goes in steps (the row changes midway between samples): a diagonal
step √5 px long would shift all dashes after it between pixels, and they would blur.
