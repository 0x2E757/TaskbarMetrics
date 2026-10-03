#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Palette(pub [u32; 14]);
impl Palette {
    /// Hover and pressed backgrounds are those of the taskbar's own buttons, such as
    /// the weather: a white layer in the light theme, Fluent's fills in the dark one.
    pub fn light() -> Self {
        Self([
            0x05000000, 0xff282828, 0xff1a1a1a, 0xff640000, 0xff964b32, 0xff005fb8, 0xff964b32,
            0xff007dc8, 0xff0064c8, 0x99ff0000, 0xffc87d32, 0x80ffffff, 0x4dffffff, 0xfff3f3f3,
        ])
    }
    pub fn dark() -> Self {
        Self([
            0x0fffffff, 0xffc8c8c8, 0xffffffff, 0xffffc8c8, 0xffff967d, 0xff3296e1, 0xffff967d,
            0xff3296e1, 0xff197de1, 0xb3ff4b4b, 0xffffb454, 0x15ffffff, 0x08ffffff, 0xff202024,
        ])
    }
    /// Hover and pressed colors of earlier defaults, which darkened a light tile,
    /// become the current defaults; colors chosen by hand stay.
    pub fn retire_defaults(&mut self, dark: bool) {
        let (retired, current) = if dark {
            ([0x16ffffff, 0x22ffffff], Self::dark())
        } else {
            ([0x16000000, 0x22000000], Self::light())
        };
        for (role, old) in [11, 12].into_iter().zip(retired) {
            if self.0[role] == old {
                self.0[role] = current.0[role];
            }
        }
    }
    pub fn parse(text: &str) -> Option<u32> {
        let hex = text.trim().strip_prefix('#')?;
        if !matches!(hex.len(), 6 | 8) || !hex.bytes().all(|c| c.is_ascii_hexdigit()) {
            return None;
        }
        let value = u32::from_str_radix(hex, 16).ok()?;
        Some(if hex.len() == 6 {
            value | 0xff000000
        } else {
            value
        })
    }
    pub fn hex(&self, role: usize) -> String {
        format!("#{:08X}", self.0[role])
    }
    pub fn apply(&self, markup: &str, fade: f64, id: &str) -> String {
        let result = self.apply_theme(markup, fade, id, "Default");
        self.apply_theme(&result, fade, id, "Light")
    }
    pub fn apply_defaults(markup: &str, fade: f64, id: &str) -> String {
        let result = Self::dark().apply_theme(markup, fade, id, "Default");
        Self::light().apply_theme(&result, fade, id, "Light")
    }
    pub(super) fn apply_theme(&self, markup: &str, fade: f64, id: &str, theme: &str) -> String {
        let temperature = matches!(id, "cpu" | "gpu");
        let mut resources = String::new();
        for (key, role) in [
            ("MetricTile", 0),
            ("MetricDim", 1),
            ("MetricForeground", 2),
            ("MetricHot", 3),
            ("MetricTemperatureText", 4),
            ("MetricDown", 5),
            ("MetricUp", 6),
            ("MetricHover", 11),
            ("MetricPressed", 12),
        ] {
            resources.push_str(&format!(
                r#"<SolidColorBrush x:Key="{key}" Color="{}"/>"#,
                self.hex(role)
            ));
        }
        for (key, role) in [
            ("MetricAccent", 7),
            ("MetricArea", 8),
            ("MetricTemperature", 9),
            ("MetricUpload", 10),
        ] {
            resources.push_str(&format!(
                r#"<Color x:Key="{key}">{}</Color>"#,
                self.hex(role)
            ));
        }
        for (key, role) in [
            ("MetricAccentFaded", 7),
            ("MetricAreaFaded", 8),
            ("SecondaryFaded", if temperature { 9 } else { 10 }),
        ] {
            let color = self.0[role];
            let alpha = ((color >> 24) as f64 * fade / 100.0).round() as u32;
            resources.push_str(&format!(
                "<Color x:Key=\"{key}\">#{:08X}</Color>",
                (color & 0xffffff) | (alpha << 24)
            ));
        }
        // The elevation border of a hovered or pressed tile, darker on its bottom
        // pixel row, as Windows draws it around the weather: measured on the light
        // taskbar, Fluent's values in the dark theme.
        let (top, bottom) = if theme == "Light" {
            ("#0B000000", "#15000000")
        } else {
            ("#18FFFFFF", "#12FFFFFF")
        };
        resources.push_str(&format!(
            r#"<LinearGradientBrush x:Key="MetricHoverBorder" MappingMode="Absolute" StartPoint="0,0" EndPoint="0,38"><GradientStop Color="{top}" Offset="0.973"/><GradientStop Color="{bottom}" Offset="0.974"/></LinearGradientBrush>"#
        ));
        let mut result = markup.to_string();
        {
            let marker = format!("<ResourceDictionary x:Key=\"{theme}\">");
            if let Some(start) = result.find(&marker).map(|i| i + marker.len()) {
                if let Some(end) = result[start..].find("</ResourceDictionary>") {
                    result.replace_range(start..start + end, &resources);
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn earlier_hover_defaults_retire_and_chosen_colors_stay() {
        let mut light = Palette::light();
        light.0[11] = 0x16000000;
        light.0[12] = 0x33123456;
        light.retire_defaults(false);
        assert_eq!(light.0[11], Palette::light().0[11]);
        assert_eq!(light.0[12], 0x33123456);
        let mut dark = Palette::dark();
        dark.0[12] = 0x22ffffff;
        dark.retire_defaults(true);
        assert_eq!(dark, Palette::dark());
        let markup = Palette::light().apply_theme(
            r#"<ResourceDictionary x:Key="Light"></ResourceDictionary>"#,
            15.0,
            "cpu",
            "Light",
        );
        assert!(markup.contains(r#"x:Key="MetricHoverBorder""#) && markup.contains("#15000000"));
    }
    #[test]
    fn hex_colors_require_complete_valid_input() {
        assert_eq!(Palette::parse("#aAbBcC"), Some(0xffaabbcc));
        assert_eq!(Palette::parse(" #80AABBCC "), Some(0x80aabbcc));
        for invalid in [
            "",
            "#",
            "red",
            "#12345",
            "#1234567",
            "#GG0000",
            "#123456789",
            "<Color/>",
        ] {
            assert_eq!(Palette::parse(invalid), None);
        }
    }
}
