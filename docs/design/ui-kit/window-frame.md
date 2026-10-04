# Window frame

[UI kit](../UI-KIT.md)

```
┌ System title bar ──────────────────────────────────────────┐  caption color = bg, text = text
│ [logo] Taskbar Metrics                         [–][□][×]    │
├ Nav 248 ─┬ Content (layer) ────────────────────────────────┤
│ CPU  43% │ Header                                           │  border top+left 1px border,
│ GPU  33% │ Device options                                   │  top-left radius 8,
│ …        │ Stats                                            │  padding 18/24/20, gap 14
│          │ ChartCard                                        │
│          │ TableCard (flex 1)                               │
│ Settings │                                                  │
└──────────┴──────────────────────────────────────────────────┘
```

- **Title bar:** the standard Windows one, titled "Taskbar Metrics vX.Y.Z" (the version from `Cargo.toml`), tinted through DWM: caption color `bg`, text color `text`, dark mode in the dark theme. The window icon is the app logo, drawn for the window's DPI.
- Under the navigation lies `bg`. The content layer is `layer`; cards are `card` with a `border` outline, radius 8 and the paddings from [ChartCard](chart-card.md) and [TableCard](table-card.md).
- Order of the vertical stack: Header → Device options → Stats → ChartCard → TableCard.
- **Device options** of the shown device, 20 apart: CheckBox "Show on taskbar", CheckBox "Always monitor" (ToolTip "Keep the history of this device while the window is closed"; checked and disabled while the device has a taskbar tile), then the "How it’s measured" button ([ChartCard](chart-card.md)). Changes are written to the configuration at once. The row sits 4 closer to the header; the stats have 16 below them instead of 14.
- **TableCard takes the remaining height** (at least 130). Only the list of rows scrolls inside it. The toolbar, the table header and the "Other processes" row do not scroll.
- **Chart height:** 270, 210 below a width of 1100.
- **Size:** opens at 1280 × 1024 of visible frame, centered on the main monitor's work area and no larger than it; it cannot be made smaller than 780 × 960 (at 100 %; on a smaller work area, down to its size).

Width classes: [Responsiveness](responsiveness.md).
