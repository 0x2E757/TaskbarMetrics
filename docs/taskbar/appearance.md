# Appearance

Part of [Taskbar tiles](../taskbar.md).

Default tiles: height 36, radius 5, gap 6, widths CPU/GPU 100, RAM 80,
NET/DISK 108 DIP (RAM is 20 px narrower than CPU, NET and DISK are 8 px wider). All five tiles
take 520 DIP. The light and dark theme palettes are defined in ThemeDictionaries. Each
tile has the `Normal`, `PointerOver` and `Pressed` states; their backgrounds are the palette's
"Hover background" and "Pressed background" colors (`MetricHover`/`MetricPressed`), set in the
[tile editor](tile-editor.md). By default they are those of the taskbar's own buttons, such as the
weather: a white layer, `#80FFFFFF`/`#4DFFFFFF`, in the light theme and Fluent's
`#15FFFFFF`/`#08FFFFFF` in the dark one. Saved appearance and editor colors that still hold the
earlier defaults, which darkened a light tile (`#16000000`/`#22000000`, `#16FFFFFF`/`#22FFFFFF`), are
read as the current ones; colors chosen by hand stay. In both states the tile also gets the
elevation border Windows draws around the weather: 1 px, darker on its bottom row
(`#0F000000`/`#1A000000` in the light theme, `#18FFFFFF`/`#12FFFFFF` in the dark one), with the
background drawn under it. Like there, it lies just outside the tile (margin −1, corner radius one
more than the tile's), so it frames the tile instead of covering its edge and the chart on it; the
layout does not change, while UI Automation reports the button 1 px larger on each side. The Windows XAML Button handles the mouse states and the pointer
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
