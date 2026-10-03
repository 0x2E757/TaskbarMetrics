//! Read-only hardware descriptions for the main window's section headers.

use super::pdh::PdhCounter;
use super::{abi::*, com::Com};
use std::ptr;

/// `IOCTL_STORAGE_QUERY_PROPERTY` on a volume or physical drive, opened with zero
/// access: it only reads device metadata and needs no rights.
struct StorageQuery;

impl StorageQuery {
    /// Standard query of `property` into `output`; the number of bytes returned.
    fn property(path: &str, property: u32, output: &mut [u32]) -> Option<u32> {
        const QUERY_PROPERTY: u32 = 0x002D_1400;
        let path = wide(path);
        // SAFETY: the path is NUL terminated; zero access only queries device metadata.
        let raw =
            unsafe { CreateFileW(path.as_ptr(), 0, 3, ptr::null_mut(), 3, 0, ptr::null_mut()) };
        if raw as isize == -1 {
            return None;
        }
        let device = Handle::new(raw).ok()?;
        // PropertyId, PropertyStandardQuery, no additional parameters.
        let query = [property, 0, 0];
        let mut returned = 0;
        // SAFETY: buffers outlive the call and their sizes are passed exactly.
        let ok = unsafe {
            DeviceIoControl(
                device.0,
                QUERY_PROPERTY,
                query.as_ptr().cast(),
                std::mem::size_of_val(&query) as u32,
                output.as_mut_ptr().cast(),
                std::mem::size_of_val(output) as u32,
                &mut returned,
                ptr::null_mut(),
            )
        };
        (ok != 0).then_some(returned)
    }
}

/// «SSD» or «HDD»: whether a physical disk (`\\.\PhysicalDriveN`, the number of
/// its `\PhysicalDisk` instance) incurs a seek penalty.
pub struct DiskMedia;

impl DiskMedia {
    pub fn of(number: u32) -> Option<&'static str> {
        // StorageDeviceSeekPenaltyProperty: Version, Size, IncursSeekPenalty.
        let mut descriptor = [0u32; 3];
        let returned =
            StorageQuery::property(&format!(r"\\.\PhysicalDrive{number}"), 7, &mut descriptor)?;
        (returned >= 9).then_some(if descriptor[2] & 0xFF != 0 {
            "HDD"
        } else {
            "SSD"
        })
    }
}

/// «NVMe»: the bus a drive letter's disk is attached to.
pub struct DriveBus;

impl DriveBus {
    /// `STORAGE_DEVICE_DESCRIPTOR.BusType` (StorageDeviceProperty).
    pub fn of(letter: &str) -> Option<&'static str> {
        let mut descriptor = [0u32; 64];
        if StorageQuery::property(&format!(r"\\.\{letter}"), 0, &mut descriptor)? < 32 {
            return None;
        }
        // BusType follows Version, Size, four type bytes and four offsets.
        match descriptor[7] {
            17 => Some("NVMe"),
            11 => Some("SATA"),
            7 => Some("USB"),
            3 => Some("ATA"),
            10 => Some("SAS"),
            1 => Some("SCSI"),
            8 => Some("RAID"),
            _ => None,
        }
    }
}

/// Physical GPU identified by the LUID in PDH instance names (`luid_0xHIGH_0xLOW_phys_N`).
#[derive(Clone, PartialEq, Debug)]
pub struct GpuAdapter {
    /// DXGI description, e.g. «NVIDIA GeForce RTX 4070».
    pub name: String,
    pub dedicated_bytes: u64,
    /// Integrated GPUs expose at most a small carve-out as dedicated memory.
    /// DXCore's `IsIntegrated` is no help: AMD reports the Ryzen 7000 iGPU,
    /// with its 512 MB carve-out, as not integrated.
    pub discrete: bool,
}

impl GpuAdapter {
    /// `(high, low)` LUID parts of an engine (`pid_N_luid_…`) or adapter instance name.
    pub fn luid(instance: &str) -> Option<(i32, u32)> {
        let mut parts = instance.split_once("luid_")?.1.split('_');
        let high = u32::from_str_radix(parts.next()?.strip_prefix("0x")?, 16).ok()? as i32;
        let low = u32::from_str_radix(parts.next()?.strip_prefix("0x")?, 16).ok()?;
        Some((high, low))
    }

    /// Adapter with the given LUID from DXGI.
    pub fn find(luid: (i32, u32)) -> Option<Self> {
        const FACTORY: Guid = Guid::from_u128(0x770aae78_f26f_4dba_a829_253c83d1b387);
        let mut raw = ptr::null_mut();
        // SAFETY: out pointer receives an owned IDXGIFactory1.
        if unsafe { CreateDXGIFactory1(&FACTORY, &mut raw) } != 0 {
            return None;
        }
        // SAFETY: the factory pointer was just returned owned by CreateDXGIFactory1.
        let factory = unsafe { Com::owned(raw) }.ok()?;
        for index in 0.. {
            let mut raw = ptr::null_mut();
            // SAFETY: IDXGIFactory1::EnumAdapters1 is vtable slot 12.
            let adapter = unsafe {
                let enumerate: unsafe extern "system" fn(Raw, u32, *mut Raw) -> Hr =
                    factory.slot(12);
                if enumerate(factory.raw(), index, &mut raw) != 0 {
                    return None;
                }
                Com::owned(raw).ok()?
            };
            let mut description = AdapterDescription::default();
            // SAFETY: IDXGIAdapter1::GetDesc1 is vtable slot 10 and fills the struct.
            unsafe {
                let describe: unsafe extern "system" fn(Raw, *mut AdapterDescription) -> Hr =
                    adapter.slot(10);
                if describe(adapter.raw(), &mut description) != 0 {
                    continue;
                }
            }
            if (description.luid_high, description.luid_low) == luid {
                let dedicated_bytes = description.dedicated_video_memory as u64;
                let length = description
                    .description
                    .iter()
                    .position(|c| *c == 0)
                    .unwrap_or(description.description.len());
                return Some(Self {
                    name: String::from_utf16_lossy(&description.description[..length]),
                    dedicated_bytes,
                    discrete: dedicated_bytes >= 1 << 30,
                });
            }
        }
        None
    }
}

#[repr(C)]
struct AdapterDescription {
    description: [u16; 128],
    vendor: u32,
    device: u32,
    subsystem: u32,
    revision: u32,
    dedicated_video_memory: usize,
    dedicated_system_memory: usize,
    shared_system_memory: usize,
    luid_low: u32,
    luid_high: i32,
    flags: u32,
}

impl Default for AdapterDescription {
    fn default() -> Self {
        // SAFETY: plain integers and arrays; all-zero is a valid value.
        unsafe { std::mem::zeroed() }
    }
}

/// Dedicated GPU memory in use, per adapter, from `\GPU Adapter Memory(*)\Dedicated Usage`.
pub struct GpuMemory {
    counter: PdhCounter,
}

impl GpuMemory {
    pub fn new() -> Self {
        Self {
            counter: PdhCounter::new(r"\GPU Adapter Memory(*)\Dedicated Usage"),
        }
    }

    /// Bytes in use on the adapter with `luid`.
    pub fn used(&mut self, luid: (i32, u32)) -> Option<f64> {
        self.counter
            .entries()
            .ok()?
            .into_iter()
            .find(|(name, _)| GpuAdapter::luid(name) == Some(luid))
            .map(|(_, bytes)| bytes)
    }
}

/// «AMD Ryzen 7 9800X3D»: the processor with its physical cores and threads.
#[derive(Clone, Debug, PartialEq)]
pub struct Processor {
    /// Empty when the registry does not name it.
    pub name: String,
    /// Sockets; more than one on multiprocessor servers.
    pub packages: u32,
    pub cores: u32,
    pub threads: u32,
}

impl Processor {
    pub fn read() -> Option<Self> {
        Some(Self {
            name: Self::name().map_or_else(String::new, |name| Self::short(&name)),
            // RelationProcessorPackage and RelationProcessorCore.
            packages: Self::count(3)?,
            cores: Self::count(0)?,
            // SAFETY: ALL_PROCESSOR_GROUPS counts every group.
            threads: unsafe { GetActiveProcessorCount(0xffff) },
        })
    }

    /// The name Task Manager shows, from the registry.
    fn name() -> Option<String> {
        const LOCAL_MACHINE: isize = 0x8000_0002u32 as i32 as isize;
        let key = wide(r"HARDWARE\DESCRIPTION\System\CentralProcessor\0");
        let value = wide("ProcessorNameString");
        let mut buffer = [0u16; 256];
        let mut size = std::mem::size_of_val(&buffer) as u32;
        // SAFETY: RRF_RT_REG_SZ into a buffer of `size` bytes, NUL terminated on success.
        let status = unsafe {
            RegGetValueW(
                LOCAL_MACHINE as Raw,
                key.as_ptr(),
                value.as_ptr(),
                2,
                ptr::null_mut(),
                buffer.as_mut_ptr().cast(),
                &mut size,
            )
        };
        if status != 0 {
            return None;
        }
        let length = buffer.iter().position(|c| *c == 0).unwrap_or(buffer.len());
        Some(String::from_utf16_lossy(&buffer[..length]))
    }

    /// Drops marks, the clock and the «N-Core Processor» tail the counts already say:
    /// «Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz» → «Intel Core i7-8700K».
    fn short(name: &str) -> String {
        let name = name
            .replace("(R)", "")
            .replace("(TM)", "")
            .replace("(tm)", "");
        let name = name.split(" @ ").next().unwrap_or_default();
        let mut words: Vec<&str> = name.split_whitespace().collect();
        if words.last() == Some(&"Processor") {
            words.pop();
            if words
                .last()
                .and_then(|word| word.strip_suffix("-Core"))
                .is_some_and(|count| count.parse::<u32>().is_ok())
            {
                words.pop();
            }
        }
        if words.last() == Some(&"CPU") {
            words.pop();
        }
        words.join(" ")
    }

    /// Records of one `LOGICAL_PROCESSOR_RELATIONSHIP` kind.
    fn count(relation: u32) -> Option<u32> {
        let mut size = 0u32;
        // SAFETY: a null buffer only reports the size needed.
        unsafe { GetLogicalProcessorInformationEx(relation, ptr::null_mut(), &mut size) };
        let mut buffer = vec![0u64; (size as usize).div_ceil(8)];
        // SAFETY: the buffer holds `size` bytes, 8-byte aligned.
        if unsafe {
            GetLogicalProcessorInformationEx(relation, buffer.as_mut_ptr().cast(), &mut size)
        } == 0
        {
            return None;
        }
        // SAFETY: the u64 buffer is viewed as the `size` bytes the call filled.
        let bytes =
            unsafe { std::slice::from_raw_parts(buffer.as_ptr().cast::<u8>(), size as usize) };
        let (mut offset, mut count) = (0usize, 0u32);
        // Each record starts with Relationship and its own Size.
        while offset + 8 <= bytes.len() {
            let record = u32::from_le_bytes(bytes[offset + 4..offset + 8].try_into().ok()?);
            if record == 0 {
                break;
            }
            offset += record as usize;
            count += 1;
        }
        (count > 0).then_some(count)
    }
}

#[link(name = "advapi32")]
extern "system" {
    fn RegGetValueW(
        key: Raw,
        subkey: *const u16,
        value: *const u16,
        flags: u32,
        kind: *mut u32,
        data: *mut core::ffi::c_void,
        size: *mut u32,
    ) -> i32;
}

#[link(name = "kernel32")]
extern "system" {
    fn GetActiveProcessorCount(group: u16) -> u32;
    fn GetLogicalProcessorInformationEx(
        relation: u32,
        buffer: *mut core::ffi::c_void,
        size: *mut u32,
    ) -> i32;
    fn CreateFileW(
        name: *const u16,
        access: u32,
        share: u32,
        security: *mut core::ffi::c_void,
        disposition: u32,
        flags: u32,
        template: Raw,
    ) -> Raw;
    fn DeviceIoControl(
        device: Raw,
        code: u32,
        input: *const core::ffi::c_void,
        input_size: u32,
        output: *mut core::ffi::c_void,
        output_size: u32,
        returned: *mut u32,
        overlapped: *mut core::ffi::c_void,
    ) -> i32;
}

#[link(name = "dxgi")]
extern "system" {
    fn CreateDXGIFactory1(iid: *const Guid, factory: *mut Raw) -> Hr;
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn luid_is_parsed_from_pdh_instance_names() {
        assert_eq!(
            GpuAdapter::luid("luid_0x00000000_0x0000D1B5_phys_0_eng_3_engtype_3D"),
            Some((0, 0xD1B5))
        );
        assert_eq!(GpuAdapter::luid("phys_0"), None);
    }

    #[test]
    fn processor_names_lose_marks_clock_and_core_tail() {
        for (raw, short) in [
            (
                "AMD Ryzen 7 9800X3D 8-Core Processor           ",
                "AMD Ryzen 7 9800X3D",
            ),
            (
                "Intel(R) Core(TM) i7-8700K CPU @ 3.70GHz",
                "Intel Core i7-8700K",
            ),
            (
                "13th Gen Intel(R) Core(TM) i7-13700K",
                "13th Gen Intel Core i7-13700K",
            ),
            (
                "AMD Ryzen 7 7840HS w/ Radeon 780M Graphics",
                "AMD Ryzen 7 7840HS w/ Radeon 780M Graphics",
            ),
            (
                "Intel(R) Xeon(R) Gold 6338 CPU @ 2.00GHz",
                "Intel Xeon Gold 6338",
            ),
            (
                "Snapdragon(R) X Elite - X1E78100 - Qualcomm(R) Oryon(TM) CPU",
                "Snapdragon X Elite - X1E78100 - Qualcomm Oryon",
            ),
        ] {
            assert_eq!(Processor::short(raw), short);
        }
    }

    #[test]
    fn the_main_gpu_is_discrete_whenever_there_is_one() {
        use super::super::gpu_temperature::GpuAdapters;
        let adapters: Vec<_> = GpuAdapters::list()
            .into_iter()
            .map(GpuAdapter::find)
            .collect();
        if adapters.iter().flatten().any(|adapter| adapter.discrete) {
            assert!(adapters[GpuAdapters::main()]
                .as_ref()
                .is_some_and(|adapter| adapter.discrete));
        }
    }

    #[test]
    fn this_machine_reports_cores_within_threads() {
        let processor = Processor::read().unwrap();
        assert!(processor.packages >= 1);
        assert!(processor.cores >= processor.packages);
        assert!(processor.threads >= processor.cores);
    }
}
