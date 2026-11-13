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
            Err(fail("手表拒绝了信任请求。请在手表上确认后重试。"))
        }
        Ok(Err(IdeviceError::PasswordProtected | IdeviceError::DeviceLocked)) => {
            Err(fail("请解锁 iPhone 和 Apple Watch，保持手表亮屏后重试。"))
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
            "这台 iPhone 没有可用的配对手表。请先在 iPhone 的 Watch App 中完成配对。",
        )),
        _ => Err(fail(
            "发现多块配对手表。第一版只支持一块手表，请先在 iPhone 中确认目标手表。",
        )),
    }
}

fn developer_mode(value: &Value) -> Result<bool, AppError> {
    value
        .as_boolean()
        .ok_or_else(|| fail("手表没有返回可识别的开发者模式状态。请重新配对并检查。"))
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
        "无法连接手表转发服务。请保持手表解锁、靠近 iPhone 后重试。",
        CompanionProxy::connect(provider),
    )
    .await?;
    step(
        "无法转发手表服务。这套系统可能需要不同的传输路径。",
        proxy.start_forwarding_service_port(port, Some(service), None),
    )
    .await
}

async fn close_forward(provider: &impl IdeviceProvider, port: u16) -> Result<(), AppError> {
    let mut proxy = step(
        "无法关闭手表转发，请断开并重新连接 iPhone。",
        CompanionProxy::connect(provider),
    )
    .await?;
    step(
        "无法关闭手表转发，请断开并重新连接 iPhone。",
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
        "手表服务连接超时。请保持手表解锁、靠近 iPhone 后重试。",
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
        .map_err(|_| fail("设备正在准备中，请稍后重试。"))?;
    let directory = app
        .path()
        .app_data_dir()
        .map_err(|_| fail("无法访问应用数据目录。"))?;
    let _lease = InstallLease::acquire(&directory)?;
    let selected = device_state
        .lock()
        .unwrap()
        .as_ref()
        .map(|d| d.info.clone())
        .ok_or(AppError::NoDeviceSelected)?;
    if selected.connection_type != "USB" || selected.network_address.is_some() {
        return Err(fail(
            "开发者模式准备需要 USB 连接。请连接 iPhone 数据线并选择 USB 设备。",
        ));
    }
    let provider = get_provider(&selected).await?;
    let mut proxy = step(
        "无法连接配对手表，请检查 iPhone 信任和 USB 连接。",
        CompanionProxy::connect(&provider),
    )
    .await?;
    let registry = step("无法读取配对手表列表。", proxy.get_device_registry()).await?;
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
        return Err(fail("请先点击“配对手表”，在手表上确认信任后再显示开关。"));
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
                "无法验证手表信任，请重新配对手表。",
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
                    return Err(fail("手表信任已失效，请点击“配对手表”重新建立会话。"));
                }
                let phone_pairing =
                    step("无法读取 iPhone 配对信息。", provider.get_pairing_file()).await?;
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
                            "手表拒绝了信任请求。请解锁手表并保持亮屏，等待约 1 分钟后重新点击“配对手表”；若仍然失败，长按侧边按钮与数码表冠重启手表后再试。",
                        ));
                    }
                    _ => return Err(fail("手表配对未完成。请解锁手表并确认信任提示，然后重试。")),
                };
                // A record is admitted only after a successful authenticated session.
                let legacy = match step(
                    "无法验证手表信任，请重新配对手表。",
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
            "无法验证目标手表。",
            lockdown.get_value(Some("UniqueDeviceID"), None),
        )
        .await?;
        if actual_id.as_string() != Some(watch_id.as_str()) {
            sessions.0.lock().unwrap().remove(&cache_key);
            return Err(fail("转发目标与选择的手表不一致，请断开连接后重试。"));
        }
        // Read the identity from the forwarded Watch instead of the companion-proxy
        // registry: the registry answers the device list reliably, but not value
        // queries, on current iOS versions.
        let name = watch_name(&mut lockdown).await;
        if let Some(device_class) = watch_class(&mut lockdown).await {
            if device_class != "Watch" {
                sessions.0.lock().unwrap().remove(&cache_key);
                return Err(fail("转发到的设备不是 Apple Watch，请断开连接后重试。"));
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
                "无法读取开发者模式状态，请解锁手表后重试。",
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
                step("无法启动手表开发者模式服务。", lockdown.start_service(AMFI)).await?;
            let local_port = forward(&provider, remote_port, AMFI).await?;
            let result = async {
                let mut connection = connect_forwarded(&provider, local_port).await?;
                if ssl {
                    step(
                        "无法建立手表安全连接。",
                        connection.start_session(&pairing, legacy),
                    )
                    .await?;
                }
                step(
                    "手表未确认显示开发者模式开关，请检查系统兼容性后重试。",
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
