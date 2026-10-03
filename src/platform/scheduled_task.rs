//! Elevated collectors without a UAC prompt: the installer, running elevated once,
//! registers a Task Scheduler task per collector that starts it with the highest
//! rights of whoever runs the task; the launcher then runs the task instead of
//! asking for elevation. Only binaries in Program Files are registered, so no
//! process of the user can swap the file the task elevates.
use super::{abi::*, com::Com, executables::Executables};
use std::{path::Path, ptr};

/// The registered tasks, one per elevated collector.
pub(crate) struct CollectorTask {
    name: &'static str,
    executable: &'static str,
}
impl CollectorTask {
    pub const HISTORY: Self = Self {
        name: "History",
        executable: Executables::HISTORY,
    };
    pub const SENSORS: Self = Self {
        name: "Sensors",
        executable: Executables::SENSORS,
    };
    const ALL: [Self; 2] = [Self::HISTORY, Self::SENSORS];
    const FOLDER: &'static str = "Taskbar Metrics";

    /// Registers both tasks for the binaries in `directory`; needs elevation.
    pub fn register_all(directory: &Path) -> Result<()> {
        if !ProgramFiles::contains(directory) {
            eprintln!("Tasks are registered only for a copy in Program Files.");
            return Err(E_FAIL);
        }
        let scheduler = TaskScheduler::connect()?;
        let folder = scheduler.folder()?;
        for task in Self::ALL {
            let xml = TaskXml::new(&directory.join(task.executable).to_string_lossy());
            folder.register(task.name, &xml.text)?;
        }
        Ok(())
    }
    /// Removes the tasks and their folder; missing ones are not an error.
    pub fn remove_all() -> Result<()> {
        let scheduler = TaskScheduler::connect()?;
        if let Ok(folder) = scheduler.folder() {
            for task in Self::ALL {
                folder.delete(task.name);
            }
        }
        scheduler.delete_folder();
        Ok(())
    }
    /// Starts `executable --serve <pid>` through the task in this session, when the
    /// task starts this very file; an error otherwise, and the caller asks for UAC.
    pub fn run(&self, executable: &Path, pid: u32) -> Result<()> {
        let scheduler = TaskScheduler::connect()?;
        let task = scheduler.folder()?.task(self.name)?;
        let command = TaskXml::command(&task.xml()?).ok_or(E_FAIL)?;
        if !command.eq_ignore_ascii_case(&executable.to_string_lossy()) {
            return Err(E_FAIL);
        }
        task.run(&pid.to_string())
    }
}

/// The task definition: the collector of `command` with the PID of Explorer as
/// `$(Arg0)`, elevated for the members of Administrators, in the caller's session.
struct TaskXml {
    text: String,
}
impl TaskXml {
    fn new(command: &str) -> Self {
        let command = command
            .replace('&', "&amp;")
            .replace('<', "&lt;")
            .replace('>', "&gt;");
        Self {
            text: format!(
                r#"<?xml version="1.0" encoding="UTF-16"?>
<Task version="1.3" xmlns="http://schemas.microsoft.com/windows/2004/02/mit/task">
  <RegistrationInfo><Description>Starts a Taskbar Metrics collector with administrator rights.</Description></RegistrationInfo>
  <Principals><Principal id="Users"><GroupId>S-1-5-32-545</GroupId><RunLevel>HighestAvailable</RunLevel></Principal></Principals>
  <Settings>
    <MultipleInstancesPolicy>Parallel</MultipleInstancesPolicy>
    <DisallowStartIfOnBatteries>false</DisallowStartIfOnBatteries>
    <StopIfGoingOnBatteries>false</StopIfGoingOnBatteries>
    <ExecutionTimeLimit>PT0S</ExecutionTimeLimit>
    <Priority>5</Priority>
    <IdleSettings><StopOnIdleEnd>false</StopOnIdleEnd><RestartOnIdle>false</RestartOnIdle></IdleSettings>
  </Settings>
  <Actions Context="Users"><Exec><Command>{command}</Command><Arguments>--serve $(Arg0)</Arguments></Exec></Actions>
</Task>"#
            ),
        }
    }
    /// The executable a registered definition starts.
    fn command(xml: &str) -> Option<String> {
        let start = xml.find("<Command>")? + "<Command>".len();
        let end = start + xml[start..].find("</Command>")?;
        Some(
            xml[start..end]
                .replace("&lt;", "<")
                .replace("&gt;", ">")
                .replace("&quot;", "\"")
                .replace("&apos;", "'")
                .replace("&amp;", "&"),
        )
    }
}

/// Binaries in `%ProgramFiles%`, which only administrators can change.
struct ProgramFiles;
impl ProgramFiles {
    fn contains(directory: &Path) -> bool {
        std::env::var_os("ProgramW6432")
            .or_else(|| std::env::var_os("ProgramFiles"))
            .is_some_and(|root| {
                let root = root.to_string_lossy().to_lowercase();
                let directory = directory.to_string_lossy().to_lowercase();
                directory
                    .strip_prefix(&root)
                    .is_some_and(|rest| rest.starts_with('\\'))
            })
    }
}

/// `ITaskService`, connected to this computer.
struct TaskScheduler {
    service: Com,
    _apartment: Apartment,
}
impl TaskScheduler {
    /// Administrators and the system manage the tasks; users read and run them.
    const TASK_ACCESS: &'static str = "D:(A;;GA;;;BA)(A;;GA;;;SY)(A;;GRGX;;;AU)";
    const FOLDER_ACCESS: &'static str = "D:(A;;GA;;;BA)(A;;GA;;;SY)(A;;GR;;;AU)";
    fn connect() -> Result<Self> {
        let apartment = Apartment::mta()?;
        let mut raw = ptr::null_mut();
        unsafe {
            check(CoCreateInstance(
                &Guid::from_u128(0x0f87369f_a4e5_4cfc_bd3e_73e6154572dd),
                ptr::null_mut(),
                1,
                &Guid::from_u128(0x2faba4c7_4da9_4013_9697_20cc3fd40f85),
                &mut raw,
            ))?;
            let service = Com::owned(raw)?;
            let connect: unsafe extern "system" fn(Raw, Variant, Variant, Variant, Variant) -> Hr =
                service.slot(10);
            check(connect(
                service.raw(),
                Variant::EMPTY,
                Variant::EMPTY,
                Variant::EMPTY,
                Variant::EMPTY,
            ))?;
            Ok(Self {
                service,
                _apartment: apartment,
            })
        }
    }
    fn root(&self) -> Result<TaskFolder> {
        let path = Bstr::new("\\");
        let mut raw = ptr::null_mut();
        unsafe {
            let get: unsafe extern "system" fn(Raw, *const u16, *mut Raw) -> Hr =
                self.service.slot(7);
            check(get(self.service.raw(), path.0, &mut raw))?;
            Ok(TaskFolder(Com::owned(raw)?))
        }
    }
    /// `\Taskbar Metrics`, created when missing.
    fn folder(&self) -> Result<TaskFolder> {
        let root = self.root()?;
        root.child(CollectorTask::FOLDER)
            .or_else(|_| root.create(CollectorTask::FOLDER, Self::FOLDER_ACCESS))
    }
    fn delete_folder(&self) {
        if let Ok(root) = self.root() {
            let name = Bstr::new(CollectorTask::FOLDER);
            unsafe {
                let delete: unsafe extern "system" fn(Raw, *const u16, i32) -> Hr = root.0.slot(12);
                delete(root.0.raw(), name.0, 0);
            }
        }
    }
}

/// `ITaskFolder`.
struct TaskFolder(Com);
impl TaskFolder {
    fn child(&self, name: &str) -> Result<Self> {
        let name = Bstr::new(name);
        let mut raw = ptr::null_mut();
        unsafe {
            let get: unsafe extern "system" fn(Raw, *const u16, *mut Raw) -> Hr = self.0.slot(9);
            check(get(self.0.raw(), name.0, &mut raw))?;
            Ok(Self(Com::owned(raw)?))
        }
    }
    fn create(&self, name: &str, access: &str) -> Result<Self> {
        let (name, access) = (Bstr::new(name), Bstr::new(access));
        let mut raw = ptr::null_mut();
        unsafe {
            let create: unsafe extern "system" fn(Raw, *const u16, Variant, *mut Raw) -> Hr =
                self.0.slot(11);
            check(create(
                self.0.raw(),
                name.0,
                Variant::text(&access),
                &mut raw,
            ))?;
            Ok(Self(Com::owned(raw)?))
        }
    }
    fn task(&self, name: &str) -> Result<RegisteredTask> {
        let name = Bstr::new(name);
        let mut raw = ptr::null_mut();
        unsafe {
            let get: unsafe extern "system" fn(Raw, *const u16, *mut Raw) -> Hr = self.0.slot(13);
            check(get(self.0.raw(), name.0, &mut raw))?;
            Ok(RegisteredTask(Com::owned(raw)?))
        }
    }
    fn delete(&self, name: &str) {
        let name = Bstr::new(name);
        unsafe {
            let delete: unsafe extern "system" fn(Raw, *const u16, i32) -> Hr = self.0.slot(15);
            delete(self.0.raw(), name.0, 0);
        }
    }
    fn register(&self, name: &str, xml: &str) -> Result<()> {
        const CREATE_OR_UPDATE: i32 = 6;
        const LOGON_GROUP: i32 = 4;
        let (name, xml) = (Bstr::new(name), Bstr::new(xml));
        let access = Bstr::new(TaskScheduler::TASK_ACCESS);
        let mut raw = ptr::null_mut();
        unsafe {
            #[allow(clippy::type_complexity)]
            let register: unsafe extern "system" fn(
                Raw,
                *const u16,
                *const u16,
                i32,
                Variant,
                Variant,
                i32,
                Variant,
                *mut Raw,
            ) -> Hr = self.0.slot(16);
            check(register(
                self.0.raw(),
                name.0,
                xml.0,
                CREATE_OR_UPDATE,
                Variant::EMPTY,
                Variant::EMPTY,
                LOGON_GROUP,
                Variant::text(&access),
                &mut raw,
            ))?;
            drop(Com::owned(raw));
        }
        Ok(())
    }
}

/// `IRegisteredTask`.
struct RegisteredTask(Com);
impl RegisteredTask {
    fn xml(&self) -> Result<String> {
        let mut raw = ptr::null_mut();
        unsafe {
            let get: unsafe extern "system" fn(Raw, *mut *mut u16) -> Hr = self.0.slot(20);
            check(get(self.0.raw(), &mut raw))?;
        }
        Ok(Bstr(raw).text())
    }
    /// `RunEx` with one argument, in the session of this process.
    fn run(&self, argument: &str) -> Result<()> {
        const USE_SESSION_ID: i32 = 4;
        let argument = Bstr::new(argument);
        let mut session = 0;
        let mut raw = ptr::null_mut();
        unsafe {
            if ProcessIdToSessionId(GetCurrentProcessId(), &mut session) == 0 {
                return Err(last_error());
            }
            let run: unsafe extern "system" fn(Raw, Variant, i32, i32, *const u16, *mut Raw) -> Hr =
                self.0.slot(13);
            check(run(
                self.0.raw(),
                Variant::text(&argument),
                USE_SESSION_ID,
                session as i32,
                ptr::null(),
                &mut raw,
            ))?;
            drop(Com::owned(raw));
        }
        Ok(())
    }
}

/// An owned `BSTR`.
struct Bstr(*mut u16);
impl Bstr {
    fn new(text: &str) -> Self {
        Self(unsafe { SysAllocString(wide(text).as_ptr()) })
    }
    fn text(&self) -> String {
        if self.0.is_null() {
            return String::new();
        }
        unsafe {
            let length = SysStringLen(self.0) as usize;
            String::from_utf16_lossy(std::slice::from_raw_parts(self.0, length))
        }
    }
}
impl Drop for Bstr {
    fn drop(&mut self) {
        unsafe { SysFreeString(self.0) }
    }
}

/// A `VARIANT` passed by value: empty, or a `BSTR` the caller keeps alive.
#[repr(C)]
#[derive(Clone, Copy)]
struct Variant {
    kind: u16,
    reserved: [u16; 3],
    value: [usize; 2],
}
impl Variant {
    const EMPTY: Self = Self {
        kind: 0,
        reserved: [0; 3],
        value: [0; 2],
    };
    fn text(text: &Bstr) -> Self {
        Self {
            kind: 8,
            value: [text.0 as usize, 0],
            ..Self::EMPTY
        }
    }
}

#[link(name = "ole32")]
extern "system" {
    fn CoCreateInstance(
        class: *const Guid,
        outer: Raw,
        context: u32,
        iid: *const Guid,
        result: *mut Raw,
    ) -> Hr;
}
#[link(name = "oleaut32")]
extern "system" {
    fn SysAllocString(text: *const u16) -> *mut u16;
    fn SysFreeString(text: *mut u16);
    fn SysStringLen(text: *mut u16) -> u32;
}
#[link(name = "kernel32")]
extern "system" {
    fn ProcessIdToSessionId(pid: u32, session: *mut u32) -> i32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_definition_starts_the_escaped_command_and_reads_it_back() {
        let command = r"C:\Program Files\R&D <x>\TaskbarMetrics.History.exe";
        let xml = TaskXml::new(command);
        assert!(xml.text.contains(r"R&amp;D &lt;x&gt;"));
        assert!(xml.text.contains("<Arguments>--serve $(Arg0)</Arguments>"));
        assert_eq!(TaskXml::command(&xml.text).as_deref(), Some(command));
    }
    #[test]
    fn only_copies_inside_program_files_are_registered() {
        let root = std::env::var("ProgramW6432")
            .or_else(|_| std::env::var("ProgramFiles"))
            .unwrap();
        assert!(ProgramFiles::contains(
            &Path::new(&root).join("Taskbar Metrics")
        ));
        assert!(!ProgramFiles::contains(Path::new(&format!("{root} (x)"))));
        assert!(!ProgramFiles::contains(Path::new(
            r"C:\Users\me\Taskbar Metrics"
        )));
    }
}
