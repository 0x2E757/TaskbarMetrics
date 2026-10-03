# Checks

Part of [Taskbar tiles](../taskbar.md).

- `TaskbarMetrics.Window.exe --verify-tiles` runs in a separate process with no window and no
  Explorer (with the system hover brushes replaced by test ones). It loads all five templates in
  Light/Dark and updates values/points, checks that numbers change at most once a second and show
  the average of the last two samples, and drags the first tile to the end without a pointer:
  the strip must reorder its children and save the new order.
- `TaskbarMetrics.Window.exe --verify-settings` shows the editor window and checks the creation of
  the interface, the extreme values of all sliders, the HEX fields and the color picker
  (a picked and a typed color, invalid HEX), the alert pulse starting and stopping, a value typed
  into a field reaching its slider and invalid input in a field.

Visual match, live theme switching, multiple monitors and DPI require checking on
screen.
