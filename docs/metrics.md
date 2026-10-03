# How the metrics are measured

An exact match with Task Manager is not promised: the intervals and rounding differ.
The first sample warms up the rate counters. When a counter is unavailable, the connection is
retried after a delay.

Every tile and every window section refers to a single device. A device id is
`type@label` (`disk@D:`, `net@Wi‑Fi`, `gpu@1`); a type without a label is the main device
(see [configuration.md](configuration.md)). The device catalog is `src/platform/devices.rs`.

## CPU

The English PDH counter `\Processor Information(_Total)\% Processor Utility`. It is a
frequency-normalized value; process CPU in the history is computed from execution
time relative to all logical CPUs, so their sum may differ from the
total load.

CPU temperature — [cpu-temperature.md](cpu-temperature.md).

## GPU

`\GPU Engine(*)\Utilization Percentage`, only for the instances whose LUID matches the
device's adapter. Processes of the same engine (LUID + physical GPU +
engine) are added up, then the busiest engine is picked. Values of different graphics cards are not
mixed.

### GPU temperature

Read through Windows WDDM: `GpuTemperatureProvider` uses `D3DKMTEnumAdapters2` and
`D3DKMTQueryAdapterInfo(KMTQAITYPE_ADAPTERPERFDATA)` from the system GDI32. Neither PawnIO nor
elevation is required. This is the main sensor, not hotspot/VRAM. The reading is the maximum of the
physical sensors of the same graphics card the load belongs to. On an error or
when there is no sensor, as in a virtual machine, the tile shows no temperature; a stale
reading is not kept. The adapter handle is
enumerated again every 30 seconds and closed with `D3DKMTCloseAdapter`.
Verified without elevation on a system with an RX 9070 and an integrated Radeon: about 57 °C.
[Microsoft's description of the sensor](https://learn.microsoft.com/en-us/windows-hardware/drivers/ddi/d3dkmthk/ns-d3dkmthk-_d3dkmt_adapter_perfdata).

Temperature cannot be attributed to an individual process.

## RAM

The share of physical memory in use via `GlobalMemoryStatusEx` (all minus available);
the history also stores it in bytes (`ram_used`). The per-process memory breakdown is
in [sections-gpu-and-ram.md](window/sections-gpu-and-ram.md#ram).

## DISK

`\PhysicalDisk(*)\Disk Read Bytes/sec` and `Disk Write Bytes/sec` of the drive's own instance,
in decimal MB/s. The main drive is the one holding `%SystemDrive%`. The history stores
values with the device label: `disk_read@C:`, `disk_write@C:`. The % Disk Time percentage is not
used: it says little about the load of modern SSDs.

## NET

`\Network Interface(*)\Bytes Received/sec` and `Bytes Sent/sec` of the adapter's own instance,
in decimal MB/s. The main adapter is the one with a default gateway, else the first. Adapters are not
summed. Receive and send of one adapter are read with a single query: collecting
`Network Interface` polls all adapters and costs about a millisecond.

## Per process

Per-process DISK and NET come from ETW; see [history.md](history.md).
