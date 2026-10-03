use std::path::PathBuf;

/// The main window's normal frame in screen pixels and whether it was maximized.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(super) struct Placement {
    pub maximized: bool,
    pub left: i32,
    pub top: i32,
    pub right: i32,
    pub bottom: i32,
}
impl Placement {
    /// «1,100,80,1380,1104»: maximized flag, then the frame.
    fn parse(text: &str) -> Option<Self> {
        let numbers = text
            .split(',')
            .map(|part| part.trim().parse::<i32>().ok())
            .collect::<Option<Vec<_>>>()?;
        let [maximized, left, top, right, bottom] = numbers[..] else {
            return None;
        };
        (right > left && bottom > top).then_some(Self {
            maximized: maximized != 0,
            left,
            top,
            right,
            bottom,
        })
    }
    fn line(&self) -> String {
        format!(
            "{},{},{},{},{}",
            u8::from(self.maximized),
            self.left,
            self.top,
            self.right,
            self.bottom
        )
    }
}

/// `taskbar-metrics.window`: whether a closed window opens afresh, centred at its
/// default size (the default), and otherwise where it was closed.
#[derive(Clone, Copy, Debug, PartialEq)]
struct WindowState {
    reset: bool,
    placement: Option<Placement>,
}
impl Default for WindowState {
    fn default() -> Self {
        Self {
            reset: true,
            placement: None,
        }
    }
}
impl WindowState {
    fn parse(text: &str) -> Self {
        let mut state = Self::default();
        for line in text.lines() {
            match line.split_once('=') {
                Some(("reset", value)) => state.reset = value.trim() != "false",
                Some(("placement", value)) => state.placement = Placement::parse(value),
                _ => {}
            }
        }
        state
    }
    fn text(&self) -> String {
        let mut text = format!("reset={}\n", self.reset);
        if let Some(placement) = self.placement {
            text.push_str(&format!("placement={}\n", placement.line()));
        }
        text
    }
}

pub(super) struct WindowMemory {
    path: PathBuf,
}
impl WindowMemory {
    pub fn locate() -> Option<Self> {
        Some(Self {
            path: crate::platform::data_directory::DataDirectory::file("taskbar-metrics.window")
                .ok()?,
        })
    }
    fn state(&self) -> WindowState {
        std::fs::read_to_string(&self.path)
            .map(|text| WindowState::parse(&text))
            .unwrap_or_default()
    }
    fn store(&self, state: WindowState) -> std::io::Result<()> {
        let temporary = self.path.with_extension("window.tmp");
        std::fs::write(&temporary, state.text())?;
        std::fs::rename(temporary, &self.path)
    }
    pub fn resets(&self) -> bool {
        self.state().reset
    }
    pub fn set_resets(&self, reset: bool) -> std::io::Result<()> {
        self.store(WindowState {
            reset,
            ..self.state()
        })
    }
    /// Where to open the window; None for a fresh centred window.
    pub fn placement(&self) -> Option<Placement> {
        let state = self.state();
        state.placement.filter(|_| !state.reset)
    }
    /// Keeps the frame of a closing window unless the window opens afresh.
    pub fn remember(&self, placement: Option<Placement>) -> std::io::Result<()> {
        let state = self.state();
        self.store(WindowState {
            placement: placement.filter(|_| !state.reset),
            ..state
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn the_window_opens_afresh_unless_told_to_remember() {
        assert_eq!(WindowState::parse(""), WindowState::default());
        let placement = Placement {
            maximized: true,
            left: -1200,
            top: 40,
            right: -100,
            bottom: 900,
        };
        let state = WindowState {
            reset: false,
            placement: Some(placement),
        };
        assert_eq!(WindowState::parse(&state.text()), state);
        assert_eq!(state.text(), "reset=false\nplacement=1,-1200,40,-100,900\n");
        // An empty or broken frame is ignored.
        assert_eq!(Placement::parse("0,10,10,10,20"), None);
        assert_eq!(Placement::parse("0,10,10,x,20"), None);
        assert_eq!(Placement::parse("0,10,10,20"), None);
    }
}
