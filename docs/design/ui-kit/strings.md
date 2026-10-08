# Strings

[UI kit](../UI-KIT.md) · Number and time formats: [Formatting](formatting.md).

The Russian translations live in the `CATALOG` of [`src/platform/metrics_window/monitor/locale.rs`](../../../src/platform/metrics_window/monitor/locale.rs).

| Key | EN |
|---|---|
| nav.cpu/gpu/mem/disk/net/settings | CPU / GPU / RAM / Disk / Network / Settings (device titles: see [Navigation](navigation.md)) |
| mode.live | Live |
| mode.history | History |
| mode.back | Back to live |
| mode.ago | {t} ago ({t} = "4 min 28 s") |
| mode.collected | {a} of 5:00 collected |
| mode.totalsOnly | totals only |
| legend.total.cpu / gpu / mem / disk / net / short | Total load / Busiest engine / RAM in use / Disk total (PDH) / Total (PDH) / Total |
| legend.temp / temp.short / moment / nodata / etw | Temperature / Temp. / Moment / No samples / Awaiting ETW attribution |
| tooltip.missed | Some samples are missing here |
| legend.unshared | Window closed: no shared memory |
| chart.now / aboveScale / collecting | now / above scale, peak {v} / Collecting history |
| chart.dir.disk / net | Read, Write / Receive, Send (one label above and one below the axis) |
| info.button | How it’s measured |
| options.taskbar / always / alwaysHint | Show on taskbar / Always monitor / Keep the history of this device while the window is closed |
| table.title / process | Processes / Process |
| table.columns | Private / Shared / Both / Read / Write / Receive / Send / All time |
| table.group / groupHint | Group by name / Add up the processes of one program, such as a browser |
| table.top / all | Top 10 / All · {n} |
| table.others | Other processes ({n}) |
| table.nodata | No data at this moment |
| table.pin / unpin / pinLimit | Pin / Unpin / Up to 6 pinned |
| search.placeholder / count / none / clear | Search by name or PID / {a} of {b} / No matches · pinned stay visible / Clear |
| off.title / body | Process monitoring is off / Totals are still collected. Turn on background process history to see which processes used resources at any moment of the last 5 minutes. |
| off.enable / openSettings / pinsKept | Turn on monitoring / Open settings / Pins are kept and return when enabled |
| wait.title / body | Waiting for history… / Recorder unavailable. Enable background monitoring in Settings. |

Settings strings are carried over from the `Settings-*` mockups verbatim; for EN, a direct translation in the same case (sentence case).
