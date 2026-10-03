use super::{tokens, ui::Ui};

/// Independent from the taskbar palette and all persisted widget settings.
#[derive(Clone, Copy, PartialEq)]
pub struct Design {
    pub dark: bool,
}

impl Design {
    pub fn color(self, name: &str) -> &'static str {
        tokens::COLORS
            .iter()
            .find(|(key, _, _)| *key == name)
            .map(|(_, light, dark)| if self.dark { *dark } else { *light })
            .unwrap_or("Transparent")
    }

    pub fn pin(self, index: usize) -> &'static str {
        tokens::PINS[usize::from(self.dark)][index % 6]
    }

    /// Total, pinned and hovered series fill alphas for the current theme.
    pub fn fill(self, series: usize) -> f64 {
        tokens::FILLS[usize::from(self.dark)][series]
    }

    /// `#RRGGBB` → `#AARRGGBB` with the given opacity.
    pub fn alpha(color: &str, opacity: f64) -> String {
        let rgb = color.trim_start_matches('#');
        let rgb = &rgb[rgb.len().saturating_sub(6)..];
        format!(
            "#{:02X}{rgb}",
            (opacity.clamp(0.0, 1.0) * 255.0).round() as u8
        )
    }

    pub fn icon(self, name: &str, color: &str) -> String {
        self.icon_sized(name, color, 16.0, 1.3)
    }

    /// A 16×16 token icon scaled to `size`; `stroke` is in 16-unit space like the SVG source.
    pub fn icon_sized(self, name: &str, color: &str, size: f64, stroke: f64) -> String {
        let mut result = format!(
            r#"<Viewbox Width="{size}" Height="{size}" VerticalAlignment="Center" IsHitTestVisible="False"><Canvas Width="16" Height="16">"#
        );
        if let Some((_, paths)) = tokens::ICONS.iter().find(|(key, _)| *key == name) {
            for (path, fill) in *paths {
                result.push_str(&format!(r#"<Path Data="{}" {}="{}" StrokeThickness="{stroke}" StrokeStartLineCap="Round" StrokeEndLineCap="Round" StrokeLineJoin="Round"/>"#, Ui::xml(path), if *fill { "Fill" } else { "Stroke" }, color));
            }
        }
        result.push_str("</Canvas></Viewbox>");
        result
    }

    /// Expands `$icon:name:size:stroke:colorToken$` placeholders of a XAML template.
    fn icons(self, markup: &str) -> String {
        let mut result = String::with_capacity(markup.len());
        let mut rest = markup;
        while let Some(start) = rest.find("$icon:") {
            result.push_str(&rest[..start]);
            let tail = &rest[start + 6..];
            let Some(end) = tail.find('$') else {
                rest = &rest[start..];
                break;
            };
            let parts: Vec<_> = tail[..end].split(':').collect();
            match parts[..] {
                [name, size, stroke, color] => result.push_str(&self.icon_sized(
                    name,
                    self.color(color),
                    size.parse().unwrap_or(16.0),
                    stroke.parse().unwrap_or(1.3),
                )),
                _ => result.push_str(&rest[start..start + 7 + end]),
            }
            rest = &tail[end + 1..];
        }
        result.push_str(rest);
        result
    }

    pub fn markup(self, markup: &str) -> String {
        let mut result = self.icons(markup).replace(
            "RequestedTheme=\"Default\"",
            if self.dark {
                "RequestedTheme=\"Dark\""
            } else {
                "RequestedTheme=\"Light\""
            },
        );
        for (key, _, _) in tokens::COLORS {
            result = result.replace(&format!("${key}$"), self.color(key));
        }
        for (key, color, opacity) in [
            ("accentHover", "accent", 0.9),
            ("accentPress", "accent", 0.8),
            ("accentRing", "accent", 0.22),
            ("accentBorder", "accent", 0.35),
            ("scrollThumb", "text3", 0.45),
        ] {
            result = result.replace(
                &format!("${key}$"),
                &Self::alpha(self.color(color), opacity),
            );
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn alpha_prefixes_rgb_and_icons_scale_without_resizing_layout() {
        assert_eq!(Design::alpha("#005FB8", 0.35), "#59005FB8");
        let icon = Design { dark: false }.icon_sized("clock", "#000000", 11.0, 1.6);
        assert!(icon.starts_with(r#"<Viewbox Width="11" Height="11""#));
        assert!(icon.contains(r#"StrokeThickness="1.6""#));
    }

    #[test]
    fn template_icons_and_colors_are_expanded() {
        let markup = Design { dark: false }.markup(r#"<A B="$text$">$icon:menu:16:1.3:text3$</A>"#);
        assert!(markup.starts_with(r##"<A B="#1B1B1B"><Viewbox Width="16""##));
        assert!(markup.contains(r##"Stroke="#62666C""##));
        assert!(!markup.contains('$'));
    }
}
