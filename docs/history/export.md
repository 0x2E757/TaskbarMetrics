# Export and probe

Part of [Background history](../history.md).

`TaskbarMetrics.exe --dump` prints the recorded history, so scripts and agents need not
measure again; `TaskbarMetrics.exe --help` lists its options. The other programs answer
`--help` with a pointer to it. It reads a local read-only pipe of the recorder (access for
the current user and administrators) that sends the whole history in the window's binary
format; the choice is made by the reader.

```powershell
.\TaskbarMetrics.exe --dump --last 30 --metrics cpu,disk@C: --processes 5 --by disk
.\TaskbarMetrics.exe --dump --raw --output history.csv
.\TaskbarMetrics.History.exe --probe 30
```

By default it prints, for the last 60 s (`--last`, 1–300), one CSV row a total with its
unit and the average, maximum and last measured value; `--series` prints every 500 ms frame
instead. `--metrics` picks totals by kind (`disk_read`), a word of it (`disk`, `temperature`)
or an id (`disk_read@C:`, `disk@C:`). `--processes N` adds the top N processes by `--by`
(`cpu` by default): CPU and GPU averaged over the window, the largest private working set,
disk and network bytes added up. Lines starting with `#` describe the data or say why a
part is missing: process history off, ETW without administrator rights, a recorder of
another build. Errors go to stderr with exit code 1.

`--probe` runs for the given seconds (1–600) and writes `TaskbarMetrics.History.probe.txt`:
`frames`, `processes`, `status`, the wall-clock collection duration (`sample_wall_ms_p50`,
`p95`) and the collector's CPU time (`collector_cpu_ms_per_second`). ETW needs an elevated
run; without it `status` reads `ETW unavailable 0x…`.

## Raw CSV

`--raw` writes every frame with every process. The header is `# history_v3` with `enabled`,
`frames`, `interval_ms`, `retention_ms` and `status`. Rows hold counters of lost and
unsupported events; `etw_active=false` means no DISK/NET data, not zero activity.
`shared_share_bytes` is filled only while the window is open. Names are quoted, `"` doubled,
line breaks replaced by spaces.

- `gpu_percent` — the process's busiest engine across all graphics cards (its instances on
  one engine added up), capped at 100;
- `# gpu_engine` / `# gpu_share` — the busiest engine of the graphics card (`gpu@1`) and
  the additive process contributions to it from the same sample;
- `# total` — totals with the device label (`disk_read@C:`);
- `# device_io` — a process's bytes per drive or adapter (`disk@C:` read/write,
  `net@Wi‑Fi` receive/send).
