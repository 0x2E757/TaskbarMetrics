# GPU and RAM sections

Part of [History window](../window.md#sections). DISK and NET:
[sections-disk-and-net.md](sections-disk-and-net.md).

## GPU

The total chart and the process contributions refer to the single busiest
physical engine of the graphics card at each sample. The legend calls the line "Busiest
engine" and does not name the engine (3D, compute, copy, video). That is why the
contributions, including "Other processes", add up to the total percentage. If the sum of the raw
counters of one engine exceeds 100%, the contributions are scaled proportionally; the window
reports this. The raw per-process maximums across all engines are also kept, but
are not presented as additive shares of the total chart.

## RAM

The chart line is the memory in use, as on the taskbar (all minus available),
the axis is in GB, values are in binary MB/GB, as in Windows. The table rows together give
exactly this value:

- processes: the private working set and "Shared" — an estimate of the process's writable
  shared memory (for example, the guest OS memory of VMware);
- rows without a PID from the `\Memory\*` counters: kernel pools, drivers and kernel code,
  modified pages, the system file cache;
- "Shared memory" — the remainder: DLLs and files mapped by processes, and processes
  closed to inspection.

Windows reports the number of processes sharing a page only for DLL pages; for
data sections `QueryWorkingSetEx` always answers "7 or more". That is why
writable shared pages are attributed to a process in full, and if their sum does not
fit in the remainder (a section is mapped twice or by several processes), the shares
are reduced proportionally. A lower bound is reserved for DLLs: the largest
set of read-only shared pages of a single process. Working sets are analyzed
while the window is open (it updates `taskbar-metrics.watch` every 2 s): once a
second the processes whose shared part of the working set changed
by more than 4 MB or 1/32 are rescanned, the rest every 10–19 s, large sets even less often
(+30 s per GB). The elevated collector enables SeDebugPrivilege for itself to see
the working sets of services and virtual machines; the memory of other processes is not read.
