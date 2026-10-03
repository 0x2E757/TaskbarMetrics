/// Wall-clock time of a 500 ms history bucket in the user's time zone.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct LocalTime {
    pub hour: u16,
    pub minute: u16,
    pub second: u16,
    pub tenth: u16,
}

impl LocalTime {
    pub fn of(bucket: u64) -> Self {
        #[repr(C)]
        #[derive(Default)]
        struct SystemTime {
            year: u16,
            month: u16,
            day_of_week: u16,
            day: u16,
            hour: u16,
            minute: u16,
            second: u16,
            millis: u16,
        }

        #[repr(C)]
        struct FileTime {
            low: u32,
            high: u32,
        }

        #[link(name = "kernel32")]
        extern "system" {
            fn FileTimeToLocalFileTime(utc: *const FileTime, local: *mut FileTime) -> i32;
            fn FileTimeToSystemTime(time: *const FileTime, system: *mut SystemTime) -> i32;
        }
        let time = bucket
            .saturating_mul(5_000_000)
            .saturating_add(116444736000000000);
        let utc = FileTime {
            low: time as u32,
            high: (time >> 32) as u32,
        };
        let mut local = FileTime { low: 0, high: 0 };
        let mut system = SystemTime::default();
        unsafe {
            FileTimeToLocalFileTime(&utc, &mut local);
            FileTimeToSystemTime(&local, &mut system);
        }
        Self {
            hour: system.hour,
            minute: system.minute,
            second: system.second,
            tenth: system.millis / 100,
        }
    }
}
