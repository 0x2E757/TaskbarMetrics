# Navigation (`Nav`)

[UI kit](../UI-KIT.md)

- Container: padding 8/8/12, gap 2 (in the 56 mode: padding 4/8/12, gap 4).
- No caption above the items: the ModePill in the [Header](header.md) shows the mode and the selected moment.
- **Item:** button of height 36, radius 4, padding 0 12 0 14, gap 12. Contents: icon 16, label 13, value 12 `text2` on the right, tabular digits.
- **One item per device** of the machine, re-enumerated every 10 s. Devices of one kind are numbered only when there are several: "GPU 1", "GPU 2", but "SSD" alone.

| Device | Label | Icon | Value in navigation |
|---|---|---|---|
| CPU | "CPU" | `cpu` | `43 %` |
| GPU | "GPU", "GPU 1" … | `gpu` | `33 %` |
| RAM | "RAM" | `mem` | `3.4 GB` |
| Disk | its media: "SSD", "HDD"; "Disk" when unknown | `disk` | `read · write MB/s` (`14 · 9.8 MB/s`) |
| Network | the adapter's connection name ("Wi‑Fi", "Ethernet") | `wifi` or `ethernet` | `receive · send MB/s` |

- Rates below 10 have one decimal, otherwise none.
- **Selected item:** background `navSel`, text 600, a 3×16 indicator in `accent` with radius 2 at the left edge (top 10).
- **Hover:** background `navHover`, press — `navPress`. Both are darker than the pane in the light theme and lighter in the dark one, like the selected `navSel`, so a click does not flash white and gray.
- "Settings" (icon `gear`) is pinned to the bottom.
- **56 mode:** icons only; ☰ (`menu`, bottom margin 10) at the top opens the full 248 navigation over the content (background `bg`, right border `border`); each icon has a `ToolTip` with the item's name.
- In a historical position the values are taken **at the selected moment** (it is shared by all sections), otherwise the current ones. They update with the table, about 1×/s.
