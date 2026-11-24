//! Experimental application-layer pacing inspired by slow start and AIMD
//! (RFC 5681), not an implementation of TCP: local writes are not Watch ACKs.
//! Windowed observations avoid treating one blocked 32 KiB write as link speed.
use super::*;
use std::collections::VecDeque;
use tokio::time::Instant;

const WINDOW: usize = 32;
const RECOVERY_STEP: Duration = Duration::from_millis(500);

pub(super) struct UncappedPacer {
    rate: Option<u64>,
    last_paced_rate: Option<u64>,
    recovery_threshold: u64,
    next_write: Instant,
    last_change: Instant,
    last_pressure: Option<Instant>,
    samples: u64,
    baseline_ms: f64,
    recent_slow: u8,
    writes: VecDeque<(usize, f64)>,
}

impl UncappedPacer {
    pub fn new(now: Instant) -> Self {
        Self {
            rate: None,
            last_paced_rate: None,
            recovery_threshold: 0,
            next_write: now,
            last_change: now,
            last_pressure: None,
            samples: 0,
            baseline_ms: 0.5,
            recent_slow: 0,
            writes: VecDeque::with_capacity(WINDOW),
        }
    }

    // Zero means no intentional rate limit, not a measured speed.
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

    fn observed_rate(&self) -> u64 {
        let bytes: usize = self.writes.iter().map(|(bytes, _)| *bytes).sum();
        // Deliberate pacing sleeps are excluded. This remains a local I/O rate,
        // not a downstream delivery-rate or receiver-memory measurement.
        let milliseconds: f64 = self.writes.iter().map(|(_, time)| *time).sum();
        ((bytes as f64 / 1024.0) / (milliseconds.max(0.001) / 1000.0)).max(32.0) as u64
    }

    pub fn observe(&mut self, bytes: usize, started: Instant, finished: Instant) {
        let elapsed_ms = finished.duration_since(started).as_secs_f64() * 1000.0;
        let slow = elapsed_ms >= (self.baseline_ms * 4.0).clamp(80.0, 250.0);
        // EWMA smooths ordinary writes; stalls must not redefine the fast baseline.
        if !slow {
            self.baseline_ms = self.baseline_ms * 0.875 + elapsed_ms * 0.125;
        }
        self.writes.push_back((bytes, elapsed_ms.max(0.001)));
        if self.writes.len() > WINDOW {
            self.writes.pop_front();
        }
        self.recent_slow = (self.recent_slow << 1) | u8::from(slow);
        self.samples = self.samples.saturating_add(1);
        let observed = self.observed_rate();
        let severe = elapsed_ms >= 1000.0;
        let persistent = slow && self.recent_slow.count_ones() >= 2;
        if severe || persistent {
            let previous = self.rate.or(self.last_paced_rate);
            let factor = if severe { 0.5 } else { 0.7 };
            let measured = (observed as f64 * factor).max(32.0) as u64;
            let rate = previous.map_or(measured, |old| {
                measured.min((old as f64 * factor).max(32.0) as u64)
            });
            self.recovery_threshold = previous.unwrap_or(observed).max(rate);
            self.rate = Some(rate);
            self.last_paced_rate = Some(rate);
            self.last_pressure = Some(finished);
            self.last_change = finished;
            self.samples = 0;
            self.recent_slow = 0;
            // No extra 400 ms pause after a 200 ms write that already blocked.
            self.next_write = finished;
            info!(target:"isideload::sideload::watch_install", "Uncapped Watch sender v2: sustained/long write pressure ({elapsed_ms:.1} ms); window {} writes, observed {observed} KiB/s; temporary target {rate} KiB/s", self.writes.len());
        } else if slow {
            info!(target:"isideload::sideload::watch_install", "Uncapped Watch sender v2: isolated stall ({elapsed_ms:.1} ms); observing without reset; window {observed} KiB/s");
        } else if self.rate.is_some()
            && self.recent_slow == 0
            && self.samples >= 8
            && finished.duration_since(self.last_change) >= RECOVERY_STEP
            && self
                .last_pressure
                .is_some_and(|time| finished.duration_since(time) >= Duration::from_secs(1))
        {
            let previous = self.rate.unwrap();
            let headroom = observed >= previous.saturating_mul(2);
            if headroom
                && self.writes.len() >= 16
                && previous >= self.recovery_threshold
                && self
                    .last_pressure
                    .is_some_and(|time| finished.duration_since(time) >= Duration::from_secs(4))
            {
                self.last_paced_rate = Some(previous);
                self.rate = None;
                self.next_write = finished;
                info!(target:"isideload::sideload::watch_install", "Uncapped Watch sender v2: stable window with headroom; probing without an intentional rate limit");
            } else {
                // Slow-start-style recovery to the learned threshold, then
                // additive growth. Neither phase has a preset rate ceiling.
                let rate = if previous < self.recovery_threshold {
                    previous.saturating_mul(2).min(self.recovery_threshold)
                } else {
                    previous.saturating_add(128)
                };
                self.rate = Some(rate);
                self.last_paced_rate = Some(rate);
                info!(target:"isideload::sideload::watch_install", "Uncapped Watch sender v2: stable window; temporary target {rate} KiB/s");
            }
            self.last_change = finished;
            self.samples = 0;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn write(pacer: &mut UncappedPacer, time: &mut Instant, milliseconds: u64) {
        let start = pacer.plan(SEND_CHUNK, *time).unwrap_or(*time);
        let end = start + Duration::from_millis(milliseconds);
        pacer.observe(SEND_CHUNK, start, end);
        *time = end;
    }

    #[test]
    fn fast_96_mib_has_no_512_refill_or_four_mib_ceiling() {
        let now = Instant::now();
        let mut pacer = UncappedPacer::new(now);
        for _ in 0..96 * 1024 * 1024 / SEND_CHUNK {
            assert_eq!(pacer.plan(SEND_CHUNK, now), None);
            pacer.observe(SEND_CHUNK, now, now);
        }
        assert_eq!(pacer.target_kbps(), 0);
    }

    #[test]
    fn isolated_stalls_from_the_device_trace_do_not_reset_to_100_kib() {
        let mut time = Instant::now();
        let mut pacer = UncappedPacer::new(time);
        for spike in [206, 164, 170, 198, 181, 156, 113, 215] {
            for _ in 0..64 {
                write(&mut pacer, &mut time, 1);
            }
            write(&mut pacer, &mut time, spike);
            assert_eq!(pacer.target_kbps(), 0);
        }
    }

    #[test]
    fn sustained_pressure_uses_a_whole_window_instead_of_one_chunk() {
        let mut time = Instant::now();
        let mut pacer = UncappedPacer::new(time);
        for _ in 0..64 {
            write(&mut pacer, &mut time, 1);
        }
        write(&mut pacer, &mut time, 200);
        assert_eq!(pacer.target_kbps(), 0);
        write(&mut pacer, &mut time, 200);
        assert!(pacer.target_kbps() > 1024 && pacer.target_kbps() < 4096);
    }

    #[test]
    fn long_stall_still_backs_off_immediately_without_a_fixed_ceiling() {
        let mut time = Instant::now();
        let mut pacer = UncappedPacer::new(time);
        write(&mut pacer, &mut time, 1200);
        assert!(pacer.target_kbps() > 0 && pacer.target_kbps() <= 64);
        assert!(pacer.plan(SEND_CHUNK, time).is_some());
    }

    #[test]
    fn stable_recovery_is_fast_and_can_probe_unpaced_again() {
        let mut time = Instant::now();
        let mut pacer = UncappedPacer::new(time);
        for _ in 0..64 {
            write(&mut pacer, &mut time, 1);
        }
        write(&mut pacer, &mut time, 200);
        write(&mut pacer, &mut time, 200);
        let reduced = pacer.target_kbps();
        let start = time;
        let mut grew = false;
        let mut unpaced = false;
        for _ in 0..100 {
            time += Duration::from_millis(100);
            write(&mut pacer, &mut time, 1);
            let rate = pacer.target_kbps();
            grew |= rate > reduced;
            if rate == 0 {
                unpaced = true;
                break;
            }
        }
        assert!(grew && unpaced);
        assert!(time.duration_since(start) < Duration::from_secs(8));
    }
}
