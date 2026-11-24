//! Bounded write-pressure probing. Credits are a sending allowance, NOT a
//! measurement of iPhone memory or a claim that the Watch has received bytes.
use super::*;
use tokio::time::Instant;

const MIN_KBPS: u64 = 128;
const RECOVERY: Duration = Duration::from_secs(4);
const INCREASE_INTERVAL: Duration = Duration::from_secs(2);

pub(super) struct AdaptivePacer {
    target: u64,
    ceiling: u64,
    refill_kbps: u64,
    capacity: f64,
    credits: f64,
    budget_time: Instant,
    next_write: Instant,
    last_change: Instant,
    last_pressure: Option<Instant>,
    samples: u64,
    write_ewma_ms: f64,
}

impl AdaptivePacer {
    pub fn new(config: TransferConfig, now: Instant) -> Self {
        let ceiling = config.max_kbps.clamp(MIN_KBPS, 65536);
        let capacity = config
            .unpaced_bytes
            .clamp(SEND_CHUNK as u64, 32 * 1024 * 1024) as f64;
        Self {
            target: ceiling,
            ceiling,
            refill_kbps: config.kbps.clamp(MIN_KBPS, ceiling),
            capacity,
            credits: (config.unpaced_bytes as f64).min(capacity),
            budget_time: now,
            next_write: now,
            last_change: now,
            last_pressure: None,
            samples: 0,
            write_ewma_ms: 0.5,
        }
    }

    pub fn target_kbps(&self) -> u64 {
        self.target
    }

    pub fn budget_kbps(&self) -> u64 {
        self.refill_kbps.min(self.target)
    }

    pub fn plan(&mut self, bytes: usize, now: Instant) -> Instant {
        let refill = self.refill_kbps.min(self.target) as f64 * 1024.0;
        let mut when = now.max(self.next_write).max(self.budget_time);
        self.credits = (self.credits
            + when.duration_since(self.budget_time).as_secs_f64() * refill)
            .min(self.capacity);
        self.budget_time = when;
        if self.credits < bytes as f64 {
            when += Duration::from_secs_f64((bytes as f64 - self.credits) / refill);
            self.credits = bytes as f64;
            self.budget_time = when;
        }
        self.credits -= bytes as f64;
        self.next_write =
            when + Duration::from_secs_f64(bytes as f64 / (self.target as f64 * 1024.0));
        when
    }

    pub fn observe(&mut self, started: Instant, finished: Instant) {
        let elapsed_ms = finished.duration_since(started).as_secs_f64() * 1000.0;
        let pressured = elapsed_ms >= (self.write_ewma_ms * 4.0).clamp(80.0, 500.0);
        self.write_ewma_ms = self.write_ewma_ms * 0.875 + elapsed_ms * 0.125;
        self.samples += 1;
        if pressured {
            self.target = (self.target / 2).max(MIN_KBPS);
            self.last_pressure = Some(finished);
            self.last_change = finished;
            self.samples = 0;
            self.credits = 0.0;
            self.budget_time = finished;
            self.next_write =
                finished + Duration::from_millis((elapsed_ms * 2.0).clamp(250.0, 1000.0) as u64);
            info!(target: "isideload::sideload::watch_install", "Adaptive Watch sender: write pressure; peak target {} KiB/s, burst credits reset", self.target);
        } else if self.samples >= 8
            && finished.duration_since(self.last_change) >= INCREASE_INTERVAL
            && self
                .last_pressure
                .is_none_or(|time| finished.duration_since(time) >= RECOVERY)
        {
            let previous = self.target;
            self.target = (self.target + (self.target / 8).max(64)).min(self.ceiling);
            self.last_change = finished;
            self.samples = 0;
            if previous != self.target {
                info!(target: "isideload::sideload::watch_install", "Adaptive Watch sender: stable writes; peak target {} KiB/s", self.target);
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> TransferConfig {
        TransferConfig {
            kbps: 512,
            unpaced_bytes: 32 * 1024 * 1024,
            adaptive: true,
            max_kbps: 4096,
        }
    }

    #[test]
    fn fast_writes_cannot_bypass_the_total_credit_envelope() {
        let now = Instant::now();
        let mut pacer = AdaptivePacer::new(config(), now);
        let mut when = now;
        let chunks = 96 * 1024 * 1024 / SEND_CHUNK;
        for _ in 0..chunks {
            when = pacer.plan(SEND_CHUNK, when);
            pacer.observe(when, when);
        }
        // Even zero-time writes must pay for the 64 MiB beyond the burst.
        assert!(when.duration_since(now) >= Duration::from_secs(127));
        assert!(when.duration_since(now) < Duration::from_secs(130));
        assert_eq!(pacer.target_kbps(), 4096);
    }

    #[test]
    fn blocking_write_halves_target_cancels_credit_and_requires_stable_recovery() {
        let now = Instant::now();
        let mut pacer = AdaptivePacer::new(config(), now);
        pacer.observe(now, now + Duration::from_millis(200));
        assert_eq!(pacer.target_kbps(), 2048);
        assert_eq!(pacer.credits, 0.0);
        let next = pacer.plan(SEND_CHUNK, now + Duration::from_millis(200));
        assert!(next >= now + Duration::from_millis(600));
        for i in 1..=8 {
            let start = now + Duration::from_millis(200 + i * 100);
            pacer.observe(start, start + Duration::from_millis(1));
        }
        assert_eq!(pacer.target_kbps(), 2048);
        let later = now + Duration::from_secs(5);
        pacer.observe(later, later + Duration::from_millis(1));
        assert!(pacer.target_kbps() > 2048 && pacer.target_kbps() <= 4096);
    }

    #[test]
    fn long_idle_never_accumulates_more_than_one_burst_and_zero_burst_is_valid() {
        let now = Instant::now();
        let mut cfg = config();
        cfg.unpaced_bytes = 0;
        let mut pacer = AdaptivePacer::new(cfg, now);
        pacer.plan(SEND_CHUNK, now);
        let start = now + Duration::from_secs(3600);
        assert_eq!(pacer.plan(SEND_CHUNK, start), start);
        assert!(pacer.plan(SEND_CHUNK, start) > start);
        assert!(pacer.credits >= 0.0 && pacer.credits <= SEND_CHUNK as f64);
    }

    #[test]
    fn repeated_pressure_has_a_floor_and_never_switches_to_unlimited() {
        let now = Instant::now();
        let mut pacer = AdaptivePacer::new(config(), now);
        for i in 0..20 {
            let start = now + Duration::from_secs(i);
            pacer.observe(start, start + Duration::from_millis(900));
        }
        assert_eq!(pacer.target_kbps(), MIN_KBPS);
        assert!(
            pacer.plan(SEND_CHUNK, now + Duration::from_secs(30)) >= now + Duration::from_secs(30)
        );
    }
}
