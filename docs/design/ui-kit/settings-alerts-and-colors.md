# Settings: alert range, colors, color picker

[UI kit](../UI-KIT.md) · Groups: [Settings groups](settings-groups.md).

## Alert range row

- Label, a track of width 260 and the rule's Toggle on the right. Three zones:
  - from the minimum to the lower threshold — `sliderTrack`;
  - between the thresholds — a `sliderTrack` → `danger` gradient;
  - above the upper one — a strip of height 8 alternating `danger` 6 px and `danger` α 0.35 4 px (the pulse zone). Number color rules do not pulse: that zone is solid `danger`, height 4.
- Two thumbs (20, step 1).
- Values 11 `text2` under the thumbs, pushed apart when they are close; none while the rule is off.
- Turning a rule off parks both thresholds at the top of the scale; turning it on restores its defaults.

## Colors

- Column header: Element / Light theme / Dark theme. Subgroups "Tile", "Charts and directions" and "States and background".
- Cell: Swatch 26 (the color over a checkerboard, radius 4) + a HEX field 120×30 (12).
- At the bottom: "Format #RRGGBB or #AARRGGBB, where AA is opacity." + [Reset colors] (subtle). An invalid value shows "Invalid or incomplete HEX: the last valid color is used for this field." there instead.

## Color picker

Flyout from the swatch.

- Size 300, padding 16, radius 8. Title "<element> · <theme>" 13/600.
- Spectrum 268×150, radius 6 (S/V).
- Hue slider of height 12 and Alpha slider of height 12 (over the checkerboard); thumbs 20 with a 2 white border.
- Fields: HEX `AARRGGBB` 130, "Alpha %" 58, "Before/after" swatches 30.
- Presets 8×22. The [Done] (primary) and [Cancel] buttons share the width equally.
- Esc closes the flyout like Cancel.
