# Supported processors

Part of [CPU temperature via PawnIO](../cpu-temperature.md).

All processors for which PawnIO has a module with a temperature sensor are supported.
The formulas are checked against the Linux drivers `k10temp`, `k8temp`, `coretemp` and `via-cputemp`:

| CPU | Module | What is read |
|---|---|---|
| AMD 17h–1Ah (Zen) | `AMDFamily17.bin` | **Tctl**, SMN 0x59800 |
| AMD 10h–16h | `AMDFamily10.bin` | Tctl, northbridge function 3 register A4h; for 15h models 60h–7Fh — via the SMU |
| AMD 0Fh (K8) | `AMDFamily0F.bin` | THERMTRIP of the hottest core (up to two), +21 °C on desktop revision G; revisions before SH-C0 have no sensor |
| Intel | `IntelMSR.bin` | TjMax − the package sensor reading; without it — the hottest core of processor group 0 (up to 64 logical CPUs) |
| VIA 7 (Zhaoxin) | `ZhaoxinMSR.bin` | MSR 1423h, whole degrees |

- Physically verified only on a Ryzen 5 9600X; the other families need verification on
  the corresponding hardware.
- Tctl is the control temperature, not the average core temperature; the SKU-dependent
  Tdie offsets for older Ryzen are not subtracted.
- On Intel, TjMax is taken from MSR 1A2h, and on CPUs without this register it is assumed to be 100 °C,
  as in Linux.
- Family 10h sensors are disabled because of erratum 319 (unreliable readings) on socket F
  and on AM2+ models below 4 or model 4 up to stepping 2, as in Linux.
- Readings outside 0–125 °C are treated as a sensor fault and give `--°`.
- Hygon and ARM are not supported: `SensorCatalog` rejects the `HygonGenuine` vendor (the AMD
  modules accept only `AuthenticAMD`), and the collector is built for x64 only.

## Composition

`SensorCatalog` picks a `TemperatureSensor` by CPUID (`ZenTemperature`,
`K10Temperature`, `K8Temperature`, `IntelTemperature`, `ZhaoxinTemperature`);
`Celsius` discards impossible readings; `PawnModule` owns the DLL and the
driver handle; `PciMutex` serializes PCI/SMN reads with other monitors;
`TemperatureChannel` publishes a consistent snapshot with a monotonic timestamp;
`CpuTemperatureProvider` adapts it to the common metric contract. Adding another
processor family requires no changes to the Explorer code or the charts.
