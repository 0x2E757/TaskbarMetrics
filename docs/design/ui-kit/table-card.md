# TableCard

[UI kit](../UI-KIT.md)

**Toolbar** (height 40, padding 0 12 0 16, gap 12):
`"Processes" 15/600` → `caption 12 text2` → ⟶ → `SearchBox 240` → `Segmented [Top 10 | All · 243]`

Toolbar captions: live — "now · updates 1×/s"; history — "as of 14:00:50.5" (below a width of 1100 only the time); NET/DISK live — "as of … · 2 s behind live" (because of the ETW delay); monitoring off — "no data".

**Table header** (height 30, padding 0 12 0 4, gap 12, 12 `text3`, bottom border `divider`):
`[28 space for the pin] "Process" | PID 72 | value columns (right-aligned)`

- CPU and GPU have one value column, always the sort column: `text` with `chev` 12.
- In RAM, DISK and NET each value header is a button: a click sorts by that column, descending. The sort column is in `text` with `chev` 12 before its title, the others in `text3`. By default the sort is by "Both", the sum of the two values.

| Section | Value columns (width) |
|---|---|
| CPU | CPU 110 (90 below 1100) |
| GPU | GPU 110 |
| Memory | Private, MB 104 · Shared, MB 104 · Both, MB 104 |
| Disk | Read, MB/s 104 · Write, MB/s 104 · Both, MB/s 104 · All time, MB 196 |
| Network | Receive, MB/s 104 · Send, MB/s 104 · Both, MB/s 104 · All time, MB 196 |

- **All time** (≥ 1100 only; the PID column gives up 20 to its indent): bytes since the recorder started, left-aligned, in MB to hundredths: `1820.00 (1500.00 + 320.00)`, or `0.00` without I/O. It sorts like the other columns.
- Values: 13, tabular; CPU/GPU `12.34 %`, the others to hundredths; "—" for a missing one.

**List body** (padding 4 8), in order:
1. Rows of pinned processes: name 600, a 10 swatch of the pin color (fill α 0.35, stroke 1.5; dashed without data).
2. Separator, only after pinned rows: 1 px `divider`, margins 4 8 0 44.
3. Top 10 rows; every row in "All" mode or while searching. No matches — the row "No matches · pinned stay visible" (12 `text3`, indent 44).

**Row:** height 32, radius 4, padding 0 12 0 4, gap 12. Pin button 28 (`pin` 14 `text3`, pinned — `pinf` in `text`), name 13, PID 12 `text3`, values. Without data at the moment — "No data at this moment", italic 12 `text3`, over the value columns. Hover — `rowHover`; keyboard focus — a 2 px `ink` outline.

- **Rows of memory outside processes** in RAM ("Kernel: paged pool", "Shared memory", "System file cache" and so on) — no PID, name in `textMuted` (#868C95 / #7B8087: one and a half steps from `text` towards `text3`); values in the normal color.

**Footer** (top border `divider`) — the row "Other processes (229)" with summed values, `text2`, no pin. Hidden in "All" mode and while searching.

**Scrolling:** a thin, always visible scrollbar of width 3, radius 2, `text3` α 0.45.
