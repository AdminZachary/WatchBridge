//! Passive installation observations. These never control protocol completion.
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};

pub type ProgressObserver<'a> = dyn Fn(InstallProgress) + Send + Sync + 'a;

#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub enum InstallStage {
    Preparing,
    Signing,
    PhoneInstall,
    WatchConnect,
    WatchTransfer,
    WatchInstall,
}

#[derive(Clone, Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct InstallProgress {
    pub stage: InstallStage,
    pub percent: Option<u64>,
    pub bytes_sent: Option<u64>,
    pub total_bytes: Option<u64>,
    pub app_index: Option<usize>,
    pub app_count: Option<usize>,
}

impl InstallProgress {
    pub fn stage(stage: InstallStage) -> Self {
        Self {
            stage,
            percent: None,
            bytes_sent: None,
            total_bytes: None,
            app_index: None,
            app_count: None,
        }
    }

    pub fn percent(stage: InstallStage, percent: u64) -> Self {
        Self {
            percent: Some(percent.min(100)),
            ..Self::stage(stage)
        }
    }
}

pub(super) struct TransferProgress {
    total: u64,
    state: Mutex<(u64, Instant)>,
}

impl TransferProgress {
    pub fn new(total: u64) -> Self {
        Self {
            total,
            state: Mutex::new((0, Instant::now())),
        }
    }

    pub fn advance(&self, bytes: usize) -> Option<InstallProgress> {
        let mut state = self.state.lock().unwrap();
        state.0 = state.0.saturating_add(bytes as u64).min(self.total);
        if state.0 != self.total && state.1.elapsed() < Duration::from_millis(100) {
            return None;
        }
        state.1 = Instant::now();
        let mut update = InstallProgress::percent(
            InstallStage::WatchTransfer,
            if self.total == 0 {
                100
            } else {
                ((state.0 as u128 * 100) / self.total as u128) as u64
            },
        );
        update.bytes_sent = Some(state.0);
        update.total_bytes = Some(self.total);
        Some(update)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn throttled_transfer_still_reports_exact_final_bytes_and_clamps_percent() {
        let progress = TransferProgress::new(100_000);
        assert!(progress.advance(32_768).is_none());
        assert!(progress.advance(32_768).is_none());
        let final_update = progress.advance(34_464).unwrap();
        assert_eq!(final_update.percent, Some(100));
        assert_eq!(final_update.bytes_sent, Some(100_000));
        assert_eq!(final_update.total_bytes, Some(100_000));
        assert_eq!(
            InstallProgress::percent(InstallStage::PhoneInstall, 200).percent,
            Some(100)
        );
        assert_eq!(InstallProgress::stage(InstallStage::Signing).percent, None);
    }
}
