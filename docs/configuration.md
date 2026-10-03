# Configuration

## taskbar-metrics.conf

The file lives in `%LOCALAPPDATA%\Taskbar Metrics`. If it is missing, the defaults are used; the first change
made by the program creates it with only the changed key. The build does not ship one.
Unknown keys, metrics, duplicates and invalid values are rejected.

```ini
metrics=cpu,gpu,ram,disk,net
history=disk@D:
interval_ms=500
process_monitoring=true
width=260
gap=12
```

| Key | Default | Meaning |
|---|---|---|
| `metrics` | `cpu,gpu,ram,net,disk` | Taskbar tiles in display order |
| `history` | empty | Devices without a tile whose history is recorded in the background ("Always monitor") |
| `monitors` | empty | Monitors whose taskbars show the tiles; empty means every monitor |
| `interval_ms` | `500` | Collection and chart interval, 250–60000 ms |
| `process_monitoring` | `true` | Background process history; `false` stops process polling and ETW, the taskbar metrics keep working |
| `width` | `260` | Minimum area width, 100–800 XAML logical pixels; the actual area is no smaller than the total width of the tiles |
| `gap` | `12` | Margin from the edge and neighboring buttons, 0–100 logical pixels |

An element of `metrics` and `history` is a type (`cpu`, `gpu`, `ram`, `disk`, `net`) or a specific
device `type@label`:

- `disk@D:` — the drive's first letter or its number; a network drive, its mapped letter
  (`disk@Z:`);
- `net@Ethernet 2` — the adapter name in Windows Settings;
- `gpu@1` — the graphics card number.

A type without a label is the main device: the system drive, the adapter with the default gateway,
the discrete graphics card (out of several — the one with the most dedicated memory).

An element of `monitors` is a monitor's hardware id and connection, `GSM7819/7&3aed0478&0&UID256`,
from the monitor path Windows reports; it stays the same across restarts and display changes. When
none of the chosen monitors is connected, the main monitor shows the tiles. The settings page
writes the key; with every monitor checked it writes it empty, so a monitor connected later shows
the tiles too, and chosen monitors that are disconnected at the time stay in the list.

The config is written by tile dragging and by the window (the "Show on taskbar" and "Always
monitor" checkboxes, the monitor checkboxes, the process monitoring switch). Writes are atomic, through a temporary
`*.conf.<pid>.tmp`.

### When changes take effect

- `metrics` — within a second, or within one interval if `interval_ms` is longer: Explorer
  checks the config on a sampling tick at most once a second. It rebuilds the tiles, keeping
  their history; the samplers are recreated only when the set of devices changes, not on a
  reorder. A drag on the taskbar already shows the new order and does not rebuild.
- `monitors` — within a second: each taskbar checks the config and the connected monitors once a
  second, removes its tiles or builds them anew.
- `history` and `process_monitoring` — within half a second: the history collector rereads
  the config on every tick. Turning `process_monitoring` on restarts the collector with administrator
  rights.
- `interval_ms`, `width` and `gap` — on the next start: `TaskbarMetrics.exe --stop`,
  then `TaskbarMetrics.exe`, or together with the next change of `metrics`, which applies
  the whole reread file. The tile width is calculated automatically; DPI scaling is done by
  the XAML panel itself.

The other files the program keeps there are listed in [files.md](files.md).
