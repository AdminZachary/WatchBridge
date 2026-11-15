//! Watch preparation follows oacevedo/xtool's experimental-watchos helper.
//! Native Rust transport keeps Python and pymobiledevice3 out of the application.
//! See THIRD_PARTY_NOTICES.md for source revisions and licenses.
use crate::{
    device::{DeviceInfoMutex, get_provider},
    error::AppError,
    install_lock::InstallLease,
};
use idevice::{
    Idevice, IdeviceError, IdeviceService, lockdown::LockdownClient, pairing_file::PairingFile,
    provider::IdeviceProvider, services::companion_proxy::CompanionProxy,
};
use plist::{Dictionary, Value};
use serde::{Deserialize, Serialize};
use std::{collections::HashMap, future::Future, sync::Mutex, time::Duration};
use tauri::{AppHandle, Manager, State};

const LOCKDOWN: u16 = 62078;
const AMFI: &str = "com.apple.amfi.lockdown";
const SERVICE_TIMEOUT: Duration = Duration::from_secs(10);

// Pairing keys and the Watch identity live in memory only. They are never returned
// to the webview or logged.
#[derive(Clone)]
struct WatchSession {
    pairing: PairingFile,
    name: String,
}

#[derive(Default)]
pub struct WatchSessions(Mutex<HashMap<(String, String), WatchSession>>);

#[derive(Clone, Copy, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "camelCase")]
pub enum Action {
    Check,
    Pair,
    Reveal,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct WatchStatus {
    name: String,
    paired: bool,
    developer_mode: Option<bool>,
    switch_revealed: bool,
}

fn fail(message: &str) -> AppError {
    AppError::Misc(message.into())
}

// Report stage and useful recovery guidance, never raw responses or pairing data.
async fn step<T>(
    message: &str,
    work: impl Future<Output = Result<T, IdeviceError>>,
) -> Result<T, AppError> {
    match tokio::time::timeout(SERVICE_TIMEOUT, work).await {
        Ok(Ok(value)) => Ok(value),
        Ok(Err(IdeviceError::UserDeniedPairing)) => {
            Err(fail("err.watch_denied_retry"))
        }
        Ok(Err(IdeviceError::PasswordProtected | IdeviceError::DeviceLocked)) => {
            Err(fail("err.watch_locked"))
        }
        Ok(Err(error)) => {
            tracing::warn!(target: "watch_setup", kind = %error_kind(&error), "watch step failed");
            Err(fail(message))
        }
        Err(_) => {
            tracing::warn!(target: "watch_setup", step = message, "watch step timed out");
            Err(fail(message))
        }
    }
}

// Error payloads can carry device data, so only the variant name is logged.
fn error_kind(error: &IdeviceError) -> String {
    let rendered = format!("{error:?}");
    rendered
        .split('(')
        .next()
        .unwrap_or("Unknown")
        .chars()
        .take(48)
        .collect()
}

fn single_watch(registry: &[String]) -> Result<&str, AppError> {
    match registry {
        [id] => Ok(id),
        [] => Err(fail(
            "err.watch_none_paired",
        )),
        _ => Err(fail(
            "err.watch_multiple",
        )),
    }
}

fn developer_mode(value: &Value) -> Result<bool, AppError> {
    value
        .as_boolean()
        .ok_or_else(|| fail("err.watch_state_unknown"))
}

fn reveal_request() -> Dictionary {
    let mut request = Dictionary::new();
    // Action 0 reveals the setting. Never send enable/reboot/accept actions 1 or 2.
    request.insert("action".into(), Value::Integer(0.into()));
    request
}

fn reveal_accepted(response: &Dictionary) -> bool {
    response.get("success").and_then(Value::as_boolean) == Some(true)
}

async fn reveal(connection: &mut Idevice) -> Result<(), IdeviceError> {
    let mut body = Vec::new();
    Value::Dictionary(reveal_request()).to_writer_xml(&mut body)?;
    let mut frame = (body.len() as u32).to_be_bytes().to_vec();
    frame.extend_from_slice(&body);
    connection.send_raw(&frame).await?;
    let prefix = connection.read_raw(4).await?;
    let length = u32::from_be_bytes(
        prefix
            .try_into()
            .map_err(|_| IdeviceError::UnexpectedResponse("Invalid AMFI frame".into()))?,
    ) as usize;
    if length > 65536 {
        return Err(IdeviceError::UnexpectedResponse(
            "AMFI response is too large".into(),
        ));
    }
    let response: Dictionary = plist::from_bytes(&connection.read_raw(length).await?)?;
    if reveal_accepted(&response) {
        Ok(())
    } else {
        Err(IdeviceError::UnexpectedResponse(
            "Watch did not reveal the setting".into(),
        ))
    }
}

async fn forward(
    provider: &impl IdeviceProvider,
    port: u16,
    service: &str,
) -> Result<u16, AppError> {
    let mut proxy = step(
        "err.watch_proxy_connect",
        CompanionProxy::connect(provider),
    )
    .await?;
    step(
        "err.watch_forward_unsupported",
        proxy.start_forwarding_service_port(port, Some(service), None),
    )
    .await
}

async fn close_forward(provider: &impl IdeviceProvider, port: u16) -> Result<(), AppError> {
    let mut proxy = step(
        "err.watch_forward_close",
        CompanionProxy::connect(provider),
    )
    .await?;
    step(
        "err.watch_forward_close",
        proxy.stop_forwarding_service_port(port),
    )
    .await
}

async fn connect_forwarded(
    provider: &impl IdeviceProvider,
    port: u16,
) -> Result<Idevice, AppError> {
    // Forwarded ports can appear shortly after CompanionProxy acknowledges them.
    for _ in 0..20 {
        if let Ok(Ok(connection)) =
            tokio::time::timeout(Duration::from_secs(1), provider.connect(port)).await
        {
            return Ok(connection);
        }
        tokio::time::sleep(Duration::from_millis(50)).await;
    }
    Err(fail(
        "err.watch_service_timeout",
    ))
}

// Value queries can miss while the Watch is asleep or busy; retry briefly.
async fn watch_value(lockdown: &mut LockdownClient, key: &str) -> Option<Value> {
    for _ in 0..3 {
        if let Ok(Ok(value)) =
            tokio::time::timeout(SERVICE_TIMEOUT, lockdown.get_value(Some(key), None)).await
        {
            return Some(value);
        }
        tokio::time::sleep(Duration::from_millis(400)).await;
    }
    None
}

// The name is cosmetic, so a miss falls back instead of aborting preparation.
async fn watch_name(lockdown: &mut LockdownClient) -> String {
    match watch_value(lockdown, "DeviceName").await {
        Some(value) => value.as_string().unwrap_or("Apple Watch").to_owned(),
        None => {
            tracing::warn!(target: "watch_setup", "watch name unavailable; continuing");
            "Apple Watch".to_owned()
        }
    }
}

// UniqueDeviceID already proved the target; DeviceClass is a redundant guard.
async fn watch_class(lockdown: &mut LockdownClient) -> Option<String> {
    match watch_value(lockdown, "DeviceClass").await {
        Some(value) => value.as_string().map(str::to_owned),
        None => {
            tracing::warn!(target: "watch_setup", "watch class unavailable; continuing");
            None
        }
    }
}

// Await cleanup after success, errors and local timeouts. A cleanup failure must
// not be reported as a fully successful preparation operation.
async fn finish<T>(
    result: Result<T, AppError>,
    cleanup: impl Future<Output = Result<(), AppError>>,
) -> Result<T, AppError> {
    let cleaned = cleanup.await;
    match result {
        Err(error) => Err(error),
        Ok(value) => cleaned.map(|_| value),
    }
}

#[tauri::command]
pub async fn prepare_watch(
    app: AppHandle,
    device_state: State<'_, DeviceInfoMutex>,
    sessions: State<'_, WatchSessions>,
    operation: State<'_, crate::wifi_pairing::PairingOperation>,
    action: Action,
) -> Result<WatchStatus, AppError> {
    let _selection = operation
        .try_lock()
        .map_err(|_| fail("err.device_busy"))?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| fail("err.app_data_dir"))?;
    let _lease = InstallLease::acquire(&directory)?;
    let selected = device_state
        .lock()
        .unwrap()
        .as_ref()
        .map(|d| d.info.clone())
        .ok_or(AppError::NoDeviceSelected)?;
    if selected.connection_type != "USB" || selected.network_address.is_some() {
        return Err(fail(
            "err.watch_usb_required",
        ));
    }
    let provider = get_provider(&selected).await?;
    let mut proxy = step(
        "err.watch_pair_connect",
        CompanionProxy::connect(&provider),
    )
    .await?;
    let registry = step("err.watch_registry_read", proxy.get_device_registry()).await?;
    let watch_id = single_watch(&registry)?.to_owned();
    drop(proxy);
    let cache_key = (selected.udid, watch_id.clone());
    let cached = sessions.0.lock().unwrap().get(&cache_key).cloned();
    if action == Action::Check && cached.is_none() {
        return Ok(WatchStatus {
            name: "Apple Watch".into(),
            paired: false,
            developer_mode: None,
            switch_revealed: false,
        });
    }
    if action == Action::Reveal && cached.is_none() {
        return Err(fail("err.watch_pair_first"));
    }
    let port = forward(&provider, LOCKDOWN, "com.apple.mobile.lockdownd").await?;
    let result = async {
        let mut lockdown = LockdownClient::new(connect_forwarded(&provider, port).await?);
        // Reuse a session that still authenticates instead of pairing again. A
        // trusted Watch does not need a second pairing round, and repeating it can
        // leave the session in a state where value queries stop answering.
        let mut reused = None;
        if let Some(session) = cached.as_ref() {
            if let Ok(legacy) = step(
                "err.watch_trust_invalid",
                lockdown.start_session(&session.pairing),
            )
            .await
            {
                reused = Some((session.pairing.clone(), legacy));
            }
        }
        let (pairing, legacy) = match reused {
            Some(pair) => pair,
            None => {
                if action != Action::Pair {
                    sessions.0.lock().unwrap().remove(&cache_key);
                    return Err(fail("err.watch_trust_lost"));
                }
                let phone_pairing =
                    step("err.watch_phone_pairing", provider.get_pairing_file()).await?;
                let result = tokio::time::timeout(
                    Duration::from_secs(45),
                    lockdown.pair(
                        phone_pairing.host_id,
                        phone_pairing.system_buid,
                        Some("WatchBridge"),
                    ),
                )
                .await;
                let pairing = match result {
                    Ok(Ok(record)) => record,
                    Ok(Err(IdeviceError::UserDeniedPairing)) => {
                        return Err(fail(
                            "err.watch_pair_denied",
                        ));
                    }
                    _ => return Err(fail("err.watch_pair_incomplete")),
                };
                // A record is admitted only after a successful authenticated session.
                let legacy = match step(
                    "err.watch_trust_invalid",
                    lockdown.start_session(&pairing),
                )
                .await
                {
                    Ok(legacy) => legacy,
                    Err(error) => {
                        sessions.0.lock().unwrap().remove(&cache_key);
                        return Err(error);
                    }
                };
                (pairing, legacy)
            }
        };
        let actual_id = step(
            "err.watch_target_invalid",
            lockdown.get_value(Some("UniqueDeviceID"), None),
        )
        .await?;
        if actual_id.as_string() != Some(watch_id.as_str()) {
            sessions.0.lock().unwrap().remove(&cache_key);
            return Err(fail("err.watch_forward_mismatch"));
        }
        // Read the identity from the forwarded Watch instead of the companion-proxy
        // registry: the registry answers the device list reliably, but not value
        // queries, on current iOS versions.
        let name = watch_name(&mut lockdown).await;
        if let Some(device_class) = watch_class(&mut lockdown).await {
            if device_class != "Watch" {
                sessions.0.lock().unwrap().remove(&cache_key);
                return Err(fail("err.watch_forward_not_watch"));
            }
        }
        sessions.0.lock().unwrap().insert(
            cache_key.clone(),
            WatchSession {
                pairing: pairing.clone(),
                name: name.clone(),
            },
        );
        let enabled = developer_mode(
            &step(
                "err.watch_dev_mode_read",
                lockdown.get_value(
                    Some("DeveloperModeStatus"),
                    Some("com.apple.security.mac.amfi"),
                ),
            )
            .await?,
        )?;
        let mut revealed = false;
        if action == Action::Reveal && !enabled {
            let (remote_port, ssl) =
                step("err.watch_dev_service_start", lockdown.start_service(AMFI)).await?;
            let local_port = forward(&provider, remote_port, AMFI).await?;
            let result = async {
                let mut connection = connect_forwarded(&provider, local_port).await?;
                if ssl {
                    step(
                        "err.watch_secure_session",
                        connection.start_session(&pairing, legacy),
                    )
                    .await?;
                }
                step(
                    "err.watch_switch_not_revealed",
                    reveal(&mut connection),
                )
                .await
            }
            .await;
            finish(result, close_forward(&provider, remote_port)).await?;
            revealed = true;
        }
        Ok(WatchStatus {
            name,
            paired: true,
            developer_mode: Some(enabled),
            switch_revealed: revealed,
        })
    }
    .await;
    finish(result, close_forward(&provider, LOCKDOWN)).await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn ambiguous_watch_targets_are_rejected() {
        assert!(single_watch(&[]).is_err());
        assert!(single_watch(&["first".into(), "second".into()]).is_err());
        assert_eq!(single_watch(&["only".into()]).unwrap(), "only");
    }
    #[test]
    fn mode_is_never_inferred_from_missing_or_malformed_values() {
        assert_eq!(developer_mode(&Value::Boolean(false)).unwrap(), false);
        assert_eq!(developer_mode(&Value::Boolean(true)).unwrap(), true);
        assert!(developer_mode(&Value::String("true".into())).is_err());
        assert!(developer_mode(&Value::Dictionary(Dictionary::new())).is_err());
    }
    #[test]
    fn amfi_only_reveals_and_requires_positive_acknowledgement() {
        assert_eq!(reveal_request().len(), 1);
        assert_eq!(reveal_request()["action"].as_signed_integer(), Some(0));
        assert!(!reveal_accepted(&Dictionary::new()));
        let mut response = Dictionary::new();
        response.insert("success".into(), Value::Boolean(false));
        assert!(!reveal_accepted(&response));
        response.insert("success".into(), Value::Boolean(true));
        assert!(reveal_accepted(&response));
    }
    #[tokio::test]
    async fn cleanup_runs_on_error_and_preserves_the_original_failure() {
        let cleaned = std::sync::atomic::AtomicBool::new(false);
        let result: Result<(), _> = finish(Err(fail("original")), async {
            cleaned.store(true, std::sync::atomic::Ordering::SeqCst);
            Err(fail("cleanup"))
        })
        .await;
        assert!(cleaned.load(std::sync::atomic::Ordering::SeqCst));
        assert_eq!(result.unwrap_err().to_string(), "original");
    }
    #[tokio::test]
    async fn cleanup_failure_cannot_be_reported_as_success() {
        assert!(
            finish(Ok(()), async { Err(fail("cleanup")) })
                .await
                .is_err()
        );
    }
}
