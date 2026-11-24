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

// The tested uncapped V2 controller is the only Watch transfer policy.
// No environment setting can select a fixed rate or a credit-limited sender.
pub(super) struct TransferPacer {
    controller: super::uncapped::UncappedPacer,
}

impl TransferPacer {
    pub fn new() -> Self {
        Self {
            controller: super::uncapped::UncappedPacer::new(Instant::now()),
        }
    }

    pub fn before_write(&mut self, bytes: usize, now: Instant) -> Option<Instant> {
        self.controller.plan(bytes, now)
    }

    pub fn observe_write(&mut self, bytes: usize, started: Instant, finished: Instant) {
        self.controller.observe(bytes, started, finished);
    }

    pub fn describe(&self, update: &mut InstallProgress) {
        update.transfer_mode = Some("adaptiveUnlimited");
        update.target_kbps = Some(self.controller.target_kbps());
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
