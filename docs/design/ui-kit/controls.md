# Controls: buttons, rows, search

[UI kit](../UI-KIT.md) · More controls: [Form and settings controls](controls-forms.md).

Common to all controls:
- Keyboard focus — the system focus visual (`UseSystemFocusVisuals`), shown only during keyboard navigation. Table rows instead get a 2 px `ink` outline inside the row while its pin button has keyboard focus.
- Hit targets — 28×28 for pin buttons; the search clear button and the Segmented items are 22 high, the sortable column headers 24.

| Control | Base look | States |
|---|---|---|
| **Button** | 32, padding 0 12, radius 4, border 1, 13, gap 8, icon 16. Background `ctrl`, border `ctrlBorder` | hover: `ctrlHover`; pressed: `ctrlPress` + text `text2`; disabled: background `subtle`, text `disabled`; focus |
| **Primary** | Background `accent`, text `onAccent` | hover: α 0.9; pressed: α 0.8; inactive: background `subtle`, text `disabled` (for example, "Apply to taskbar" with no changes) |
| **Subtle** | Transparent background and border | hover: `ctrlHover` |
| **PinButton** | 28×28, no border. Icon 14: `pin` in `text3` / `pinf` in `text` | hover: background `ctrlHover`; with 6 pinned, the others are disabled with the tooltip "Up to 6 pinned" |
| **Row** | 32, radius 4, padding 0 12 0 4, gap 12. [Pin] [Swatch 10] Name 13 · PID 12 `text3` · values on the right in tabular digits | regular — `pin` icon, name and values in `text`; hover — `rowHover` + the process highlighted on the chart; focus — outline inside; pinned — swatch (fill α 0.35 + 1.5 stroke in the pin color) + name 600 + `pinf`; pinned-nodata — dashed swatch, and instead of all values, italic 12 `text3` "No data at this moment"; memory rows — name in `textMuted`, no PID |
| **SearchBox** | 32, radius 4, background `ctrl`, border `ctrlBorder`, bottom border 1 `textBoxBottom`, icon `search` 14 `text3`, placeholder "Search by name or PID" | focus: bottom border 2 `accent`; filled: counter 11 `text3` + clear button 22 (`x` 12) |
| **Segmented** | Container: padding 2, radius 6, background `subtle`, border `divider`. Item 22, padding 0 10, radius 4, 12 | active: background `card`, border `border`, text `text` 600; inactive: `text2`; hover: `ctrlHover` |
