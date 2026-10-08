use std::time::{Duration, SystemTime, UNIX_EPOCH};

/// One sample in each 500 ms bucket of the wall clock. The wake-ups are aimed at the
/// bucket grid rather than at 500 ms after the previous sample: a sample that took
/// longer than a bucket is followed at once instead of half a second later, which
/// skipped a bucket and broke the chart line.
pub struct SampleSchedule;

impl SampleSchedule {
    const INTERVAL_MS: u64 = 500;
    /// Early in the bucket, so a late wake-up still lands in the bucket it was meant for.
    const PHASE_MS: u64 = 50;

    /// The bucket the wall clock is in now.
    pub fn bucket() -> Result<u64, String> {
        Ok(Self::now_ms()? / Self::INTERVAL_MS)
    }

    /// Time to sleep after sampling `bucket` until the next one is due.
    pub fn delay(bucket: u64) -> Result<Duration, String> {
        Ok(Self::delay_at(bucket, Self::now_ms()?))
    }

    fn delay_at(bucket: u64, now_ms: u64) -> Duration {
        Duration::from_millis(
            ((bucket + 1) * Self::INTERVAL_MS + Self::PHASE_MS).saturating_sub(now_ms),
        )
    }

    fn now_ms() -> Result<u64, String> {
        Ok(SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .map_err(|e| e.to_string())?
            .as_millis() as u64)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn wakes_early_in_the_next_bucket() {
        assert_eq!(
            SampleSchedule::delay_at(10, 5_050),
            Duration::from_millis(500)
        );
        assert_eq!(
            SampleSchedule::delay_at(10, 5_400),
            Duration::from_millis(150)
        );
    }

    #[test]
    fn a_long_sample_is_followed_at_once_in_the_next_bucket() {
        let delay = SampleSchedule::delay_at(10, 5_700);
        assert_eq!(delay, Duration::ZERO);
        assert_eq!((5_700 + delay.as_millis() as u64) / 500, 11);
    }
}
