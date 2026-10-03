#![windows_subsystem = "windows"]

fn main() {
    if let Err(hr) = taskbar_metrics_host::platform::metrics_window::MetricsWindow::run() {
        let Ok(path) = taskbar_metrics_host::platform::data_directory::DataDirectory::file(
            "TaskbarMetrics.Window.error.log",
        ) else {
            std::process::exit(1);
        };
        let _ = std::fs::write(
            path,
            format!("Metrics window failed: 0x{:08X}\n", hr as u32),
        );
        std::process::exit(1);
    }
}
