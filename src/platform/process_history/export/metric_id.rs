/// A reading id the recorder keeps a total of: a kind and, for a device that can
/// be one of several, its id after `@` (`cpu`, `ram_used`, `disk_read@C:`).
pub(super) struct MetricId<'a> {
    kind: &'a str,
    device: Option<&'a str>,
}

impl<'a> MetricId<'a> {
    pub fn parse(id: &'a str) -> Self {
        match id.split_once('@') {
            Some((kind, device)) => Self {
                kind,
                device: Some(device),
            },
            None => Self {
                kind: id,
                device: None,
            },
        }
    }

    /// The unit of the values, as the tiles show them.
    pub fn unit(&self) -> &'static str {
        if self.kind == "ram_used" {
            "bytes"
        } else if self.kind.ends_with("temperature") {
            "celsius"
        } else if self.kind.starts_with("disk_") || self.kind.starts_with("net_") {
            "mb_per_s"
        } else {
            "percent"
        }
    }
}

/// An item of `--metrics`: a whole kind (`disk_read`) or a word of it (`disk`,
/// `temperature`), with or without a device (`disk@C:`).
pub(super) struct MetricSelector {
    word: String,
    device: Option<String>,
}

impl MetricSelector {
    pub fn parse(text: &str) -> Self {
        let id = MetricId::parse(text);
        Self {
            word: id.kind.to_ascii_lowercase(),
            device: id.device.map(str::to_owned),
        }
    }

    pub fn matches(&self, id: &MetricId) -> bool {
        let kind = id.kind.to_ascii_lowercase();
        (kind == self.word || kind.split('_').any(|word| word == self.word))
            && self.device.as_ref().is_none_or(|device| {
                id.device
                    .is_some_and(|other| other.eq_ignore_ascii_case(device))
            })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn matches(selector: &str, id: &str) -> bool {
        MetricSelector::parse(selector).matches(&MetricId::parse(id))
    }

    #[test]
    fn units_follow_the_kind() {
        let unit = |id| MetricId::parse(id).unit();
        assert_eq!(unit("cpu"), "percent");
        assert_eq!(unit("gpu@1"), "percent");
        assert_eq!(unit("ram_used"), "bytes");
        assert_eq!(unit("disk_write@C:"), "mb_per_s");
        assert_eq!(unit("net_down@Wi-Fi"), "mb_per_s");
        assert_eq!(unit("gpu_temperature@1"), "celsius");
    }

    #[test]
    fn selectors_match_kinds_words_and_devices() {
        assert!(matches("disk", "disk_read@C:"));
        assert!(matches("disk_read@c:", "disk_read@C:"));
        assert!(!matches("disk_read@D:", "disk_read@C:"));
        assert!(matches("disk@C:", "disk_write@C:"));
        assert!(matches("temperature", "cpu_temperature"));
        assert!(matches("cpu", "cpu_temperature"));
        assert!(matches("ram", "ram_used"));
        assert!(!matches("cpu", "gpu@1"));
        assert!(!matches("gpu@1", "gpu"));
        assert!(matches("net@Wi-Fi", "net_up@Wi-Fi"));
    }
}
