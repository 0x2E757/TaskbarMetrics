//! A compiled Win32 resource file (.res) that the MSVC linker takes as an input:
//! the app icon in several sizes and the version block Windows shows as the
//! program's name (taskbar, Task Manager, UAC, file properties).

/// What the version block says about one executable.
pub struct VersionInfo<'a> {
    pub description: &'a str,
    /// The publisher: without it the Startup apps page of Settings shows the file
    /// name instead of the description.
    pub company: &'a str,
    pub file_name: &'a str,
    pub version: &'a str,
}

pub struct ResourceFile {
    bytes: Vec<u8>,
}

impl ResourceFile {
    const RT_ICON: u16 = 3;
    const RT_GROUP_ICON: u16 = 14;
    const RT_VERSION: u16 = 16;
    const ENGLISH: u16 = 0x0409;

    /// A .res file opens with an empty entry that marks its format.
    pub fn new() -> Self {
        let mut file = Self { bytes: Vec::new() };
        file.entry(0, 0, 0, 0, &[]);
        file
    }

    pub fn bytes(&self) -> &[u8] {
        &self.bytes
    }

    fn entry(&mut self, kind: u16, name: u16, flags: u16, language: u16, data: &[u8]) {
        let out = &mut self.bytes;
        out.extend((data.len() as u32).to_le_bytes());
        out.extend(32u32.to_le_bytes());
        for value in [0xFFFF, kind, 0xFFFF, name] {
            out.extend(value.to_le_bytes());
        }
        out.extend(0u32.to_le_bytes());
        out.extend(flags.to_le_bytes());
        out.extend(language.to_le_bytes());
        out.extend([0u8; 8]);
        out.extend(data);
        pad(out);
    }

    /// Icon group 1 with one 32-bit image per size, a bitmap or a PNG; Explorer
    /// and the shell pick the size they need from it.
    pub fn icons(mut self, images: &[(u32, Vec<u8>)]) -> Self {
        let mut group = Vec::new();
        for value in [0u16, 1, images.len() as u16] {
            group.extend(value.to_le_bytes());
        }
        for (index, (size, image)) in images.iter().enumerate() {
            let id = index as u16 + 1;
            // A 256 px side does not fit the byte and is written as 0.
            let side = (*size % 256) as u8;
            group.extend([side, side, 0, 0]);
            group.extend(1u16.to_le_bytes());
            group.extend(32u16.to_le_bytes());
            group.extend((image.len() as u32).to_le_bytes());
            group.extend(id.to_le_bytes());
            self.entry(Self::RT_ICON, id, 0x1010, Self::ENGLISH, image);
        }
        self.entry(Self::RT_GROUP_ICON, 1, 0x1030, Self::ENGLISH, &group);
        self
    }

    /// A DIB icon image: the header with a doubled height, the colors bottom-up,
    /// then the 1-bit mask, set where the image is fully transparent.
    pub fn bitmap(size: u32, pixels: &[u32]) -> Vec<u8> {
        let stride = size.div_ceil(32) * 4;
        let colors = size * size * 4;
        let mut out = Vec::new();
        for value in [40u32, size, size * 2] {
            out.extend(value.to_le_bytes());
        }
        out.extend(1u16.to_le_bytes());
        out.extend(32u16.to_le_bytes());
        out.extend(0u32.to_le_bytes());
        out.extend((colors + stride * size).to_le_bytes());
        out.extend([0u8; 16]);
        for row in (0..size).rev() {
            for pixel in &pixels[(row * size) as usize..((row + 1) * size) as usize] {
                out.extend(pixel.to_le_bytes());
            }
        }
        for row in (0..size).rev() {
            let mut mask = vec![0u8; stride as usize];
            for column in 0..size {
                if pixels[(row * size + column) as usize] >> 24 == 0 {
                    mask[(column / 8) as usize] |= 0x80 >> (column % 8);
                }
            }
            out.extend(mask);
        }
        out
    }

    pub fn version(mut self, info: &VersionInfo) -> Self {
        let mut numbers = info
            .version
            .split('.')
            .map(|part| part.parse::<u32>().unwrap_or(0));
        let mut next = || numbers.next().unwrap_or(0);
        let (major, minor, patch) = (next(), next(), next());
        let mut fixed = Vec::new();
        for value in [
            0xFEEF04BD,
            0x0001_0000,
            major << 16 | minor,
            patch << 16,
            major << 16 | minor,
            patch << 16,
            0x3F,
            0,
            0x0004_0004, // VOS_NT_WINDOWS32
            1,           // VFT_APP
            0,
            0,
            0,
        ] {
            fixed.extend(u32::to_le_bytes(value));
        }
        let stem = info.file_name.trim_end_matches(".exe");
        let strings: Vec<_> = [
            ("CompanyName", info.company),
            ("FileDescription", info.description),
            ("FileVersion", info.version),
            ("InternalName", stem),
            ("OriginalFilename", info.file_name),
            ("ProductName", "Taskbar Metrics"),
            ("ProductVersion", info.version),
        ]
        .iter()
        .map(|(key, value)| {
            let text = wide(value);
            block(key, &text, (text.len() / 2) as u16, 1, &[])
        })
        .collect();
        let table = block("040904B0", &[], 0, 1, &strings);
        let string_info = block("StringFileInfo", &[], 0, 1, &[table]);
        let translation = [Self::ENGLISH, 0x04B0]
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect::<Vec<_>>();
        let var = block("Translation", &translation, 4, 0, &[]);
        let var_info = block("VarFileInfo", &[], 0, 1, &[var]);
        let root = block(
            "VS_VERSION_INFO",
            &fixed,
            fixed.len() as u16,
            0,
            &[string_info, var_info],
        );
        self.entry(Self::RT_VERSION, 1, 0x0030, Self::ENGLISH, &root);
        self
    }
}

/// UTF-16LE with the terminating NUL.
fn wide(text: &str) -> Vec<u8> {
    text.encode_utf16()
        .chain(Some(0))
        .flat_map(u16::to_le_bytes)
        .collect()
}

fn pad(out: &mut Vec<u8>) {
    while !out.len().is_multiple_of(4) {
        out.push(0);
    }
}

/// A version block: length, value length, type, key, value and children, each
/// starting on a 4-byte boundary; the length leaves out the trailing padding.
fn block(key: &str, value: &[u8], value_length: u16, kind: u16, children: &[Vec<u8>]) -> Vec<u8> {
    let mut out = vec![0u8; 2];
    out.extend(value_length.to_le_bytes());
    out.extend(kind.to_le_bytes());
    out.extend(wide(key));
    pad(&mut out);
    out.extend(value);
    for child in children {
        pad(&mut out);
        out.extend(child);
    }
    let length = out.len() as u16;
    out[..2].copy_from_slice(&length.to_le_bytes());
    out
}
