# Accessibility

[UI kit](../UI-KIT.md)

- Interactive elements are real `Button`/`CheckBox`/`ToggleSwitch`/`TextBox`/`Slider` controls. `AutomationProperties.Name` is set on the navigation items, icon-only buttons, Back to live, InfoButton, search, Top 10 / All, sortable column headers, pin buttons, checkboxes, sliders, number fields, settings tabs, expanders and pickers; buttons with a visible text label and the monitoring ToggleSwitch have none. The PinButton name = "Pin {name}" / "Unpin {name}".
- **Chart:** a `Canvas` with `AutomationProperties.Name` "Timeline"; it is not a tab stop. A moment is chosen with the pointer (press or drag) and stepped with the mouse wheel; a right click or Esc returns to live.
- **Keyboard:** Esc closes the overlay navigation, otherwise returns to live; Ctrl+F focuses the search. ↑/↓ move between table rows.
- **Tab order:** navigation → Back to live → "Show on taskbar" → "Always monitor" → InfoButton → search → clear button → Top 10 / All → sortable column headers (RAM, DISK, NET) → pin buttons of the rows.
- **Contrast** (all token pairs checked): `text2` and `text3` on `card`/`layer` ≥ 4.5:1 in both themes; chart lines ≥ 3:1 against `card`.
- No state is conveyed by color alone. The list of redundant cues: the pin icon, the bold name, the dashed "no data" marker, the mode pill with an icon and text, the `alert` icon on alerts, the marker shapes in the legend, the ↓/↑ arrows on directions.
