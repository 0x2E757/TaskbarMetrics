# Numbers

Part of [Taskbar tiles](../taskbar.md).

Numbers update once a second and show the average of the last two samples. If
only one sample is available, it is shown; missing data is not replaced with zero.
No data leaves the number empty, with a gap in the chart; the tile name stays. CPU/GPU/RAM percentages smoothly turn red between two
thresholds (80 → 85 % by default); the thresholds are set in the settings, group "Number color";
the top position turns the reddening off.

DISK and NET show two speeds in decimal MB/s: DISK — ↓ read and ↑ write,
NET — receive and send. A speed has 2 decimals below 10, 1 below 100 and none from 100 up
(`4.12`, `23.4`, `218`). The % Disk Time percentage is not used: it says little about the load of
modern SSDs. Speed is not a percentage of channel utilization.

The CPU temperature is available through a separate PawnIO collector
([cpu-temperature.md](../cpu-temperature.md)), the GPU temperature through WDDM
([metrics.md](../metrics.md#gpu-temperature)). No demo numbers are substituted.
The temperature number is not rounded to the chart scale; it is shown in whole degrees.

## Alerts

The tile background turns red by CPU/GPU temperature and RAM usage: between the thresholds X and
Y its opacity grows from 0 to the pulse intensity, and from Y up it pulses between the intensity and
a quarter of it. Defaults: CPU 65/80 °C, GPU 70/80 °C, RAM 85/95 %, intensity 20 %, period 2 s.
A laptop runs hotter, so there the defaults are CPU 85/95 °C and GPU 75/85 °C. It counts as a
laptop when Windows reports a lid, or a battery that is not a UPS. Saved settings keep their
thresholds; the defaults apply without them and when a rule is turned back on.
Y is kept at least 1 above X. Missing data clears the alert. The thresholds, intensity and
period are set in the settings, group "Temperature and memory".
