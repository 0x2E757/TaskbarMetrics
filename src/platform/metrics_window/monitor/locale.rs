//! English is the source language. Every user-facing key has a Russian translation.
//! This catalog is the only place in the repository with Russian text.
use super::clock::LocalTime;
use crate::platform::process_history::{memory::MemoryRows, store::Identity};

#[derive(Clone, Copy, PartialEq, Eq)]
pub enum Language {
    English,
    Russian,
}
impl Language {
    /// Locale digits: RU groups thousands with U+202F, EN with commas; the fraction
    /// always follows a point.
    pub fn number(self, value: f64, precision: usize) -> String {
        let text = format!("{:.precision$}", value.abs());
        let (whole, fraction) = text.split_once('.').unwrap_or((&text, ""));
        let (group, decimal) = if self == Self::Russian {
            ('\u{202F}', '.')
        } else {
            (',', '.')
        };
        let mut result = String::new();
        if value < 0.0 && text.chars().any(|c| c.is_ascii_digit() && c != '0') {
            result.push('−');
        }
        for (index, digit) in whole.chars().enumerate() {
            if index > 0 && (whole.len() - index) % 3 == 0 {
                result.push(group);
            }
            result.push(digit);
        }
        if !fraction.is_empty() {
            result.push(decimal);
            result.push_str(fraction);
        }
        result
    }
    pub fn percent(self, value: f64, precision: usize) -> String {
        format!("{}\u{A0}%", self.number(value, precision))
    }
    pub fn celsius(self, value: f64) -> String {
        format!("{} °C", self.number(value.round(), 0))
    }
    pub fn rate(self, value: f64, precision: usize) -> String {
        format!("{} MB/s", self.number(value, precision))
    }
    /// "3 d 4 h", "4 h 15 min", "15 min": the two largest units there are.
    pub fn uptime(self, uptime: std::time::Duration) -> String {
        let minutes = uptime.as_secs() / 60;
        let (days, hours, minutes) = (minutes / 1440, minutes / 60 % 24, minutes % 60);
        let [d, h, m] = if self == Self::Russian {
            ["д", "ч", "мин"]
        } else {
            ["d", "h", "min"]
        };
        let mut parts = Vec::new();
        if days > 0 {
            parts.push(format!("{days}\u{A0}{d}"));
        }
        if days > 0 || hours > 0 {
            parts.push(format!("{hours}\u{A0}{h}"));
        }
        if days == 0 {
            parts.push(format!("{minutes}\u{A0}{m}"));
        }
        parts.join(" ")
    }
    /// `HH:MM:SS.d`.
    pub fn time(self, bucket: u64) -> String {
        format!("{}.{}", self.clock(bucket), LocalTime::of(bucket).tenth)
    }
    pub fn clock(self, bucket: u64) -> String {
        let time = LocalTime::of(bucket);
        format!("{:02}:{:02}:{:02}", time.hour, time.minute, time.second)
    }
    pub fn minute(self, bucket: u64) -> String {
        let time = LocalTime::of(bucket);
        format!("{:02}:{:02}", time.hour, time.minute)
    }
    /// `m:ss`, as in "0:42 of 5:00 collected".
    pub fn elapsed(self, seconds: u64) -> String {
        format!("{}:{:02}", seconds / 60, seconds % 60)
    }
    /// "4 min 26 s"; minutes are omitted below one minute.
    pub fn span(self, seconds: u64) -> String {
        let (minutes, seconds) = (seconds / 60, seconds % 60);
        let (m, s) = if self == Self::Russian {
            ("мин", "с")
        } else {
            ("min", "s")
        };
        if minutes == 0 {
            format!("{seconds} {s}")
        } else if seconds == 0 {
            format!("{minutes} {m}")
        } else {
            format!("{minutes} {m} {seconds} {s}")
        }
    }
    pub fn ago(self, seconds: u64) -> String {
        self.text("{t} ago").replace("{t}", &self.span(seconds))
    }
    /// "8 cores".
    pub fn cores(self, count: u32) -> String {
        self.plural(count, ["core", "cores"], ["ядро", "ядра", "ядер"])
    }
    /// "16 threads".
    pub fn threads(self, count: u32) -> String {
        self.plural(count, ["thread", "threads"], ["поток", "потока", "потоков"])
    }
    /// "3 changes not applied to the taskbar".
    pub fn unapplied(self, count: u32) -> String {
        self.plural(
            count,
            [
                "change not applied to the taskbar",
                "changes not applied to the taskbar",
            ],
            [
                "изменение не применено к таскбару",
                "изменения не применены к таскбару",
                "изменений не применено к таскбару",
            ],
        )
    }
    /// "3 rules".
    pub fn rules(self, count: u32) -> String {
        self.plural(count, ["rule", "rules"], ["правило", "правила", "правил"])
    }
    /// The name of the language in itself, for the language menu.
    pub fn name(self) -> &'static str {
        match self {
            Self::English => "English",
            Self::Russian => "Русский",
        }
    }
    /// `@text@`, translated by `markup`, when the catalog has `text`; otherwise `text`
    /// as it is, such as a unit both languages write alike.
    pub fn marked(text: &str) -> String {
        if CATALOG.iter().any(|(en, _)| *en == text) {
            format!("@{text}@")
        } else {
            text.to_owned()
        }
    }
    /// `english` is one and many; `russian` is one, few (2–4) and many.
    fn plural(self, count: u32, english: [&str; 2], russian: [&str; 3]) -> String {
        let form = if self == Self::English {
            english[usize::from(count != 1)]
        } else {
            match (count % 10, count % 100) {
                (1, n) if n != 11 => russian[0],
                (2..=4, n) if !(12..=14).contains(&n) => russian[1],
                _ => russian[2],
            }
        };
        format!("{count} {form}")
    }
    /// Saved choice; demo screenshots may force one with `--language en|ru`.
    pub fn load() -> Self {
        let args: Vec<_> = std::env::args().collect();
        if let Some(pair) = args.windows(2).find(|pair| pair[0] == "--language") {
            return if pair[1] == "ru" {
                Self::Russian
            } else {
                Self::English
            };
        }
        let saved =
            crate::platform::data_directory::DataDirectory::file("taskbar-metrics.language")
                .ok()
                .and_then(|p| std::fs::read_to_string(p).ok());
        match saved.as_deref() {
            Some("ru") => Self::Russian,
            Some("en") => Self::English,
            _ if SystemLanguage::russian() => Self::Russian,
            _ => Self::English,
        }
    }
    pub fn save(self) -> std::io::Result<()> {
        std::fs::write(
            crate::platform::data_directory::DataDirectory::file("taskbar-metrics.language")?,
            if self == Self::Russian { "ru" } else { "en" },
        )
    }
    pub fn text(self, key: &str) -> &str {
        if self == Self::English {
            return key;
        }
        CATALOG
            .iter()
            .find(|(en, _)| *en == key)
            .map(|(_, ru)| *ru)
            .unwrap_or(key)
    }
    /// Name of a history row: memory rows carry an English name to translate.
    pub fn process(self, identity: &Identity) -> &str {
        if identity.pid == MemoryRows::PID {
            self.text(&identity.name)
        } else {
            &identity.name
        }
    }
    pub fn markup(self, markup: &str) -> String {
        let mut result = markup.replace(
            "RequestedTheme=\"Default\"",
            if self == Self::English {
                "Language=\"en-US\" RequestedTheme=\"Default\""
            } else {
                "Language=\"ru-RU\" RequestedTheme=\"Default\""
            },
        );
        for (en, ru) in CATALOG {
            result = result.replace(
                &format!("@{en}@"),
                if self == Self::English { en } else { ru },
            );
        }
        result
    }
}
/// The language Windows suggests before one is chosen: Russian when it is the
/// display language or one of the keyboard layouts.
struct SystemLanguage;
impl SystemLanguage {
    fn russian() -> bool {
        // SAFETY: a null list of size 0 only asks for the number of layouts.
        let count = unsafe { GetKeyboardLayoutList(0, std::ptr::null_mut()) };
        let mut layouts = vec![0isize; count.max(0) as usize];
        // SAFETY: the list holds `count` handles.
        let count = unsafe { GetKeyboardLayoutList(count, layouts.as_mut_ptr()) };
        layouts.truncate(count.max(0) as usize);
        // SAFETY: no arguments.
        Self::any_russian(unsafe { GetUserDefaultUILanguage() }, &layouts)
    }
    /// The low word of a layout handle is its language; the low ten bits of a
    /// language id are the primary language, 0x19 for Russian.
    fn any_russian(display: u16, layouts: &[isize]) -> bool {
        std::iter::once(display)
            .chain(layouts.iter().map(|layout| *layout as u16))
            .any(|language| language & 0x3FF == 0x19)
    }
}
#[link(name = "user32")]
extern "system" {
    fn GetKeyboardLayoutList(count: i32, list: *mut isize) -> i32;
}
#[link(name = "kernel32")]
extern "system" {
    fn GetUserDefaultUILanguage() -> u16;
}

const CATALOG: &[(&str, &str)] = &[
    ("All", "Все"),
    ("All · {n}", "Все · {n}"),
    ("now", "сейчас"),
    ("Back to live", "Вернуться в реальное время"),
    ("History", "История"),
    ("Drive {v}", "Диск {v}"),
    ("Drives {v}", "Диски {v}"),
    ("{t} ago", "{t} назад"),
    ("{a} of 5:00 collected", "собрано {a} из 5:00"),
    ("totals only", "только общие метрики"),
    ("How it’s measured", "Как измеряется"),
    ("Load", "Загрузка"),
    ("Temperature", "Температура"),
    ("Temp.", "Темп."),
    ("5-min peak", "Пик за 5 мин"),
    ("above alert threshold {v}", "выше порога алерта {v}"),
    ("RAM usage", "Использование RAM"),
    ("RAM in use", "Используемая RAM"),
    ("In use", "Используется"),
    ("Private", "Частная"),
    ("Shared", "Общая"),
    ("Kernel: nonpaged pool", "Ядро: невыгружаемый пул"),
    ("Kernel: paged pool", "Ядро: выгружаемый пул"),
    ("Drivers and kernel code", "Драйверы и код ядра"),
    ("Modified pages", "Изменённые страницы"),
    ("System file cache", "Системный файловый кэш"),
    ("Shared memory", "Общая память"),
    (
        "The line is the memory in use, as on the taskbar: all memory minus the available part (standby cache and free pages).",
        "Линия — используемая память, как на таскбаре: вся память минус доступная (резервный кэш и свободные страницы).",
    ),
    (
        "Private: pages of this process alone. Shared: an estimate of its writable shared memory, such as the guest memory of a virtual machine, fitted so that the rows add up to the line. Measured once a second while this window is open; the hatched spans are the time it was closed.",
        "Частная — страницы только этого процесса. Общая — оценка его разделяемой памяти с правом записи (например, памяти гостевой ОС виртуальной машины), подогнанная так, чтобы строки в сумме давали линию. Измеряется раз в секунду, пока открыто это окно; штриховка — время, когда оно было закрыто.",
    ),
    (
        "Rows without a PID are memory outside processes. «Shared memory» is the rest: DLLs and files mapped by processes, and processes closed to inspection.",
        "Строки без PID — память вне процессов. «Общая память» — остаток: DLL и файлы, отображённые процессами, и процессы, закрытые для разбора.",
    ),
    ("All rows add up to the line.", "Вместе все строки дают значение линии."),
    ("Installed", "Всего"),
    ("Frequency", "Частота"),
    ("Up time", "Время работы"),
    ("Page file", "Файл подкачки"),
    ("Signal", "Сигнал"),
    (
        "Load of the busiest GPU engine: 3D, compute, copy or video",
        "Загрузка самого занятого движка GPU: 3D, вычисления, копирование или видео",
    ),
    ("excellent", "отличный"),
    ("good", "хороший"),
    ("fair", "средний"),
    ("weak", "слабый"),
    ("Scale", "Шкала"),
    ("min {v}, grows with peak", "мин. {v}, растёт по пику"),
    ("Read", "Чтение"),
    ("Write", "Запись"),
    ("Receive", "Приём"),
    ("Send", "Отправка"),
    ("Total load", "Общая загрузка"),
    ("Total", "Общая"),
    ("Both", "Вместе"),
    ("All time", "Всего"),
    ("Busiest engine", "Самый занятый движок"),
    ("Disk total (PDH)", "Всего по диску (PDH)"),
    ("Total (PDH)", "Всего по сети (PDH)"),
    ("Moment", "Момент"),
    ("No samples", "Нет измерений"),
    ("Awaiting ETW attribution", "Ожидание атрибуции ETW"),
    ("Collecting history", "История накапливается"),
    (
        "Collected {a} of 5:00. Left of the data is not zero but missing samples.",
        "Собрано {a} из 5:00. Слева от данных — не ноль, а отсутствие измерений.",
    ),
    ("no samples · {d}", "нет измерений · {d}"),
    ("above scale, peak {v}", "выше шкалы, пик {v}"),
    ("per-process · ETW ~2 s", "по процессам · ETW ~2 с"),
    ("shared memory not measured", "общая память не измерялась"),
    (
        "Window closed: no shared memory",
        "Окно было закрыто: без общей памяти",
    ),
    ("no data", "нет данных"),
    ("Pin", "Закрепить"),
    ("Unpin", "Открепить"),
    ("Up to 6 pinned", "Не больше 6 закреплённых"),
    ("No data at this moment", "Нет данных за этот момент"),
    ("Timeline", "Временная шкала"),
    ("Settings", "Настройки"),
    ("Menu", "Меню"),
    ("Disk", "Диск"),
    ("Network", "Сеть"),
    ("Live", "В реальном времени"),
    ("Processes", "Процессы"),
    ("Process", "Процесс"),
    ("now · updates 1×/s", "сейчас · обновление 1 раз/с"),
    ("as of {t}", "на {t}"),
    ("as of {t} · 2 s behind live", "на {t} · 2 с от реального времени"),
    ("Search by name or PID", "Поиск по имени или PID"),
    ("{a} of {b}", "{a} из {b}"),
    (
        "No matches · pinned stay visible",
        "Ничего не найдено · закреплённые видны всегда",
    ),
    ("Top 10", "Топ-10"),
    ("Test", "Тестовые"),
    ("Clear", "Очистить"),
    ("discrete", "дискретная"),
    ("Show on taskbar", "Отображать на taskbar"),
    ("Always monitor", "Мониторить всегда"),
    (
        "Keep the history of this device while the window is closed",
        "История устройства пишется и при закрытом окне",
    ),
    ("integrated", "встроенная"),
    ("Dedicated memory", "Выделенная память"),
    ("Live values", "Живые"),
    ("above the upper threshold: pulsing", "выше верхнего: пульсация"),
    ("between thresholds: the tile turns red", "между порогами: плитка краснеет"),
    ("Other processes", "Остальные процессы"),
    ("Process monitoring is off", "Мониторинг процессов выключен"),
    (
        "Totals are still collected. Turn on background process history to see which processes used resources at any moment of the last 5 minutes.",
        "Общие графики продолжают собираться. Чтобы видеть, какие процессы потребляли ресурсы в любой момент последних 5 минут, включите фоновую историю процессов.",
    ),
    ("Turn on monitoring", "Включить мониторинг"),
    ("Open settings", "Открыть настройки"),
    (
        "Pins are kept and return when enabled",
        "Закрепления сохранены и вернутся после включения",
    ),
    ("Waiting for history…", "Ожидание истории…"),
    (
        "Recorder unavailable. Enable background monitoring in Settings.",
        "Сборщик недоступен. Включите фоновый мониторинг в настройках.",
    ),
    (
        "Total load is the PDH counter “% Processor Utility”; process time is CPU time over the interval divided by the number of logical processors. The process sum can differ from the total load.",
        "Общая загрузка — счётчик PDH «% Processor Utility»; время процессов — CPU time за интервал, делённое на число логических процессоров. Сумма процессов может не совпадать с общей загрузкой.",
    ),
    (
        "Processes are sampled every 0.5 s: a short one that starts and exits between samples is not in the table, though the total load counts it.",
        "Процессы опрашиваются раз в 0.5 с: короткий процесс, который запустился и завершился между опросами, в таблицу не попадёт, хотя в общей загрузке он учтён.",
    ),
    (
        "Temperature is the package sensor; the line is capped at 100 °C.",
        "Температура — датчик пакета, линия ограничена 100 °C.",
    ),
    (
        "The total line is the busiest physical engine at each sample. Process percentages are their share of exactly that engine.",
        "Общая линия = самый загруженный физический движок в каждом замере. Проценты процессов — их вклад именно в этот движок.",
    ),
    (
        "Totals come from PDH counters; per-process traffic comes from ETW. The two can differ slightly.",
        "Итоги — PDH, по процессам — ETW; значения могут немного различаться.",
    ),
    (
        "ETW delivers events with a delay, so live per-process values trail the total by about 2 s (hatched strip).",
        "ETW доставляет события с задержкой, поэтому в live значения процессов отстают примерно на 2 с (заштрихованная полоса).",
    ),
    (
        "Read is drawn above the axis, write below it, on the same scale.",
        "Чтение — над осью, запись — под осью, шкала общая.",
    ),
    (
        "Receive is drawn above the axis, send below it, on the same scale.",
        "Приём — над осью, отправка — под осью, шкала общая.",
    ),
    (
        "Counter overshoot: contributions scaled proportionally to 100%.",
        "Счётчики превысили 100%: вклады пропорционально приведены к 100%.",
    ),
    (
        "I/O attribution unavailable or incomplete",
        "Атрибуция ввода-вывода недоступна или неполна",
    ),
    ("Language", "Язык"),
    ("System", "Как в системе"),
    ("Light", "Светлая"),
    ("Dark", "Тёмная"),
    ("Apply to taskbar", "Применить к таскбару"),
    ("Applied", "Применено"),
    ("Save failed", "Не удалось сохранить"),
    // Settings page.
    ("General", "Общие"),
    ("Taskbar", "Панель задач"),
    ("Widget preview", "Превью виджетов"),
    ("100 % scale, both themes", "масштаб 100 %, обе темы"),
    ("Preview values", "Значения превью"),
    ("used only in the preview", "подставляются только в превью"),
    ("Revert changes", "Отменить изменения"),
    (
        "“Apply to taskbar” saves the tile appearance; the taskbar picks it up within a second.",
        "«Применить к таскбару» сохраняет вид плиток, панель задач подхватит его за секунду.",
    ),
    (
        "“Revert changes” returns the tile settings to the last applied ones.",
        "«Отменить изменения» возвращает настройки плиток к последним применённым.",
    ),
    ("14 elements · 2 themes", "14 элементов · 2 темы"),
    ("changed: {n}", "изменено: {n}"),
    ("Taskbar widgets", "Виджеты на панели задач"),
    ("Alerts", "Алерты"),
    ("Sizes and spacing", "Размеры и промежутки"),
    (
        "Tile width, spacing, corner radius",
        "Ширина плиток, отступы, скругление",
    ),
    ("Typography", "Типографика"),
    (
        "Header, main value, temperature / upload",
        "Заголовок, основное число, температура / отдача",
    ),
    ("Tile charts", "Графики в плитках"),
    (
        "Line width, area opacity, fade, temperature",
        "Толщина линии, прозрачность заливки, затухание, температура",
    ),
    ("Colors", "Цвета"),
    (
        "Separately for light and dark themes, HEX with alpha",
        "Отдельно для светлой и тёмной темы, HEX с альфой",
    ),
    ("Temperature and memory", "Температура и память"),
    (
        "Between the thresholds the tile turns red; above the upper one it pulses",
        "Между порогами плитка постепенно краснеет, выше верхнего — пульсирует",
    ),
    ("Number color", "Цвет чисел"),
    (
        "Between the thresholds the tile number gradually turns red",
        "Между порогами число на плитке постепенно краснеет",
    ),
    ("CPU / GPU width", "Ширина CPU / GPU"),
    ("Tile spacing", "Промежуток между плитками"),
    ("Corner radius", "Скругление углов"),
    ("Header size", "Размер заголовка"),
    ("Main value size", "Размер основного числа"),
    ("Temperature / upload size", "Размер температуры / upload"),
    ("Line width", "Толщина графика"),
    ("Area opacity", "Непрозрачность заливки"),
    ("Value X offset", "Смещение чисел по X"),
    ("Value Y offset", "Смещение чисел по Y"),
    ("Chart left inset", "Начало графика слева"),
    ("Fade start from text edge", "Начало затухания от края текста"),
    ("Fade length", "Длина затухания"),
    ("Chart visibility on the left", "Видимость графика слева"),
    ("CPU: red tint starts at X", "CPU: начало покраснения X"),
    ("CPU: pulse starts at Y", "CPU: пульсация от Y"),
    ("GPU: red tint starts at X", "GPU: начало покраснения X"),
    ("GPU: pulse starts at Y", "GPU: пульсация от Y"),
    ("RAM: red tint starts at X", "RAM: начало покраснения X"),
    ("RAM: pulse starts at Y", "RAM: пульсация от Y"),
    ("Pulse intensity", "Интенсивность пульсации"),
    ("Pulse period", "Период пульсации"),
    ("s", "с"),
    ("Preview: CPU temperature", "Тест: температура CPU"),
    ("Preview: GPU temperature", "Тест: температура GPU"),
    ("Preview: RAM usage", "Тест: загрузка RAM"),
    ("Preview: CPU usage", "Тест: загрузка CPU"),
    ("Preview: GPU usage", "Тест: загрузка GPU"),
    ("Preview: DISK read", "Тест: чтение DISK"),
    ("Preview: DISK write", "Тест: запись DISK"),
    ("Preview: NET receive", "Тест: загрузка NET"),
    ("Preview: NET send", "Тест: отдача NET"),
    ("CPU: number starts turning red at X", "CPU: число краснеет от X"),
    ("CPU: number is fully red at Y", "CPU: число красное от Y"),
    ("GPU: number starts turning red at X", "GPU: число краснеет от X"),
    ("GPU: number is fully red at Y", "GPU: число красное от Y"),
    ("RAM: number starts turning red at X", "RAM: число краснеет от X"),
    ("RAM: number is fully red at Y", "RAM: число красное от Y"),
    ("Dashed temperature line", "Пунктирная линия температуры"),
    ("CPU temperature on the tile", "Температура CPU в плитке"),
    ("GPU temperature on the tile", "Температура GPU в плитке"),
    ("CPU temperature", "CPU, температура"),
    ("GPU temperature", "GPU, температура"),
    ("CPU load", "CPU, загрузка"),
    ("GPU load", "GPU, загрузка"),
    ("Element", "Элемент"),
    ("Light theme", "Светлая тема"),
    ("Dark theme", "Тёмная тема"),
    ("light theme", "светлая тема"),
    ("dark theme", "тёмная тема"),
    ("Tile", "Плитка"),
    ("Charts and directions", "Графики и направления"),
    ("States and background", "Состояния и фон"),
    ("Tile background", "Фон плитки"),
    ("Header", "Заголовок"),
    ("Main value", "Основное число"),
    ("High usage value", "Число при высокой нагрузке"),
    ("Temperature value", "Число температуры"),
    ("Download arrow", "Загрузка / чтение: стрелка"),
    ("Upload arrow and value", "Отдача / запись: стрелка и число"),
    ("Primary chart line", "Основная линия графика"),
    ("Chart area", "Заливка графика"),
    ("Temperature line", "Линия температуры"),
    ("Upload line", "Линия отдачи / записи"),
    ("Hover background", "Фон при наведении"),
    ("Pressed background", "Фон при нажатии"),
    ("Background behind widgets", "Фон под виджетами"),
    ("Choose color", "Выбрать цвет"),
    (
        "Format #RRGGBB or #AARRGGBB, where AA is opacity.",
        "Формат #RRGGBB или #AARRGGBB, где AA — непрозрачность.",
    ),
    (
        "Invalid or incomplete HEX: the last valid color is used for this field.",
        "Неверный или неполный HEX: для этого поля используется последний корректный цвет.",
    ),
    ("Reset colors", "Сбросить цвета"),
    ("Alpha", "Альфа"),
    ("Before / after", "Было / стало"),
    ("Done", "Готово"),
    ("Cancel", "Отмена"),
    ("Startup and window", "Запуск и окно"),
    ("Monitors", "Мониторы"),
    ("Taskbar tiles", "Плитки на панели задач"),
    (
        "If none of the chosen monitors is connected, the tiles show on the main one",
        "Если ни один из выбранных мониторов не подключён, плитки появятся на основном",
    ),
    (
        "At least one monitor shows the tiles",
        "Хотя бы один монитор должен показывать плитки",
    ),
    ("Built-in display", "Встроенный экран"),
    ("Monitor", "Монитор"),
    ("main", "основной"),
    ("Start when signing in to Windows", "Запускать при входе в Windows"),
    (
        "Taskbar tiles and the CPU temperature sensor",
        "Плитки на панели задач и датчик температуры CPU",
    ),
    (
        "Reset the window size and position when it closes",
        "Сбрасывать размер и положение окна при закрытии",
    ),
    (
        "The window opens in the middle of the main monitor at 1280 × 1024",
        "Окно открывается по центру основного монитора в размере 1280 × 1024",
    ),
    (
        "The window opens where it was closed",
        "Окно открывается там, где его закрыли",
    ),
    ("Monitoring", "Мониторинг"),
    ("Background process history", "Фоновая история процессов"),
    (
        "Collect process usage while the window is closed. 5 min at 0.5 s.",
        "Собирать потребление процессов, пока окно закрыто. 5 мин, шаг 0,5 с.",
    ),
    ("On", "Вкл."),
    ("Off", "Выкл."),
    (
        "Background monitoring enabled. History: 5 minutes at 500 ms intervals.",
        "Фоновый сбор включён. История: 5 минут, шаг: 0,5 с.",
    ),
    (
        "Background process monitoring disabled. Taskbar metrics continue to work.",
        "Фоновый сбор процессов отключён. Метрики таскбара продолжают работать.",
    ),
    ("Could not save setting", "Не удалось сохранить настройку"),
    ("Window appearance", "Вид окна"),
    ("Window theme", "Тема окна"),
    (
        "“System” follows the Windows app theme",
        "«Как в системе» следует за темой приложений Windows",
    ),
    ("Chart line width", "Толщина линии графика"),
    (
        "Total load; pinned processes in proportion",
        "Общая загрузка; закреплённые процессы — пропорционально",
    ),
    ("Display language", "Язык интерфейса"),
    (
        "Applies immediately, no restart needed",
        "Применяется сразу, без перезапуска",
    ),
];
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn russian_is_suggested_by_the_display_language_or_a_keyboard_layout() {
        // en-US display; US and Russian (0x04190419) layouts.
        assert!(SystemLanguage::any_russian(
            0x0409,
            &[0x04090409, 0x04190419]
        ));
        assert!(SystemLanguage::any_russian(0x0419, &[0x04090409]));
        assert!(!SystemLanguage::any_russian(
            0x0409,
            &[0x04090409, 0x04070407]
        ));
    }
    #[test]
    fn translation_keys_are_unique_and_complete() {
        let mut keys = std::collections::HashSet::new();
        for (en, ru) in CATALOG {
            assert!(!en.is_empty() && !ru.is_empty());
            assert!(keys.insert(en));
        }
    }
    #[test]
    fn numbers_follow_locale_grouping_and_decimal_marks() {
        assert_eq!(Language::Russian.number(3431.034, 2), "3\u{202F}431.03");
        assert_eq!(Language::English.number(3431.034, 2), "3,431.03");
        assert_eq!(Language::Russian.percent(37.26, 1), "37.3\u{A0}%");
        assert_eq!(Language::English.number(-0.001, 1), "0.0");
        assert_eq!(Language::Russian.celsius(105.4), "105 °C");
        assert_eq!(Language::Russian.span(266), "4 мин 26 с");
        assert_eq!(Language::English.ago(38), "38 s ago");
        assert_eq!(Language::Russian.elapsed(42), "0:42");
    }
    #[test]
    fn pending_changes_and_rules_use_plural_forms() {
        assert_eq!(
            Language::Russian.unapplied(3),
            "3 изменения не применены к таскбару"
        );
        assert_eq!(
            Language::Russian.unapplied(1),
            "1 изменение не применено к таскбару"
        );
        assert_eq!(
            Language::Russian.unapplied(11),
            "11 изменений не применено к таскбару"
        );
        assert_eq!(
            Language::English.unapplied(1),
            "1 change not applied to the taskbar"
        );
        assert_eq!(Language::English.rules(3), "3 rules");
    }
    #[test]
    fn units_are_marked_only_when_they_read_differently() {
        assert_eq!(Language::marked("s"), "@s@");
        assert_eq!(Language::marked("px"), "px");
    }
    #[test]
    fn core_and_thread_counts_use_plural_forms() {
        assert_eq!(Language::Russian.cores(8), "8 ядер");
        assert_eq!(Language::Russian.cores(2), "2 ядра");
        assert_eq!(Language::Russian.cores(21), "21 ядро");
        assert_eq!(Language::Russian.threads(12), "12 потоков");
        assert_eq!(Language::Russian.threads(24), "24 потока");
        assert_eq!(Language::English.cores(1), "1 core");
        assert_eq!(Language::English.threads(16), "16 threads");
    }
}
