//! TaskbarMetrics-Portable.exe: the packaged program in one file. Built by
//! tools\release.ps1 with `--features portable` and TASKBAR_METRICS_PAYLOAD set
//! to the package folder.
use taskbar_metrics_host::platform::portable::{Payload, PortableCopy};

macro_rules! payload {
    ($($name:literal),* $(,)?) => {
        [$((
            $name,
            include_bytes!(concat!(env!("TASKBAR_METRICS_PAYLOAD"), "/", $name)) as &[u8],
        )),*]
    };
}

static FILES: &Payload = &payload!(
    "TaskbarMetrics.exe",
    "TaskbarMetrics.Window.exe",
    "TaskbarMetrics.History.exe",
    "TaskbarMetrics.Sensors.exe",
    "TaskbarMetrics.Host.dll",
    "README.md",
    "pawnio/AMDFamily0F.bin",
    "pawnio/AMDFamily10.bin",
    "pawnio/AMDFamily17.bin",
    "pawnio/IntelMSR.bin",
    "pawnio/ZhaoxinMSR.bin",
    "pawnio/COPYING",
    "pawnio/README.md",
    // LGPL: the modules travel with their corresponding source.
    "pawnio/source/PawnIO.Modules-0.2.11.zip",
);

fn main() {
    std::process::exit(PortableCopy::run(FILES));
}
