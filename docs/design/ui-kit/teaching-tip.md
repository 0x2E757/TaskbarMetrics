# TeachingTip "How it’s measured"

[UI kit](../UI-KIT.md)

- Size 380 × auto, padding 12 14, radius 8, background `card`, border `border`, no shadow, like the stats hints.
- Title 13/600, then list items 6 apart: 12, line height 17, color `text2`, bullet `•` in `text3` 8 before the text.
- Its right edge matches the right edge of the [InfoButton](chart-card.md) in the device options row, 6 below it; it lies over the stats and the chart. It appears on hover or focus and disappears when the pointer leaves the button or focus moves on. The card does not react to the pointer.

| Section | Items |
|---|---|
| CPU | Total load is the PDH counter “% Processor Utility”; process time is CPU time over the interval divided by the number of logical processors. The process sum can differ from the total load. · Processes are sampled every 0.5 s: a short one that starts and exits between samples is not in the table, though the total load counts it. · Temperature is the package sensor; the line is capped at 100 °C. |
| GPU | The total line is the busiest physical engine at each sample. Process percentages are their share of exactly that engine. |
| Memory | The line is the memory in use, as on the taskbar: all memory minus the available part (standby cache and free pages). · Private: pages of this process alone. Shared: an estimate of its writable shared memory, such as the guest memory of a virtual machine, fitted so that the rows add up to the line. Measured once a second while this window is open; the hatched spans are the time it was closed. · Rows without a PID are memory outside processes. «Shared memory» is the rest: DLLs and files mapped by processes, and processes closed to inspection. · All rows add up to the line. |
| Disk/Network | Totals come from PDH counters; per-process traffic comes from ETW. The two can differ slightly. · ETW delivers events with a delay, so live per-process values trail the total by about 2 s (hatched strip). · Disk: "Read is drawn above the axis, write below it, on the same scale." Network: "Receive is drawn above the axis, send below it, on the same scale." |

Conditional items are appended:
- GPU, when the engine counters exceed 100 %: "Counter overshoot: contributions scaled proportionally to 100%."
- Disk/Network, when ETW is inactive or lost or could not decode events: "I/O attribution unavailable or incomplete".
- A failed save of pins, device options or "Turn on monitoring": "Save failed: <error>".
