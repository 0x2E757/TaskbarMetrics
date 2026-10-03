use taskbar_metrics_host::platform::launcher::Launcher;

fn main() {
    std::process::exit(Launcher::new().execute());
}
