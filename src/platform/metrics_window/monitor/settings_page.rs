use super::super::{playground::Playground, *};
use super::{
    alert_range::{AlertRange, ALERTS, RULES},
    chart_line::{ChartLine, ChartLineSlider},
    design::Design,
    locale::Language,
    monitor_options::MonitorOptions,
    plain_button::PlainButton,
    scroll_indicator::ScrollIndicator,
    startup_options::StartupOptions,
    theme::ThemeChoice,
    ui::Ui,
};

use crate::platform::{
    devices::{DeviceCatalog, DeviceId},
    displays::Displays,
    process_history::store::Frame,
    xaml::{appearance::Appearance, events::Subscription, TileStyle},
};

use std::{cell::RefCell, rc::Rc};

/// What the settings page asks of the window after a refresh.
pub enum SettingsOutcome {
    Language(Language),
    /// Another window theme, to save and apply.
    Theme(ThemeChoice),
    /// New appearance (and alert thresholds) were written for the taskbar.
    Applied,
    /// New width of the chart line, already saved.
    ChartLine(f64),
}

/// A tab of the settings page: what applies at once, and what reaches the
/// taskbar with the apply button. The first one opens from the menu.
#[derive(Clone, Copy, PartialEq, Eq, Default, Debug)]
pub enum SettingsTab {
    #[default]
    General,
    Taskbar,
}

impl SettingsTab {
    const ALL: [Self; 2] = [Self::General, Self::Taskbar];

    /// Middle of the names `Tab{name}`, `Tab{name}On`, `Tab{name}Off`, `Tab{name}Line`.
    fn name(self) -> &'static str {
        match self {
            Self::Taskbar => "Taskbar",
            Self::General => "General",
        }
    }

    /// Elements shown only while this tab is open.
    fn elements(self) -> &'static [&'static str] {
        match self {
            Self::Taskbar => &["TaskbarPreview", "TaskbarScroll"],
            Self::General => &["GeneralScroll"],
        }
    }
}

enum SettingsEvent {
    Tab(SettingsTab),
    Toggle(usize),
    Apply,
    Revert,
    Language(Language),
    Theme(ThemeChoice),
    /// "Live values" (true) or "Test" preview values.
    Live(bool),
    ResetColors,
}

/// One collapsible group: icon, title, description and its rows.
struct Group {
    icon: &'static str,
    title: &'static str,
    description: &'static str,
}

const GROUPS: [Group; 6] = [
    Group {
        icon: "layout",
        title: "Sizes and spacing",
        description: "Tile width, spacing, corner radius",
    },
    Group {
        icon: "type",
        title: "Typography",
        description: "Text sizes and value offsets",
    },
    Group {
        icon: "chart",
        title: "Tile charts",
        description: "Line width, area opacity, fade, temperature",
    },
    Group {
        icon: "palette",
        title: "Colors",
        description: "Separately for light and dark themes, HEX with alpha",
    },
    Group {
        icon: "bell",
        title: "Temperature and memory",
        description: "Between the thresholds the tile turns red; above the upper one it pulses",
    },
    Group {
        icon: "alert",
        title: "Number color",
        description: "Between the thresholds the tile number gradually turns red",
    },
];

/// Settings section of the main window: live preview, grouped widget settings,
/// monitoring and language. Widget logic stays in `Playground`.
pub struct SettingsPage {
    root: Com,
    playground: Playground,
    /// "Chart line width".
    chart_line: ChartLineSlider,
    /// "Startup and window".
    startup: StartupOptions,
    /// "Monitors".
    monitors: MonitorOptions,
    design: Design,
    language: Language,
    theme: ThemeChoice,
    tab: SettingsTab,
    events: Rc<RefCell<Vec<SettingsEvent>>>,
    _subscriptions: Vec<Subscription>,
    expanded: [bool; 6],
    saved: TileStyle,
    shown: Option<(usize, [String; 6], Vec<String>)>,
    live: bool,
    /// Rule switches and the state each was last shown in.
    switches: Vec<(Com, bool)>,
    /// Thresholds the alert tracks were painted for.
    painted: Option<[(f64, f64); RULES.len()]>,
    /// Reading ids of the tiles' devices in the recorder's frames.
    live_keys: [String; 9],
}

impl SettingsPage {
    fn expander(d: Design, index: usize, body: &str) -> String {
        let group = &GROUPS[index];
        format!(
            r#"<Border Background="$card$" BorderBrush="$border$" BorderThickness="1" CornerRadius="6"><StackPanel><Button x:Name="Expander{index}" Style="{{StaticResource ExpanderButton}}" AutomationProperties.Name="@{title}@"><Grid ColumnSpacing="16"><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition/><ColumnDefinition Width="Auto"/><ColumnDefinition Width="32"/></Grid.ColumnDefinitions>{icon}<StackPanel Grid.Column="1" Spacing="1" VerticalAlignment="Center"><TextBlock Text="@{title}@"/><TextBlock Text="@{description}@" FontSize="12" Foreground="$text2$" TextWrapping="Wrap"/></StackPanel><TextBlock x:Name="Summary{index}" Grid.Column="2" FontSize="12" Foreground="$text2$" VerticalAlignment="Center" Typography.NumeralAlignment="Tabular"/><Grid Grid.Column="3" HorizontalAlignment="Center"><Grid x:Name="Chevron{index}">{down}</Grid><Grid x:Name="ChevronUp{index}" Visibility="Collapsed">{up}</Grid></Grid></Grid></Button><Border x:Name="ExpanderBody{index}" Visibility="Collapsed" BorderBrush="$divider$" BorderThickness="0,1,0,0">{body}</Border></StackPanel></Border>"#,
            icon = d.icon(group.icon, d.color("text2")),
            title = group.title,
            description = group.description,
            down = d.icon_sized("chev", d.color("text2"), 14.0, 1.3),
            up = d.icon_sized("chevu", d.color("text2"), 14.0, 1.3),
        )
    }

    /// Rows of `RULES[range]`, divided from each other but not from their group.
    fn rules(range: std::ops::Range<usize>) -> String {
        let first = range.start;
        range
            .map(|index| AlertRange::row(index, &RULES[index], index > first))
            .collect()
    }

    fn markup(design: Design, language: Language, displays: &Displays) -> String {
        let bodies = [
            Playground::rows(&[0, 1, 2]),
            Playground::rows(&[3, 4, 5, 8, 9]),
            format!(
                "<StackPanel>{}{}</StackPanel>",
                Playground::rows(&[6, 7, 10, 11, 12, 13]),
                Playground::switch_rows()
            ),
            super::super::colors::ColorEditor::markup(true),
            format!(
                "<StackPanel>{}{}</StackPanel>",
                Self::rules(0..ALERTS),
                Playground::rows(&[20, 21]).replacen(
                    r#"BorderThickness="0,0,0,0""#,
                    r#"BorderThickness="0,1,0,0""#,
                    1
                )
            ),
            format!(
                "<StackPanel>{}</StackPanel>",
                Self::rules(ALERTS..RULES.len())
            ),
        ];
        let mut markup = include_str!("settings.xaml")
            .replace("@DEMO@", &Playground::preview_values())
            .replace("@CHARTLINE@", &ChartLine::markup())
            .replace("@MONITORS@", &MonitorOptions::markup(displays, language));
        for (index, body) in bodies.iter().enumerate() {
            markup = markup.replace(
                &format!("@EXPANDER{index}@"),
                &Self::expander(design, index, body),
            );
        }
        let markup = PlainButton::markup(&ScrollIndicator::markup(&markup));
        design.markup(&language.markup(&markup))
    }

    pub fn new(
        design: Design,
        language: Language,
        appearance: Option<std::path::PathBuf>,
        tab: SettingsTab,
    ) -> Result<Self> {
        let saved = appearance
            .and_then(|path| Appearance::read(&path).ok())
            .unwrap_or_default();
        let displays = Displays::current();
        let root = Ui::load(&Self::markup(design, language, &displays))?;
        let mut page = Self {
            playground: Playground::new(&root)?,
            chart_line: ChartLineSlider::new(&root)?,
            startup: StartupOptions::new(&root, language)?,
            monitors: MonitorOptions::new(&root, displays, language)?,
            root,
            design,
            language,
            theme: ThemeChoice::load(),
            tab,
            events: Rc::new(RefCell::new(Vec::new())),
            _subscriptions: Vec::new(),
            expanded: [true, false, false, false, true, true],
            saved,
            shown: None,
            live: false,
            switches: Vec::new(),
            painted: None,
            live_keys: Self::live_keys(),
        };
        for index in 0..RULES.len() {
            let switch = Ui::find(&page.root, &format!("AlertOn{index}"))?
                .query(&Guid::from_u128(0x331d8f00_c5f9_46a5_b6c8_ede539304567))?;
            page.switches.push((switch, false));
        }
        page.bind()?;
        page.render_tab()?;
        page.render_expanders()?;
        page.render_language()?;
        page.render_theme()?;
        page.render_values_mode()?;
        Ok(page)
    }

    pub fn root(&self) -> &Com {
        &self.root
    }

    /// The open tab, for a page built again after a language or theme change.
    pub fn tab(&self) -> SettingsTab {
        self.tab
    }

    pub fn open(&mut self, tab: SettingsTab) -> Result<()> {
        self.tab = tab;
        self.render_tab()
    }

    fn render_tab(&self) -> Result<()> {
        for tab in SettingsTab::ALL {
            let open = tab == self.tab;
            let name = tab.name();
            Ui::visible(&self.root, &format!("Tab{name}On"), open)?;
            Ui::visible(&self.root, &format!("Tab{name}Off"), !open)?;
            Ui::visible(&self.root, &format!("Tab{name}Line"), open)?;
            for element in tab.elements() {
                Ui::visible(&self.root, element, open)?;
            }
        }
        Ok(())
    }

    fn click(&mut self, name: &str, event: impl Fn() -> SettingsEvent + 'static) -> Result<()> {
        let queue = self.events.clone();
        self._subscriptions.push(Subscription::click(
            &Ui::find(&self.root, name)?,
            move || {
                queue.borrow_mut().push(event());
                Ok(())
            },
        )?);
        Ok(())
    }

    fn bind(&mut self) -> Result<()> {
        for tab in SettingsTab::ALL {
            self.click(&format!("Tab{}", tab.name()), move || {
                SettingsEvent::Tab(tab)
            })?;
        }
        for index in 0..GROUPS.len() {
            self.click(&format!("Expander{index}"), move || {
                SettingsEvent::Toggle(index)
            })?;
        }
        self.click("Apply", || SettingsEvent::Apply)?;
        self.click("Revert", || SettingsEvent::Revert)?;
        self.click("ResetColors", || SettingsEvent::ResetColors)?;
        self.click("PreviewTest", || SettingsEvent::Live(false))?;
        self.click("PreviewLive", || SettingsEvent::Live(true))?;
        self.click("LanguageEnglish", || {
            SettingsEvent::Language(Language::English)
        })?;
        self.click("LanguageRussian", || {
            SettingsEvent::Language(Language::Russian)
        })?;
        for choice in ThemeChoice::ALL {
            self.click(&choice.item(), move || SettingsEvent::Theme(choice))?;
        }
        self.hint("ApplyHintTarget", "ApplyHintCard")
    }

    /// Shows `card` while the pointer is over `target`: a tooltip would wait for
    /// the pointer to rest.
    fn hint(&mut self, target: &str, card: &str) -> Result<()> {
        let target = Ui::find(&self.root, target)?;
        let card = Ui::find(&self.root, card)?;
        // PointerEntered, PointerExited.
        for (slot, shown) in [(63, true), (65, false)] {
            let card = card.clone();
            self._subscriptions
                .push(Subscription::pointer(&target, slot, move |_, _| {
                    Ui::show(&card, shown)
                })?);
        }
        Ok(())
    }

    fn render_expanders(&self) -> Result<()> {
        for (index, open) in self.expanded.iter().enumerate() {
            Ui::visible(&self.root, &format!("ExpanderBody{index}"), *open)?;
            Ui::visible(&self.root, &format!("Chevron{index}"), !*open)?;
            Ui::visible(&self.root, &format!("ChevronUp{index}"), *open)?;
        }
        Ok(())
    }

    /// One option of a segmented control: the active one is raised on a card.
    fn segment(&self, label: &str, active: bool) -> String {
        let d = self.design;
        format!(
            r#"<Border xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Height="22" Padding="10,0" CornerRadius="4" BorderThickness="1" Background="{}" BorderBrush="{}"><TextBlock Text="{label}" FontSize="12" FontWeight="{}" Foreground="{}" VerticalAlignment="Center" Margin="0,-2,0,0"/></Border>"#,
            if active {
                d.color("card")
            } else {
                "Transparent"
            },
            if active {
                d.color("border")
            } else {
                "Transparent"
            },
            if active { "SemiBold" } else { "Normal" },
            d.color(if active { "text" } else { "text2" })
        )
    }

    /// Dropdown with the current language and its two menu items.
    fn render_language(&self) -> Result<()> {
        for (name, language) in [
            ("LanguageEnglish", Language::English),
            ("LanguageRussian", Language::Russian),
        ] {
            let label = language.name();
            let active = language == self.language;
            if active {
                Ui::text(&self.root, "LanguageName", label)?;
            }
            let content = Ui::load(&format!(
                r#"<TextBlock xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Text="{label}" FontWeight="{}" VerticalAlignment="Center"/>"#,
                if active { "SemiBold" } else { "Normal" }
            ))?;
            Ui::content(&Ui::find(&self.root, name)?, &content)?;
        }
        Ok(())
    }

    /// Dropdown with the current window theme and its three menu items.
    fn render_theme(&self) -> Result<()> {
        for choice in ThemeChoice::ALL {
            let (active, label) = (choice == self.theme, self.language.text(choice.label()));
            if active {
                Ui::text(&self.root, "ThemeName", label)?;
            }
            let content = Ui::load(&format!(
                r#"<TextBlock xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Text="{label}" FontWeight="{}" VerticalAlignment="Center"/>"#,
                if active { "SemiBold" } else { "Normal" }
            ))?;
            Ui::content(&Ui::find(&self.root, &choice.item())?, &content)?;
        }
        Ok(())
    }

    fn render_values_mode(&self) -> Result<()> {
        for (name, live, label) in [
            ("PreviewTest", false, self.language.text("Test")),
            ("PreviewLive", true, self.language.text("Live values")),
        ] {
            let content = Ui::load(&self.segment(label, live == self.live))?;
            Ui::content(&Ui::find(&self.root, name)?, &content)?;
        }
        self.playground.lock_preview(self.live)
    }

    /// Live preview values from the newest recorded sample.
    /// Readings the tile previews show: those of the main devices, like the tiles.
    fn live_keys() -> [String; 9] {
        let main = |kind| DeviceCatalog::canonical(&DeviceId::new(kind, None));
        let (gpu, disk, net) = (main("gpu"), main("disk"), main("net"));
        [
            "cpu_temperature".into(),
            gpu.reading("gpu_temperature"),
            "ram".into(),
            "cpu".into(),
            gpu.reading("gpu"),
            disk.reading("disk_read"),
            disk.reading("disk_write"),
            net.reading("net_down"),
            net.reading("net_up"),
        ]
    }

    fn live_values(&self, frame: &Frame) -> [Option<f64>; 9] {
        self.live_keys.clone().map(|id| {
            frame
                .totals
                .iter()
                .find(|(key, _)| *key == id)
                .and_then(|(_, v)| *v)
        })
    }

    /// "GPU — between thresholds: the tile turns red" for each alerting preview tile.
    fn notes(&self, style: &TileStyle) -> Vec<String> {
        let a = style.alerts;
        [
            ("CPU", style.demo_cpu, a.cpu_x, a.cpu_y),
            ("GPU", style.demo_gpu, a.gpu_x, a.gpu_y),
            ("RAM", style.demo_ram, a.ram_x, a.ram_y),
        ]
        .into_iter()
        .filter_map(|(name, value, x, y)| {
            let text = if value >= y.max(x + 1.0) {
                "above the upper threshold: pulsing"
            } else if value > x {
                "between thresholds: the tile turns red"
            } else {
                return None;
            };
            Some(format!("{name} — {}", self.language.text(text)))
        })
        .collect()
    }

    /// Keeps each rule switch and its painted track in step with the thresholds.
    /// Switching a rule off parks both thresholds at the top of the scale.
    fn render_alerts(&mut self) -> Result<()> {
        let Some(style) = self.playground.style() else {
            return Ok(());
        };
        let thresholds = RULES.map(|rule| (rule.thresholds)(&style.alerts));
        for (index, (rule, (x, _))) in RULES.iter().zip(thresholds).enumerate() {
            let (switch, shown) = &mut self.switches[index];
            let mut on = 0u8;
            unsafe {
                let get: unsafe extern "system" fn(Raw, *mut u8) -> Hr = switch.slot(6);
                check(get(switch.raw(), &mut on))?;
            }
            let on = on != 0;
            if on != *shown {
                // The user flipped the switch.
                *shown = on;
                let (x, y) = if on { rule.defaults() } else { rule.off() };
                self.playground.set_alert(rule.name, x, y)?;
            } else if on != rule.enabled(x) {
                *shown = rule.enabled(x);
                unsafe {
                    let set: unsafe extern "system" fn(Raw, u8) -> Hr = switch.slot(7);
                    check(set(switch.raw(), u8::from(*shown)))?;
                }
            }
        }
        if self.painted != Some(thresholds) {
            let range = AlertRange {
                design: self.design,
                language: self.language,
            };
            for (index, (rule, (x, y))) in RULES.iter().zip(thresholds).enumerate() {
                Ui::children(
                    &Ui::find(&self.root, &format!("AlertTrack{index}"))?,
                    &Ui::load(&range.track(rule, x, y))?,
                )?;
            }
            self.painted = Some(thresholds);
        }
        Ok(())
    }

    fn summaries(&self, style: &TileStyle) -> [String; 6] {
        let n = |v: f64| {
            self.language
                .number(v, if v.fract().abs() < 1e-9 { 0 } else { 1 })
        };
        let changed = (0..14)
            .filter(|i| {
                style.light.0[*i] != self.saved.light.0[*i]
                    || style.dark.0[*i] != self.saved.dark.0[*i]
            })
            .count();
        let colors = match changed {
            0 => self.language.text("14 elements · 2 themes").to_owned(),
            n => self
                .language
                .text("changed: {n}")
                .replace("{n}", &n.to_string()),
        };
        let rules = |count: usize| self.language.rules(count as u32);
        [
            format!(
                "{} · {} · {} px",
                n(style.width),
                n(style.gap),
                n(style.radius)
            ),
            format!(
                "{} · {} · {} px",
                n(style.label_size),
                n(style.value_size),
                n(style.secondary_size)
            ),
            format!(
                "{} px · {} % · {} %",
                n(style.stroke),
                n(style.opacity),
                n(style.fade_opacity)
            ),
            colors,
            rules(ALERTS),
            rules(RULES.len() - ALERTS),
        ]
    }

    /// `live` is the newest recorded sample, shown in the preview in "Live values" mode.
    pub fn refresh(&mut self, live: Option<&Frame>) -> Result<Option<SettingsOutcome>> {
        let mut outcome = None;
        let events = std::mem::take(&mut *self.events.borrow_mut());
        for event in events {
            match event {
                SettingsEvent::Tab(tab) => self.open(tab)?,
                SettingsEvent::Toggle(index) => {
                    self.expanded[index] = !self.expanded[index];
                    self.render_expanders()?;
                }
                SettingsEvent::Apply => {
                    let result = self.playground.apply_to_taskbar();
                    match result {
                        Ok(()) => {
                            self.saved = self.playground.style().unwrap_or(self.saved);
                            outcome = Some(SettingsOutcome::Applied);
                        }
                        Err(error) => Ui::text(
                            &self.root,
                            "ApplyStatus",
                            &format!("{}: {error}", self.language.text("Save failed")),
                        )?,
                    }
                    self.shown = None;
                }
                SettingsEvent::Revert => {
                    let saved = self.saved;
                    self.playground.load(&saved)?;
                }
                SettingsEvent::Language(language) if language != self.language => {
                    outcome = Some(SettingsOutcome::Language(language));
                }
                SettingsEvent::Language(_) => {}
                SettingsEvent::Theme(choice) => {
                    Ui::hide_flyout(&Ui::find(&self.root, "ThemeFlyout")?)?;
                    if choice != self.theme {
                        self.theme = choice;
                        self.render_theme()?;
                        outcome = Some(SettingsOutcome::Theme(choice));
                    }
                }
                SettingsEvent::ResetColors => self.playground.reset_colors()?,
                SettingsEvent::Live(live) => {
                    self.live = live;
                    self.render_values_mode()?;
                }
            }
        }
        if let (true, Some(frame)) = (self.live, live) {
            self.playground.preview(&self.live_values(frame))?;
        }
        self.playground.refresh()?;
        self.startup.refresh()?;
        self.monitors.refresh()?;
        if let Some(width) = self.chart_line.refresh()? {
            outcome = Some(SettingsOutcome::ChartLine(width));
        }
        self.render_alerts()?;
        if let Some(style) = self.playground.style() {
            let pending = Appearance::differences(&style, &self.saved);
            let summaries = self.summaries(&style);
            let notes = self.notes(&style);
            if self.shown.as_ref() != Some(&(pending, summaries.clone(), notes.clone())) {
                Ui::visible(&self.root, "PendingState", pending > 0)?;
                Ui::visible(&self.root, "Apply", pending > 0)?;
                Ui::visible(&self.root, "ApplyIdle", pending == 0)?;
                Ui::visible(&self.root, "Revert", pending > 0)?;
                Ui::visible(&self.root, "RevertIdle", pending == 0)?;
                Ui::text(
                    &self.root,
                    "ApplyStatus",
                    &self.language.unapplied(pending as u32),
                )?;
                for (index, summary) in summaries.iter().enumerate() {
                    Ui::text(&self.root, &format!("Summary{index}"), summary)?;
                }
                let mut markup = String::from(
                    r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Orientation="Horizontal" Spacing="16">"#,
                );
                for note in &notes {
                    markup.push_str(&format!(
                        r#"<TextBlock Text="{}" FontSize="11" Foreground="{}"/>"#,
                        Ui::xml(note),
                        self.design.color("text3")
                    ));
                }
                markup.push_str("</StackPanel>");
                Ui::children(&Ui::find(&self.root, "PreviewNotes")?, &Ui::load(&markup)?)?;
                self.shown = Some((pending, summaries, notes));
            }
        }
        Ok(outcome)
    }

    pub fn verify(&mut self) -> Result<()> {
        for tab in [SettingsTab::Taskbar, self.tab] {
            self.events.borrow_mut().push(SettingsEvent::Tab(tab));
        }
        for index in 0..GROUPS.len() {
            self.events.borrow_mut().push(SettingsEvent::Toggle(index));
        }
        self.events.borrow_mut().push(SettingsEvent::Revert);
        self.events.borrow_mut().push(SettingsEvent::Live(true));
        self.refresh(None)?;
        self.events.borrow_mut().push(SettingsEvent::Live(false));
        self.refresh(None)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_text_of_the_page_is_in_the_catalog() {
        for language in [Language::English, Language::Russian] {
            let markup =
                SettingsPage::markup(Design { dark: false }, language, &Displays::current());
            assert!(
                !markup.contains('@'),
                "untranslated marker in the settings page"
            );
            assert!(markup.contains(language.text("Number color")));
        }
    }
}
