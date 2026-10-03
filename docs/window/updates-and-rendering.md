# Updates and rendering

Part of [History window](../window.md).

The first connection receives the full history over a separate local read-only pipe
`.Live`; after that only new/changed snapshots are sent. Reading and decoding
run off the UI thread; waiting for a client never blocks the collector.
The window updates at the system timer rate (about 64 times a second), so
the chart tooltip and the process highlight follow the cursor without a frame of delay.
Only what changed is redrawn: hovering a row redraws the process series (it is not in the
legend: the highlighted line is visible anyway), an unchanged table is not recreated, and the live
table and chart are updated in different frames. A minimized window and a window on another
virtual desktop draw nothing and keep receiving history.
Closing the window does not stop the background history.
