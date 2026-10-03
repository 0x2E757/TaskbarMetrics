# Visual reference

[UI kit](../UI-KIT.md)

Canvas: https://claude.ai/artifact/XSNAARDRX1y52ZNLNNU9Wz

| Artboard | What to check against it |
|---|---|
| `Main` (A · light), `A-dark` | Base layout, history, hover, 4 pinned |
| `Disk-history-light` | Mirrored chart, value columns, "no data" |
| `Net-live-dark-en` | Live, EN locale, ETW strip, "How it’s measured" tip |
| `Gpu-live-light` | Temperature > 100 °C, missing samples |
| `Empty-history-light` | The first seconds after launch |
| `Monitoring-off-dark` | Process monitoring turned off |
| `Compact-820-light`, `Narrow-480-dark` | Responsiveness, keyboard focus (480 cannot occur in the app: see [Responsiveness](responsiveness.md)) |
| `Settings-light`, `Settings-dark` | Settings, preview, alerts, colors, color picker |
| `Controls` | Every control state in both themes |

The app is the source of truth: where an artboard and the [UI kit](../UI-KIT.md) pages differ, the pages describe the app. Variants B and C on the canvas are rejected directions; do not implement them.
