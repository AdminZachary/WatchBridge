use std::sync::{
    Arc, Mutex,
    atomic::{AtomicU64, Ordering},
};
use std::time::Duration;

use isideload::auth::apple_account::{TwoFactorCallbackParams, TwoFactorCallbackResponse};
use rootcause::prelude::*;
use serde::{Deserialize, Serialize};
use tauri::{Emitter, EventId, Listener, Window};
use tokio::sync::oneshot;

static NEXT_REQUEST_ID: AtomicU64 = AtomicU64::new(1);

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
struct Request {
    request_id: String,
    #[serde(flatten)]
    params: TwoFactorCallbackParams,
}

#[derive(Deserialize)]
#[serde(rename_all = "camelCase")]
struct Response {
    request_id: String,
    response: TwoFactorCallbackResponse,
}

struct PendingResponse {
    request_id: String,
    number_ids: Vec<u32>,
    sender: Mutex<Option<oneshot::Sender<TwoFactorCallbackResponse>>>,
}

impl PendingResponse {
    fn accept(&self, payload: &str) -> bool {
        let Ok(mut response) = serde_json::from_str::<Response>(payload) else {
            return false;
        };
        if response.request_id != self.request_id {
            return false;
        }
        match &mut response.response {
            TwoFactorCallbackResponse::SubmitCode(code) => {
                *code = code.trim().to_string();
                if code.len() != 6 || !code.bytes().all(|digit| digit.is_ascii_digit()) {
                    return false;
                }
            }
            TwoFactorCallbackResponse::SendSms(id) if !self.number_ids.contains(id) => {
                return false;
            }
            _ => {}
        }
        let Some(sender) = self.sender.lock().unwrap().take() else {
            return false;
        };
        sender.send(response.response).is_ok()
    }
}

// Drop also runs on an emit error, timeout, or cancelled future.
struct Subscription {
    window: Window,
    event_id: EventId,
    request_id: String,
}

impl Drop for Subscription {
    fn drop(&mut self) {
        self.window.unlisten(self.event_id);
        let _ = self.window.emit("2fa-finished", &self.request_id);
    }
}

pub(super) async fn request(
    window: &Window,
    params: TwoFactorCallbackParams,
) -> Result<TwoFactorCallbackResponse, Report> {
    let request_id = NEXT_REQUEST_ID.fetch_add(1, Ordering::Relaxed).to_string();
    let (sender, receiver) = oneshot::channel();
    let pending = Arc::new(PendingResponse {
        request_id: request_id.clone(),
        number_ids: params.numbers.iter().map(|number| number.id).collect(),
        sender: Mutex::new(Some(sender)),
    });
    // Register before notifying the frontend so even an immediate reply is buffered.
    let event_id = window.listen("2fa-response", move |event| {
        pending.accept(event.payload());
    });
    let _subscription = Subscription {
        window: window.clone(),
        event_id,
        request_id: request_id.clone(),
    };
    window
        .emit("2fa-required", Request { request_id, params })
        .context("Failed to request two-factor verification")?;

    Ok(tokio::time::timeout(Duration::from_secs(120), receiver)
        .await
        .context("Timed out waiting for the two-factor response")?
        .context("Two-factor response channel closed")?)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pending() -> (
        PendingResponse,
        oneshot::Receiver<TwoFactorCallbackResponse>,
    ) {
        let (sender, receiver) = oneshot::channel();
        (
            PendingResponse {
                request_id: "current".into(),
                number_ids: vec![7, 42],
                sender: Mutex::new(Some(sender)),
            },
            receiver,
        )
    }

    #[test]
    fn immediate_responses_are_buffered_and_consumed_once() {
        let (pending, mut receiver) = pending();
        assert!(pending.accept(r#"{"requestId":"current","response":{"SendSms":42}}"#));
        assert!(!pending.accept(r#"{"requestId":"current","response":"Abort"}"#));
        assert!(matches!(
            receiver.try_recv().unwrap(),
            TwoFactorCallbackResponse::SendSms(42)
        ));
    }

    #[test]
    fn stale_malformed_and_unknown_number_responses_do_not_consume_the_request() {
        let (pending, mut receiver) = pending();
        for payload in [
            r#"{"requestId":"old","response":"Abort"}"#,
            r#"{"requestId":"current","response":{"SendSms":1}}"#,
            r#"{"requestId":"current","response":{"SubmitCode":"１２３４５６"}}"#,
            "not json",
        ] {
            assert!(!pending.accept(payload));
        }
        assert!(pending.accept(r#"{"requestId":"current","response":{"SubmitCode":" 123456 "}}"#));
        assert!(
            matches!(receiver.try_recv().unwrap(), TwoFactorCallbackResponse::SubmitCode(code) if code == "123456")
        );
    }

    #[test]
    fn cancellation_resend_and_device_switch_cross_the_bridge() {
        for action in ["Abort", "ResendCode", "SendToDevices"] {
            let (pending, mut receiver) = pending();
            let payload = serde_json::json!({"requestId":"current", "response":action}).to_string();
            assert!(pending.accept(&payload));
            assert_eq!(
                serde_json::to_value(receiver.try_recv().unwrap()).unwrap(),
                action
            );
        }
    }

    #[test]
    fn dropped_receiver_rejects_late_delivery() {
        let (pending, receiver) = pending();
        drop(receiver);
        assert!(!pending.accept(r#"{"requestId":"current","response":"ResendCode"}"#));
    }
}
