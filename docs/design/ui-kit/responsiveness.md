# Responsiveness

[UI kit](../UI-KIT.md)

| Window width | Navigation | Header | Stats | Table |
|---|---|---|---|---|
| ≥ 1100 | Full, 248 | Pill with "N ago", button with full text | All blocks, value 20, gap 36; whatever does not fit wraps to the next row | PID column present (DISK/NET also "All time"), search 240, content padding X 24 |
| 760–1099 | Icons 56 + tooltips; ☰ opens the full navigation over the content (overlay) | No subtitle; pill without "N ago" | One row, value 18, gap 28: blocks in order while the width lasts, the rest hidden | No PID (and no "All time"), CPU column 90, search 170, content padding X 16; history caption is the bare time |
| < 760 | Hidden; ☰ before the h1 opens the overlay | h1 20, no subtitle, no `Kbd` "Esc" | As at 760–1099 | As at 760–1099, search 150; content padding 12/12/16, no rounded corner |

- The window does not shrink below a visible frame of 780 × 960 (at 100 %; on a monitor with a smaller work area, down to its size), so the < 760 layout, though the code keeps it, never occurs.
- Below a width of 1100 the legend uses short labels ("Total", "Temp.") and drops the "Moment" entry; whatever does not fit wraps to the next line. The "How it’s measured" button is in the device options row, see [Window frame](window-frame.md).
- Width thresholds switch without animation: the page is rebuilt for the new class. The chart is rebuilt immediately, without scaling a bitmap.
