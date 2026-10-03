# Settings: structure and preview

[UI kit](../UI-KIT.md) · Also: [Settings groups](settings-groups.md), [Alerts and colors](settings-alerts-and-colors.md).

**Structure, top to bottom:**
1. Header: "Settings" 28/600, no subtitle.
2. Tabs "General" | "Taskbar" above a `divider` separator: height 40, text 14; the active one is 600 `text` with a 3 px `accent` line under the text, the inactive one 400 `text2`, hover `ctrlHover`. A tab's width does not change when switching. Going to "Settings" from another section always opens "General"; changing the language or the theme keeps the open tab.
3. "General": "Startup and window", "Monitoring", "Window appearance", "Language" — everything that applies immediately.
4. "Taskbar": the preview card (sticks to the top, only the groups below it scroll), then the groups "Taskbar widgets" and "Alerts" — everything applied with a button. Group titles 14/600; cards radius 6.

**Preview card:**
- Title "Widget preview" 14/600 + "100 % scale, both themes".
- Two taskbar strips (light and dark) **one above the other**, each with a caption 11 `text3` of width 60 on the left. Under the strips, captions 11 `text3` for each CPU/GPU/RAM preview tile past its threshold: "GPU — between thresholds: the tile turns red" · "RAM — above the upper threshold: pulsing".
- A separator, then **"Preview values"** (below the preview, not beside it): title 12/600 + "used only in the preview" + Segmented [Test | Live values]. Below, one column per tile in tile order: title 12 `text2` (CPU, GPU, RAM, NET, DISK), under it a 70 field + unit — % and °C for CPU and GPU, % for RAM, ↓ and ↑ in MB/s for NET and DISK with the arrow before the field. "Live values" fills them from the newest sample.
- Footer: status dot + "N changes not applied to the taskbar" · `info` 14 `text3` with a card on hover (what both buttons do; grows up and to the left) · [Revert changes] (`reset`) · [Apply to taskbar] (primary, `check`). With no changes, both buttons are shown disabled and the status is hidden. A failed apply puts "Save failed: …" in the status.

**Taskbar strip:**
- Height 48, radius 6, background `#EEF1F6` in the light theme / `#1F1F1F` in the dark one, border `#DDE1E7` / `#2E2E2E`.
- Tiles on the left, spaced by the "Tile spacing" setting; **one** neutral 24 square (radius 6) on the right, a hint at the rest of the taskbar.
- The tiles are drawn **by the same code as on the real taskbar**, with the current unsaved settings.
