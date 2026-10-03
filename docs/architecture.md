# Architecture and extension

All application components are structs with methods, combined through composition
(SOLID and KISS, see [AGENTS.md](../AGENTS.md)). Traits are used only at the boundaries
of changeable behavior.

| Component | Responsibility |
|---|---|
| `metrics::MetricProvider` | A single data source and its own resources |
| `metrics::MetricRegistry` | An ordered set of sources, unique IDs, isolation of Rust panics |
| `application::MonitoringService` | Composition of the sources, the formatter and the sink |
| `presentation::MetricFormatter` | Turning a snapshot into text |
| `application::MetricSink` | Receiver of the typed snapshot and its text representation |
| `presentation::history::MetricHistory` | Bounded history, gaps and chart coordinates |
| `presentation::LeftPlacement` | Free space calculation without the Windows API |
| `platform::devices` | Device catalog, `type@label` ids, the main device of each type |
| `platform::composition::Composition` | The only place where concrete sources are registered |
| `platform::pdh` / `providers` | RAII for PDH, Windows implementations of CPU/GPU/RAM/DISK/NET |
| `platform::tap::session` | Tree subscription, collection, stop and resume |
| `platform::tap` | COM factory, callback and DLL entry points |
| `platform::xaml::dispatcher` | Passing operations to the UI queue |
| `platform::xaml::geometry` | Reading the geometry of the stock buttons |
| `platform::xaml::button` | Independent tiles and updating their text/charts |
| `platform::xaml::clock` / `resources` | Finding the clock in a separate XAML root and copying the needed resources |
| `platform::xaml::visual_tree` | Bounded XAML tree walk for finding and inheriting resources |
| `platform::xaml` | Creating, updating and removing our own element |
| `platform::process_history` | Background history collector, ETW and the pipe for the window |
| `platform::temperature` | CPU temperature collector via PawnIO |
| `platform::metrics_window` | History window, settings and tile editor |
| `platform::com` / `abi` | Internal COM/HSTRING/handle wrappers and the Windows ABI |

- [How to add a metric](architecture/adding-a-metric.md)
- [Explorer integration and its risks](architecture/explorer-integration.md)

## Threads

A single worker collects the metrics; XAML is changed only on the corresponding UI thread.
The elements' COM objects are kept in a thread-local table and are not `Send/Sync`.
An `IAgileReference` to the queue is passed between threads. At most one pending update
is allowed per taskbar. Each new container has its own
generation, so that an old removal does not affect a new element.

A source that reads PDH owns its query, except that the download and upload of one network
adapter share a single query.
