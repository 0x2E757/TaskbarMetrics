# Placement

Part of [Taskbar tiles](../taskbar.md).

The adapter finds `RootGrid` inside `Taskbar.TaskbarFrame` and adds its own XAML strip with
separate metric buttons. Existing elements and column definitions are not changed.
The geometry of the buttons, including widgets, is translated into the coordinates of the root panel.
`LeftPlacement` looks for the first sufficient free interval in the left half. If there is no
room or the tree could not be recognized, the tile strip is hidden (opacity 0). When hidden,
hit-testing is turned off as well, so that the invisible strip does not get in the taskbar's way.

The main scenario is the standard horizontal taskbar with centered buttons.
With left-aligned buttons there may be no free space. The geometry is rechecked
at the collection interval; a taskbar rebuild/animation between measurements
may briefly change the layout. Auto-hide and scaling
are inherited from the parent XAML panel. A separate UI target is created
for each taskbar root found.

Tiles are reordered by dragging, like tray icons: press a tile and
drag it (from 4 px), the neighboring tiles make room. Once the drag starts, the strip takes the
pointer capture from the tile button, which also cancels its click. After release the new order
is written to `metrics` right away ([configuration.md](../configuration.md)). A click without
dragging opens the window on that device; clicking again raises the already open
window and switches the section.
