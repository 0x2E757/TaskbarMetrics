#[path = "../src/platform/app_icon.rs"]
mod app_icon;
#[path = "../src/platform/executables.rs"]
#[allow(dead_code)]
mod executables;
mod png_icons;
mod resource_file;

use app_icon::AppIcon;
use executables::Executables;
use png_icons::PngIcons;
use resource_file::{ResourceFile, VersionInfo};
use std::path::PathBuf;

/// Icon sizes of the window and the taskbar launcher: the shell's 16-48 px at
/// 100-150 %, plus the large views of Explorer, which come as PNG files.
const SIZES: [u32; 12] = [16, 20, 24, 30, 32, 36, 40, 48, 64, 96, 128, 256];
/// The background executables only appear in small lists.
const SMALL_SIZES: [u32; 6] = [16, 20, 24, 32, 40, 48];

/// Publisher in the version block of every executable.
const COMPANY: &str = "Taskbar Metrics";

/// The portable executable, which carries the packaged ones.
const PORTABLE: (&str, &str) = ("taskbar-metrics-portable", "TaskbarMetrics-Portable.exe");

/// Name and icon sizes that Windows shows for each executable.
const EXECUTABLES: [(&str, &str, &[u32]); 5] = [
    ("metrics-window", "Taskbar Metrics", &SIZES),
    ("taskbar-metrics", "Taskbar Metrics", &SIZES),
    (PORTABLE.0, "Taskbar Metrics", &SIZES),
    (
        "metrics-history",
        "Taskbar Metrics history collector",
        &SMALL_SIZES,
    ),
    (
        "metrics-sensors",
        "Taskbar Metrics CPU temperature sensor",
        &SMALL_SIZES,
    ),
];

fn main() {
    let root = PathBuf::from(std::env::var_os("CARGO_MANIFEST_DIR").unwrap());
    // The window's DPI awareness; the launcher's console only inside a terminal.
    let manifests = [
        (
            "metrics-window",
            root.join("src/platform/metrics_window/app.manifest"),
        ),
        (
            "taskbar-metrics",
            root.join("src/platform/launcher.manifest"),
        ),
        (PORTABLE.0, root.join("src/platform/launcher.manifest")),
    ];
    for path in [
        &manifests[0].1,
        &manifests[1].1,
        &root.join("src/platform/app_icon.rs"),
        &root.join("src/platform/executables.rs"),
        &root.join("build/png_icons.rs"),
        &root.join("build/resource_file.rs"),
    ] {
        println!("cargo:rerun-if-changed={}", path.display());
    }
    for (bin, manifest) in &manifests {
        println!("cargo:rustc-link-arg-bin={bin}=/MANIFEST:EMBED");
        println!(
            "cargo:rustc-link-arg-bin={bin}=/MANIFESTINPUT:{}",
            manifest.display()
        );
    }
    let out = PathBuf::from(std::env::var_os("OUT_DIR").unwrap());
    let version = std::env::var("CARGO_PKG_VERSION").unwrap();
    let png = PngIcons::new(&root, &out);
    println!("cargo:rerun-if-changed={}", png.assets().display());
    let large = png
        .load(AppIcon::pixels)
        .unwrap_or_else(|error| panic!("{error}"));
    // A PNG where there is one, otherwise a bitmap.
    let image = |size: u32| {
        large
            .iter()
            .find(|(png_size, _)| *png_size == size)
            .map(|(_, png)| png.clone())
            .unwrap_or_else(|| ResourceFile::bitmap(size, &AppIcon::pixels(size)))
    };
    for (name, description, sizes) in EXECUTABLES {
        // The version block names the file as it is packaged.
        let file_name = if name == PORTABLE.0 {
            PORTABLE.1
        } else {
            Executables::packaged(&format!("{name}.exe")).unwrap()
        };
        let resources = ResourceFile::new()
            .icons(
                &sizes
                    .iter()
                    .map(|&size| (size, image(size)))
                    .collect::<Vec<_>>(),
            )
            .version(&VersionInfo {
                description,
                company: COMPANY,
                file_name,
                version: &version,
            });
        let path = out.join(format!("{name}.res"));
        std::fs::write(&path, resources.bytes()).unwrap();
        println!("cargo:rustc-link-arg-bin={name}={}", path.display());
    }
    // Rust generates the .def file. COM entry points are looked up dynamically, never
    // linked through the generated import library; LNK4104's PRIVATE suggestion is benign.
    println!("cargo:rustc-cdylib-link-arg=/IGNORE:4104");
}
