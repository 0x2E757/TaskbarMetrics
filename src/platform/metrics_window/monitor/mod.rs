mod alert_range;
mod chart;
mod chart_line;
mod client;
mod clock;
mod demo;
mod design;
mod device_options;
mod devices;
mod elements;
mod header;
mod layout;
mod legend;
mod locale;
mod menu;
mod model;
mod monitor_options;
mod nav;
mod pins;
mod plain_button;
mod row_window;
mod scroll_indicator;
mod settings_page;
mod startup_options;
mod stats;
mod table;
mod theme;
mod tokens;
mod tooltip;
mod totals;
pub(super) mod ui;
use super::{window_memory::WindowMemory, *};
use crate::platform::{
    devices::DeviceId,
    process_history::store::{Frame, IoBytes},
    system_activity::SystemActivity,
    xaml::{appearance::Appearance, events::Subscription, AlertSettings},
};
use chart::{ChartLayout, ChartRenderer, WINDOW};
use chart_line::ChartLine;
use client::HistoryClient;
use design::Design;
use device_options::DeviceOptions;
use devices::Devices;
use elements::Elements;
use header::{HeaderMarkup, Mode};
use layout::WindowLayout;
use legend::{ChartLegend, LegendEntry, LegendFlow, LegendMark, LegendWidths};
pub(crate) use locale::Language;
use menu::DeviceMenu;
use model::{Device, ProcessKey, Resource, Timeline};
use nav::Navigation;
use pins::PinnedLayer;
use plain_button::PlainButton;
use row_window::RowWindow;
use scroll_indicator::ScrollIndicator;
use stats::{StatWidths, StatsPanel, StatsSource};
use std::{
    cell::{Cell, RefCell},
    rc::Rc,
    sync::{
        atomic::{AtomicU32, Ordering},
        Arc,
    },
    time::{Duration, Instant},
};
use table::{RowContent, RowKind, TableLayout};
use theme::ThemeChoice;
use tooltip::ChartTooltip;
use totals::IoTotals;
use ui::{Shown, Ui};
/// Device id another launch asked to show (`WM_COPYDATA`).
pub(super) static REQUEST: std::sync::Mutex<Option<String>> = std::sync::Mutex::new(None);
pub(super) static KEY: AtomicU32 = AtomicU32::new(0);
/// The language of the settings: the window's, or English in the standalone editor.
pub(super) fn settings_language() -> Language {
    if super::opens_editor() {
        Language::English
    } else {
        Language::load()
    }
}
fn verifying() -> bool {
    std::env::args().any(|a| a == "--verify-monitor")
}
/// `--demo <scenario>` replaces the recorder with a synthetic artboard state.
fn scenario() -> Option<demo::Scenario> {
    let args: Vec<_> = std::env::args().collect();
    args.windows(2)
        .find(|pair| pair[0] == "--demo")
        .and_then(|pair| demo::Scenario::parse(&pair[1]))
}
/// The theme chosen in the settings; demo screenshots may force one with
/// `--theme dark|light`.
fn dark_theme() -> Result<bool> {
    let args: Vec<_> = std::env::args().collect();
    match args
        .windows(2)
        .find(|pair| pair[0] == "--theme")
        .map(|pair| pair[1].as_str())
    {
        Some("dark") => Ok(true),
        Some("light") => Ok(false),
        _ => ThemeChoice::load().dark(),
    }
}
/// Pins and the language are written only by the interactive, recorder-backed window.
fn persistent() -> bool {
    !verifying() && scenario().is_none()
}

enum Action {
    Top,
    ChartHover(Option<(f64, f64)>),
    ChartPoint(f64),
    Pin(Arc<crate::platform::process_history::store::Identity>),
    /// Menu entry by position.
    Tab(usize),
    Point(f64),
    /// Moves the chosen moment by this many samples, later if positive.
    Step(isize),
    Hover(Option<ProcessKey>),
    Live,
    All,
    Settings,
    Menu(bool),
    EnableMonitoring,
    SearchFocus(bool),
    ClearSearch,
    /// «Show on taskbar» of the shown device was clicked.
    Taskbar,
    /// «Always monitor» of the shown device was clicked.
    Always,
    /// A value column header of the table: sort by that column.
    Sort(usize),
}
struct Dashboard {
    design: Design,
    /// Width of the total line in the chart, from the settings page.
    chart_line: f64,
    chrome: window::WindowChrome,
    theme_check: Instant,
    layout: WindowLayout,
    compact: bool,
    chart_layout: ChartLayout,
    hardware: Devices,
    pins: pins::PinnedProcesses,
    pin_error: Option<String>,
    alerts: AlertSettings,
    monitoring: Option<monitoring::MonitoringConfig>,
    position: f64,
    root: Com,
    elements: Elements,
    /// Markup of hosts whose content is replaced only when it changes.
    shown: Shown,
    legend_widths: LegendWidths,
    client: HistoryClient,
    language: Language,
    /// Kind of `device`.
    resource: Resource,
    menu: DeviceMenu,
    device: Device,
    /// Tiles and background history as last read from, or written to, the configuration.
    device_settings: crate::config::Settings,
    events: Rc<RefCell<Vec<Action>>>,
    subscriptions: Vec<Subscription>,
    row_events: Vec<Subscription>,
    /// Sort buttons of the loaded table header.
    header_events: Vec<Subscription>,
    /// Markup of the loaded table header and rows, and the processes the rows'
    /// handlers act on: an unchanged table is not rebuilt, which also keeps the
    /// focused row. Markup alone does not tell apart processes of the same name.
    table_markup: (String, String, Vec<(usize, ProcessKey)>),
    frames: Vec<Arc<Frame>>,
    /// I/O of each process since the recorder started, from the packet of `frames`.
    io_totals: Arc<std::collections::HashMap<ProcessKey, IoBytes>>,
    frozen: Option<Vec<Arc<Frame>>>,
    selected: Option<u64>,
    hover: Option<ProcessKey>,
    pointer: Option<(f64, f64)>,
    table_frame: Option<Arc<Frame>>,
    /// Measured table width the rows were trimmed for.
    table_width: f64,
    all: bool,
    /// Value column of the process table the rows are sorted by.
    sort: usize,
    search: String,
    revision: u64,
    /// Everything on the chart card must be rebuilt.
    dirty: bool,
    /// Only the moment, the highlighted process and the texts that follow them.
    overlay_dirty: bool,
    /// Only the highlighted process: its series, the dimmed chart and the legend.
    hover_dirty: bool,
    /// Series canvas of the base chart, dimmed while a process is highlighted.
    chart_series: Option<Com>,
    /// Current statistics row with its `StatsPanel::shape`, and the widths its items reached.
    stats_row: Option<(Com, String)>,
    /// Pointer events that show the hints of the current statistics row.
    stat_events: Vec<Subscription>,
    stat_widths: StatWidths,
    table_dirty: bool,
    /// The regular live update of the table waits for a frame without a chart redraw.
    table_due: bool,
    /// The table follows a dragged moment at most every `TABLE_PERIOD`.
    table_deferred: bool,
    /// Regular rows created by the last table render.
    row_window: RowWindow,
    settings: Option<settings_page::SettingsPage>,
    /// The tab the settings page had when it was last dropped.
    settings_tab: settings_page::SettingsTab,
    settings_visible: bool,
    navigation_open: bool,
    last_table: Instant,
    /// The settings page polls its controls every other frame, at the 32 Hz it always had.
    settings_polled: Instant,
    connected: bool,
    enabled: bool,
    demo: bool,
}
impl Dashboard {
    fn new(
        island: &XamlIsland,
        requested: Option<DeviceId>,
        chrome: window::WindowChrome,
    ) -> Result<Self> {
        let language = Language::load();
        let design = Design {
            dark: dark_theme()?,
        };
        let monitoring = monitoring::MonitoringConfig::locate().ok();
        let menu = if scenario().is_some() {
            DeviceMenu::fixed(demo::devices())
        } else if verifying() {
            DeviceMenu::fixed(
                Resource::ALL
                    .iter()
                    .map(|r| DeviceId::new(r.id(), None))
                    .collect(),
            )
        } else {
            DeviceMenu::system(monitoring.as_ref().map(|m| m.watch_path()))
        };
        let device = requested
            .and_then(|id| menu.position(&id))
            .or(Some(0))
            .and_then(|index| menu.devices.get(index).cloned())
            .ok_or(E_FAIL)?;
        let root = island.load(&Self::template(design, language, WindowLayout::Full, &menu))?;
        let mut result = Self {
            design,
            chart_line: ChartLine::load(),
            chrome,
            theme_check: Instant::now(),
            layout: WindowLayout::Full,
            compact: false,
            chart_layout: ChartLayout::new(900.0, 250.0),
            hardware: Devices::new(),
            pins: if !persistent() {
                pins::PinnedProcesses::default()
            } else {
                crate::platform::data_directory::DataDirectory::file("taskbar-metrics.pins")
                    .map(|p| pins::PinnedProcesses::load(&p))
                    .unwrap_or_default()
            },
            pin_error: None,
            alerts: AlertSettings::default(),
            monitoring,
            position: 1000.0,
            elements: Elements::find(&root)?,
            shown: Shown::default(),
            legend_widths: LegendWidths::default(),
            root,
            client: HistoryClient::new(),
            language,
            resource: device.resource,
            menu,
            device,
            device_settings: crate::config::Settings::default(),
            events: Rc::new(RefCell::new(Vec::new())),
            subscriptions: Vec::new(),
            row_events: Vec::new(),
            header_events: Vec::new(),
            table_markup: Default::default(),
            frames: Vec::new(),
            io_totals: Arc::default(),
            frozen: None,
            selected: None,
            hover: None,
            pointer: None,
            table_frame: None,
            table_width: 0.0,
            all: false,
            // Disk and network rows start by read plus write.
            sort: 2,
            search: String::new(),
            revision: u64::MAX,
            dirty: true,
            overlay_dirty: false,
            hover_dirty: false,
            chart_series: None,
            stats_row: None,
            stat_events: Vec::new(),
            stat_widths: StatWidths::default(),
            table_dirty: true,
            table_due: false,
            table_deferred: false,
            row_window: RowWindow::default(),
            settings: None,
            settings_tab: Default::default(),
            settings_visible: false,
            navigation_open: false,
            last_table: Instant::now(),
            settings_polled: Instant::now(),
            connected: false,
            enabled: false,
            demo: false,
        };
        result.load_alerts();
        result.load_device_settings();
        chrome.icon();
        result.apply_chrome();
        result.bind()?;
        Ok(result)
    }
    fn template(
        design: Design,
        language: Language,
        layout: WindowLayout,
        menu: &DeviceMenu,
    ) -> String {
        let titles: Vec<String> = menu.devices.iter().map(|d| d.title(language)).collect();
        let markup = layout
            .markup(include_str!("monitor.xaml"))
            .replace(
                "<!--NAV-->",
                &Navigation::buttons("", &titles, layout.icon_only()),
            )
            .replace(
                "<!--OVERLAY-->",
                &Navigation::buttons("Overlay", &titles, false),
            );
        let markup = PlainButton::markup(&ScrollIndicator::markup(&markup));
        SystemTheme { dark: design.dark }.markup(&design.markup(&language.markup(&markup)))
    }
    fn apply_chrome(&self) {
        self.chrome.apply(
            self.design.dark,
            self.design.color("bg"),
            self.design.color("text"),
        );
    }
    fn load_alerts(&mut self) {
        if let Some(style) = self
            .monitoring
            .as_ref()
            .and_then(|m| Appearance::read(&m.appearance_path()).ok())
        {
            self.alerts = style.alerts;
        }
    }
    fn find(&self, name: &str) -> Result<Com> {
        Ui::find(&self.root, name)
    }
    fn click(&mut self, name: &str, action: impl Fn() -> Action + 'static) -> Result<()> {
        let queue = self.events.clone();
        self.subscriptions
            .push(Subscription::click(&self.find(name)?, move || {
                queue.borrow_mut().push(action());
                Ok(())
            })?);
        Ok(())
    }
    fn bind(&mut self) -> Result<()> {
        for index in 0..self.menu.devices.len() {
            self.click(&format!("Tab{index}"), move || Action::Tab(index))?;
            self.click(&format!("OverlayTab{index}"), move || Action::Tab(index))?;
        }
        self.click("Live", || Action::Live)?;
        self.click("All", || Action::All)?;
        self.click("Top", || Action::Top)?;
        self.click("Settings", || Action::Settings)?;
        self.click("OverlaySettings", || Action::Settings)?;
        self.click("OpenSettings", || Action::Settings)?;
        self.click("EnableMonitoring", || Action::EnableMonitoring)?;
        self.click("SearchClear", || Action::ClearSearch)?;
        self.click("Menu", || Action::Menu(true))?;
        self.click("MenuBar", || Action::Menu(true))?;
        self.click("NavDismiss", || Action::Menu(false))?;
        self.click("OnTaskbar", || Action::Taskbar)?;
        self.click("AlwaysMonitor", || Action::Always)?;
        // Pressing selects a moment; holding the button and moving scrubs through time.
        // The right button clears the selection instead.
        let chart = self.find("Chart")?;
        let dragging = Rc::new(Cell::new(false));
        let (queue, held) = (self.events.clone(), dragging.clone());
        self.subscriptions
            .push(Subscription::pointer(&chart, 57, move |sender, args| {
                if Subscription::secondary(sender, args)? {
                    queue.borrow_mut().push(Action::Live);
                    return Ok(());
                }
                let (x, _) = Subscription::position(sender, args)?;
                Subscription::capture(sender, args)?;
                held.set(true);
                queue.borrow_mut().push(Action::ChartPoint(x as f64));
                Ok(())
            })?);
        // PointerReleased, PointerCaptureLost.
        for slot in [61, 67] {
            let held = dragging.clone();
            self.subscriptions
                .push(Subscription::pointer(&chart, slot, move |_, _| {
                    held.set(false);
                    Ok(())
                })?);
        }
        for slot in [59, 65] {
            let (queue, held) = (self.events.clone(), dragging.clone());
            self.subscriptions
                .push(Subscription::pointer(&chart, slot, move |sender, args| {
                    let point = if slot == 59 {
                        let (x, y) = Subscription::position(sender, args)?;
                        if held.get() {
                            queue.borrow_mut().push(Action::ChartPoint(x as f64));
                        }
                        Some((x as f64, y as f64))
                    } else {
                        None
                    };
                    queue.borrow_mut().push(Action::ChartHover(point));
                    Ok(())
                })?);
        }
        // PointerWheelChanged: with the button up, each notch steps the chosen
        // moment one sample, forward in time when turned away from the user.
        // Smooth wheels and touchpads add up to whole notches.
        let (queue, turned) = (self.events.clone(), Cell::new(0));
        self.subscriptions
            .push(Subscription::pointer(&chart, 71, move |sender, args| {
                if dragging.get() {
                    return Ok(());
                }
                let total = turned.get() + Subscription::wheel(sender, args)?;
                let notches = total / 120;
                turned.set(total - notches * 120);
                if notches != 0 {
                    queue.borrow_mut().push(Action::Step(notches as isize));
                }
                Ok(())
            })?);
        // «How it's measured» shows its card while the pointer is over the button,
        // or while it has keyboard focus.
        let (info, card) = (self.find("Info")?, self.find("InfoCard")?);
        let show = move |visible: bool| Ui::show(&card, visible);
        // PointerEntered and PointerExited, also when the button handled them.
        for (event, visible) in [(8, true), (12, false)] {
            let show = show.clone();
            self.subscriptions
                .push(Subscription::pointer_handled(&info, event, move |_, _| {
                    show(visible)
                })?);
        }
        // GotFocus and LostFocus.
        for (slot, visible) in [(45, true), (47, false)] {
            let show = show.clone();
            self.subscriptions
                .push(Subscription::routed(&info, slot, move || show(visible))?);
        }
        let search = self.find("Search")?;
        for (slot, focused) in [(45, true), (47, false)] {
            let queue = self.events.clone();
            self.subscriptions
                .push(Subscription::routed(&search, slot, move || {
                    queue.borrow_mut().push(Action::SearchFocus(focused));
                    Ok(())
                })?);
        }
        Ui::range(&self.elements.timeline, Some(self.position))?;
        self.render_options()?;
        self.render_navigation()
    }
    /// The configuration of the interactive window; demo and verification keep the
    /// default tiles in memory.
    fn load_device_settings(&mut self) {
        let loaded = self
            .monitoring
            .as_ref()
            .filter(|_| persistent())
            .and_then(|config| config.settings().ok());
        self.device_settings = loaded.unwrap_or_else(|| {
            crate::config::Settings::parse("metrics=cpu,gpu,ram,net,disk").unwrap_or_default()
        });
    }
    fn device_options(&self) -> DeviceOptions<'_, impl Fn(&DeviceId) -> DeviceId + '_> {
        DeviceOptions {
            settings: &self.device_settings,
            canonical: |id: &DeviceId| self.menu.canonical(id),
        }
    }
    /// Check boxes of the shown device; a tile implies «Always monitor».
    fn render_options(&self) -> Result<()> {
        let (taskbar, always) = self.device_options().state(&self.device.id);
        Ui::checked(&self.find("OnTaskbar")?, taskbar)?;
        Ui::checked(&self.find("AlwaysMonitor")?, always)?;
        Ui::enable(&self.find("AlwaysMonitor")?, !taskbar)
    }
    /// Stores changed tiles or history; the taskbar and the recorder follow.
    fn save_devices(&mut self, metrics: Vec<String>, history: Vec<String>) {
        if persistent() {
            if let Some(config) = &self.monitoring {
                if let Err(error) = config.store_devices(&metrics, &history) {
                    self.pin_error =
                        Some(format!("{}: {error}", self.language.text("Save failed")));
                    return;
                }
            }
        }
        self.device_settings.metrics = metrics;
        self.device_settings.history = history;
    }
    fn navigation(&self, compact: bool) -> Navigation {
        Navigation {
            design: self.design,
            language: self.language,
            compact,
        }
    }
    fn render_navigation(&self) -> Result<()> {
        let frame = self.selected_frame();
        let mut hosts = vec![("", self.compact)];
        if self.navigation_open {
            hosts.push(("Overlay", false));
        }
        for (prefix, compact) in hosts {
            let navigation = self.navigation(compact);
            for (index, device) in self.menu.devices.iter().enumerate() {
                let selected = *device == self.device && !self.settings_visible;
                let item = navigation.item(
                    Navigation::icon(device),
                    &device.title(self.language),
                    &navigation.value(device, frame.as_deref()),
                    selected,
                );
                self.shown
                    .content(&self.root, &format!("{prefix}Tab{index}"), &item)?;
            }
            let settings = navigation.item(
                "gear",
                self.language.text("Settings"),
                "",
                self.settings_visible,
            );
            self.shown
                .content(&self.root, &format!("{prefix}Settings"), &settings)?;
        }
        Ok(())
    }
    fn rebuild(&mut self, island: &XamlIsland) -> Result<()> {
        self.drop_settings();
        self.subscriptions.clear();
        self.row_events.clear();
        self.header_events.clear();
        self.table_markup = Default::default();
        self.hover = None;
        self.pointer = None;
        self.navigation_open = false;
        self.root = island.load(&Self::template(
            self.design,
            self.language,
            self.layout,
            &self.menu,
        ))?;
        self.elements = Elements::find(&self.root)?;
        self.shown.clear();
        self.legend_widths.clear();
        self.stats_row = None;
        self.search.clear();
        self.bind()?;
        if self.settings_visible {
            self.show_settings()?;
        }
        self.dirty = true;
        self.table_dirty = true;
        Ok(())
    }
    /// Drops the settings page; the next one opens on the same tab.
    fn drop_settings(&mut self) {
        if let Some(page) = self.settings.take() {
            self.settings_tab = page.tab();
        }
    }
    fn show_settings(&mut self) -> Result<()> {
        if self.settings.is_none() {
            let page = settings_page::SettingsPage::new(
                self.design,
                self.language,
                self.monitoring.as_ref().map(|m| m.appearance_path()),
                self.settings_tab,
            )?;
            Ui::content(&self.find("SettingsHost")?, page.root())?;
            self.settings = Some(page);
        }
        self.settings_visible = true;
        self.render_navigation()?;
        Ui::visible(&self.root, "Performance", false)?;
        Ui::visible(&self.root, "SettingsPage", true)
    }
    fn action(&mut self, action: Action) -> Result<()> {
        match action {
            Action::ChartHover(point) => {
                self.pointer = point;
                return self.render_pointer();
            }
            Action::ChartPoint(x) => {
                let fraction = if let (Some(first), Some(last)) =
                    (self.timeline().first(), self.timeline().last())
                {
                    let bucket = self.chart_layout.bucket(x, last.bucket);
                    ((bucket - first.bucket as f64)
                        / last.bucket.saturating_sub(first.bucket).max(1) as f64)
                        .clamp(0.0, 1.0)
                } else {
                    self.chart_layout.fraction(x)
                };
                self.action(Action::Point(fraction))?;
            }
            Action::Pin(identity) => {
                self.pins.toggle(identity);
                self.hover = None;
                self.table_dirty = true;
                if persistent() {
                    self.pin_error = crate::platform::data_directory::DataDirectory::file(
                        "taskbar-metrics.pins",
                    )
                    .and_then(|p| self.pins.save(&p))
                    .err()
                    .map(|e| format!("{}: {e}", self.language.text("Save failed")));
                }
            }
            Action::Tab(index) => {
                let Some(device) = self.menu.devices.get(index).cloned() else {
                    return Ok(());
                };
                self.resource = device.resource;
                self.device = device;
                self.refresh_devices();
                self.load_device_settings();
                self.render_options()?;
                self.hover = None;
                self.pointer = None;
                self.settings_visible = false;
                self.set_navigation(false)?;
                Ui::visible(&self.root, "SettingsPage", false)?;
                Ui::visible(&self.root, "Performance", true)?;
                self.load_alerts();
                self.table_dirty = true;
                self.render_navigation()?;
            }
            Action::Point(fraction) => {
                let paused = self.frozen.is_some();
                self.position = fraction.clamp(0.0, 1.0) * 1000.0;
                Ui::range(&self.elements.timeline, Some(self.position))?;
                if self.frozen.is_none() {
                    self.frozen = Some(self.frames.clone());
                }
                if let Some(index) = Timeline::nearest(self.timeline(), fraction) {
                    self.selected = Some(self.timeline()[index].bucket);
                    self.hover = None;
                    self.table_deferred = true;
                }
                // The base chart of a paused timeline does not depend on the moment.
                if paused {
                    self.overlay_dirty = true;
                    return Ok(());
                }
                self.table_dirty = true;
            }
            Action::Step(offset) => {
                let timeline = self.timeline();
                let Some(index) = self
                    .selected
                    .and_then(|bucket| timeline.iter().position(|f| f.bucket == bucket))
                else {
                    return Ok(());
                };
                let target = index.saturating_add_signed(offset).min(timeline.len() - 1);
                if target == index {
                    return Ok(());
                }
                let (first, last) = (timeline[0].bucket, timeline[timeline.len() - 1].bucket);
                let fraction = (timeline[target].bucket - first) as f64
                    / last.saturating_sub(first).max(1) as f64;
                return self.action(Action::Point(fraction));
            }
            Action::Hover(key) => {
                // Only the overlay follows the pointer; the table catches up with
                // live data on its next regular update.
                self.hover = key;
                self.hover_dirty = true;
                return Ok(());
            }
            Action::Live => {
                self.position = 1000.0;
                Ui::range(&self.elements.timeline, Some(self.position))?;
                self.frozen = None;
                self.selected = None;
                self.hover = None;
                self.table_dirty = true;
            }
            Action::All | Action::Top => {
                self.all = matches!(action, Action::All);
                self.hover = None;
                self.table_dirty = true;
                return Ok(());
            }
            Action::Sort(column) => {
                self.sort = column;
                self.table_dirty = true;
                return Ok(());
            }
            Action::Settings => {
                self.set_navigation(false)?;
                // From another section the settings open on their first tab.
                if !self.settings_visible {
                    self.settings_tab = Default::default();
                    if let Some(page) = &mut self.settings {
                        page.open(Default::default())?;
                    }
                }
                self.show_settings()?;
            }
            Action::Menu(open) => self.set_navigation(open)?,
            Action::EnableMonitoring => {
                if let Some(config) = &self.monitoring {
                    if persistent() {
                        if let Err(error) = config.store(true) {
                            self.pin_error =
                                Some(format!("{}: {error}", self.language.text("Save failed")));
                        }
                    }
                }
                // The settings page re-reads the switch next time it is shown.
                self.drop_settings();
            }
            Action::ClearSearch => {
                self.elements
                    .search
                    .query(&Guid::from_u128(0xe48f5a8b_1dff_4352_a1f4_e516514ec882))?
                    .set_string(7, "")?;
                return Ok(());
            }
            Action::Taskbar | Action::Always => {
                // External edits count: toggle against the file, not a stale copy.
                self.load_device_settings();
                let change = {
                    let options = self.device_options();
                    let (taskbar, always) = options.state(&self.device.id);
                    match action {
                        Action::Taskbar => options
                            .taskbar(&self.device.id, !taskbar)
                            .map(|metrics| (metrics, self.device_settings.history.clone())),
                        _ => Some((
                            self.device_settings.metrics.clone(),
                            options.history(&self.device.id, !always),
                        )),
                    }
                };
                if let Some((metrics, history)) = change {
                    self.save_devices(metrics, history);
                }
                return self.render_options();
            }
            Action::SearchFocus(focused) => {
                Ui::visible(&self.root, "SearchFocus", focused)?;
                return Ok(());
            }
        }
        self.dirty = true;
        Ok(())
    }
    /// Redraws in the chosen theme, or in the system's once it changed; checked
    /// every second and right after a choice in the settings.
    fn follow_theme(&mut self, island: &XamlIsland) -> Result<()> {
        self.theme_check = Instant::now();
        // A changed text size lays items out anew.
        self.legend_widths.clear();
        let design = Design {
            dark: dark_theme()?,
        };
        if design != self.design {
            self.design = design;
            self.apply_chrome();
            self.rebuild(island)?;
        }
        Ok(())
    }
    fn set_language(&mut self, language: Language, island: &XamlIsland) -> Result<()> {
        self.language = language;
        if persistent() {
            self.language.save().map_err(|_| E_FAIL)?;
        }
        self.rebuild(island)
    }
    fn set_navigation(&mut self, open: bool) -> Result<()> {
        self.navigation_open = open;
        Ui::visible(&self.root, "NavOverlay", open)?;
        if open {
            self.render_navigation()?;
        }
        Ok(())
    }
    fn timeline(&self) -> &[Arc<Frame>] {
        self.frozen.as_deref().unwrap_or(&self.frames)
    }
    fn live_end(&self) -> u64 {
        self.frames.last().map_or(0, |f| f.bucket)
    }
    /// Per-process history exists, or the recorder is not reachable at all.
    fn processes(&self) -> bool {
        self.enabled || !self.connected
    }
    fn selected_frame(&self) -> Option<Arc<Frame>> {
        if let Some(bucket) = self.selected {
            self.timeline().iter().find(|f| f.bucket == bucket).cloned()
        } else if self.hover.is_some() {
            self.table_frame.clone()
        } else {
            // ETW buffers arrive after the corresponding PDH sample. Keep the
            // live I/O table two seconds behind; chart clicks may inspect newer
            // buckets, which continue receiving late corrections while paused.
            self.frames
                .iter()
                .rev()
                .nth(if self.resource.dual() {
                    (chart::ETW_DELAY as usize).min(self.frames.len().saturating_sub(1))
                } else {
                    0
                })
                .cloned()
        }
    }
    fn refresh(&mut self, island: &XamlIsland) -> Result<()> {
        match KEY.swap(0, Ordering::Relaxed) {
            27 if self.navigation_open => self.set_navigation(false)?,
            27 if self.selected.is_some() => self.action(Action::Live)?,
            0x46 if !self.settings_visible && self.processes() => Ui::focus(&self.elements.search)?,
            _ => {}
        }
        // Nothing is on screen: the history is kept current, so frames the window no
        // longer shows are released, and drawn once the window is shown again.
        if self.chrome.hidden() {
            self.refresh_menu(island)?;
            return self.sync_history();
        }
        if self.theme_check.elapsed().as_secs() >= 1 {
            self.follow_theme(island)?;
        }
        let (window_width, _) = Ui::size(&self.root)?;
        let layout = WindowLayout::of(window_width);
        if layout != self.layout {
            self.layout = layout;
            self.compact = layout.compact();
            self.rebuild(island)?;
        }
        Ui::height(&self.elements.chart, ChartLayout::height(self.compact))?;
        let request = REQUEST.lock().ok().and_then(|mut request| request.take());
        if let Some(index) = request
            .as_deref()
            .and_then(DeviceId::parse)
            .and_then(|id| self.menu.position(&id))
        {
            self.action(Action::Tab(index))?;
        }
        self.refresh_menu(island)?;
        let actions = std::mem::take(&mut *self.events.borrow_mut());
        // Pointer moves arrive faster than frames: only the latest position matters.
        let last_hover = actions
            .iter()
            .rposition(|a| matches!(a, Action::ChartHover(_)));
        let last_point = actions
            .iter()
            .rposition(|a| matches!(a, Action::ChartPoint(_)));
        for (index, action) in actions.into_iter().enumerate() {
            match action {
                Action::ChartHover(_) if Some(index) != last_hover => {}
                Action::ChartPoint(_) if Some(index) != last_point => {}
                action => self.action(action)?,
            }
        }
        self.sync_history()?;
        if self.settings_visible {
            if self.settings_polled.elapsed() < Duration::from_millis(25) {
                return Ok(());
            }
            self.settings_polled = Instant::now();
            let outcome = match &mut self.settings {
                Some(page) => page.refresh(self.frames.last().map(|f| f.as_ref()))?,
                None => None,
            };
            match outcome {
                Some(settings_page::SettingsOutcome::Language(language)) => {
                    self.set_language(language, island)?
                }
                Some(settings_page::SettingsOutcome::Applied) => self.load_alerts(),
                Some(settings_page::SettingsOutcome::Theme(choice)) => {
                    // A choice that cannot be saved leaves the theme as it was.
                    let _ = choice.save();
                    self.follow_theme(island)?
                }
                Some(settings_page::SettingsOutcome::ChartLine(width)) => {
                    self.chart_line = width;
                    self.dirty = true;
                }
                None => {}
            }
            return Ok(());
        }
        let position = Ui::range(&self.elements.timeline, None)?;
        let (width, height) = Ui::size(&self.elements.chart)?;
        let layout = ChartLayout::new(width, height)
            .scale(self.chrome.scale())
            .for_resource(self.resource)
            .compact(self.compact);
        if layout != self.chart_layout {
            self.chart_layout = layout;
            self.dirty = true;
        }
        if (position - self.position).abs() > 0.001 {
            self.action(Action::Point(position / 1000.0))?;
        }
        let search = self
            .elements
            .search
            .query(&Guid::from_u128(0xe48f5a8b_1dff_4352_a1f4_e516514ec882))?
            .string(6)?;
        let table_width = Ui::size(&self.elements.table_card)?.0;
        if (table_width - self.table_width).abs() >= 1.0 {
            self.table_width = table_width;
            self.table_dirty = true;
        }
        if search != self.search {
            Ui::visible(&self.root, "SearchClear", !search.is_empty())?;
            self.search = search;
            // The chart does not depend on the search; only the highlight it drops.
            self.hover = None;
            self.table_dirty = true;
            self.hover_dirty = true;
        }
        // Rebuilding the table is the slowest step; while the moment is dragged it
        // follows at 10 Hz and the overlay keeps the frame rate.
        if self.table_deferred && self.last_table.elapsed() >= Duration::from_millis(100) {
            self.table_dirty = true;
        }
        // Scrolled past the created rows: create the ones now in view.
        if !self.table_dirty && self.processes() {
            let (offset, viewport) = Ui::scroll(&self.elements.rows_scroll)?;
            self.table_dirty = !self.row_window.shows(offset, viewport);
        }
        if self.table_due && !self.dirty {
            self.table_dirty = true;
        }
        if self.table_dirty {
            self.table_deferred = false;
            self.table_due = false;
            self.render_rows()?;
            self.table_dirty = false;
            self.last_table = Instant::now();
        }
        if self.dirty {
            self.render_chart()?;
        } else if self.overlay_dirty {
            self.render_overlay()?;
        } else if self.hover_dirty {
            self.render_hover()?;
        }
        self.dirty = false;
        self.overlay_dirty = false;
        self.hover_dirty = false;
        Ok(())
    }
    /// Takes the recorder's newest history; runs even while nothing is on screen,
    /// so frames the window no longer shows are released.
    fn sync_history(&mut self) -> Result<()> {
        if !self.demo {
            let state = self.client.state.lock().unwrap_or_else(|e| e.into_inner());
            if state.revision != self.revision {
                self.revision = state.revision;
                self.frames = state.frames.values().cloned().collect();
                self.io_totals = state.io_totals.clone();
                // A paused chart changes only with late corrections of its frames.
                let mut chart_changed = self.frozen.is_none()
                    || self.connected != state.connected
                    || self.enabled != state.enabled;
                if let Some(frozen) = &mut self.frozen {
                    for frame in frozen {
                        if let Some(updated) = state.frames.get(&frame.bucket) {
                            if !Arc::ptr_eq(frame, updated) {
                                if self.selected == Some(frame.bucket) && self.hover.is_none() {
                                    self.table_dirty = true;
                                }
                                *frame = updated.clone();
                                chart_changed = true;
                            }
                        }
                    }
                }
                self.connected = state.connected;
                self.enabled = state.enabled;
                // Otherwise only the texts that count time from the live end move.
                if chart_changed {
                    self.dirty = true;
                } else {
                    self.overlay_dirty = true;
                }
                drop(state);
                self.refresh_devices();
                if self.selected.is_none()
                    && self.hover.is_none()
                    && self.last_table.elapsed().as_millis() >= 950
                {
                    self.table_due = true;
                }
            }
        }
        if let Some(bucket) = self.selected {
            // The ring buffer dropped the selected moment: return to live.
            if self.frames.first().is_some_and(|f| f.bucket > bucket) {
                self.action(Action::Live)?;
            }
        }
        Ok(())
    }
    /// A device was added or removed: keep showing the current one if it is left.
    fn refresh_menu(&mut self, island: &XamlIsland) -> Result<()> {
        if self.menu.refresh() {
            let index = self.menu.position(&self.device.id).unwrap_or(0);
            if let Some(device) = self.menu.devices.get(index).cloned() {
                self.resource = device.resource;
                self.device = device;
            }
            self.rebuild(island)?;
        }
        Ok(())
    }
    fn refresh_devices(&mut self) {
        let engine = self
            .frames
            .last()
            .and_then(|f| self.device.engine(f))
            .map(|e| e.name.clone());
        self.hardware.refresh(&self.device, engine.as_deref());
    }
    fn subtitle(&self) -> String {
        self.hardware.subtitle(&self.device, self.language)
    }
    /// History shorter than the 5-minute window, in seconds.
    fn collected(&self) -> Option<u64> {
        let first = self.frames.first()?.bucket;
        let last = self.live_end();
        // Up to two seconds may still be in flight from the recorder.
        (last - first + 1 + chart::ETW_DELAY < WINDOW).then_some((last - first).div_ceil(2))
    }
    fn render_header(&self) -> Result<()> {
        let header = HeaderMarkup {
            design: self.design,
            language: self.language,
        };
        let mode = match self.selected {
            Some(bucket) => Mode::History {
                bucket,
                ago: (!self.compact).then(|| self.live_end().saturating_sub(bucket) / 2),
            },
            None => Mode::Live {
                tail: if !self.processes() {
                    Some(self.language.text("totals only").to_owned())
                } else {
                    self.collected().map(|seconds| {
                        self.language
                            .text("{a} of 5:00 collected")
                            .replace("{a}", &self.language.elapsed(seconds))
                    })
                },
            },
        };
        self.shown
            .content(&self.root, "ModeHost", &header.pill(&mode))?;
        Ui::visible(&self.root, "LiveActions", self.selected.is_some())?;
        if self.selected.is_some() {
            let frozen = self.timeline().last().map_or(0, |f| f.bucket);
            let pending = self.live_end().saturating_sub(frozen) / 2;
            self.shown
                .content(&self.root, "Live", &header.back(pending))?;
        }
        Ui::text(&self.root, "Title", &self.device.title(self.language))?;
        Ui::text(&self.root, "Subtitle", &self.subtitle())
    }
    fn render_legend(&self, frame: Option<&Frame>) -> Result<()> {
        let d = self.design;
        let short = self.compact;
        let total = match (self.resource, short) {
            (_, true) => "Total",
            (Resource::Cpu, _) => "Total load",
            (Resource::Gpu, _) => "Busiest engine",
            (Resource::Ram, _) => "RAM in use",
            (Resource::Disk, _) => "Disk total (PDH)",
            (Resource::Net, _) => "Total (PDH)",
        };
        let mut entries = vec![LegendEntry {
            mark: LegendMark::Line {
                color: d.color("total").into(),
                dashed: false,
            },
            label: self.language.text(total).into(),
        }];
        if self.resource.temperature_axis() {
            entries.push(LegendEntry {
                mark: LegendMark::Line {
                    color: d.color("temp").into(),
                    dashed: true,
                },
                label: self
                    .language
                    .text(if short { "Temp." } else { "Temperature" })
                    .into(),
            });
        }
        if self.processes() {
            for layer in self.pins.layers(d, self.language) {
                let present = self.selected.is_none()
                    || frame
                        .is_some_and(|f| f.processes.iter().any(|p| Timeline::key(p) == layer.key));
                entries.push(LegendEntry {
                    mark: if present {
                        LegendMark::Area { color: layer.color }
                    } else {
                        LegendMark::Missing { color: layer.color }
                    },
                    label: layer.name,
                });
            }
        }
        // The hovered process needs no entry: its series stands out as the pointer
        // moves over the rows.
        if !short && self.selected.is_some() {
            entries.push(LegendEntry {
                mark: LegendMark::Moment,
                label: self.language.text("Moment").into(),
            });
        }
        if self
            .timeline()
            .windows(2)
            .any(|p| p[1].bucket > p[0].bucket + 2)
        {
            entries.push(LegendEntry {
                mark: LegendMark::Hatch,
                label: self.language.text("No samples").into(),
            });
        }
        if self.resource.dual() && self.selected.is_none() && self.processes() {
            entries.push(LegendEntry {
                mark: LegendMark::Hatch,
                label: self.language.text("Awaiting ETW attribution").into(),
            });
        }
        if self.resource == Resource::Ram
            && !chart::ChartRenderer::unshared(self.timeline()).is_empty()
        {
            entries.push(LegendEntry {
                mark: LegendMark::Hatch,
                label: self.language.text("Window closed: no shared memory").into(),
            });
        }
        let legend = ChartLegend { design: d };
        let items: Vec<String> = entries.iter().map(|e| legend.item(e)).collect();
        let widths = items
            .iter()
            .map(|item| {
                self.legend_widths
                    .width(item, || Ok(Ui::measure(&Ui::load(item)?)?.0))
            })
            .collect::<Result<Vec<_>>>()?;
        let flow = LegendFlow::new(&widths, self.chart_layout.width);
        self.shown.content(
            &self.root,
            "LegendHost",
            &ChartLegend::markup(&items, &flow),
        )
    }
    fn render_info(&self, frame: Option<&Frame>) -> Result<()> {
        let mut points: Vec<String> = match self.resource {
            Resource::Cpu => vec![
                "Total load is the PDH counter “% Processor Utility”; process time is CPU time over the interval divided by the number of logical processors. The process sum can differ from the total load.",
                "Processes are sampled every 0.5 s: a short one that starts and exits between samples is not in the table, though the total load counts it.",
                "Temperature is the package sensor; the line is capped at 100 °C.",
            ],
            Resource::Gpu => vec![
                "The total line is the busiest physical engine at each sample. Process percentages are their share of exactly that engine.",
            ],
            Resource::Ram => vec![
                "The line is the memory in use, as on the taskbar: all memory minus the available part (standby cache and free pages).",
                "Private: pages of this process alone. Shared: an estimate of its writable shared memory, such as the guest memory of a virtual machine, fitted so that the rows add up to the line. Measured once a second while this window is open; the hatched spans are the time it was closed.",
                "Rows without a PID are memory outside processes. «Shared memory» is the rest: DLLs and files mapped by processes, and processes closed to inspection.",
                "All rows add up to the line.",
            ],
            Resource::Disk if self.device.network_drive() => vec![
                "The totals are the SMB client's traffic to the share, from the PDH counters “SMB Client Shares”. Until Windows connects to the share they are zero.",
                "Read is drawn above the axis, write below it, on the same scale.",
            ],
            Resource::Disk | Resource::Net => vec![
                "Totals come from PDH counters; per-process traffic comes from ETW. The two can differ slightly.",
                "ETW delivers events with a delay, so live per-process values trail the total by about 2 s (hatched strip).",
                if self.resource == Resource::Disk {
                    "Read is drawn above the axis, write below it, on the same scale."
                } else {
                    "Receive is drawn above the axis, send below it, on the same scale."
                },
            ],
        }
        .into_iter()
        .map(|key| self.language.text(key).to_owned())
        .collect();
        if self.resource == Resource::Gpu
            && frame
                .and_then(|f| self.device.engine(f))
                .is_some_and(|e| e.raw_total > 100.0)
        {
            points.push(
                self.language
                    .text("Counter overshoot: contributions scaled proportionally to 100%.")
                    .into(),
            );
        }
        if self.resource.dual()
            && frame.is_some_and(|f| !f.etw_active || f.lost_events > 0 || f.undecoded > 0)
        {
            points.push(
                self.language
                    .text("I/O attribution unavailable or incomplete")
                    .into(),
            );
        }
        points.extend(self.pin_error.clone());
        let d = self.design;
        let mut markup = format!(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Spacing="6"><TextBlock Text="{}" FontSize="13" FontWeight="SemiBold" Foreground="{}"/>"#,
            self.language.text("How it’s measured"),
            d.color("text")
        );
        for point in points {
            markup.push_str(&format!(
                r#"<Grid ColumnSpacing="8"><Grid.ColumnDefinitions><ColumnDefinition Width="Auto"/><ColumnDefinition/></Grid.ColumnDefinitions><TextBlock Text="•" FontSize="12" Foreground="{}" LineHeight="17"/><TextBlock Grid.Column="1" Text="{}" FontSize="12" Foreground="{}" TextWrapping="Wrap" LineHeight="17"/></Grid>"#,
                d.color("text3"),
                Ui::xml(&point),
                d.color("text2")
            ));
        }
        markup.push_str("</StackPanel>");
        self.shown.children(&self.root, "InfoPanel", &markup)
    }
    fn render_pointer(&self) -> Result<()> {
        let markup = match self.pointer {
            Some(point) => ChartTooltip {
                design: self.design,
                language: self.language,
            }
            .markup(
                point,
                self.chart_layout,
                self.timeline(),
                &self.device,
                &self.layers(),
            ),
            None => {
                r#"<Canvas xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation"/>"#
                    .into()
            }
        };
        Ui::children(&self.elements.hover_host, &Ui::load(&markup)?)
    }
    fn layers(&self) -> Vec<PinnedLayer> {
        if self.processes() {
            self.pins.layers(self.design, self.language)
        } else {
            Vec::new()
        }
    }
    fn render_chart(&mut self) -> Result<()> {
        let chart = Ui::load(
            &ChartRenderer {
                design: self.design,
                language: self.language,
                line: self.chart_line,
            }
            .markup(
                self.timeline(),
                &self.device,
                self.selected,
                &self.layers(),
                self.chart_layout,
            ),
        )?;
        Ui::children(&self.find("GraphHost")?, &chart)?;
        self.chart_series = Ui::find(&chart, "Series").ok();
        self.render_overlay()
    }
    /// Parts of the chart card that follow the moment and the highlighted process.
    fn render_overlay(&self) -> Result<()> {
        self.render_header()?;
        self.render_legend(self.selected_frame().as_deref())?;
        self.render_hover()?;
        if self.pointer.is_some() {
            self.render_pointer()?;
        }
        Ok(())
    }
    /// Parts of the chart card that follow the highlighted process, including the
    /// notes of the frame it selects; the legend does not.
    fn render_hover(&self) -> Result<()> {
        let frame = self.selected_frame();
        self.render_info(frame.as_deref())?;
        let overlay = Ui::load(
            &ChartRenderer {
                design: self.design,
                language: self.language,
                line: self.chart_line,
            }
            .overlay(
                self.timeline(),
                &self.device,
                self.selected,
                self.hover,
                &self.layers(),
                self.chart_layout,
            ),
        )?;
        Ui::children(&self.elements.overlay_host, &overlay)?;
        if let Some(series) = &self.chart_series {
            Ui::opacity(series, if self.hover.is_some() { 0.6 } else { 1.0 })?;
        }
        Ok(())
    }
    fn render_stats(&mut self) -> Result<()> {
        // Widths the previous row was laid out with keep the columns in place.
        let same = self.stat_widths.track((self.resource, self.compact));
        if same {
            if let Some((row, _)) = &self.stats_row {
                let mut index = 0;
                while let Ok(item) = Ui::find(row, &format!("Stat{index}")) {
                    self.stat_widths.grow(index, Ui::size(&item)?.0);
                    index += 1;
                }
            }
        }
        let frame = self.selected_frame();
        let layers = self.layers();
        let stats = StatsSource {
            language: self.language,
            device: &self.device,
            frame: frame.as_deref(),
            timeline: self.timeline(),
            alerts: self.alerts,
            scale: ChartRenderer::scale(self.timeline(), &self.device, &layers),
            // Memory in use is sampled live, so a past moment shows only the size.
            gpu_memory: self
                .hardware
                .gpu_memory()
                .map(|(used, total)| (used.filter(|_| self.selected.is_none()), total)),
            facts: if self.selected.is_none() {
                self.hardware.facts()
            } else {
                devices::Facts::default()
            },
        }
        .stats();
        // The statistics have the whole width of the page.
        let available = Ui::size(&self.find("Performance")?)?.0;
        let panel = StatsPanel {
            design: self.design,
            compact: self.compact,
            available: if available > 0.0 {
                available
            } else {
                f64::INFINITY
            },
        };
        let stats = panel.fitting(stats, &self.stat_widths);
        let shape = panel.shape(&stats, &self.stat_widths);
        match &self.stats_row {
            Some((row, known)) if same && *known == shape => {
                StatsPanel::update(row, &stats, &self.stat_widths)
            }
            _ => {
                let markup = panel.markup(&stats, &self.stat_widths);
                let row = Ui::load(&markup)?;
                Ui::children(&self.find("Stats")?, &row)?;
                self.stat_events = StatsPanel::hints(&row, &stats)?;
                self.stats_row = Some((row, shape));
                Ok(())
            }
        }
    }
    fn render_toolbar(&self, frame: Option<&Frame>, found: Option<usize>) -> Result<()> {
        let moment = match (frame, self.selected) {
            _ if !self.processes() => self.language.text("no data").to_owned(),
            (None, _) => String::new(),
            (Some(f), Some(_)) if self.compact => self.language.time(f.bucket),
            (Some(f), Some(_)) => self
                .language
                .text("as of {t}")
                .replace("{t}", &self.language.time(f.bucket)),
            (Some(f), None) if self.resource.dual() => self
                .language
                .text("as of {t} · 2 s behind live")
                .replace("{t}", &self.language.time(f.bucket)),
            (Some(_), None) => self.language.text("now · updates 1×/s").to_owned(),
        };
        Ui::text(&self.root, "Moment", &moment)?;
        let total = match frame.map_or(0, |f| {
            f.processes.iter().filter(|p| self.device.lists(p)).count()
        }) {
            // Without process history the tab still counts what is running now.
            0 if !self.processes() => SystemActivity::read().map_or(0, |a| a.processes as usize),
            count => count,
        };
        Ui::text(
            &self.root,
            "SearchCount",
            &match found {
                None => String::new(),
                Some(0) => "0".into(),
                Some(n) => self
                    .language
                    .text("{a} of {b}")
                    .replace("{a}", &n.to_string())
                    .replace("{b}", &total.to_string()),
            },
        )?;
        let d = self.design;
        for (name, active, label) in [
            ("Top", !self.all, self.language.text("Top 10").to_owned()),
            (
                "All",
                self.all,
                if total > 0 {
                    self.language
                        .text("All · {n}")
                        .replace("{n}", &total.to_string())
                } else {
                    self.language.text("All").to_owned()
                },
            ),
        ] {
            let content = format!(
                r#"<Border xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Height="22" Padding="10,0" CornerRadius="4" BorderThickness="1" Background="{}" BorderBrush="{}"><TextBlock Text="{}" FontSize="12" FontWeight="{}" Foreground="{}" VerticalAlignment="Center"/></Border>"#,
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
                Ui::xml(&label),
                if active { "SemiBold" } else { "Normal" },
                d.color(if active { "text" } else { "text2" })
            );
            self.shown.content(&self.root, name, &content)?;
        }
        Ok(())
    }
    fn render_off(&self) -> Result<bool> {
        // A network drive's traffic belongs to System, so it has no process rows.
        let share = self.device.network_drive();
        let off = !self.processes() || share;
        Ui::visible(&self.root, "OffPanel", off)?;
        for (name, shown) in [
            ("OffIcon", !share),
            ("ShareIcon", share),
            ("OffActions", !share),
        ] {
            Ui::visible(&self.root, name, shown)?;
        }
        let (title, text) = if share {
            (
                "No per-process data for network drives",
                "Windows sends network drive traffic through the System process, so it cannot be split by process. The chart shows the drive's total read and write.",
            )
        } else {
            (
                "Process monitoring is off",
                "Totals are still collected. Turn on background process history to see which processes used resources at any moment of the last 5 minutes.",
            )
        };
        Ui::text(&self.root, "OffTitle", self.language.text(title))?;
        Ui::text(&self.root, "OffText", self.language.text(text))?;
        // Nothing to search without per-process history, as in the artboard.
        Ui::enable(&self.elements.search, !off)?;
        Ui::visible(&self.root, "SearchLine", !off)?;
        for name in ["TableHeader", "RowsScroll"] {
            Ui::visible(&self.root, name, !off)?;
        }
        if off {
            Ui::visible(&self.root, "RestBorder", false)?;
            let layers = self.pins.layers(self.design, self.language);
            Ui::visible(&self.root, "OffPins", !share && !layers.is_empty())?;
            let mut chips = String::from(
                r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Orientation="Horizontal" Spacing="6">"#,
            );
            for layer in layers {
                chips.push_str(&format!(
                    r#"<Border Height="24" CornerRadius="12" Padding="8,0" BorderBrush="{}" BorderThickness="1"><StackPanel Orientation="Horizontal" Spacing="6"><Rectangle Width="10" Height="10" RadiusX="2.25" RadiusY="2.25" Stroke="{}" StrokeThickness="1.5" StrokeDashArray="2,1.5" VerticalAlignment="Center"/><TextBlock Text="{}" FontSize="12" Foreground="{}" VerticalAlignment="Center"/></StackPanel></Border>"#,
                    self.design.color("border"),
                    layer.color,
                    Ui::xml(&layer.name),
                    self.design.color("text2")
                ));
            }
            chips.push_str("</StackPanel>");
            Ui::children(&self.find("OffChips")?, &Ui::load(&chips)?)?;
        }
        Ok(off)
    }
    fn table(&self) -> TableLayout {
        TableLayout {
            sort: self.sort,
            resource: self.resource,
            compact: self.compact,
            design: self.design,
            language: self.language,
            width: self.table_width,
        }
        .sorted_by(self.sort)
    }
    fn render_rows(&mut self) -> Result<()> {
        self.render_stats()?;
        self.render_navigation()?;
        let frame = self.selected_frame();
        self.table_frame = frame.clone();
        let searching = !self.search.is_empty();
        let table = self.table();
        // Bytes since the recorder started, as of the table's moment.
        let totals = frame
            .as_deref()
            .filter(|_| table.totals())
            .map(|f| IoTotals::at(&self.device, &self.io_totals, &self.frames, f.bucket));
        let total_of = |key: ProcessKey| totals.as_ref().map(|t| t.of(key).unwrap_or([0; 2]));
        let (display, rest_indices) = self.pins.rows(
            frame.as_deref(),
            &self.device,
            &self.search,
            self.all,
            table.sort,
            totals.as_ref(),
        );
        let pinned = display.iter().filter(|p| p.pinned).count();
        let found = display.len() - pinned;
        self.render_toolbar(frame.as_deref(), searching.then_some(found))?;
        if self.render_off()? {
            return Ok(());
        }
        let header_markup = table.header();
        if header_markup != self.table_markup.0 {
            let header = Ui::load(&header_markup)?;
            self.header_events.clear();
            if self.resource.paired() {
                for column in 0..table.sortable() {
                    let queue = self.events.clone();
                    self.header_events.push(Subscription::click(
                        &Ui::find(&header, &format!("Sort{column}"))?,
                        move || {
                            queue.borrow_mut().push(Action::Sort(column));
                            Ok(())
                        },
                    )?);
                }
            }
            Ui::children(&self.find("TableHeader")?, &header)?;
            self.table_markup.0 = header_markup;
        }
        let d = self.design;
        let mut markup = String::from(
            r#"<StackPanel xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" xmlns:x="http://schemas.microsoft.com/winfx/2006/xaml">"#,
        );
        let mut keys = Vec::new();
        if frame.is_none() {
            markup.push_str(&format!(r#"<StackPanel Margin="44,20" Spacing="8"><TextBlock Text="{}" FontSize="16" FontWeight="SemiBold" TextWrapping="Wrap"/><TextBlock Text="{}" FontSize="12" TextWrapping="Wrap" Foreground="{}"/></StackPanel>"#,self.language.text("Waiting for history…"),self.language.text("Recorder unavailable. Enable background monitoring in Settings."),d.color("text2")));
        }
        // Regular rows follow the pinned ones; only those near the viewport are created.
        let (offset, viewport) = Ui::scroll(&self.elements.rows_scroll)?;
        let window = RowWindow::around(found, RowWindow::top(pinned), offset, viewport);
        self.row_window = window;
        let spacer = |rows: usize| {
            if rows > 0 {
                format!(r#"<Border Height="{}"/>"#, rows as f64 * RowWindow::ROW)
            } else {
                String::new()
            }
        };
        for (row, item) in display.iter().enumerate() {
            // A divider alone separates the pinned rows from the rest.
            if !item.pinned && row > 0 && display[row - 1].pinned {
                markup.push_str(&table.divider());
            }
            if !item.pinned {
                let regular = row - pinned;
                if regular == 0 {
                    markup.push_str(&spacer(window.start));
                }
                if !(window.start..window.end).contains(&regular) {
                    continue;
                }
            }
            keys.push((row, item.identity.clone()));
            let color = self
                .pins
                .process_color((item.identity.pid, item.identity.created), d);
            let values = match (&frame, item.sample) {
                (Some(f), Some(index)) => Some(self.device.process(f, &f.processes[index])),
                _ => None,
            };
            markup.push_str(&table.row(&RowContent {
                index: row,
                name: self.language.process(&item.identity),
                pid: item.identity.pid,
                kind: match (item.pinned, values.is_some()) {
                    (false, _) => RowKind::Regular,
                    (true, true) => RowKind::Pinned,
                    (true, false) => RowKind::PinnedMissing,
                },
                color: color.as_deref(),
                total: values.and(total_of((item.identity.pid, item.identity.created))),
                values,
                locked: !item.pinned && self.pins.full(),
                hovered: self.hover == Some((item.identity.pid, item.identity.created)),
            }));
        }
        markup.push_str(&spacer(found - window.end));
        if frame.is_some() && searching && found == 0 {
            if pinned > 0 {
                markup.push_str(&table.divider());
            }
            markup.push_str(
                &table.note(None, self.language.text("No matches · pinned stay visible")),
            );
        }
        markup.push_str("</StackPanel>");
        let show_rest = !rest_indices.is_empty() && !self.all && !searching;
        Ui::visible(&self.root, "RestBorder", show_rest)?;
        if let (Some(f), true) = (&frame, show_rest) {
            let mut rest = [None, None];
            let mut rest_total = totals.as_ref().map(|_| [0u64; 2]);
            for index in &rest_indices {
                let process = &f.processes[*index];
                for (total, value) in rest.iter_mut().zip(self.device.process(f, process)) {
                    if let Some(value) = value {
                        *total = Some(total.unwrap_or(0.0) + value);
                    }
                }
                if let (Some(sum), Some(bytes)) =
                    (&mut rest_total, total_of(Timeline::key(process)))
                {
                    for (sum, bytes) in sum.iter_mut().zip(bytes) {
                        *sum += bytes;
                    }
                }
            }
            self.shown.children(
                &self.root,
                "Rest",
                &table.footer(rest_indices.len(), rest, rest_total),
            )?;
        }
        // Values of idle processes often stay the same from one update to the next.
        let row_keys: Vec<_> = keys
            .iter()
            .map(|(row, identity)| (*row, (identity.pid, identity.created)))
            .collect();
        if markup == self.table_markup.1 && row_keys == self.table_markup.2 {
            return Ok(());
        }
        let rows = Ui::load(&markup)?;
        self.row_events.clear();
        let brushes = [d.color("rowHover"), "Transparent"].map(|color| {
            Ui::load(&format!(
                r#"<SolidColorBrush xmlns="http://schemas.microsoft.com/winfx/2006/xaml/presentation" Color="{color}"/>"#
            ))
        });
        let [hover_brush, clear_brush] = brushes;
        let (hover_brush, clear_brush) = (hover_brush?, clear_brush?);
        for (row, identity) in keys {
            let key = (identity.pid, identity.created);
            let queue = self.events.clone();
            self.row_events.push(Subscription::click(
                &Ui::find(&rows, &format!("Pin{row}"))?,
                move || {
                    queue.borrow_mut().push(Action::Pin(identity.clone()));
                    Ok(())
                },
            )?);
            // Keyboard focus (Tab, ↑↓) outlines the whole row like the artboard.
            let (pin, outline) = (
                Ui::find(&rows, &format!("Pin{row}"))?,
                Ui::find(&rows, &format!("Focus{row}"))?,
            );
            for (slot, focused) in [(45, true), (47, false)] {
                let (button, outline) = (pin.clone(), outline.clone());
                self.row_events
                    .push(Subscription::routed(&pin, slot, move || {
                        Ui::show(&outline, focused && Ui::keyboard_focused(&button)?)
                    })?);
            }
            let element = Ui::find(&rows, &format!("Row{row}"))?;
            for (slot, action) in [(63, Some(key)), (65, None)] {
                let queue = self.events.clone();
                let element = element.clone();
                let brush = if action.is_some() {
                    hover_brush.clone()
                } else {
                    clear_brush.clone()
                };
                self.row_events.push(Subscription::pointer(
                    &element.clone(),
                    slot,
                    move |_, _| {
                        Ui::background(&element, &brush)?;
                        queue.borrow_mut().push(Action::Hover(action));
                        Ok(())
                    },
                )?);
            }
        }
        Ui::children(&self.find("Rows")?, &rows)?;
        self.table_markup.1 = markup;
        self.table_markup.2 = row_keys;
        Ok(())
    }

    fn demo(&mut self, scenario: demo::Scenario) -> Result<()> {
        let end = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .unwrap_or_default()
            .as_millis() as u64
            / 500;
        self.demo = true;
        self.frames = demo::DemoHistory::frames(scenario, end);
        self.io_totals = Arc::new(demo::DemoHistory::io_totals(&self.frames));
        self.refresh_devices();
        self.connected = true;
        self.enabled = scenario != demo::Scenario::Off;
        for name in demo::pins(scenario, self.resource) {
            self.pins.toggle(demo::identity(name));
        }
        if scenario == demo::Scenario::Settings {
            self.show_settings()?;
        }
        if scenario == demo::Scenario::History {
            self.action(Action::Point(0.83))?;
            let hovered = if self.resource == Resource::Disk {
                "System"
            } else {
                "msedge.exe"
            };
            let identity = demo::identity(hovered);
            self.action(Action::Hover(Some((identity.pid, identity.created))))?;
        }
        Ok(())
    }
    fn verify(&mut self, island: &XamlIsland) -> Result<()> {
        use crate::platform::process_history::store::{Identity, IoBytes, Sample};
        self.frames = (0..30)
            .map(|step| {
                Arc::new(Frame {
                    gpu_engines: vec![(
                        "gpu".into(),
                        crate::metrics::GpuEngineUsage {
                            name: "luid_test_phys_0_eng_0".into(),
                            label: "3D".into(),
                            total: 50.0,
                            raw_total: 50.0,
                            processes: (0..16)
                                .map(|index| (index + 100, index as f64 * 50.0 / 120.0))
                                .collect(),
                        },
                    )],
                    bucket: 3_580_000_000 + step,
                    elapsed_ms: 500.0,
                    etw_active: true,
                    lost_events: 0,
                    undecoded: 0,
                    sample_ms: 1.0,
                    totals: vec![
                        ("cpu".into(), Some(step as f64)),
                        ("cpu_temperature".into(), Some(90.0 + step as f64)),
                        ("gpu".into(), Some(50.0)),
                        ("ram".into(), Some(44.0)),
                        ("disk".into(), Some(38.0)),
                        ("disk_read".into(), Some(step as f64 / 2.0)),
                        ("disk_write".into(), Some(2.0)),
                        ("net_down".into(), Some(3.0)),
                        ("net_up".into(), Some(0.0)),
                    ],
                    processes: (0..16)
                        .map(|index| {
                            Sample::new(
                                Arc::new(Identity {
                                    pid: index + 100,
                                    created: 500 + index as u64,
                                    name: format!("Process {index} & tést.exe").into(),
                                }),
                                Some((step + index as u64) as f64 / 2.0),
                                Some(index as f64),
                                [Some(index as u64 * 1_000_000), None, None],
                                Sample::boxed_io(IoBytes {
                                    total: [step * 50_000, index as u64 * 20_000, 100_000, 0],
                                    devices: Vec::new(),
                                }),
                            )
                        })
                        .collect(),
                })
            })
            .collect();
        self.connected = true;
        self.enabled = true;
        for (language, dark) in [
            (Language::English, false),
            (Language::Russian, false),
            (Language::English, true),
            (Language::Russian, true),
        ] {
            self.language = language;
            self.design = Design { dark };
            self.rebuild(island)?;
            if !self.pins.contains((100, 500)) {
                self.action(Action::Pin(self.frames[0].processes[0].identity.clone()))?;
            }
            for (index, resource) in Resource::ALL.into_iter().enumerate() {
                self.action(Action::Tab(index))?;
                let (rows, _) = self.pins.rows(
                    self.selected_frame().as_deref(),
                    &self.device,
                    "",
                    false,
                    0,
                    None,
                );
                if rows
                    .first()
                    .is_none_or(|r| !r.pinned || r.identity.pid != 100)
                {
                    return Err(E_FAIL);
                }
                for compact in [false, true] {
                    self.compact = compact;
                    self.render_rows()?;
                    for width in [300.0, 500.0, 1300.0] {
                        self.chart_layout = ChartLayout::new(width, 250.0)
                            .for_resource(resource)
                            .compact(compact);
                        self.render_chart()?;
                    }
                }
                self.compact = false;
                self.action(Action::Point(0.5))?;
                if self.selected != Some(3_580_000_014) {
                    return Err(E_FAIL);
                }
                self.render_rows()?;
                self.action(Action::Hover(Some((115, 515))))?;
                self.render_chart()?;
                self.action(Action::Hover(None))?;
                self.action(Action::ChartHover(Some((100.0, 60.0))))?;
                self.action(Action::ChartHover(None))?;
                self.action(Action::Menu(true))?;
                self.action(Action::Menu(false))?;
                self.action(Action::All)?;
                self.action(Action::Sort(1))?;
                self.render_rows()?;
                self.action(Action::Sort(2))?;
                self.action(Action::ClearSearch)?;
                self.render_rows()?;
                self.action(Action::Live)?;
                self.enabled = false;
                self.render_rows()?;
                self.render_chart()?;
                self.enabled = true;
            }
            self.show_settings()?;
            if let Some(settings) = &mut self.settings {
                settings.refresh(self.frames.last().map(|f| f.as_ref()))?;
                settings.verify()?;
            }
            self.action(Action::Tab(0))?;
            for layout in [
                WindowLayout::Compact,
                WindowLayout::Minimal,
                WindowLayout::Full,
            ] {
                self.layout = layout;
                self.compact = layout.compact();
                self.rebuild(island)?;
                self.render_rows()?;
                self.render_chart()?;
                self.action(Action::Menu(true))?;
                self.action(Action::Menu(false))?;
            }
        }
        Ok(())
    }
}

pub(super) fn run() -> Result<()> {
    let requested = std::env::args().find_map(|arg| DeviceId::parse(&arg));
    let handoff = requested
        .as_ref()
        .map_or_else(|| "cpu".to_owned(), DeviceId::to_string);
    let _instance = if !persistent() {
        None
    } else {
        let handle = Handle::new(unsafe {
            CreateMutexW(
                ptr::null_mut(),
                0,
                wide("Local\\TaskbarMetrics.Monitor").as_ptr(),
            )
        })?;
        if unsafe { GetLastError() } == 183 {
            for _ in 0..40 {
                if activate_existing(&handoff) {
                    return Ok(());
                }
                std::thread::sleep(std::time::Duration::from_millis(50));
            }
            return Ok(());
        }
        if activate_existing(&handoff) {
            return Ok(());
        }
        Some(handle)
    };
    let _apartment = StaApartment::new()?;
    let manager = XamlLifetime(
        factory(
            "Windows.UI.Xaml.Hosting.WindowsXamlManager",
            &Guid::from_u128(0x28258a12_7d82_505b_b210_712b04a58882),
        )?
        .object(6)?,
    );
    let window = NativeWindow::named(
        dark_theme()?,
        "TaskbarMetrics.Monitor",
        "Taskbar Metrics",
        1280,
        1024,
    )?;
    window.set_minimum(780, 960);
    let island = XamlIsland::new(&window)?;
    let mut dashboard = Dashboard::new(&island, requested, window.chrome())?;
    // Checks and demos always open centred at the default size.
    let memory = WindowMemory::locate().filter(|_| persistent());
    let restored = memory
        .as_ref()
        .and_then(WindowMemory::placement)
        .filter(|placement| window.restore(placement));
    match restored {
        Some(placement) if placement.maximized => window.show_maximized(),
        _ => window.show(),
    }
    NativeWindow::drain();
    if let Some(scenario) = scenario() {
        dashboard.demo(scenario)?;
    }
    let result = if verifying() {
        dashboard.verify(&island)
    } else {
        window.run(&island.native, NativeWindow::POINTER_FRAME, || {
            dashboard.refresh(&island)
        })
    };
    if let (Some(memory), Some(placement)) = (memory, NativeWindow::closed_placement()) {
        let _ = memory.remember(Some(placement));
    }
    drop(dashboard);
    drop(island);
    drop(window);
    drop(manager);
    NativeWindow::drain();
    result
}
/// Shows `device` in the running window (`WM_COPYDATA` with the device id).
fn activate_existing(device: &str) -> bool {
    #[repr(C)]
    struct CopyData {
        kind: usize,
        size: u32,
        data: *const u8,
    }
    #[link(name = "user32")]
    extern "system" {
        fn FindWindowW(class: *const u16, title: *const u16) -> Raw;
        fn SendMessageW(window: Raw, message: u32, wparam: usize, lparam: isize) -> isize;
        fn SetForegroundWindow(window: Raw) -> i32;
        fn ShowWindow(window: Raw, command: i32) -> i32;
    }
    unsafe {
        let window = FindWindowW(wide("TaskbarMetrics.Monitor").as_ptr(), ptr::null());
        if window.is_null() {
            return false;
        }
        let data = CopyData {
            kind: window::DEVICE_REQUEST,
            size: device.len() as u32,
            data: device.as_ptr(),
        };
        SendMessageW(window, 0x004A, 0, &data as *const CopyData as isize);
        ShowWindow(window, 9);
        SetForegroundWindow(window);
        true
    }
}
