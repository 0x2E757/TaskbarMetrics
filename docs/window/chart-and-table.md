# Chart and table

Part of [History window](../window.md).

- The chart shows the accumulated history of up to 5 minutes in 500 ms steps. A click pins the
  time range and selects the nearest real snapshot. "Live"
  returns to the moving history. A single missing snapshot is bridged by a half-transparent line;
  longer gaps in the data are hatched and not joined.
- The table refers to the selected time, not to the current processes. By default it shows the
  top 10 and the rest; all processes and search by name/PID are available. PID + creation
  time tell an exited process apart from a new process that got the same PID.
- "Group by name" (on at start, not saved) adds up the processes of one name, regardless of case, in one
  row, so a browser reads as one program. The PID column shows the busiest process in the
  sort column and how many more the row adds: `27564 (+12)`. Groups rank by their sum; pinned
  processes keep rows of their own, and the pin button of a group pins its busiest process.
  Hovering a group highlights the sum of its name's unpinned processes. The chart tooltip's
  top 3 follows the box too: grouped, it shows the three busiest names with their sums.
- RAM, DISK and NET sort by a click on a value column header: either value of the pair, their
  sum ("Both", the default) or "All time". CPU and GPU sort by their only value.
- The pin button pins a process above the top 10 on all tabs, including search; pinned rows
  follow the sort too. At most 6 processes can be pinned (one per palette color); a seventh
  pin is refused. Pressing it again unpins it. If the process is not in the selected snapshot,
  the row stays with a no-data mark. Pins are saved in
  `taskbar-metrics.pins` in the [data folder](../files.md) and refer to a specific PID + start time.
  On the chart, pinned processes are always visible as semi-transparent filled
  areas; the color matches the pin in the list. The areas overlap from the zero
  line and are not stacked. DISK/NET charts are mirrored: read/receive above the axis,
  write/send below it, on the same scale; both are solid lines of the same color, the lower
  fill is slightly fainter (×0.85). Gaps of 2 or more snapshots break both the line and
  the fill.
- The temperature dash pattern on the window chart is not built with `StrokeDashArray` but
  from separate dashes (4 px, 3 px gap) tied to time: a dash covers
  the same span of buckets on every redraw. `StrokeDashArray` counts from
  the first point of the path, and with a full 5-minute history the pattern would restart from
  the left edge on every shift.
- Hovering a row highlights the process's time series. The base series (total, pins,
  temperature) are dimmed to 60 % opacity. The total is drawn in the `total` color (blue) in
  both directions, the highlighted process in `ink` (near black / white), also in both
  directions for DISK/NET.
