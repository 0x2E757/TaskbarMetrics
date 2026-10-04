# Controls: form and settings controls

[UI kit](../UI-KIT.md) · Common focus and hit-target rules: [Controls](controls.md).

| Control | Base look | States |
|---|---|---|
| **ToggleSwitch** | Track 40×20, radius 10, label 13 on the left, right-aligned, 12 apart (as in Windows Settings) | on: track `accent`, thumb 12 `onAccent`; off: border `text2`, thumb 10 `text2`; disabled: border and thumb `disabled` |
| **Slider** | Track 4, color `sliderTrack`, fill `accent`. Thumb 20: background `card`, border `ctrlBorder`, an `accent` dot 12 inside | hover: dot 14; pressed: dot 10; disabled: dot `disabled`; focus and keys of the system Slider |
| **NumberBox** (a `TextBox` bound to its slider) | 30×64, radius 4, padding 5 8 0, background `ctrl`, border `ctrlBorder`, text right-aligned, unit 12 `text3` on the right | hover: background `ctrlHover`; focus: background `card`, border `accent`; disabled: background `subtle` |
| **ModePill / BackToLive / Kbd** | see [Header](header.md) | — |
| **SettingsCard** | Min height 64, padding 10 16, radius 6, background `card`, border `border`. Icon 16 `text2`, title 13, description 12 `text2`, control on the right. 4 between cards | hover: background `ctrlHover` (if the card is clickable) |
| **Expander** | Header like a SettingsCard + summary 12 `text2` on the right + chevron 14 `text2` | open: the body follows inside the same card, under a `divider` line; rows inside — min 48, padding 0 16 0 48, `divider` separators; slider row: label flex, Slider 220, NumberBox 64 + unit |
| **ComboBox** (a button with a flyout) | 32, min width 160, padding 0 10 0 12, background `ctrl`, chevron 14 `text2` | flyout: background `card`, border `border`, radius 8, padding 4; items 32, padding 0 12 |
