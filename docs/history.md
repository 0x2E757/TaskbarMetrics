# Background history

`TaskbarMetrics.History.exe` starts together with the application and records the history of:

- devices with taskbar tiles;
- devices from `history` ([configuration.md](configuration.md));
- while the main window is open — all devices shown in it (every 2 s the window updates
  `taskbar-metrics.watch` beside the config).

Without `process_monitoring` the collector runs without elevation and records only the
totals. Process history with DISK/NET ETW tracing needs elevation; the elevated collector
replaces the one running without it. An installed copy starts it through a scheduled task
without a UAC prompt ([installer.md](building/installer.md)). Cancelling UAC does not affect the taskbar. The process exits when the application stops or Explorer exits.

## What is stored

Up to 600 snapshots (5 minutes in 500 ms steps) of all processes: PID, creation time, name,
CPU, GPU, private working set, working set and private bytes; while the main window is
open, also the process's share of shared pages (a page in n working sets counts 1/n).
DISK — bytes of completed physical I/O, NET — TCP/UDP IPv4/IPv6 send/receive, per drive and adapter, placed by the time
they occurred, including late ETW delivery. Stacks, file paths and packet contents are not
recorded. History lives only in memory and is lost when the collector closes.

The "Background process history" switch in the window settings changes
`process_monitoring`. Turning it off stops the process queries and ETW; the saved process
history stays until its 5 minutes run out, and the device totals keep being recorded.

`--dump` exports the history to CSV and `--probe` measures the collector's own cost:
[export.md](history/export.md).

## Limitations

- A process that lived entirely between snapshots may be missing from CPU/RAM; its I/O may
  be kept with a PID but no name.
- Disk I/O with an unknown initiator is put into `Unattributed disk I/O` rather than
  attributed to a random process.
- Process CPU is relative to all logical CPUs, unlike the frequency-normalized total
  ([metrics.md](metrics.md#cpu)).
- The drive is taken from the disk number in the DiskIo event, the adapter from the local TCP/UDP
  address, or else the remote one; loopback traffic and I/O of unknown devices go only into the total columns.
- The full working set includes shared memory.
