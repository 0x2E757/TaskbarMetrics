# Placement

Part of [Taskbar tiles](../taskbar.md).

The adapter finds `RootGrid` inside `Taskbar.TaskbarFrame` and adds its own XAML strip with
separate metric buttons. Existing elements and column definitions are not changed.
The geometry of the buttons, including widgets, is translated into the coordinates of the root panel.
The weather (Widgets) is measured by its `BackgroundElement`, the part that shows: its container,
`Taskbar.AugmentedEntryPointButton`, reserves about 60 px of empty space after it while the weather
shows text, and the tiles stood that far from it. The tiles follow the weather at their own spacing
(the tile editor's "Spacing", 6 px by default), as if it were one more tile, measured to its fill:
its 1 px hover border lies inside its background, while a tile's lies outside the tile, so the
background's right edge counts 1 px to the left; the edge and the other
buttons keep the `gap` of the configuration. Without that element, as in another Windows build, the
whole container counts, with `gap`.
`LeftPlacement` looks for the first sufficient free interval in the left half. If there is no
room or the tree could not be recognized, the tile strip is hidden (opacity 0). When hidden,
hit-testing is turned off as well, so that the invisible strip does not get in the taskbar's way.

The main scenario is the standard horizontal taskbar with centered buttons.
With left-aligned buttons there may be no free space. The geometry is rechecked
at the collection interval; a taskbar rebuild/animation between measurements
may briefly change the layout. Auto-hide and scaling
are inherited from the parent XAML panel. A separate UI target is created
for each taskbar root found.

Each taskbar is a XAML island. The visual tree reports its `DesktopWindowXamlSource` as a root
with the island's root element as a child; the source gives its window
(`IDesktopWindowXamlSourceNative`), and the window above it is `Shell_TrayWnd` or
`Shell_SecondaryTrayWnd`. `MonitorFromWindow` tells the monitor, which `Displays` matches to the
ids of `monitors` ([configuration.md](../configuration.md)). A taskbar on a monitor that is
not chosen gets no tiles; once a second it checks the choice again and builds them or removes
them. A taskbar whose window is not found shows the tiles, unless `monitors` is `none`.

Tiles are reordered by dragging, like tray icons: press a tile and
drag it (from 4 px), the neighboring tiles make room. Once the drag starts, the strip takes the
pointer capture from the tile button, which also cancels its click. After release the new order
is written to `metrics` right away ([configuration.md](../configuration.md)). A click without
dragging opens the window on that device; clicking again raises the already open
window and switches the section.
