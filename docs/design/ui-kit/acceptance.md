# Acceptance

[UI kit](../UI-KIT.md) · Artboards: [Visual reference](visual-reference.md).

- [ ] The light and dark themes match `Main`/`A-dark` in layout and tokens; the theme changes on the fly.
- [ ] Live → click on the chart → history: pill, button with badge, Esc, table/navigation/stats values at the moment in every section.
- [ ] Hovering a row highlights the whole history of the process and dims the other series; hovering the chart shows a tooltip.
- [ ] 1–6 pinned: colors are stable, areas start from zero, pinned processes are visible under any search and in every section; "No data at this moment" for a missing instance.
- [ ] DISK/NET: mirrored chart, Read/Write (Receive/Send) and Both columns plus "All time" at ≥ 1100, ETW strip in live, minimum scales of 10 / 1 MB/s.
- [ ] GPU: temperature > 100 °C — the number in full, the line clipped + label; missing samples — hatching, lines not joined.
- [ ] Empty history, monitoring off, missing rights — as on the artboards.
- [ ] Windows at 1280 and 820: navigation, columns and stats switch; text and lines do not scale. The window is at least 780 wide, so the layout below 760 never appears.
- [ ] Settings: "Taskbar" / "General" tabs, the open tab is kept when the language changes; preview of both themes without overflow; preview values under the strips; "Apply to taskbar" and "Revert changes" are active only when there are changes; alerts with two thresholds; color picker with HEX/alpha; the language applies immediately.
- [ ] Keyboard: Tab reaches every control in the order of [Accessibility](accessibility.md), focus visible; Esc returns to live, Ctrl+F focuses the search. A moment is chosen with the pointer.
