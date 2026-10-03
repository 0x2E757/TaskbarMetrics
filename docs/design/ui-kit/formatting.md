# Formatting

[UI kit](../UI-KIT.md) · UI strings: [Strings](strings.md).

- **Numbers:** a decimal point in both languages and a no-break space `U+00A0` before `%`. Thousands: `U+202F` in RU, a comma in EN. Two Russian sentences write the sampling step with a decimal comma.
- **Precision:**
  - % — 2 places in the table, 1 in the stats and the chart tooltip, 0 in the navigation and in the RAM share ("26.2 GB (85 %)");
  - MB/s — 2 places in the table and the stats; in the navigation 1 below 10, 0 from 10;
  - MB — 2 places; the "All time" column in MB with 2 places and no thousands separators; GB — 1;
  - °C — whole numbers.
- **Time:** marker/pill `HH:MM:SS.d`; axis `HH:MM`; relative — "4 min 28 s ago" (Russian uses its own unit abbreviations).
- **String length:** every key is longer in RU than in EN. The layout must not depend on length: no fixed widths on text buttons; process names get an ellipsis; the pill collapses its "ago" tail (see [Responsiveness](responsiveness.md)).
