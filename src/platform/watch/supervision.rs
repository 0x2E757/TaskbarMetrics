//! When the watcher acts: decisions over what one check found, apart from the
//! processes themselves.
use std::time::{Duration, Instant};

/// What one check found.
#[derive(Clone, Copy, Debug, Default)]
pub struct Observation {
    /// The Explorer that shows the taskbar.
    pub explorer: Option<u32>,
    /// The tiles of that Explorer were stopped.
    pub stopped: bool,
    pub recorder: bool,
    pub sensor: bool,
    /// A start of the launcher is still running.
    pub attaching: bool,
}

/// What to start, for the Explorer with this id.
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum Action {
    Attach(u32),
    Recorder(u32),
    Sensor(u32),
}

/// One collector the watcher brings back: when it was missing on two checks in a
/// row, which a recorder handing over to an elevated one never is, and once a
/// minute at most. A start that failed, as a declined UAC prompt, waits for the
/// next attach.
#[derive(Default)]
struct Restart {
    missing: bool,
    last: Option<Instant>,
    failed: bool,
}
impl Restart {
    const PAUSE: Duration = Duration::from_secs(60);
    fn due(&mut self, alive: bool, now: Instant) -> bool {
        let due = !alive
            && self.missing
            && !self.failed
            && self.last.is_none_or(|last| now - last >= Self::PAUSE);
        self.missing = !alive;
        if due {
            self.last = Some(now);
        }
        due
    }
}

pub struct Supervision {
    /// The Explorer the tiles were attached to.
    attached: Option<u32>,
    /// Attaches tried since then, and the last one.
    attempts: u32,
    last_attach: Option<Instant>,
    /// The sensor ran once, so it is brought back; a copy without PawnIO never runs it.
    sensor_wanted: bool,
    recorder: Restart,
    sensor: Restart,
}
impl Supervision {
    /// Attaches are tried every 5 s for a minute, as after sign-in, then once a minute.
    const QUICK_ATTEMPTS: u32 = 12;
    const QUICK: Duration = Duration::from_secs(5);
    const SLOW: Duration = Duration::from_secs(60);

    pub fn new(attached: Option<u32>) -> Self {
        Self {
            attached,
            attempts: 0,
            last_attach: None,
            sensor_wanted: false,
            recorder: Restart::default(),
            sensor: Restart::default(),
        }
    }
    pub fn next(&mut self, seen: Observation, now: Instant) -> Vec<Action> {
        let Some(pid) = seen.explorer else {
            return Vec::new();
        };
        if seen.attaching {
            return Vec::new();
        }
        if self.attached != Some(pid) {
            let pause = if self.attempts < Self::QUICK_ATTEMPTS {
                Self::QUICK
            } else {
                Self::SLOW
            };
            if self.last_attach.is_some_and(|last| now - last < pause) {
                return Vec::new();
            }
            self.attempts += 1;
            self.last_attach = Some(now);
            return vec![Action::Attach(pid)];
        }
        if seen.stopped {
            return Vec::new();
        }
        self.sensor_wanted |= seen.sensor;
        let mut actions = Vec::new();
        if self.recorder.due(seen.recorder, now) {
            actions.push(Action::Recorder(pid));
        }
        if self.sensor_wanted && self.sensor.due(seen.sensor, now) {
            actions.push(Action::Sensor(pid));
        }
        actions
    }
    /// The launcher started for `pid` ended; it attached unless it failed.
    pub fn attached(&mut self, pid: u32, success: bool) {
        if success {
            self.attached = Some(pid);
            self.attempts = 0;
            self.last_attach = None;
            self.recorder = Restart::default();
            self.sensor = Restart::default();
        }
    }
    /// A collector did not start; it is not tried again until the next attach.
    pub fn failed(&mut self, action: Action) {
        match action {
            Action::Recorder(_) => self.recorder.failed = true,
            Action::Sensor(_) => self.sensor.failed = true,
            Action::Attach(_) => {}
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    fn seen(explorer: u32) -> Observation {
        Observation {
            explorer: Some(explorer),
            recorder: true,
            ..Observation::default()
        }
    }
    #[test]
    fn a_new_explorer_is_attached_quickly_then_once_a_minute() {
        let start = Instant::now();
        let mut watch = Supervision::new(Some(1));
        assert!(watch.next(seen(1), start).is_empty());
        assert_eq!(watch.next(seen(2), start), [Action::Attach(2)]);
        let attaching = Observation {
            attaching: true,
            ..seen(2)
        };
        assert!(watch
            .next(attaching, start + Duration::from_secs(9))
            .is_empty());
        // The launcher failed: the next try comes 5 s later.
        assert!(watch
            .next(seen(2), start + Duration::from_secs(4))
            .is_empty());
        assert_eq!(
            watch.next(seen(2), start + Duration::from_secs(5)),
            [Action::Attach(2)]
        );
        let mut later = start + Duration::from_secs(5);
        for _ in 2..Supervision::QUICK_ATTEMPTS {
            later += Duration::from_secs(5);
            assert_eq!(watch.next(seen(2), later), [Action::Attach(2)]);
        }
        assert!(watch
            .next(seen(2), later + Duration::from_secs(5))
            .is_empty());
        assert_eq!(
            watch.next(seen(2), later + Duration::from_secs(60)),
            [Action::Attach(2)]
        );
        watch.attached(2, true);
        assert!(watch
            .next(seen(2), later + Duration::from_secs(61))
            .is_empty());
    }
    #[test]
    fn collectors_come_back_when_missing_twice_and_once_a_minute() {
        let start = Instant::now();
        let mut watch = Supervision::new(Some(1));
        let gone = Observation {
            recorder: false,
            ..seen(1)
        };
        assert!(watch.next(gone, start).is_empty());
        assert_eq!(watch.next(gone, start), [Action::Recorder(1)]);
        assert!(watch.next(gone, start + Duration::from_secs(30)).is_empty());
        assert_eq!(
            watch.next(gone, start + Duration::from_secs(60)),
            [Action::Recorder(1)]
        );
        // Declined: not again until the next attach.
        watch.failed(Action::Recorder(1));
        assert!(watch
            .next(gone, start + Duration::from_secs(200))
            .is_empty());
        // Stopped tiles keep their collectors stopped.
        let mut stopped = Supervision::new(Some(1));
        let quiet = Observation {
            stopped: true,
            ..gone
        };
        assert!(stopped.next(quiet, start).is_empty());
        assert!(stopped.next(quiet, start).is_empty());
    }
    #[test]
    fn only_a_sensor_that_ran_is_brought_back() {
        let start = Instant::now();
        let mut watch = Supervision::new(Some(1));
        assert!(watch.next(seen(1), start).is_empty());
        assert!(watch.next(seen(1), start).is_empty());
        let running = Observation {
            sensor: true,
            ..seen(1)
        };
        assert!(watch.next(running, start).is_empty());
        assert!(watch.next(seen(1), start).is_empty());
        assert_eq!(watch.next(seen(1), start), [Action::Sensor(1)]);
    }
}
