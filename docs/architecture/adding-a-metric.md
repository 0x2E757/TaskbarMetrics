# How to add a metric

Part of [Architecture and extension](../architecture.md).

1. Create a struct implementing `MetricProvider`: `descriptor()` and `sample()`.
2. Compose it from the adapters/resources it needs. Keep the measurement state
   inside the source; release system resources through `Drop`.
3. Add the new kind to `DeviceId::KINDS` (`platform::devices`): the configuration
   rejects any other kind. Map the kind to its sources in `Composition::build`, which
   `Composition::monitor` calls, and add the kind to `metrics` in the configuration.
4. Cover the source's algorithm with a test. Changing the XAML, dispatching,
   `MonitoringService` or existing sources is not required.

`MetricDescriptor` holds the ID, the label and the unit. `MetricValue`
distinguishes a value, warm-up and unavailability. Expected source errors should
return `Unavailable`.

A taskbar tile shows the reading whose ID equals the kind (`kind@tag` for a specific
device). The tile ignores the unit and the formatter: it rounds the value to an integer,
adds `%` and leaves the number empty when there is no data. Only `disk` and `net` tiles show two
rates. The `MetricFormatter` text (the standard formatter keeps the unit and rounds to
an integer) is used only by `TaskbarMetrics.exe --sample`. The history window lists
only its own five resources.

This is source-level extension with a rebuild, not loading of third-party DLLs.
The public Rust API is not stabilized yet.
