use super::super::{window_memory::WindowMemory, *};
use super::{locale::Language, ui::Ui};
use crate::platform::autostart::Autostart;

/// An on/off option kept outside the window.
trait Flag {
    fn get(&self) -> bool;
    fn set(&self, on: bool) -> std::io::Result<()>;
    /// Caption under the box, a key of the translation catalog.
    fn caption(&self, on: bool) -> &'static str;
}
impl Flag for Autostart {
    fn get(&self) -> bool {
        self.enabled()
    }
    fn set(&self, on: bool) -> std::io::Result<()> {
        Autostart::set(self, on)
    }
    fn caption(&self, _: bool) -> &'static str {
        "Taskbar tiles and the CPU temperature sensor"
    }
}
impl Flag for WindowMemory {
    fn get(&self) -> bool {
        self.resets()
    }
    fn set(&self, on: bool) -> std::io::Result<()> {
        self.set_resets(on)
    }
    fn caption(&self, on: bool) -> &'static str {
        if on {
            "The window opens in the middle of the main monitor at 1280 × 1024"
        } else {
            "The window opens where it was closed"
        }
    }
}
/// A check box bound to a flag, with the caption below it.
struct FlagBox {
    root: Com,
    check: Com,
    status: &'static str,
    flag: Box<dyn Flag>,
    shown: bool,
    language: Language,
}
impl FlagBox {
    fn new(
        root: &Com,
        (check, status): (&str, &'static str),
        flag: Box<dyn Flag>,
        language: Language,
    ) -> Result<Self> {
        let flag_box = Self {
            root: root.clone(),
            check: Ui::find(root, check)?,
            status,
            shown: flag.get(),
            flag,
            language,
        };
        Ui::checked(&flag_box.check, flag_box.shown)?;
        flag_box.caption(flag_box.flag.caption(flag_box.shown))?;
        Ok(flag_box)
    }
    fn caption(&self, text: &str) -> Result<()> {
        Ui::text(&self.root, self.status, self.language.text(text))
    }
    /// Stores a click; a failed write puts the box back and says why.
    fn refresh(&mut self, writable: bool) -> Result<()> {
        let on = Ui::is_checked(&self.check)?;
        if on == self.shown {
            return Ok(());
        }
        if writable {
            if let Err(error) = self.flag.set(on) {
                Ui::checked(&self.check, self.shown)?;
                let failed = self.language.text("Could not save setting");
                return Ui::text(&self.root, self.status, &format!("{failed}: {error}"));
            }
        }
        self.shown = on;
        self.caption(self.flag.caption(on))
    }
}

/// "Startup and window": start at sign-in, and whether a closed window opens afresh.
/// Checks and demos show the boxes without writing anything.
pub(super) struct StartupOptions {
    boxes: Vec<FlagBox>,
    writable: bool,
}
impl StartupOptions {
    pub fn new(root: &Com, language: Language) -> Result<Self> {
        let exe = std::env::current_exe().map_err(|_| E_FAIL)?;
        let directory = exe.parent().ok_or(E_FAIL)?;
        Ok(Self {
            boxes: vec![
                FlagBox::new(
                    root,
                    ("Autostart", "AutostartStatus"),
                    Box::new(Autostart::new(directory)),
                    language,
                )?,
                FlagBox::new(
                    root,
                    ("ResetWindow", "ResetWindowStatus"),
                    Box::new(WindowMemory::locate().ok_or(E_FAIL)?),
                    language,
                )?,
            ],
            writable: super::persistent(),
        })
    }
    pub fn refresh(&mut self) -> Result<()> {
        for flag_box in &mut self.boxes {
            flag_box.refresh(self.writable)?;
        }
        Ok(())
    }
}
