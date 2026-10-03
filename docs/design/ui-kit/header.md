# Header

[UI kit](../UI-KIT.md)

`[h1 28/600] [subtitle 12 text2, baseline-aligned, gap 12] ⟶ [ModePill]` in live mode;
`[h1] [subtitle] ⟶ [Kbd "Esc"] [BackToLive]` in history, with the ModePill under BackToLive at the right edge, gap 8.
The pill under the button does not add to the Header height (negative bottom margin) and does not shift the page when a tick is selected.
The h1 is the title of the selected device, as in the [navigation](navigation.md).

- **Subtitles:**
  - CPU — "<processor name> · 8 cores · 16 threads"; several sockets — "2 × <name> · …".
  - GPU — "<adapter name> · discrete" (or "integrated").
  - RAM — modules from SMBIOS: "2 × 16 GB DDR5-6000", different modules joined with " + ".
  - Disk — "NVMe · Drives C:, D:" or "NVMe · Drive C:": the bus, then all volume letters, without `\` ("Drive" for one volume, "Drives" for several).
  - Network — the adapter name.
- **ModePill:** height 28, radius 14, padding 0 10, gap 7, text 12, one line.
  - *Live:* background `subtle`, border `divider`. A dot 8 in `accent` with a 3 px ring of `accent` α 0.22, then "Live" in `text2`. In two states a `text3` tail is added to the text: "· 0:42 of 5:00 collected" (history shorter than 5 min) or "· totals only" (monitoring off).
  - *History:* background `accentSoft`, border `accent` α 0.35. Icon `history` 14 in `accent`, "History" (600), the time `HH:MM:SS.d`, then `text3` "· 4 min 28 s ago" (not below a width of 1100). The "ago" tail updates with each new sample.
- **BackToLive** (only in history): primary button of height 32, radius 4, padding 0 12, text 13/600. Icon `return` 16, "Back to live", badge `+38 s` (11, background `onAccent` α 0.16, radius 8, padding 1 6) — how much new data has accumulated since the view was frozen. Once ≥ 5 min has accumulated, the badge shows `+5 min`. Before the button, `Kbd` "Esc": 11 `text3`, border `border`, radius 3, padding 1 5.
- **Device options** live in their own row under the header, see [Window frame](window-frame.md).
