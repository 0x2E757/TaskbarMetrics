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
| `interval_ms` | `500` | Collection and chart interval, 250–60000 ms |
| `process_monitoring` | `true` | Background process history; `false` stops process polling and ETW, the taskbar metrics keep working |
| `width` | `260` | Minimum area width, 100–800 XAML logical pixels; the actual area is no smaller than the total width of the tiles |
| `gap` | `12` | Margin from the edge and neighboring buttons, 0–100 logical pixels |

An element of `metrics` and `history` is a type (`cpu`, `gpu`, `ram`, `disk`, `net`) or a specific
device `type@label`:

- `disk@D:` — the drive's first letter or its number;
- `net@Ethernet 2` — the adapter name in Windows Settings;
- `gpu@1` — the graphics card number.

A type without a label is the main device: the system drive, the adapter with the default gateway,
the discrete graphics card (out of several — the one with the most dedicated memory).

The config is written by tile dragging and by the window (the "Show on taskbar" and "Always
monitor" checkboxes, the process monitoring switch). Writes are atomic, through a temporary
`*.conf.<pid>.tmp`.

### When changes take effect

- `metrics` — within a second, or within one interval if `interval_ms` is longer: Explorer
  checks the config on a sampling tick at most once a second. It rebuilds the tiles, keeping
  their history; the samplers are recreated only when the set of devices changes, not on a
  reorder. A drag on the taskbar already shows the new order and does not rebuild.
- `history` and `process_monitoring` — within half a second: the history collector rereads
  the config on every tick. Turning `process_monitoring` on restarts the collector with administrator
  rights.
- `interval_ms`, `width` and `gap` — on the next start: `TaskbarMetrics.exe --stop`,
  then `TaskbarMetrics.exe`, or together with the next change of `metrics`, which applies
  the whole reread file. The tile width is calculated automatically; DPI scaling is done by
  the XAML panel itself.

The other files the program keeps there are listed in [files.md](files.md).
