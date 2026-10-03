# Icon and description

Part of [Building and running](../building.md).

The build script `build/main.rs` embeds an icon and a description into every exe without `rc.exe`: it builds a `.res`
(`build/resource_file.rs`) from the same rasterizer that draws the window icon
(`src/platform/app_icon.rs`). `TaskbarMetrics.Window.exe` and `TaskbarMetrics.exe` have
12 sizes from 16 to 256 px, the collectors 16–48 px. Sizes up to 64 px are embedded as
BMP images, 96, 128 and 256 px as PNG files from `assets/icon`. They are encoded by
`tools\encode-icon.ps1` (System.Drawing) from the pixels the build leaves behind; the build script
takes the PNGs only if the pixel fingerprint in `assets/icon/pixels.fnv` matches the current
logo and all three files together are no larger than 10 KB. Otherwise the build stops and
prints the command for the script.

Windows shows the description (FileDescription) on the taskbar, in Task Manager and in the
UAC prompt: "Taskbar Metrics", "Taskbar Metrics history collector", "Taskbar Metrics CPU
temperature sensor". The publisher (CompanyName) is "Taskbar Metrics" for all of them: without it,
"Settings → Apps → Startup" shows the file name instead of the description. A name
Windows has already seen for the same path stays in the `MuiCache` cache until it is cleared.
