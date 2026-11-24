use isideload::sideload::progress::{InstallProgress, InstallStage};
use serde::Serialize;
use std::{
    sync::Mutex,
    time::{Duration, Instant},
};
use tauri::{Emitter, Window};

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Update<'a> {
    request_id: &'a str,
    #[serde(flatten)]
    progress: InstallProgress,
}

#[derive(Default)]
struct Gate {
    last: Option<(InstallStage, Option<usize>, Option<u64>, Instant)>,
}

impl Gate {
    fn accept(&mut self, update: &InstallProgress, now: Instant) -> bool {
        let accept = self.last.is_none_or(|(stage, index, percent, time)| {
            update.stage != stage
                || update.app_index != index
                || (update.percent == Some(100) && percent != Some(100))
                || now.duration_since(time) >= Duration::from_millis(100)
        });
        if accept {
            self.last = Some((update.stage, update.app_index, update.percent, now));
        }
        accept
    }
}

pub struct Reporter {
    window: Window,
    request_id: String,
    gate: Mutex<Gate>,
}

impl Reporter {
    pub fn new(window: &Window, request_id: String) -> Self {
        Self {
            window: window.clone(),
            request_id,
            gate: Mutex::new(Gate::default()),
        }
    }

    pub fn emit(&self, progress: InstallProgress) {
        let accepted = self.gate.lock().unwrap().accept(&progress, Instant::now());
        if accepted {
            // Observation is best effort; a UI event must never fail/retry an install.
            let _ = self.window.emit(
                "watch-install-progress",
                Update {
                    request_id: &self.request_id,
                    progress,
                },
            );
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn stage_changes_and_completion_bypass_progress_throttling() {
        let mut gate = Gate::default();
        let now = Instant::now();
        assert!(gate.accept(
            &InstallProgress::percent(InstallStage::PhoneInstall, 0),
            now
        ));
        assert!(!gate.accept(
            &InstallProgress::percent(InstallStage::PhoneInstall, 1),
            now
        ));
        assert!(gate.accept(
            &InstallProgress::percent(InstallStage::PhoneInstall, 100),
            now
        ));
        assert!(gate.accept(&InstallProgress::stage(InstallStage::WatchConnect), now));
        assert!(gate.accept(
            &InstallProgress::percent(InstallStage::WatchTransfer, 0),
            now
        ));
        assert!(gate.accept(
            &InstallProgress::percent(InstallStage::WatchTransfer, 5),
            now + Duration::from_millis(100)
        ));
    }

    #[test]
    fn event_has_request_id_and_flat_camel_case_fields() {
        let json = serde_json::to_value(Update {
            request_id: "attempt-2",
            progress: InstallProgress::stage(InstallStage::WatchTransfer),
        })
        .unwrap();
        assert_eq!(json["requestId"], "attempt-2");
        assert_eq!(json["stage"], "watchTransfer");
        assert!(json["percent"].is_null());
        assert!(json.get("bytesSent").is_some());
    }
}
