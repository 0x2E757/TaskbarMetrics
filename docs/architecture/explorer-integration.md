# Explorer integration and its risks

Part of [Architecture and extension](../architecture.md).

The integration uses XAML Diagnostics and Explorer's internal tree. This is not a
stable public API for extending the taskbar. A Windows update may
require changes to **only the platform adapter**, but compatibility with
unknown builds is not guaranteed. The code runs inside Explorer: unsafe/ABI
errors can affect the shell; catching Rust panics does not protect against access violations.

Windows 10, Windows Server, ARM64 and third-party taskbar replacements are not supported.

Sources for the ABI and behavior:
[XAML Diagnostics — Microsoft](https://learn.microsoft.com/en-us/windows/win32/api/xamlom/nf-xamlom-initializexamldiagnosticsex),
[GPU engines — Microsoft](https://devblogs.microsoft.com/directx/gpus-in-the-task-manager/),
[observed taskbar structure](https://github.com/ramensoftware/windows-11-taskbar-styling-guide).
