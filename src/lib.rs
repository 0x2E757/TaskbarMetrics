//! Application policy is independent of COM and the Windows taskbar.
pub mod application;
pub mod config;
pub mod metrics;
pub mod platform;
pub mod presentation;

#[cfg(not(all(target_os = "windows", target_arch = "x86_64", target_env = "msvc")))]
compile_error!("Taskbar Metrics requires x86_64-pc-windows-msvc and Windows 11.");
