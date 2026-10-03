//! Device instances behind each resource: several GPUs, disks and network adapters.
//! A device id is `kind` or `kind@tag` (`disk@C:`, `net@Wi‑Fi`, `gpu@1`). A bare kind
//! names the main device of that kind, so existing configurations keep working.
use super::{gpu_temperature::GpuAdapters, pdh::PdhCounter};
use std::{
    collections::HashMap,
    net::IpAddr,
    ptr,
    sync::Arc,
    time::{Duration, Instant, SystemTime},
};

#[derive(Clone, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
pub struct DeviceId {
    pub kind: String,
    pub tag: Option<String>,
}
impl DeviceId {
    pub const KINDS: [&'static str; 5] = ["cpu", "gpu", "ram", "disk", "net"];
    /// `kind` or `kind@tag`; tags never contain the separators of the config lists.
    pub fn parse(text: &str) -> Option<Self> {
        let (kind, tag) = match text.split_once('@') {
            Some((kind, tag)) => (kind, Some(tag)),
            None => (text, None),
        };
        let valid_tag = |tag: &str| !tag.is_empty() && !tag.contains(Self::RESERVED);
        (Self::KINDS.contains(&kind) && tag.is_none_or(valid_tag)).then(|| Self {
            kind: kind.into(),
            tag: tag.map(Into::into),
        })
    }
    pub fn new(kind: &str, tag: Option<String>) -> Self {
        Self {
            kind: kind.into(),
            tag,
        }
    }
    /// Id of one of the device's readings: `disk_read` → `disk_read@C:`.
    pub fn reading(&self, base: &str) -> String {
        match &self.tag {
            Some(tag) => format!("{base}@{tag}"),
            None => base.into(),
        }
    }
    /// `id` equals `reading(base)`, compared without allocating.
    pub fn is_reading(&self, id: &str, base: &str) -> bool {
        match (&self.tag, id.strip_prefix(base)) {
            (None, Some(rest)) => rest.is_empty(),
            (Some(tag), Some(rest)) => rest.strip_prefix('@') == Some(tag.as_str()),
            (_, None) => false,
        }
    }
    /// The number a numbered device (`gpu@0`, `disk@2`) is shown with: people count
    /// from 1, while tags keep Windows' numbers, which count from 0.
    pub fn ordinal(&self) -> Option<u32> {
        Some(self.tag.as_deref()?.parse::<u32>().ok()? + 1)
    }
    /// `id` equals `to_string()`, compared without allocating.
    pub fn is(&self, id: &str) -> bool {
        self.is_reading(id, &self.kind)
    }
    /// List separator, id separator, config comment start and line breaks.
    const RESERVED: [char; 5] = [',', '@', '#', '\n', '\r'];
    /// Replaces characters that cannot appear in a tag.
    fn tag_of(text: &str) -> String {
        text.replace(Self::RESERVED, " ").trim().to_owned()
    }
}
impl std::fmt::Display for DeviceId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match &self.tag {
            Some(tag) => write!(f, "{}@{tag}", self.kind),
            None => f.write_str(&self.kind),
        }
    }
}

/// `\PhysicalDisk(*)` instances are named `0 C: D:`; a disk is tagged by its first
/// drive letter, or by its number when it has no volumes.
pub(crate) struct DiskInstance;
impl DiskInstance {
    pub fn tag(instance: &str) -> Option<String> {
        let mut parts = instance.split_whitespace();
        let number = parts.next().filter(|n| n.parse::<u32>().is_ok())?;
        Some(
            parts
                .find(|p| p.len() == 2 && p.ends_with(':'))
                .unwrap_or(number)
                .to_owned(),
        )
    }
    /// Disk number of an instance (`0 C:` → 0), as in kernel disk I/O events.
    pub fn number(instance: &str) -> Option<u32> {
        instance.split_whitespace().next()?.parse().ok()
    }
    pub fn matches(instance: &str, tag: &str) -> bool {
        let mut parts = instance.split_whitespace();
        parts.next() == Some(tag) || parts.any(|p| p.eq_ignore_ascii_case(tag))
    }
    /// The disk holding Windows.
    pub fn main_tag() -> String {
        std::env::var("SystemDrive").unwrap_or_else(|_| "C:".into())
    }
}

/// A hardware network adapter that is up.
#[derive(Clone, Debug, PartialEq)]
pub(crate) struct NetworkAdapter {
    /// Name shown in Windows settings: «Wi‑Fi», «Ethernet 2».
    pub name: String,
    pub description: String,
    pub kind: &'static str,
    pub gateway: bool,
    /// Local unicast addresses: kernel network events name them, not the adapter.
    pub addresses: Vec<IpAddr>,
}
impl NetworkAdapter {
    pub fn tag(&self) -> String {
        DeviceId::tag_of(&self.name)
    }
    /// `\Network Interface(*)` instance: the description with the characters PDH
    /// reserves for instance syntax replaced.
    pub fn instance(&self) -> String {
        self.description
            .chars()
            .map(|c| match c {
                '(' => '[',
                ')' => ']',
                '#' | '/' | '\\' => '_',
                c => c,
            })
            .collect()
    }
    /// The adapter with a default gateway: the one carrying the internet traffic.
    pub fn main(adapters: &[Self]) -> Option<&Self> {
        adapters
            .iter()
            .find(|a| a.gateway)
            .or_else(|| adapters.first())
    }
    pub fn find<'a>(adapters: &'a [Self], id: &DeviceId) -> Option<&'a Self> {
        match &id.tag {
            Some(tag) => adapters.iter().find(|a| &a.tag() == tag),
            None => Self::main(adapters),
        }
    }
    /// Ethernet, Wi‑Fi and WWAN adapters that are up and backed by hardware, so
    /// Hyper‑V, VPN and other virtual adapters stay out of the list.
    pub fn list() -> Vec<Self> {
        // GAA_FLAG_SKIP_ANYCAST | SKIP_MULTICAST | SKIP_DNS_SERVER | INCLUDE_GATEWAYS.
        const FLAGS: u32 = 0x0002 | 0x0004 | 0x0008 | 0x0080;
        let mut size = 16 * 1024u32;
        let mut buffer = Vec::<u64>::new();
        for _ in 0..3 {
            buffer = vec![0u64; (size as usize).div_ceil(8)];
            // SAFETY: the buffer holds `size` bytes, aligned for the adapter records.
            let status = unsafe {
                GetAdaptersAddresses(
                    0,
                    FLAGS,
                    ptr::null_mut(),
                    buffer.as_mut_ptr().cast(),
                    &mut size,
                )
            };
            match status {
                0 => break,
                111 => continue,
                _ => return Vec::new(),
            }
        }
        let mut adapters = Vec::new();
        let mut adapter = buffer.as_ptr().cast::<u8>();
        // IP_ADAPTER_ADDRESSES_LH (x64): IfIndex +4, Next +8, FirstUnicastAddress +24,
        // Description +64, FriendlyName +72, IfType +100, OperStatus +104,
        // FirstGatewayAddress +208.
        while !adapter.is_null() {
            // SAFETY: records form a list inside `buffer` written by the call above.
            let (index, next, unicast, description, name, kind, status, gateway) = unsafe {
                (
                    *adapter.add(4).cast::<u32>(),
                    *adapter.add(8).cast::<*const u8>(),
                    *adapter.add(24).cast::<*const u8>(),
                    *adapter.add(64).cast::<*const u16>(),
                    *adapter.add(72).cast::<*const u16>(),
                    *adapter.add(100).cast::<u32>(),
                    *adapter.add(104).cast::<u32>(),
                    *adapter.add(208).cast::<*const u8>(),
                )
            };
            let kind = match kind {
                71 => Some("Wi‑Fi"),
                6 => Some("Ethernet"),
                243 | 244 => Some("WWAN"),
                _ => None,
            };
            if let (Some(kind), 1, true) = (kind, status, Self::hardware(index)) {
                adapters.push(Self {
                    name: Self::text(name),
                    description: Self::text(description),
                    kind,
                    gateway: !gateway.is_null(),
                    addresses: Self::addresses(unicast),
                });
            }
            adapter = next;
        }
        adapters
    }
    /// IP_ADAPTER_UNICAST_ADDRESS (x64): Next +8, Address.lpSockaddr +16; the sockaddr
    /// holds the family at +0, an IPv4 address at +4 or an IPv6 address at +8.
    fn addresses(mut unicast: *const u8) -> Vec<IpAddr> {
        let mut addresses = Vec::new();
        while !unicast.is_null() {
            // SAFETY: the list and its sockaddrs live in the GetAdaptersAddresses buffer.
            unsafe {
                let address = *unicast.add(16).cast::<*const u8>();
                if !address.is_null() {
                    match *address.cast::<u16>() {
                        2 => addresses.push(IpAddr::from(*address.add(4).cast::<[u8; 4]>())),
                        23 => addresses.push(IpAddr::from(*address.add(8).cast::<[u8; 16]>())),
                        _ => {}
                    }
                }
                unicast = *unicast.add(8).cast::<*const u8>();
            }
        }
        addresses
    }
    /// `MIB_IF_ROW2.InterfaceAndOperStatusFlags.HardwareInterface`.
    fn hardware(index: u32) -> bool {
        // MIB_IF_ROW2 is 1352 bytes: InterfaceIndex at +8, the flag bits at +1152.
        let mut row = [0u64; 1352 / 8];
        // SAFETY: the buffer has the size and alignment of MIB_IF_ROW2.
        unsafe {
            *row.as_mut_ptr().cast::<u8>().add(8).cast::<u32>() = index;
            GetIfEntry2(row.as_mut_ptr().cast()) == 0
                && *row.as_ptr().cast::<u8>().add(1152) & 1 != 0
        }
    }
    fn text(pointer: *const u16) -> String {
        if pointer.is_null() {
            return String::new();
        }
        // SAFETY: GetAdaptersAddresses strings are NUL terminated inside its buffer.
        unsafe {
            let length = (0..).take_while(|&i| *pointer.add(i) != 0).count();
            String::from_utf16_lossy(std::slice::from_raw_parts(pointer, length))
        }
    }
}

/// Where kernel I/O events belong: disk numbers and adapters' local addresses,
/// mapped to device ids (`disk@C:`, `net@Wi‑Fi`).
#[derive(Default, Clone, Debug)]
pub struct IoDevices {
    disks: HashMap<u32, Arc<str>>,
    addresses: HashMap<IpAddr, Arc<str>>,
}
impl IoDevices {
    /// `disks` is a wildcard `\PhysicalDisk(*)` counter owned by the caller.
    pub(crate) fn current(disks: &mut PdhCounter) -> Self {
        let disks = disks
            .entries()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(instance, _)| {
                let tag = DiskInstance::tag(&instance)?;
                Some((
                    DiskInstance::number(&instance)?,
                    DeviceId::new("disk", Some(tag)).to_string().into(),
                ))
            })
            .collect();
        let addresses = NetworkAdapter::list()
            .into_iter()
            .flat_map(|adapter| {
                let id: Arc<str> = DeviceId::new("net", Some(adapter.tag())).to_string().into();
                adapter
                    .addresses
                    .into_iter()
                    .map(move |address| (address, id.clone()))
            })
            .collect();
        Self { disks, addresses }
    }
    pub fn disk(&self, number: u32) -> Option<&Arc<str>> {
        self.disks.get(&number)
    }
    /// The adapter owning either end of a connection; loopback traffic has none.
    pub fn adapter(&self, local: &IpAddr, remote: &IpAddr) -> Option<&Arc<str>> {
        self.addresses
            .get(local)
            .or_else(|| self.addresses.get(remote))
    }
}

/// What names a physical disk, like «GPU» names an adapter: whether it is an SSD or
/// HDD; `volumes` are its drive letters (`C:`), in the order Windows lists them.
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct DiskName {
    pub media: Option<&'static str>,
    pub volumes: Vec<String>,
}
impl DiskName {
    /// «C:, D:»: the disk's volumes; empty without any. No root backslash: in
    /// Segoe UI it hangs below the line, next to a colon on it.
    pub fn letters(&self) -> String {
        self.volumes.join(", ")
    }
}

/// Shown numbers of devices by group (GPUs, SSDs, HDDs): from 1 in the given order,
/// counted separately for each group; the only device of its group has none.
pub struct Ordinals;
impl Ordinals {
    pub fn of<G: Eq + std::hash::Hash>(groups: &[G]) -> Vec<Option<u32>> {
        let mut sizes: HashMap<&G, u32> = HashMap::new();
        for group in groups {
            *sizes.entry(group).or_default() += 1;
        }
        let mut counts: HashMap<&G, u32> = HashMap::new();
        groups
            .iter()
            .map(|group| {
                let count = counts.entry(group).or_default();
                *count += 1;
                (sizes[group] > 1).then_some(*count)
            })
            .collect()
    }
}

/// Every device of the machine, refreshed by the caller at its own pace.
pub struct DeviceCatalog {
    disks: PdhCounter,
    /// Disk names by tag, as of the last `devices`.
    names: HashMap<String, DiskName>,
    /// Media by disk number: asked once, it does not change.
    media: HashMap<u32, Option<&'static str>>,
    /// Link of each network adapter by tag, as of the last `devices`: «Wi‑Fi», «Ethernet».
    links: HashMap<String, &'static str>,
}
impl Default for DeviceCatalog {
    fn default() -> Self {
        Self::new()
    }
}
impl DeviceCatalog {
    pub fn new() -> Self {
        let mut disks = PdhCounter::new(r"\PhysicalDisk(*)\Current Disk Queue Length");
        // Primes the query: instance names are listed from the second collection on.
        let _ = disks.entries();
        Self {
            disks,
            names: HashMap::new(),
            media: HashMap::new(),
            links: HashMap::new(),
        }
    }
    /// Link of a network adapter listed by the last `devices`.
    pub fn link(&self, tag: &str) -> Option<&'static str> {
        self.links.get(tag).copied()
    }
    /// Media and volumes of a disk listed by the last `devices`.
    pub fn disk(&self, tag: &str) -> Option<&DiskName> {
        self.names.get(tag)
    }
    fn name(&mut self, instance: &str) -> DiskName {
        let media = DiskInstance::number(instance).and_then(|number| {
            *self
                .media
                .entry(number)
                .or_insert_with(|| super::hardware::DiskMedia::of(number))
        });
        DiskName {
            media,
            volumes: instance
                .split_whitespace()
                .skip(1)
                .map(Into::into)
                .collect(),
        }
    }
    /// CPU, GPUs, memory, disks and network adapters, in the order of the menu.
    pub fn devices(&mut self) -> Vec<DeviceId> {
        let mut devices = vec![DeviceId::new("cpu", None)];
        devices.extend(
            (0..GpuAdapters::list().len()).map(|i| DeviceId::new("gpu", Some(i.to_string()))),
        );
        devices.push(DeviceId::new("ram", None));
        let mut disks: Vec<_> = self
            .disks
            .entries()
            .unwrap_or_default()
            .into_iter()
            .filter_map(|(instance, _)| Some((instance.clone(), DiskInstance::tag(&instance)?)))
            .collect();
        // Windows' disk order: «10 X:» after «2 Y:».
        disks.sort_by_key(|(instance, _)| DiskInstance::number(instance));
        if disks.is_empty() {
            disks.push((String::new(), DiskInstance::main_tag()));
        }
        let names: Vec<_> = disks
            .iter()
            .map(|(instance, tag)| (tag.clone(), self.name(instance)))
            .collect();
        self.names = names.into_iter().collect();
        devices.extend(
            disks
                .into_iter()
                .map(|(_, tag)| DeviceId::new("disk", Some(tag))),
        );
        let adapters = NetworkAdapter::list();
        self.links = adapters.iter().map(|a| (a.tag(), a.kind)).collect();
        devices.extend(adapters.iter().map(|a| DeviceId::new("net", Some(a.tag()))));
        devices
    }
    /// The device a bare kind stands for: system disk, internet adapter, discrete GPU.
    pub fn canonical(id: &DeviceId) -> DeviceId {
        if id.tag.is_some() {
            return id.clone();
        }
        let tag = match id.kind.as_str() {
            "disk" => Some(DiskInstance::main_tag()),
            "gpu" => Some(GpuAdapters::main().to_string()),
            "net" => NetworkAdapter::main(&NetworkAdapter::list()).map(NetworkAdapter::tag),
            _ => None,
        };
        DeviceId::new(&id.kind, tag)
    }
}

/// Devices an open main window shows: the recorder collects them while the list
/// is fresh, so their history starts when the window opens.
pub struct WatchList;
impl WatchList {
    const FRESH: Duration = Duration::from_secs(5);
    pub fn write(path: &std::path::Path, devices: &[DeviceId]) -> std::io::Result<()> {
        let text: String = devices.iter().map(|d| format!("{d}\n")).collect();
        let temporary = path.with_extension("watch.tmp");
        std::fs::write(&temporary, text)?;
        std::fs::rename(temporary, path)
    }
    pub fn read(path: &std::path::Path) -> Vec<DeviceId> {
        let fresh = std::fs::metadata(path)
            .and_then(|m| m.modified())
            .ok()
            .and_then(|modified| SystemTime::now().duration_since(modified).ok())
            .is_some_and(|age| age < Self::FRESH);
        if !fresh {
            return Vec::new();
        }
        std::fs::read_to_string(path)
            .unwrap_or_default()
            .lines()
            .filter_map(DeviceId::parse)
            .take(64)
            .collect()
    }
}

/// Re-resolves a device to its current instance at most every 30 s: adapters come and
/// go, and GPU LUIDs change between boots.
pub(crate) struct Binding<T> {
    pub id: DeviceId,
    value: Option<T>,
    next: Instant,
}
impl<T: Clone> Binding<T> {
    pub fn new(id: DeviceId) -> Self {
        Self {
            id,
            value: None,
            next: Instant::now(),
        }
    }
    pub fn get(&mut self, resolve: impl FnOnce(&DeviceId) -> Option<T>) -> Option<T> {
        if Instant::now() >= self.next {
            self.next = Instant::now() + Duration::from_secs(30);
            self.value = resolve(&self.id);
        }
        self.value.clone()
    }
}

#[link(name = "iphlpapi")]
extern "system" {
    fn GetAdaptersAddresses(
        family: u32,
        flags: u32,
        reserved: *mut core::ffi::c_void,
        addresses: *mut core::ffi::c_void,
        size: *mut u32,
    ) -> u32;
    fn GetIfEntry2(row: *mut core::ffi::c_void) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ids_parse_with_optional_tags_and_name_readings() {
        let disk = DeviceId::parse("disk@D:").unwrap();
        assert_eq!(disk.reading("disk_read"), "disk_read@D:");
        assert_eq!(disk.to_string(), "disk@D:");
        assert_eq!(DeviceId::parse("cpu").unwrap().reading("cpu"), "cpu");
        for invalid in ["", "fan", "disk@", "net@a,b", "disk@C:@x", "net@a#b"] {
            assert_eq!(DeviceId::parse(invalid), None, "{invalid}");
        }
        let cpu = DeviceId::parse("cpu").unwrap();
        for id in [
            "disk_read@D:",
            "disk@D:",
            "disk_read",
            "disk_read@D",
            "disk_read@D:x",
            "disk_write@D:",
            "cpu",
            "cpu_temperature",
            "",
        ] {
            for base in ["disk_read", "disk", "cpu"] {
                assert_eq!(disk.is_reading(id, base), disk.reading(base) == id);
                assert_eq!(cpu.is_reading(id, base), cpu.reading(base) == id);
            }
            assert_eq!(disk.is(id), disk.to_string() == id);
            assert_eq!(cpu.is(id), cpu.to_string() == id);
        }
    }
    #[test]
    fn disks_are_tagged_by_first_letter_or_number() {
        assert_eq!(DiskInstance::tag("0 C: D:").as_deref(), Some("C:"));
        assert_eq!(DiskInstance::tag("2").as_deref(), Some("2"));
        assert_eq!(DiskInstance::tag("_Total"), None);
        assert!(DiskInstance::matches("1 D: E:", "E:"));
        assert!(DiskInstance::matches("2", "2"));
        assert!(!DiskInstance::matches("12 F:", "1"));
    }
    #[test]
    fn adapters_map_to_pdh_instances_and_the_gateway_one_is_main() {
        let adapter = |name: &str, description: &str, gateway| NetworkAdapter {
            name: name.into(),
            description: description.into(),
            kind: "Ethernet",
            gateway,
            addresses: Vec::new(),
        };
        let adapters = [
            adapter("Ethernet 2", "Intel(R) Ethernet #2", false),
            adapter("Wi‑Fi", "Intel(R) Wi-Fi 6 AX201 160MHz", true),
        ];
        assert_eq!(adapters[0].instance(), "Intel[R] Ethernet _2");
        assert_eq!(NetworkAdapter::main(&adapters).unwrap().name, "Wi‑Fi");
        let id = DeviceId::parse("net@Ethernet 2").unwrap();
        assert_eq!(
            NetworkAdapter::find(&adapters, &id).unwrap().description,
            "Intel(R) Ethernet #2"
        );
    }
    #[test]
    fn disks_list_all_their_volumes() {
        let mut catalog = DeviceCatalog::new();
        let name = |catalog: &mut DeviceCatalog, instance| catalog.name(instance).letters();
        assert_eq!(name(&mut catalog, "1 D: E:"), "D:, E:");
        assert_eq!(name(&mut catalog, "2"), "");
    }
    #[test]
    fn devices_are_numbered_within_their_group_unless_alone() {
        assert_eq!(
            Ordinals::of(&["SSD", "HDD", "SSD", "?", "HDD", "?"]),
            [Some(1), Some(1), Some(2), Some(1), Some(2), Some(2)]
        );
        assert_eq!(
            Ordinals::of(&["SSD", "HDD", "SSD"]),
            [Some(1), None, Some(2)]
        );
    }
    #[test]
    fn io_events_map_to_disks_and_to_the_adapter_of_either_address() {
        let wifi: Arc<str> = "net@Wi‑Fi".into();
        let devices = IoDevices {
            disks: HashMap::from([(1, "disk@D:".into())]),
            addresses: HashMap::from([("192.168.1.5".parse().unwrap(), wifi.clone())]),
        };
        let remote: IpAddr = "93.184.216.34".parse().unwrap();
        let local: IpAddr = "192.168.1.5".parse().unwrap();
        assert_eq!(devices.disk(1).map(|d| &**d), Some("disk@D:"));
        assert_eq!(devices.adapter(&local, &remote), Some(&wifi));
        assert_eq!(devices.adapter(&remote, &local), Some(&wifi));
        let loopback: IpAddr = "127.0.0.1".parse().unwrap();
        assert_eq!(devices.adapter(&loopback, &loopback), None);
        assert_eq!(DiskInstance::number("1 D: E:"), Some(1));
    }
    #[test]
    fn watch_list_is_ignored_once_stale() {
        let dir = std::env::temp_dir().join(format!("tm-watch-{}", std::process::id()));
        std::fs::create_dir_all(&dir).unwrap();
        let path = dir.join("taskbar-metrics.watch");
        let devices = vec![DeviceId::parse("disk@D:").unwrap()];
        WatchList::write(&path, &devices).unwrap();
        assert_eq!(WatchList::read(&path), devices);
        assert!(WatchList::read(&dir.join("missing.watch")).is_empty());
        let _ = std::fs::remove_dir_all(dir);
    }
}
