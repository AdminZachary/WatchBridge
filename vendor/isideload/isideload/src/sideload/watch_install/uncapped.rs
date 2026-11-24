//! Explicit experimental opt-in: no fixed rate ceiling or credit envelope.
//! Local write timing cannot measure the iPhone's downstream queue.
use super::*;
use tokio::time::Instant;

pub(super) struct UncappedPacer {
    rate: Option<u64>,
    next_write: Instant,
    last_change: Instant,
    last_pressure: Option<Instant>,
    samples: u64,
    stable_samples: u64,
    ewma_ms: f64,
}

impl UncappedPacer {
    pub fn new(now: Instant) -> Self {
        Self {
            rate: None,
            next_write: now,
            last_change: now,
            last_pressure: None,
            samples: 0,
            stable_samples: 0,
            ewma_ms: 0.5,
        }
    }

    // Zero is telemetry for no intentional rate limit, not a measured speed.
    pub fn target_kbps(&self) -> u64 {
        self.rate.unwrap_or(0)
    }

    pub fn plan(&mut self, bytes: usize, now: Instant) -> Option<Instant> {
        self.rate.map(|rate| {
            let when = now.max(self.next_write);
            self.next_write = when + Duration::from_secs_f64(bytes as f64 / (rate as f64 * 1024.0));
            when
        })
    }

    pub fn observe(&mut self, bytes: usize, started: Instant, finished: Instant) {
        let elapsed_ms = finished.duration_since(started).as_secs_f64() * 1000.0;
        let pressure = elapsed_ms >= (self.ewma_ms * 4.0).clamp(80.0, 500.0);
        self.ewma_ms = self.ewma_ms * 0.875 + elapsed_ms * 0.125;
        self.samples = self.samples.saturating_add(1);
        self.stable_samples = self.stable_samples.saturating_add(1);
        if pressure {
            let measured =
                ((bytes as f64 / 1024.0) / (elapsed_ms / 1000.0) * 0.75).max(32.0) as u64;
            let rate = self
                .rate
                .map_or(measured, |previous| (previous / 2).max(32).min(measured));
            self.rate = Some(rate);
            self.last_pressure = Some(finished);
            self.last_change = finished;
            self.samples = 0;
            self.stable_samples = 0;
            self.next_write =
                finished + Duration::from_millis((elapsed_ms * 2.0).clamp(250.0, 1000.0) as u64);
            info!(target: "isideload::sideload::watch_install", "Uncapped Watch sender: write pressure ({elapsed_ms:.1} ms); temporary target {rate} KiB/s");
        } else if self.rate.is_some()
            && self.samples >= 8
            && finished.duration_since(self.last_change) >= Duration::from_secs(2)
            && self
                .last_pressure
                .is_some_and(|time| finished.duration_since(time) >= Duration::from_secs(4))
        {
            if self.stable_samples >= 16
                && elapsed_ms < 8.0
                && self
                    .last_pressure
                    .is_some_and(|time| finished.duration_since(time) >= Duration::from_secs(20))
            {
                self.rate = None;
                self.next_write = finished;
                info!(target: "isideload::sideload::watch_install", "Uncapped Watch sender: stable writes; probing without an intentional rate limit");
            } else {
                let previous = self.rate.unwrap();
                let rate = previous.saturating_add((previous / 4).max(64));
                self.rate = Some(rate);
                info!(target: "isideload::sideload::watch_install", "Uncapped Watch sender: stable writes; temporary target {rate} KiB/s");
            }
            self.last_change = finished;
            self.samples = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fast_96_mib_has_no_512_refill_or_four_mib_ceiling() {
        let now = Instant::now();
        let mut pacer = UncappedPacer::new(now);
        for _ in 0..(96 * 1024 * 1024 / SEND_CHUNK) {
            assert_eq!(pacer.plan(SEND_CHUNK, now), None);
            pacer.observe(SEND_CHUNK, now, now);
        }
        assert_eq!(pacer.target_kbps(), 0);
    }

    #[test]
    fn pressure_causes_a_finite_temporary_rate_and_does_not_grant_unlimited_credit() {
        let now = Instant::now();
        let mut pacer = UncappedPacer::new(now);
        pacer.observe(SEND_CHUNK, now, now + Duration::from_millis(200));
        assert_eq!(pacer.target_kbps(), 120);
        assert!(
            pacer
                .plan(SEND_CHUNK, now + Duration::from_millis(200))
                .unwrap()
                >= now + Duration::from_millis(600)
        );
        pacer.observe(
            SEND_CHUNK,
            now + Duration::from_secs(1),
            now + Duration::from_millis(1900),
        );
        assert!(pacer.target_kbps() <= 60);
    }

    #[test]
    fn stable_writes_increase_rate_then_can_return_to_unpaced_probe() {
        let now = Instant::now();
        let mut pacer = UncappedPacer::new(now);
        pacer.observe(SEND_CHUNK, now, now + Duration::from_millis(200));
        for i in 1..=8 {
            let time = now + Duration::from_millis(200 + i * 100);
            pacer.observe(SEND_CHUNK, time, time + Duration::from_millis(1));
        }
        assert_eq!(pacer.target_kbps(), 120);
        let later = now + Duration::from_secs(5);
        pacer.observe(SEND_CHUNK, later, later + Duration::from_millis(1));
        assert!(pacer.target_kbps() > 120);
        for i in 0..16 {
            let time = now + Duration::from_secs(21) + Duration::from_millis(i * 2);
            pacer.observe(SEND_CHUNK, time, time + Duration::from_millis(1));
        }
        assert_eq!(pacer.target_kbps(), 0);
        assert_eq!(pacer.plan(SEND_CHUNK, now + Duration::from_secs(22)), None);
    }
}
