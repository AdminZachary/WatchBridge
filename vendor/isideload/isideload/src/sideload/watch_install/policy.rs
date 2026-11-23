use super::*;
use tokio::time::Instant;

pub(super) const STEP_TIMEOUT: Duration = Duration::from_secs(30);
pub(super) const PAIR_TIMEOUT: Duration = Duration::from_secs(120);
pub(super) const SESSION_ATTEMPTS: usize = 3;
pub(super) const SEND_CHUNK: usize = 32 * 1024;

#[derive(Debug, thiserror::Error)]
pub(super) enum WatchInstallError {
    #[error("The Apple Watch stopped answering during an operation")]
    StepTimeout,
    #[error("Apple Watch pairing was not confirmed. Check the trust prompt before trying again.")]
    PairingUnconfirmed,
    #[error(
        "The Apple Watch installation result is unconfirmed. Check the Watch before retrying; the app was not automatically sent again."
    )]
    TransferUnconfirmed,
    #[error(
        "The Apple Watch installation made no progress for 3 minutes. Keep it unlocked and near the iPhone, then check the Watch before retrying."
    )]
    ProgressStalled,
}

pub(super) async fn watch_step<T>(
    step: impl std::future::Future<Output = Result<T, idevice::IdeviceError>>,
) -> Result<T, Report> {
    bounded_step(STEP_TIMEOUT, step).await
}

pub(super) async fn bounded_step<T>(
    timeout: Duration,
    step: impl std::future::Future<Output = Result<T, idevice::IdeviceError>>,
) -> Result<T, Report> {
    match tokio::time::timeout(timeout, step).await {
        Ok(result) => Ok(result.map_err(Error::IdeviceError)?),
        Err(_) => Err(report!(WatchInstallError::StepTimeout).into()),
    }
}

pub(super) async fn bounded_operation<T>(
    operation: impl std::future::Future<Output = Result<T, Report>>,
) -> Result<T, Report> {
    match tokio::time::timeout(STEP_TIMEOUT, operation).await {
        Ok(result) => result,
        Err(_) => Err(report!(WatchInstallError::StepTimeout).into()),
    }
}

pub(super) fn transfer_result(result: Result<(), Report>) -> Result<(), Report> {
    result.map_err(|error| {
        if is_transient_watch_error(&error)
            || error.iter_reports().any(|cause| {
                matches!(
                    cause.downcast_current_context::<WatchInstallError>(),
                    Some(WatchInstallError::ProgressStalled)
                )
            })
        {
            error.context(WatchInstallError::TransferUnconfirmed).into()
        } else {
            // Keep an explicit watchOS rejection intact (including coordinator conflicts).
            error
        }
    })
}

pub(super) fn is_transient_watch_error(error: &Report) -> bool {
    // A transport error after sending an install request says nothing about whether
    // watchOS accepted it. Never replay that operation, or repeat a trust prompt.
    if error.iter_reports().any(|cause| {
        matches!(
            cause.downcast_current_context::<WatchInstallError>(),
            Some(
                WatchInstallError::TransferUnconfirmed
                    | WatchInstallError::PairingUnconfirmed
                    | WatchInstallError::ProgressStalled
            )
        )
    }) {
        return false;
    }
    error.iter_reports().any(|cause| {
        if matches!(
            cause.downcast_current_context::<WatchInstallError>(),
            Some(WatchInstallError::StepTimeout)
        ) {
            return true;
        }
        let device_error = cause
            .downcast_current_context::<idevice::IdeviceError>()
            .or_else(|| match cause.downcast_current_context::<Error>() {
                Some(Error::IdeviceError(error)) => Some(error),
                _ => None,
            });
        match device_error {
            Some(
                idevice::IdeviceError::Timeout | idevice::IdeviceError::NoEstablishedConnection,
            ) => true,
            Some(idevice::IdeviceError::Socket(error)) => matches!(
                error.kind(),
                std::io::ErrorKind::TimedOut
                    | std::io::ErrorKind::ConnectionReset
                    | std::io::ErrorKind::ConnectionAborted
                    | std::io::ErrorKind::ConnectionRefused
                    | std::io::ErrorKind::BrokenPipe
                    | std::io::ErrorKind::UnexpectedEof
                    | std::io::ErrorKind::NotConnected
                    | std::io::ErrorKind::Interrupted
            ),
            _ => false,
        }
    })
}

#[derive(Clone, Copy)]
pub(super) struct TransferConfig {
    pub kbps: u64,
    pub unpaced_bytes: u64,
}

impl TransferConfig {
    pub fn from_env() -> Self {
        Self::parse(
            std::env::var("WATCHBRIDGE_WATCH_KBPS").ok().as_deref(),
            std::env::var("WATCHBRIDGE_WATCH_UNPACED_MB")
                .ok()
                .as_deref(),
        )
    }

    fn parse(rate: Option<&str>, burst: Option<&str>) -> Self {
        fn setting(raw: Option<&str>, default: u64, multiplier: u64) -> u64 {
            match raw {
                None => default,
                Some(value) => value
                    .trim()
                    .parse::<u64>()
                    .ok()
                    .filter(|value| value.checked_mul(multiplier).is_some())
                    .unwrap_or_else(|| {
                        warn!("Invalid Watch transfer setting; using its default");
                        default
                    }),
            }
        }
        Self {
            kbps: setting(rate, 256, 1024),
            unpaced_bytes: setting(burst, 32, 1024 * 1024) * 1024 * 1024,
        }
    }
}

pub(super) struct TransferPacer {
    config: TransferConfig,
    sent: u64,
    deadline: Option<Instant>,
}

impl TransferPacer {
    pub fn new(config: TransferConfig) -> Self {
        Self {
            config,
            sent: 0,
            deadline: None,
        }
    }

    // Called before sending, then slept until after sending. Socket write time
    // counts towards the budget; a stalled write does not earn a future burst.
    pub fn next_deadline(&mut self, bytes: usize, now: Instant) -> Option<Instant> {
        let before = self.sent.saturating_sub(self.config.unpaced_bytes);
        self.sent = self.sent.saturating_add(bytes as u64);
        let paced = self.sent.saturating_sub(self.config.unpaced_bytes) - before;
        if self.config.kbps == 0 || paced == 0 {
            return None;
        }
        let duration = Duration::from_secs_f64(paced as f64 / (self.config.kbps as f64 * 1024.0));
        let deadline = self.deadline.unwrap_or(now).max(now) + duration;
        self.deadline = Some(deadline);
        Some(deadline)
    }
}

pub(super) struct ProgressWatchdog {
    pub deadline: Instant,
    progress: Option<(u64, String)>,
}

impl ProgressWatchdog {
    pub fn new(now: Instant) -> Self {
        Self {
            deadline: now + WATCH_RESPONSE_TIMEOUT,
            progress: None,
        }
    }

    pub fn observe(&mut self, percent: u64, status: &str, now: Instant) {
        if self
            .progress
            .as_ref()
            .is_none_or(|(old, stage)| percent > *old || stage != status)
        {
            self.progress = Some((percent, status.into()));
            self.deadline = now + WATCH_RESPONSE_TIMEOUT;
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn retries_only_typed_connection_failures_before_transfer() {
        for error in [
            idevice::IdeviceError::Timeout,
            idevice::IdeviceError::NoEstablishedConnection,
            idevice::IdeviceError::Socket(std::io::ErrorKind::ConnectionReset.into()),
        ] {
            let error: Report = report!(Error::IdeviceError(error))
                .context("outer setup wrapper")
                .into();
            assert!(is_transient_watch_error(&error));
            assert!(!is_transient_watch_error(
                &error.context(WatchInstallError::TransferUnconfirmed).into()
            ));
        }
        for error in [
            idevice::IdeviceError::UserDeniedPairing,
            idevice::IdeviceError::DeviceLocked,
            idevice::IdeviceError::Socket(std::io::ErrorKind::PermissionDenied.into()),
        ] {
            assert!(!is_transient_watch_error(
                &report!(Error::IdeviceError(error)).into()
            ));
        }
        assert!(!is_transient_watch_error(&report!(
            "device socket io failed"
        )));
        assert!(!is_transient_watch_error(
            &report!(WatchInstallError::StepTimeout)
                .context(WatchInstallError::PairingUnconfirmed)
                .into()
        ));
        assert!(!is_transient_watch_error(
            &report!(WatchInstallError::ProgressStalled).into()
        ));
    }

    #[tokio::test]
    async fn stalled_setup_is_bounded_and_classified() {
        let error = bounded_step(
            Duration::from_millis(1),
            std::future::pending::<Result<(), idevice::IdeviceError>>(),
        )
        .await
        .unwrap_err();
        assert!(is_transient_watch_error(&error));
        assert!(
            watch_step(async { Ok::<_, idevice::IdeviceError>(42) })
                .await
                .unwrap()
                == 42
        );
    }

    #[test]
    fn settings_handle_zero_invalid_and_overflow_without_panics() {
        let defaults = TransferConfig::parse(None, None);
        assert_eq!(defaults.kbps, 256);
        assert_eq!(defaults.unpaced_bytes, 32 * 1024 * 1024);
        let zero = TransferConfig::parse(Some("0"), Some("0"));
        assert_eq!(zero.kbps, 0);
        assert_eq!(zero.unpaced_bytes, 0);
        for value in ["-1", "bad", "18446744073709551615"] {
            let config = TransferConfig::parse(Some(value), Some(value));
            assert_eq!(config.kbps, defaults.kbps);
            assert_eq!(config.unpaced_bytes, defaults.unpaced_bytes);
        }
    }

    #[test]
    fn pacing_counts_send_time_crosses_threshold_and_spans_files() {
        let now = Instant::now();
        let mut pacer = TransferPacer::new(TransferConfig {
            kbps: 256,
            unpaced_bytes: 32768,
        });
        assert_eq!(pacer.next_deadline(16384, now), None);
        // Only the final 16 KiB of this chunk are paced, including across files.
        let deadline = pacer.next_deadline(32768, now).unwrap();
        assert_eq!(
            deadline - now,
            Duration::from_millis(62) + Duration::from_micros(500)
        );
        let write_finished = now + Duration::from_millis(50);
        assert_eq!(deadline - write_finished, Duration::from_micros(12500));
        let later = now + Duration::from_secs(5);
        assert_eq!(
            pacer.next_deadline(32768, later).unwrap() - later,
            Duration::from_millis(125)
        );
        let mut unlimited = TransferPacer::new(TransferConfig {
            kbps: 0,
            unpaced_bytes: 0,
        });
        assert_eq!(unlimited.next_deadline(32768, now), None);
    }

    #[test]
    fn heartbeat_does_not_extend_stalled_install_but_progress_does() {
        let now = Instant::now();
        let mut watchdog = ProgressWatchdog::new(now);
        watchdog.observe(10, "Installing", now);
        watchdog.observe(10, "Installing", now + Duration::from_secs(100));
        assert_eq!(watchdog.deadline, now + WATCH_RESPONSE_TIMEOUT);
        watchdog.observe(11, "Installing", now + Duration::from_secs(170));
        assert_eq!(watchdog.deadline, now + Duration::from_secs(350));
        watchdog.observe(0, "Verifying", now + Duration::from_secs(200));
        assert_eq!(watchdog.deadline, now + Duration::from_secs(380));
    }
}
