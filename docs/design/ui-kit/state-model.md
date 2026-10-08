# State model, moment and hover

[UI kit](../UI-KIT.md) · Pins, search and keyboard: [Pins and search](pins-and-search.md).

```
Mode = Live | History { t: SampleTime, frozenEnd: SampleTime }
Moment is global for all sections (switching sections keeps History).
Pin  = { pid, startTime, name, colorIndex }   // a process instance, not a name
Hover = Option<ProcessKey>                    // the row under the pointer
```

| Action | Result |
|---|---|
| Press on the chart | `History { t = nearest sample, frozenEnd = now }`. The table, navigation and stats show the values at `t`. Holding the button and moving scrubs `t`; the table follows at most 10×/s |
| Another press while in history | Changes only `t`; `frozenEnd` stays the same |
| Mouse wheel over the chart while in history, button up | Each notch moves `t` one sample: later when turned away from the user, earlier towards; stops at the ends. Ignored in `Live` |
| Esc / "Back to live" / right click on the chart | `Live` (Esc first closes an open navigation overlay) |
| `t` dropped out of the ring buffer | Automatic `Live` |
| Pointer enters a row | `Hover`: the process history over the whole window, the other series dimmed to 60 %, and the row on a `rowHover` background |
| Pointer leaves the row | `Hover` is cleared immediately; also on a new search, a pin, a moment or a section change |
| Hover over the chart | A vertical dashed (3 3) `text3` guide + a tooltip: the time, "Total", the temperature (CPU/GPU), pinned processes with their swatches (dashed without data), "no data" in italics; with nothing pinned, the top 3 processes of the moment with a gray `text3` dot — with "Group by name" on, the top 3 names, each with the sum of its processes (in RAM without the gray rows of memory outside processes: the kernel and the cache would win almost every time); processes with no load are skipped, and their places are taken by "—" dashes with a hollow marker so the tooltip height does not jump. Over a missed moment it shows the nearest sample within 1 s; there and over the whole half-transparent bridge of a missed sample, the note "Some samples are missing here" (11, italic, `text2`) follows the time, so it does not flicker across the bridge. With no sample within 1 s — "No samples". Width 220 (300 for RAM, disks and networks: megabytes with hundredths, or "↓ read  ↑ write" on one line), padding 10 8, radius 6, background `card` at 20 % transparency, border `border`, offset 12 from the pointer, flips at the plot edge |
