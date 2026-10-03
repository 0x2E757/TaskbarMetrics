# DISK and NET sections

Part of [History window](../window.md#sections). GPU and RAM:
[sections-gpu-and-ram.md](sections-gpu-and-ram.md).

The live table lags by 2 s for ETW delivery. Selected snapshots
keep receiving corrections from late events as long as they are in the ring buffer.

## "All time" column

The "All time" column is the process's bytes since the collector started (it restarts together with
Explorer), in MB: "1820.00 (1500.00 + 320.00)" — the sum, then read + write
(receive + send). The unit is the same for all rows, decimal, like MB/s; always with hundredths
and without a thousands separator. The column is left-aligned: the integer part of the sum is right-aligned
in the space of "00000", the fractional part takes the space of ".00", so the decimal points and brackets
of all rows line up vertically (the digits are tabular). The column and its header
start with a 20 px indent (taken from the PID column): the sort arrow appears in it,
and the header does not shift. The collector accumulates the sums, including late ETW events, and
sends them in every batch; the window subtracts the frames after the selected moment, so the
history shows the sum as of that moment. The collector forgets the sums of processes that are in no frame
of the last 5 minutes. A narrow window has no such column.
