fn main() {
    if let Err(error) = taskbar_metrics_host::platform::process_history::ProcessHistory::run() {
        eprintln!("{error}");
        if let Ok(path) = std::env::current_exe() {
            let _ = std::fs::write(path.with_extension("error.log"), error);
        }
        std::process::exit(1);
    }
}
