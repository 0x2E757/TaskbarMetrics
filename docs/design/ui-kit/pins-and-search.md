# Pins, search and keyboard

[UI kit](../UI-KIT.md) · Modes, moment and hover: [State model](state-model.md).

| Action | Result |
|---|---|
| Click on the pin, Space or Enter on a focused pin | Pin/unpin the instance. Color = the first free palette index (6 colors). A seventh pin is not available: the pins of unpinned rows are disabled, ToolTip "Up to 6 pinned" |
| The process of a pinned instance exited | The row stays pinned until the user unpins it: values at `t` where the process existed, otherwise "No data at this moment" and a dashed swatch; such rows go after the pinned rows with values |
| A new process with the same name | Does not inherit the pin (different `pid`/start time) |
| Search (Ctrl+F focuses it) | Filter by a case-insensitive substring of the name or of the PID, applied as you type. Pinned processes always stay visible. Counter in the field: "3 of 243" ("0" with no matches); a clear button ✕ appears while there is text. All matches are shown, without "Other processes". No matches — the row "No matches · pinned stay visible" |
| Top 10 / All | "All" removes "Other processes" and shows the whole list (only rows near the viewport are created) |
| ↑/↓ in the table | Focus moves through the rows' pin buttons; the focused row gets a 2 px `ink` outline. Space or Enter pins the process |

- Pinned rows follow the table sort among themselves.
- **Persistence between launches:** pins are saved to `taskbar-metrics.pins` in the data folder (PID, start time, name; at most 6) and loaded at start; checks and demos neither load nor save them. A pin whose instance no longer runs, e.g. after a restart, shows "No data at this moment". The theme and the language are saved; the section is not: the window opens on the requested device (a taskbar tile) or the first one. A closed window reopens centered on the main monitor at the default size of 1280 × 1024 (no larger than the work area). If the "Reset the window size and position when it closes" checkbox in "Startup and window" is cleared, the window opens where and at the size it was closed, including maximized; a frame that lands on no monitor opens centered.
