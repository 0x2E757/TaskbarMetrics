//! Installed memory modules, from the firmware's SMBIOS tables.
use std::ptr;

/// One populated memory slot (SMBIOS Memory Device, type 17).
#[derive(Clone, Debug, PartialEq, Eq)]
pub struct MemoryModule {
    pub megabytes: u64,
    /// «DDR5»; none for types the window does not name.
    pub kind: Option<&'static str>,
    /// Configured speed in MT/s, else the module's rated one.
    pub speed: Option<u32>,
}
impl MemoryModule {
    /// «16 GB DDR5-6000».
    fn label(&self) -> String {
        let size = if self.megabytes.is_multiple_of(1024) {
            format!("{} GB", self.megabytes / 1024)
        } else {
            format!("{} MB", self.megabytes)
        };
        match (self.kind, self.speed) {
            (Some(kind), Some(speed)) => format!("{size} {kind}-{speed}"),
            (Some(kind), None) => format!("{size} {kind}"),
            (None, _) => size,
        }
    }
}

pub struct MemoryModules;
impl MemoryModules {
    /// The modules the firmware lists; empty when it lists none. Reading the raw
    /// SMBIOS table needs no rights.
    pub fn read() -> Vec<MemoryModule> {
        const RSMB: u32 = u32::from_be_bytes(*b"RSMB");
        // SAFETY: a null buffer of size 0 only asks for the size.
        let size = unsafe { GetSystemFirmwareTable(RSMB, 0, ptr::null_mut(), 0) };
        if size == 0 {
            return Vec::new();
        }
        let mut buffer = vec![0u8; size as usize];
        // SAFETY: the buffer holds `size` bytes.
        let written = unsafe { GetSystemFirmwareTable(RSMB, 0, buffer.as_mut_ptr(), size) };
        // RawSMBIOSData: calling method, versions and the table length precede the table.
        buffer
            .get(8..written.min(size) as usize)
            .map(Self::parse)
            .unwrap_or_default()
    }
    /// «2 × 16 GB DDR5-6000»: equal modules counted, different ones joined by « + ».
    pub fn summary(modules: &[MemoryModule]) -> String {
        let mut groups: Vec<(&MemoryModule, usize)> = Vec::new();
        for module in modules {
            match groups.iter_mut().find(|(known, _)| *known == module) {
                Some((_, count)) => *count += 1,
                None => groups.push((module, 1)),
            }
        }
        groups
            .iter()
            .map(|(module, count)| match count {
                1 => module.label(),
                count => format!("{count} × {}", module.label()),
            })
            .collect::<Vec<_>>()
            .join(" + ")
    }
    /// Memory devices of a raw SMBIOS table: structures of a formatted area and
    /// strings ending with two NULs, up to the end-of-table structure (127).
    fn parse(table: &[u8]) -> Vec<MemoryModule> {
        let mut modules = Vec::new();
        let mut at = 0;
        while let Some(&[kind, length]) = table.get(at..at + 2) {
            let length = usize::from(length);
            let Some(formatted) = table.get(at..at + length).filter(|_| length >= 4) else {
                break;
            };
            match kind {
                17 => modules.extend(Self::module(formatted)),
                127 => break,
                _ => {}
            }
            let strings = at + length;
            let Some(end) = table
                .get(strings..)
                .and_then(|rest| rest.windows(2).position(|pair| pair == [0, 0]))
            else {
                break;
            };
            at = strings + end + 2;
        }
        modules
    }
    /// A populated slot of a Memory Device structure.
    fn module(device: &[u8]) -> Option<MemoryModule> {
        let word = |offset: usize| {
            device
                .get(offset..offset + 2)
                .map(|b| u16::from_le_bytes([b[0], b[1]]))
        };
        let dword = |offset: usize| {
            device
                .get(offset..offset + 4)
                .map(|b| u32::from_le_bytes([b[0], b[1], b[2], b[3]]))
        };
        let megabytes = match word(0x0C)? {
            // An empty slot, or a size the firmware does not know.
            0 | 0xFFFF => return None,
            // Extended Size, in MB.
            0x7FFF => u64::from(dword(0x1C)? & 0x7FFF_FFFF),
            // Granularity bit: the size is in KB.
            size if size & 0x8000 != 0 => u64::from(size & 0x7FFF) / 1024,
            size => u64::from(size),
        };
        let kind = match device.get(0x12)? {
            0x18 => Some("DDR3"),
            0x1A => Some("DDR4"),
            0x1D => Some("LPDDR3"),
            0x1E => Some("LPDDR4"),
            0x22 => Some("DDR5"),
            0x23 => Some("LPDDR5"),
            _ => None,
        };
        let speed = [word(0x20), word(0x15)]
            .into_iter()
            .flatten()
            .find(|speed| !matches!(speed, 0 | 0xFFFF))
            .map(u32::from);
        (megabytes > 0).then_some(MemoryModule {
            megabytes,
            kind,
            speed,
        })
    }
}

#[link(name = "kernel32")]
extern "system" {
    fn GetSystemFirmwareTable(provider: u32, id: u32, buffer: *mut u8, size: u32) -> u32;
}

#[cfg(test)]
mod tests {
    use super::*;
    /// A Memory Device structure of SMBIOS 3.x (0x28 bytes) and its strings.
    fn device(size: u16, kind: u8, speed: u16, configured: u16) -> Vec<u8> {
        let mut device = vec![0u8; 0x28];
        device[0] = 17;
        device[1] = 0x28;
        device[0x0C..0x0E].copy_from_slice(&size.to_le_bytes());
        device[0x12] = kind;
        device[0x15..0x17].copy_from_slice(&speed.to_le_bytes());
        device[0x20..0x22].copy_from_slice(&configured.to_le_bytes());
        device.extend(b"DIMMA2\0Unknown\0\0");
        device
    }
    #[test]
    fn memory_devices_are_read_from_the_table_and_counted() {
        let mut table = vec![0u8, 4, 0, 0, 0, 0];
        table.extend(device(16384, 0x22, 6400, 6000));
        table.extend(device(0, 0x02, 0, 0));
        table.extend(device(16384, 0x22, 6400, 6000));
        table.extend([127, 4, 0, 0, 0, 0]);
        let modules = MemoryModules::parse(&table);
        assert_eq!(MemoryModules::summary(&modules), "2 × 16 GB DDR5-6000");
        let mixed = [
            MemoryModule {
                megabytes: 8192,
                kind: Some("DDR4"),
                speed: Some(3200),
            },
            MemoryModule {
                megabytes: 4096,
                kind: None,
                speed: None,
            },
        ];
        assert_eq!(MemoryModules::summary(&mixed), "8 GB DDR4-3200 + 4 GB");
    }
    #[test]
    fn large_modules_use_the_extended_size() {
        let mut large = device(0x7FFF, 0x22, 5600, 0);
        large[0x1C..0x20].copy_from_slice(&65536u32.to_le_bytes());
        let modules = MemoryModules::parse(&large);
        assert_eq!(MemoryModules::summary(&modules), "64 GB DDR5-5600");
    }
}
