# Export and probe

Part of [Background history](../history.md).

The history can be exported over a local read-only pipe (access for the current
user and administrators):

```powershell
.\TaskbarMetrics.History.exe --dump <Explorer-PID> history.csv
.\TaskbarMetrics.History.exe --probe 30
```

`--probe` runs for the given seconds (1–600) and writes `TaskbarMetrics.History.probe.txt`:
`frames`, `processes`, `status`, the wall-clock collection duration (`sample_wall_ms_p50`,
`p95`) and the collector's CPU time (`collector_cpu_ms_per_second`). ETW needs an elevated
run; without it `status` reads `ETW unavailable 0x…`.

## CSV format

The header is `# history_v3` with `enabled`, `frames`, `interval_ms`, `retention_ms` and
`status`. The CSV contains counters of lost and unsupported events; `etw_active=false` means
there is no DISK/NET data, not zero activity. `shared_share_bytes` is filled only while the
main window is open. Names are quoted, `"` doubled, line breaks replaced by spaces.

- `gpu_percent` — the process's busiest engine across all graphics cards (its instances on
  one engine added up), capped at 100;
- `# gpu_engine` / `# gpu_share` — the busiest engine of the graphics card (`gpu@1`) and
  the additive process contributions to it from the same sample;
- `# total` — totals with the device label (`disk_read@C:`);
- `# device_io` — a process's bytes per drive or adapter (`disk@C:` read/write,
  `net@Wi‑Fi` receive/send).
