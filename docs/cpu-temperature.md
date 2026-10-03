# CPU temperature via PawnIO

There are no Cargo dependencies. The optional system dependency is the official PawnIO 2.2.0.
`TaskbarMetrics.Sensors.exe` is a separate elevated Rust process; the DLL in Explorer
only reads the shared snapshot, with no access to the driver. The collector polls the sensor every
500 ms. Snapshots older than five seconds, errors and unsupported CPUs leave the temperature
place of the tile empty, and the temperature chart breaks. The collector exits on `--stop` or when the
Explorer instance it was started for closes.

## Installation and running

```powershell
powershell -NoProfile -ExecutionPolicy Bypass -File tools\install-pawnio.ps1
powershell -NoProfile -ExecutionPolicy Bypass -File tools\build.ps1
.\target\package\TaskbarMetrics.exe
.\target\package\TaskbarMetrics.exe --sensors
```

`install-pawnio.ps1` downloads the official installer and verifies its signature.
Setup can install PawnIO and starts the collector without a prompt
([installer.md](building/installer.md)); for other copies `--sensors` brings up the standard
UAC prompt for the collector only. After Explorer restarts
or the application stops, this command has to be repeated; `tools\update-package.ps1`
does it itself if the collector was running before the restart. With autostart enabled in the settings, the collector
starts when signing in to Windows whenever `TaskbarMetrics.Sensors.exe` is in the package (the
build always puts it there), so sign-in brings up UAC for it; no Windows service is installed.
The taskbar part itself needs no elevation, but with `process_monitoring=true` (the default)
a start also launches the history recorder elevated, with its own UAC prompt, unless an
elevated one already runs for this Explorer ([history.md](history.md)).

For diagnostics, run `TaskbarMetrics.Sensors.exe --sample` as administrator: three
readings are also saved to `TaskbarMetrics.Sensors.sample.log`; a startup error goes to
`TaskbarMetrics.Sensors.error.log` beside the EXE. When building directly with Cargo, copy
`vendor/pawnio` beside the collector under the name `pawnio`; the build scripts do this themselves.

## Supported processors

Every processor PawnIO has a temperature module for: AMD from K8 to Zen, Intel and
VIA/Zhaoxin. The modules, formulas, caveats and the sensor classes are in
[supported-processors.md](cpu-temperature/supported-processors.md).

## Module license

The signed 0.2.11 modules ship as separate replaceable files together
with LGPL-2.1-or-later, authorship notices, checksums and an archive of the
corresponding sources in [`vendor/pawnio`](../vendor/pawnio/README.md). The driver itself
and PawnIOLib.dll are not included in the package.
[Official PawnIO API](https://github.com/namazso/PawnIO.Modules/wiki/Using-PawnIO-Modules),
[module sources](https://github.com/namazso/PawnIO.Modules/tree/0.2.11).
