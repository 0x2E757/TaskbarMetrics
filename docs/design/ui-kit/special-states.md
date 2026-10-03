# Empty and special states

[UI kit](../UI-KIT.md)

- **Empty history** — see [Overlays](chart-overlays.md#overlays). The "5-min peak" stat is computed over the part of the window already collected; "—" and the caption "no data" appear only while there is not a single temperature sample yet. The table works in live.
- **No history yet** (the recorder has sent nothing): the list shows "Waiting for history…" 16/600 and "Recorder unavailable. Enable background monitoring in Settings." 12 `text2`.
- **Process monitoring off:**
  - Chart — only the total line and the temperature; no pinned layers. Pill "Live · totals only".
  - In the TableCard under the toolbar (search disabled, caption "no data"; the "All" count is the number of running processes), centered, instead of the header, rows and footer:
    - a circle 48 (`subtle`) with icon `pause` 22;
    - "Process monitoring is off" 16/600;
    - explanation 13 `text2`, width up to 520;
    - buttons: [Turn on monitoring] (primary) and [Open settings] (icon `gear`);
    - below, if anything is pinned, "Pins are kept and return when enabled" 12 `text3` and chips of the pinned processes (height 24, radius 12, border `border`, dashed swatch, name 12 `text2`).
- **No administrator rights:** ETW needs elevation, so in DISK/NET the per-process traffic stays unattributed; the totals and the chart work. "How it’s measured" then adds "I/O attribution unavailable or incomplete".
