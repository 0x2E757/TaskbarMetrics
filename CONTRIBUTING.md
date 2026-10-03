# Contributing

- Code follows SOLID and KISS: logic is built by composing small classes ([AGENTS.md](AGENTS.md)).
- Everything is in English; only the translation catalogs hold Russian: `locale.rs` for the
  window and `installer/ru.isl` for Setup.
- Before committing, run the [checks](docs/building/checks.md).

## Commit messages

[Conventional Commits](https://www.conventionalcommits.org/): `<type>(<scope>): <description>`.

- `type` — what kind of change it is, from the first table.
- `scope` — which part of the program, from the second table. Leave it out, as in
  `<type>: <description>`, when the change has no single area.
- `description` — at most 70 characters, imperative mood ("add", not "added"), lowercase
  unless it starts with a name (`GPU`, `README`), no period at the end.
- `!` before the colon marks a change that breaks existing configuration files or arguments.

An optional body follows a blank line, says why rather than what and wraps at 72;
trailers such as `Co-Authored-By:` go last. One commit, one type: split unrelated changes.

| Type | Change |
|---|---|
| `feat` | New or changed behavior or look the user can see |
| `fix` | Wrong behavior or look corrected |
| `perf` | Less CPU, memory or I/O, same behavior |
| `refactor` | Code restructured, same behavior |
| `test` | Tests only |
| `docs` | Documentation only |
| `build` | `Cargo.toml`, `build/`, manifests, icons, packaging |
| `chore` | Other tooling and upkeep: `tools/` checks, `.github/` workflows, repository files |
| `revert` | Reverts an earlier commit |

| Scope | Area |
|---|---|
| `tiles` | Taskbar tiles in Explorer: `src/platform/xaml/`, `tap/`, `src/presentation/` |
| `window` | History window: `src/platform/metrics_window/`, except the settings |
| `settings` | Window settings and tile editor: `settings_page.rs`, `playground.rs`, `colors.rs`, `color_picker.rs` |
| `recorder` | History recorder: `src/platform/process_history/` |
| `metrics` | Measurements and devices: `src/metrics/`, `providers.rs`, `pdh.rs`, `devices.rs` |
| `sensors` | CPU temperature: `src/platform/temperature/`, `vendor/pawnio/` |
| `launcher` | `TaskbarMetrics.exe`: attaching, `--stop`, autostart, arguments |
| `config` | `taskbar-metrics.conf` and the data folder: `src/config.rs`, `data_directory.rs` |
| `installer` | Setup and the portable executable: `installer/`, `tools/release.ps1`, `portable.rs`, `scheduled_task.rs`, `unload.rs` |

Other files take the scope of the feature the change serves. Documentation uses the scope
of the part it describes.

```
feat(window): clear the chart selection on a right click
fix(tiles): keep the temperature line on whole pixel rows
perf(window): draw nothing while cloaked on another virtual desktop
docs(window): describe the 5-minute history in the UI kit
build: package the executables as TaskbarMetrics.*
```
