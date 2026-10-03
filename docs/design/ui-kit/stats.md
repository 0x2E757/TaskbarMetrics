# Stats

[UI kit](../UI-KIT.md)

- Horizontal row, gap 36 (28 below a width of 1100), wrapping.
- **Block:** label 12 `text2` / value 20/600 (18 below 1100) in tabular digits / caption 11 `text3`; an empty caption still takes its line, so all values share a baseline. An explanation that need not always be visible goes into a hint: icon `info` 12 `text3` to the right of the label; a card (radius 4, `card`, `border`, text 12, up to 320 wide) shows while the pointer is over the icon.

| Section | Blocks |
|---|---|
| CPU | Load `47.3 %` · Temperature `71 °C` · 5-min peak `78 °C` + time of the peak · Frequency `4.62 GHz` · Up time `6 d 20 h` |
| GPU | Load + (i) "Load of the busiest GPU engine: 3D, compute, copy or video" · Temperature · 5-min peak `°C` + time of the peak · Dedicated memory `2.1 / 8.0 GB` |
| Memory | In use `26.2 GB (85 %)` (alert by share, as on the taskbar) · Installed `30.9 GB` · Page file `0.2 / 32.0 GB` |
| Disk | Read · Write (`0.25 MB/s`) · Scale `0–50 MB/s` + (i) "min 10 MB/s, grows with peak" |
| Network | Receive · Send · Scale + (i) "min 1 MB/s, …" · for Wi‑Fi, Signal `−54 dBm` "good" |

- **Wrapping.** A block that does not fit in the page width starts the next row (rows 12 apart). Widths are taken from blocks already drawn and only grow, so the layout does not jump. Below 1100 there is one row: the blocks that do not fit are left out, the last first.
- Values not in the recorded history (frequency, up time, page file, signal, dedicated memory in use) show "—" for a selected past moment.
- The Wi‑Fi signal is the WLAN API RSSI (`wlan_intf_opcode_rssi`); rating: from −50 excellent, from −60 good, from −70 fair, below that weak. Windows 11 gives the SSID and the quality in % only with location access, so they are not used.
- **Alert in the stats.** When a temperature or the RAM share reaches the upper alert threshold, the value is colored `danger` and gets an `alert` 16 icon (color is not the only signal). The caption under it: "above alert threshold 80 °C".
