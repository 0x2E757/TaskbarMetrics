//! The warning after the tiles shown when the taskbar has no room for the rest.
use super::{button::Markup, *};
use crate::platform::metrics_window::Language;

/// A warning icon whose tooltip names the tiles left out.
#[derive(Clone)]
pub(super) struct OverflowMarker {
    root: Com,
    text: Com,
}
impl OverflowMarker {
    /// Width of the icon's place, in logical pixels.
    pub const WIDTH: f64 = 24.0;
    pub fn new() -> Result<Self> {
        let root = Markup::load(
            &include_str!("overflow.xaml").replace("@WIDTH@", &Self::WIDTH.to_string()),
        )?;
        let text = Markup::find(&root, "OverflowText")?.query(&TEXT)?;
        Ok(Self { root, text })
    }
    pub fn element(&self) -> Result<Com> {
        self.root.query(&UI_ELEMENT)
    }
    /// Names the `hidden` tile labels, or hides the icon when none is left out.
    pub fn show(&self, hidden: &[String]) -> Result<()> {
        if !hidden.is_empty() {
            let language = Language::load();
            self.text.set_string(
                27,
                &format!(
                    "{} {}",
                    language.text("Not enough room on the taskbar for"),
                    hidden.join(", ")
                ),
            )?;
        }
        XamlElement(self.element()?).set_enum(22, i32::from(hidden.is_empty()))
    }
}
