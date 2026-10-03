use super::{
    abi::*, composition::Composition, executables::Executables, scheduled_task::CollectorTask,
};

use crate::config::Settings;
use std::{
    ffi::{OsStr, OsString},
    os::windows::ffi::{OsStrExt, OsStringExt},
    path::{Path, PathBuf},
    ptr,
};

type Initialize =
    unsafe extern "system" fn(*const u16, u32, *const u16, *const u16, Guid, *const u16) -> Hr;

/// The argument that runs the watcher with the tray icon.
pub(crate) const WATCH: &str = "--watch";

pub struct Launcher;

impl Default for Launcher {
    fn default() -> Self {
        Self::new()
    }
}

impl Launcher {
    pub fn new() -> Self {
        Self
    }

    pub fn execute(&self) -> i32 {
        match self.run() {
            Ok(code) => code,
            Err(hr) => {
                eprintln!("Error: HRESULT 0x{:08X}", hr as u32);
                1
            }
        }
    }

    fn run(&self) -> Result<i32> {
        let args: Vec<_> = std::env::args_os().skip(1).collect();
        if args.len() > 1
            || args.first().is_some_and(|a| {
                ![
                    "--sample",
                    "--stop",
                    "--help",
                    "--sensors",
                    "--register-tasks",
                    "--remove-tasks",
                    "--unload",
                    WATCH,
                    super::autostart::Autostart::ARGUMENT,
                ]
                .iter()
                .any(|v| a == v)
            })
        {
            eprintln!(
                "Usage: TaskbarMetrics.exe [--sample | --stop | --sensors | --autostart | --watch | --register-tasks | --remove-tasks | --unload | --help]"
            );
            return Ok(1);
        }
        if args.first().is_some_and(|v| v == "--help") {
            println!(
                "Usage: TaskbarMetrics.exe [--sample | --stop | --sensors | --autostart | --watch | --register-tasks | --remove-tasks | --unload | --help]\n\
                No arguments: attach to the Windows 11 taskbar and start the tray icon.\n\
                --sample: print three samples without changing Explorer.\n\
                --stop: close the program: the tiles, the collectors, the window and the tray icon.\n\
                --sensors: start the optional elevated CPU temperature collector (UAC).\n\
                --autostart: the start at sign-in: attach, retrying for a minute, then --sensors.\n\
                --watch: the tray icon, which attaches to a new Explorer and starts collectors\n\
                that ended again; the other starts run it.\n\
                --register-tasks, --remove-tasks: the installer's elevated steps that let the\n\
                collectors of a copy in Program Files start elevated without a UAC prompt.\n\
                --unload: close the window and the tray icon, restart Explorer to free the files.\n\
                Settings: %LOCALAPPDATA%\\Taskbar Metrics\\taskbar-metrics.conf."
            );
            return Ok(0);
        }
        WindowsSupport::validate()?;
        if args.first().is_some_and(|v| v == "--stop") {
            return self.stop();
        }
        let executable = std::env::current_exe().map_err(|_| E_FAIL)?;
        // The installer's elevated steps.
        if args.first().is_some_and(|v| v == "--register-tasks") {
            CollectorTask::register_all(executable.parent().ok_or(E_FAIL)?)?;
            return Ok(0);
        }
        if args.first().is_some_and(|v| v == "--remove-tasks") {
            CollectorTask::remove_all()?;
            return Ok(0);
        }
        if args.first().is_some_and(|v| v == WATCH) {
            return super::watch::Watcher::run(&executable);
        }
        if args.first().is_some_and(|v| v == "--unload") {
            super::unload::Unload::run(executable.parent().ok_or(E_FAIL)?)?;
            return Ok(0);
        }
        if args.first().is_some_and(|v| v == "--sensors") {
            let explorer = Explorer::find()?;
            super::temperature::SensorCollector::launch(
                &executable.with_file_name(Executables::SENSORS),
                explorer.pid,
            )?;
            println!(
                "CPU collector launch requested. It exits when this Explorer instance closes."
            );
            return Ok(0);
        }
        let settings = super::data_directory::DataDirectory::file("taskbar-metrics.conf")
            .map_err(|error| error.to_string())
            .and_then(|path| Settings::load(&path))
            .map_err(|error| {
                eprintln!("Configuration: {error}");
                E_FAIL
            })?;
        if args
            .first()
            .is_some_and(|v| v == super::autostart::Autostart::ARGUMENT)
        {
            return self.sign_in(&executable, &settings);
        }
        let mut monitor = Composition::monitor(&settings).map_err(|error| {
            eprintln!("{error}");
            E_FAIL
        })?;
        if args.first().is_some_and(|v| v == "--sample") {
            // Prime rate counters before the first displayed sample.
            let _ = monitor.sample_text();
            for _ in 0..3 {
                std::thread::sleep(settings.interval);
                println!("{}", monitor.sample_text());
            }
            return Ok(0);
        }
        self.attach_tiles(&executable, &settings, 1, false)
    }

    /// Attaches the tiles, trying `attempts` times 5 s apart, and starts the history
    /// recorder, and the CPU temperature collector with `sensor`. Another copy loaded
    /// in Explorer gives way first, and its collector carries over.
    fn attach_tiles(
        &self,
        executable: &Path,
        settings: &Settings,
        attempts: usize,
        sensor: bool,
    ) -> Result<i32> {
        const PAUSE: std::time::Duration = std::time::Duration::from_secs(5);
        let dll = executable.with_file_name(Executables::HOST);
        let handover = super::handover::Handover::take(&dll);
        // The taskbar of the restarted Explorer takes a while to start.
        let attempts = if handover.is_some() {
            attempts.max(Self::SIGN_IN_ATTEMPTS)
        } else {
            attempts
        };
        let sensor = sensor || handover.is_some_and(|handover| handover.sensor);
        let mut attached = self.attach(&dll);
        for _ in 1..attempts {
            if attached.is_ok() {
                break;
            }
            std::thread::sleep(PAUSE);
            attached = self.attach(&dll);
        }
        let code = attached?;
        // History of the taskbar devices is always recorded; only process monitoring
        // needs the elevated recorder.
        if matches!(code, 0 | 2) {
            let explorer = Explorer::find()?;
            if let Err(hr) = super::process_history::ProcessHistory::launch(
                &executable.with_file_name(Executables::HISTORY),
                explorer.pid,
                settings.process_monitoring,
            ) {
                eprintln!(
                    "Process history unavailable: 0x{hr:08X}; taskbar metrics remain active."
                );
            }
        }
        let sensors = executable.with_file_name(Executables::SENSORS);
        if sensor && sensors.is_file() {
            if let Err(hr) =
                super::temperature::SensorCollector::launch(&sensors, Explorer::find()?.pid)
            {
                eprintln!(
                    "CPU temperature unavailable: 0x{hr:08X}; taskbar metrics remain active."
                );
            }
        }
        // The tray icon keeps the program running from here on.
        super::watch::Watcher::spawn(executable);
        Ok(code)
    }

    /// Attempts to attach at sign-in, 5 s apart: a minute.
    const SIGN_IN_ATTEMPTS: usize = 12;

    /// The Run value at sign-in (`--autostart`): the taskbar may still be starting, so
    /// a failed attach is retried every 5 s for a minute; then the CPU temperature
    /// collector, when it is installed. Nothing opens on screen.
    fn sign_in(&self, executable: &Path, settings: &Settings) -> Result<i32> {
        Ok(self
            .attach_tiles(executable, settings, Self::SIGN_IN_ATTEMPTS, true)
            .unwrap_or(1))
    }

    /// Closes the program, as the tray icon's menu does.
    fn stop(&self) -> Result<i32> {
        super::shutdown::Shutdown::all();
        println!(
            "Closed: the tiles go asynchronously, the collectors, the window and the tray icon exit.\n\
            The DLL stays loaded until Explorer exits."
        );
        Ok(0)
    }

    fn attach(&self, dll: &Path) -> Result<i32> {
        if !dll.is_file() {
            eprintln!("{} is missing beside the executable.", Executables::HOST);
            return Err(E_FAIL);
        }
        let explorer = Explorer::find()?;
        unsafe {
            // Serialize launchers for this Explorer instance.
            let _launch = Handle::new(CreateMutexW(
                ptr::null_mut(),
                0,
                event_name("launch", explorer.pid).as_ptr(),
            ))?;
            if GetLastError() == 183 {
                eprintln!("Another launcher is attaching.");
                return Ok(2);
            }
            let ready = Handle::new(CreateEventW(
                ptr::null_mut(),
                1,
                0,
                event_name("ready", explorer.pid).as_ptr(),
            ))?;
            ResetEvent(ready.0);
            if let Ok(existing) = Handle::new(OpenEventW(
                0x100000,
                0,
                event_name("stop", explorer.pid).as_ptr(),
            )) {
                if WaitForSingleObject(existing.0, 0) != 0 {
                    eprintln!("Metrics are already running.");
                    return Ok(2);
                }
                let idle = Handle::new(OpenEventW(
                    0x100000,
                    0,
                    event_name("idle", explorer.pid).as_ptr(),
                ))?;
                if WaitForSingleObject(idle.0, 5000) != 0 {
                    return Err(E_FAIL);
                }
                let resume = Handle::new(OpenEventW(
                    2,
                    0,
                    event_name("resume", explorer.pid).as_ptr(),
                ))?;
                if SetEvent(resume.0) == 0 {
                    return Err(last_error());
                }
            } else {
                let _apartment = Apartment::mta()?;
                let module = SystemLibrary::load("Windows.UI.Xaml.dll")?;
                let address = module.symbol(c"InitializeXamlDiagnosticsEx")?;
                let initialize: Initialize = std::mem::transmute(address);
                let path: Vec<_> = dll.as_os_str().encode_wide().chain(Some(0)).collect();
                let mut hr = ERROR_NOT_FOUND_HR;
                for index in 1..=100 {
                    hr = initialize(
                        wide(&format!("VisualDiagConnection{index}")).as_ptr(),
                        explorer.pid,
                        wide("").as_ptr(),
                        path.as_ptr(),
                        CLSID,
                        ptr::null(),
                    );
                    if hr != ERROR_NOT_FOUND_HR {
                        break;
                    }
                }
                check(hr)?;
            }
            println!(
                "Attached to Explorer {}; waiting for a visible left-hand metrics panel...",
                explorer.pid
            );
            if WaitForSingleObject(ready.0, 15000) != 0 {
                eprintln!(
                    "No visible panel confirmed within 15 s. The session may still be active.\n\
                    There may be insufficient space, or this taskbar layout is unsupported.\n\
                    Use --stop to cancel. Debug output prefix: [TaskbarMetrics]."
                );
                return Ok(3);
            }
            println!("Metrics are visible on the left side of the taskbar.");
        }
        Ok(0)
    }
}

/// The Explorer that shows the taskbar.
pub(crate) struct Explorer {
    pub pid: u32,
    _process: Handle,
}

impl Explorer {
    pub fn find() -> Result<Self> {
        unsafe {
            let tray = FindWindowW(wide("Shell_TrayWnd").as_ptr(), ptr::null());
            let mut pid = 0;
            if tray.is_null() || GetWindowThreadProcessId(tray, &mut pid) == 0 {
                return Err(ERROR_NOT_FOUND_HR);
            }
            let process = Handle::new(OpenProcess(0x1000, 0, pid))?;
            let mut image = vec![0u16; 32768];
            let mut length = image.len() as u32;
            if QueryFullProcessImageNameW(process.0, 0, image.as_mut_ptr(), &mut length) == 0 {
                return Err(last_error());
            }
            let path = String::from_utf16_lossy(&image[..length as usize]);
            if !Path::new(&path)
                .file_name()
                .unwrap_or(OsStr::new(""))
                .to_string_lossy()
                .eq_ignore_ascii_case("explorer.exe")
            {
                return Err(E_UNEXPECTED);
            }
            let (mut machine, mut native) = (0, 0);
            if IsWow64Process2(process.0, &mut machine, &mut native) == 0 {
                return Err(last_error());
            }
            if machine != 0 || native != 0x8664 {
                eprintln!("Native x64 Explorer is required.");
                return Err(E_FAIL);
            }
            Ok(Self {
                pid,
                _process: process,
            })
        }
    }

    /// The path of the module `name` this Explorer has loaded.
    pub fn module(&self, name: &str) -> Option<PathBuf> {
        unsafe {
            // PROCESS_QUERY_LIMITED_INFORMATION | PROCESS_VM_READ
            let process = Handle::new(OpenProcess(0x1010, 0, self.pid)).ok()?;
            let mut modules: Vec<Raw> = vec![ptr::null_mut(); 1024];
            loop {
                let mut needed = 0;
                let size = (modules.len() * std::mem::size_of::<Raw>()) as u32;
                if K32EnumProcessModules(process.0, modules.as_mut_ptr(), size, &mut needed) == 0 {
                    return None;
                }
                let count = needed as usize / std::mem::size_of::<Raw>();
                if count <= modules.len() {
                    modules.truncate(count);
                    break;
                }
                modules.resize(count, ptr::null_mut());
            }
            let mut path = vec![0u16; 32768];
            modules.into_iter().find_map(|module| {
                let length = K32GetModuleFileNameExW(
                    process.0,
                    module,
                    path.as_mut_ptr(),
                    path.len() as u32,
                ) as usize;
                let file = PathBuf::from(OsString::from_wide(&path[..length]));
                file.file_name()
                    .is_some_and(|file| file.eq_ignore_ascii_case(name))
                    .then_some(file)
            })
        }
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn K32EnumProcessModules(process: Raw, modules: *mut Raw, size: u32, needed: *mut u32) -> i32;
    fn K32GetModuleFileNameExW(process: Raw, module: Raw, name: *mut u16, size: u32) -> u32;
}

#[repr(C)]
struct OsVersion {
    size: u32,
    major: u32,
    minor: u32,
    build: u32,
    platform: u32,
    service_pack: [u16; 128],
    service_major: u16,
    service_minor: u16,
    suite: u16,
    product: u8,
    reserved: u8,
}

#[link(name = "ntdll")]
extern "system" {
    fn RtlGetVersion(version: *mut OsVersion) -> i32;
}

pub(crate) struct WindowsSupport;

impl WindowsSupport {
    pub fn validate() -> Result<()> {
        let mut version: OsVersion = unsafe { std::mem::zeroed() };
        version.size = std::mem::size_of::<OsVersion>() as u32;
        if unsafe { RtlGetVersion(&mut version) } != 0
            || version.major != 10
            || version.build < 22000
            || version.product != 1
        {
            eprintln!("Only Windows 11 desktop is supported.");
            return Err(E_FAIL);
        }
        Ok(())
    }
}
