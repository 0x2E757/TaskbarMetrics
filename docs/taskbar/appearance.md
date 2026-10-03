# Appearance

Part of [Taskbar tiles](../taskbar.md).

Default tiles: height 36, radius 5, gap 6, widths CPU/GPU 100, RAM 80,
NET/DISK 108 DIP (RAM is 20 px narrower than CPU, NET and DISK are 8 px wider). All five tiles
take 520 DIP. The light and dark theme palettes are defined in ThemeDictionaries. Each
tile has the `Normal`, `PointerOver` and `Pressed` states; their backgrounds are the palette's
"Hover background" and "Pressed background" colors (`MetricHover`/`MetricPressed`, by default
`#16000000`/`#22000000` in the light theme and `#16FFFFFF`/`#22FFFFFF` in the dark one), set in the
[tile editor](tile-editor.md). The Windows XAML Button handles the mouse states and the pointer
capture of a click; a drag takes the capture on the tile strip
([placement.md](placement.md)). There are no tooltips.

The tiles are built only once the `NotificationCenterButton` clock is found: the resources of
the clock and its ancestors are merged into each tile, and without a clock the view keeps waiting.
The clock may live in a separate XAML root: it is discovered through the shared
diagnostics subscription and kept as a weak reference on its UI thread.
The structure of the resource dictionaries is copied; the clock's dictionaries themselves are not reattached
to our button, because UWP does not allow two owners of one dictionary.
On builds with a different clock structure this adapter may need updating.

A device tile's header is its short name: `C:`, `WI‑FI`, `GPU2`. The main
device (a type without a label in `metrics`) keeps the usual header: CPU, GPU, RAM, DISK, NET.
