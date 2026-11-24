use idevice::{
    Idevice, IdeviceService, RsdService,
    pairing_file::PairingFile,
    provider::{IdeviceProvider, RsdProvider},
    rsd::RsdHandshake,
    services::{companion_proxy::CompanionProxy, lockdown::LockdownClient},
};
use plist::{Dictionary, Value};
use rootcause::{option_ext::OptionExt, prelude::*};
use std::{
    path::{Path, PathBuf},
    time::Duration,
};
use tracing::{info, warn};

use crate::{
    SideloadError as Error,
    sideload::{
        bundle::Bundle,
        progress::{InstallProgress, InstallStage, ProgressObserver, TransferProgress},
    },
};

const WATCH_LOCKDOWN_PORT: u16 = 62078;
const WATCH_LOCKDOWN_SERVICE: &str = "com.apple.mobile.lockdownd";
const WATCH_ZIP_SERVICE: &str = "com.apple.streaming_zip_conduit";
const WATCH_FORWARD_CONNECT_ATTEMPTS: usize = 20;
const WATCH_FORWARD_CONNECT_DELAY_MS: u64 = 50;
const WATCH_INSTALL_ATTEMPTS: usize = 7;
const WATCH_INSTALL_INITIAL_RETRY_DELAY_MS: u64 = 400;
const WATCH_INSTALL_MAX_RETRY_DELAY_MS: u64 = 2500;
/// watchOS reports progress while it installs. Without any answer for this long
/// the Watch is unreachable (asleep, out of range) and would otherwise leave the
/// install waiting forever.
const WATCH_RESPONSE_TIMEOUT: Duration = Duration::from_secs(180);
mod adaptive;
mod policy;
use policy::*;

const CENTRAL_DIRECTORY_HEADER: &[u8] = &[0x50, 0x4b, 0x01, 0x02];
const ZIP_EXTRA: &[u8] = &[
    0x55, 0x54, 0x0d, 0x00, 0x07, 0xf3, 0xa2, 0xec, 0x60, 0xf6, 0xa2, 0xec, 0x60, 0xf3, 0xa2, 0xec,
    0x60, 0x75, 0x78, 0x0b, 0x00, 0x01, 0x04, 0xf5, 0x01, 0x00, 0x00, 0x04, 0x14, 0x00, 0x00, 0x00,
];

/// Install or update already-signed embedded Watch apps directly on the paired Apple Watch.
///
/// iPhone installation can temporarily leave a process-scoped install coordinator for the
/// embedded Watch app. Do not uninstall the existing Watch app during a normal update: stream
/// the signed bundle directly, and if watchOS reports the known IXErrorDomain Code=48 coordinator
/// race, retry with a fresh `streaming_zip_conduit` connection after a bounded backoff.
pub async fn install_watch_apps(
    device_provider: &impl IdeviceProvider,
    watch_apps: &[Bundle],
    host_name: &str,
    progress_callback: impl Fn(u64) + Send + Sync,
) -> Result<(), Report> {
    install_watch_apps_observed(
        device_provider,
        watch_apps,
        host_name,
        progress_callback,
        &|_| {},
    )
    .await
}

pub async fn install_watch_apps_observed(
    device_provider: &impl IdeviceProvider,
    watch_apps: &[Bundle],
    host_name: &str,
    progress_callback: impl Fn(u64) + Send + Sync,
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    if watch_apps.is_empty() {
        return Ok(());
    }

    observer(InstallProgress::stage(InstallStage::WatchConnect));
    let mut completed_apps = 0;
    for attempt in 1..=SESSION_ATTEMPTS {
        match install_watch_apps_once(
            device_provider,
            watch_apps,
            host_name,
            &progress_callback,
            observer,
            &mut completed_apps,
        )
        .await
        {
            Err(error) if attempt < SESSION_ATTEMPTS && is_transient_watch_error(&error) => {
                warn!(
                    "Apple Watch install failed (attempt {attempt}/{SESSION_ATTEMPTS}); reconnecting: {error}"
                );
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            result => return result,
        }
    }

    unreachable!("bounded Watch session retry loop must return")
}

async fn install_watch_apps_once(
    device_provider: &impl IdeviceProvider,
    watch_apps: &[Bundle],
    host_name: &str,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
    completed_apps: &mut usize,
) -> Result<(), Report> {
    let iphone_pairing = watch_step(device_provider.get_pairing_file()).await?;

    let forwarded_lockdown_port = {
        let mut companion_proxy = watch_step(CompanionProxy::connect(device_provider))
            .await
            .context(
                "Failed to connect to fresh Apple Watch companion proxy for lockdown forwarding",
            )?;

        watch_step(companion_proxy.start_forwarding_service_port(
            WATCH_LOCKDOWN_PORT,
            Some(WATCH_LOCKDOWN_SERVICE),
            None,
        ))
        .await
        .context("Failed to forward Apple Watch lockdown")?
    };

    let result = async {
        let watch_connection =
            connect_forwarded_watch_port(device_provider, forwarded_lockdown_port, "lockdown")
                .await?;
        let mut watch_lockdown = LockdownClient::new(watch_connection);

        let watch_pairing = bounded_step(
            PAIR_TIMEOUT,
            watch_lockdown.pair(
                iphone_pairing.host_id.clone(),
                iphone_pairing.system_buid.clone(),
                Some(host_name),
            ),
        )
        .await
        .context("Failed to pair with the Apple Watch through companion proxy")
        .context(WatchInstallError::PairingUnconfirmed)?;

        let legacy = watch_step(watch_lockdown.start_session(&watch_pairing))
            .await
            .context("Failed to start Apple Watch lockdown session")?;

        for (index, watch_app) in watch_apps.iter().enumerate().skip(*completed_apps) {
            let positioned = |mut update: InstallProgress| {
                update.app_index = Some(index + 1);
                update.app_count = Some(watch_apps.len());
                observer(update);
            };
            positioned(InstallProgress::stage(InstallStage::WatchConnect));
            install_watch_app_with_retry(
                device_provider,
                &mut watch_lockdown,
                &watch_pairing,
                legacy,
                watch_app,
                progress_callback,
                &positioned,
            )
            .await?;
            *completed_apps += 1;
        }

        Ok(())
    }
    .await;

    match watch_step(CompanionProxy::connect(device_provider)).await {
        Ok(mut companion_proxy) => {
            if let Err(e) =
                watch_step(companion_proxy.stop_forwarding_service_port(WATCH_LOCKDOWN_PORT)).await
            {
                warn!("Failed to stop Apple Watch lockdown forwarding: {e}");
            }
        }
        Err(e) => {
            warn!(
                "Failed to open fresh Apple Watch companion proxy to stop lockdown forwarding: {e}"
            );
        }
    }

    result
}

/// Install or update already-signed embedded Watch apps through an existing RemotePairing/RSD
/// tunnel. The iPhone pairing record is still used to pair/authenticate with the forwarded Watch
/// lockdown service, while CompanionProxy and forwarded Watch sockets stay on the RSD transport.
/// Registers every Apple Watch paired with the iPhone as a development device,
/// over the RSD tunnel. This is the Wi-Fi counterpart of the USB registration in
/// `Sideloader::install_app`; a Watch app can only be signed for registered Watches.
pub async fn register_paired_watches_rsd(
    rsd_provider: &mut impl RsdProvider,
    handshake: &mut RsdHandshake,
    dev_session: &mut crate::dev::developer_session::DeveloperSession,
    team: &crate::dev::teams::DeveloperTeam,
) -> Result<usize, Report> {
    use crate::dev::{device_type::DeveloperDeviceType, devices::DevicesApi};

    let mut companion_proxy = watch_step(CompanionProxy::connect_rsd(rsd_provider, handshake))
        .await
        .context("Failed to connect to Apple Watch companion proxy over RSD")?;
    let paired_watches = watch_step(companion_proxy.get_device_registry())
        .await
        .context("Failed to list paired Apple Watch devices over RSD")?;

    for watch_udid in &paired_watches {
        let watch_name = watch_step(companion_proxy.get_value(watch_udid, "DeviceName"))
            .await
            .ok()
            .and_then(|value| value.as_string().map(str::to_string))
            .unwrap_or_else(|| "Apple Watch".to_string());
        dev_session
            .ensure_device_registered(
                team,
                &watch_name,
                watch_udid,
                Some(DeveloperDeviceType::Watchos),
            )
            .await
            .context("Failed to register paired Apple Watch as a development device")?;
    }
    Ok(paired_watches.len())
}

pub async fn install_watch_apps_rsd(
    rsd_provider: &mut impl RsdProvider,
    handshake: &mut RsdHandshake,
    iphone_pairing: &PairingFile,
    watch_apps: &[Bundle],
    host_name: &str,
    progress_callback: impl Fn(u64) + Send + Sync,
) -> Result<(), Report> {
    install_watch_apps_rsd_observed(
        rsd_provider,
        handshake,
        iphone_pairing,
        watch_apps,
        host_name,
        progress_callback,
        &|_| {},
    )
    .await
}

pub async fn install_watch_apps_rsd_observed(
    rsd_provider: &mut impl RsdProvider,
    handshake: &mut RsdHandshake,
    iphone_pairing: &PairingFile,
    watch_apps: &[Bundle],
    host_name: &str,
    progress_callback: impl Fn(u64) + Send + Sync,
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    if watch_apps.is_empty() {
        return Ok(());
    }
    observer(InstallProgress::stage(InstallStage::WatchConnect));
    let mut completed_apps = 0;
    for attempt in 1..=SESSION_ATTEMPTS {
        match install_watch_apps_rsd_once(
            rsd_provider,
            handshake,
            iphone_pairing,
            watch_apps,
            host_name,
            &progress_callback,
            observer,
            &mut completed_apps,
        )
        .await
        {
            Err(error) if attempt < SESSION_ATTEMPTS && is_transient_watch_error(&error) => {
                warn!(
                    "Apple Watch RSD connection setup failed (attempt {attempt}/{SESSION_ATTEMPTS}); reconnecting"
                );
                tokio::time::sleep(Duration::from_secs(3)).await;
            }
            result => return result,
        }
    }
    unreachable!("bounded Watch RSD session retry loop must return")
}

async fn install_watch_apps_rsd_once(
    rsd_provider: &mut impl RsdProvider,
    handshake: &mut RsdHandshake,
    iphone_pairing: &PairingFile,
    watch_apps: &[Bundle],
    host_name: &str,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
    completed_apps: &mut usize,
) -> Result<(), Report> {
    if watch_apps.is_empty() {
        return Ok(());
    }

    let forwarded_lockdown_port = {
        let mut companion_proxy = watch_step(CompanionProxy::connect_rsd(rsd_provider, handshake)).await
            .context(
                "Failed to connect to fresh Apple Watch RSD companion proxy for lockdown forwarding",
            )?;

        watch_step(companion_proxy.start_forwarding_service_port(
            WATCH_LOCKDOWN_PORT,
            Some(WATCH_LOCKDOWN_SERVICE),
            None,
        ))
        .await
        .context("Failed to forward Apple Watch lockdown over RSD")?
    };

    let result = async {
        let watch_connection =
            connect_forwarded_watch_port_rsd(rsd_provider, forwarded_lockdown_port, "lockdown")
                .await?;
        let mut watch_lockdown = LockdownClient::new(watch_connection);

        let watch_pairing = bounded_step(
            PAIR_TIMEOUT,
            watch_lockdown.pair(
                iphone_pairing.host_id.clone(),
                iphone_pairing.system_buid.clone(),
                Some(host_name),
            ),
        )
        .await
        .context("Failed to pair with the Apple Watch through RSD companion proxy")
        .context(WatchInstallError::PairingUnconfirmed)?;

        let legacy = watch_step(watch_lockdown.start_session(&watch_pairing))
            .await
            .context("Failed to start Apple Watch lockdown session over RSD")?;

        for (index, watch_app) in watch_apps.iter().enumerate().skip(*completed_apps) {
            let positioned = |mut update: InstallProgress| {
                update.app_index = Some(index + 1);
                update.app_count = Some(watch_apps.len());
                observer(update);
            };
            positioned(InstallProgress::stage(InstallStage::WatchConnect));
            install_watch_app_with_retry_rsd(
                rsd_provider,
                handshake,
                &mut watch_lockdown,
                &watch_pairing,
                legacy,
                watch_app,
                progress_callback,
                &positioned,
            )
            .await?;
            *completed_apps += 1;
        }

        Ok(())
    }
    .await;

    match watch_step(CompanionProxy::connect_rsd(rsd_provider, handshake)).await {
        Ok(mut companion_proxy) => {
            if let Err(e) =
                watch_step(companion_proxy.stop_forwarding_service_port(WATCH_LOCKDOWN_PORT)).await
            {
                warn!("Failed to stop Apple Watch RSD lockdown forwarding: {e}");
            }
        }
        Err(e) => {
            warn!(
                "Failed to open fresh Apple Watch RSD companion proxy to stop lockdown forwarding: {e}"
            );
        }
    }

    result
}

async fn connect_forwarded_watch_port(
    device_provider: &impl IdeviceProvider,
    forwarded_port: u16,
    service: &str,
) -> Result<Idevice, Report> {
    bounded_operation(async {
        for attempt in 1..=WATCH_FORWARD_CONNECT_ATTEMPTS {
            match device_provider.connect(forwarded_port).await {
                Ok(connection) => {
                    if attempt > 1 {
                        info!(
                            "Connected to Apple Watch {} on forwarded port {} after {} attempts",
                            service, forwarded_port, attempt
                        );
                    }
                    return Ok(connection);
                }
                Err(error) if attempt < WATCH_FORWARD_CONNECT_ATTEMPTS => {
                    warn!(
                        "Apple Watch {} forwarded port {} is not ready (attempt {}/{}): {}",
                        service, forwarded_port, attempt, WATCH_FORWARD_CONNECT_ATTEMPTS, error
                    );
                    tokio::time::sleep(Duration::from_millis(WATCH_FORWARD_CONNECT_DELAY_MS)).await;
                }
                Err(error) => {
                    return Err(report!(Error::IdeviceError(error)).context(format!(
                        "Failed to connect to Apple Watch {service} on forwarded iPhone port {forwarded_port} after {WATCH_FORWARD_CONNECT_ATTEMPTS} attempts"
                    )).into());
                }
            }
        }

        unreachable!("bounded forwarded-port retry loop must return")
    }).await
}

async fn connect_forwarded_watch_port_rsd(
    rsd_provider: &mut impl RsdProvider,
    forwarded_port: u16,
    service: &str,
) -> Result<Idevice, Report> {
    bounded_operation(async {
        for attempt in 1..=WATCH_FORWARD_CONNECT_ATTEMPTS {
            match rsd_provider.connect_to_service_port(forwarded_port).await {
                Ok(stream) => {
                    if attempt > 1 {
                        info!(
                            "Connected to Apple Watch {} over RSD on forwarded port {} after {} attempts",
                            service, forwarded_port, attempt
                        );
                    }
                    return Ok(Idevice::new(stream, "isideload-watch-rsd"));
                }
                Err(error) if attempt < WATCH_FORWARD_CONNECT_ATTEMPTS => {
                    warn!(
                        "Apple Watch {} RSD forwarded port {} is not ready (attempt {}/{}): {}",
                        service, forwarded_port, attempt, WATCH_FORWARD_CONNECT_ATTEMPTS, error
                    );
                    tokio::time::sleep(Duration::from_millis(WATCH_FORWARD_CONNECT_DELAY_MS)).await;
                }
                Err(error) => {
                    return Err(report!(Error::IdeviceError(error)).context(format!(
                        "Failed to connect to Apple Watch {service} on RSD forwarded iPhone port {forwarded_port} after {WATCH_FORWARD_CONNECT_ATTEMPTS} attempts"
                    )).into());
                }
            }
        }

        unreachable!("bounded RSD forwarded-port retry loop must return")
    }).await
}

async fn install_watch_app_with_retry(
    device_provider: &impl IdeviceProvider,
    watch_lockdown: &mut LockdownClient,
    watch_pairing: &PairingFile,
    legacy: bool,
    watch_app: &Bundle,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    let bundle_id = watch_app
        .bundle_identifier()
        .ok_or_report()
        .context("Watch app is missing CFBundleIdentifier")?;
    let mut retry_delay_ms = WATCH_INSTALL_INITIAL_RETRY_DELAY_MS;

    for attempt in 1..=WATCH_INSTALL_ATTEMPTS {
        match install_watch_app_zip_conduit(
            device_provider,
            watch_lockdown,
            watch_pairing,
            legacy,
            watch_app,
            progress_callback,
            observer,
        )
        .await
        {
            Ok(()) => {
                if attempt > 1 {
                    info!("Apple Watch app {bundle_id} installed/updated after {attempt} attempts");
                }
                return Ok(());
            }
            Err(error)
                if is_watch_coordinator_conflict(&error) && attempt < WATCH_INSTALL_ATTEMPTS =>
            {
                warn!(
                    "Apple Watch install coordinator still busy for {bundle_id} (attempt {attempt}/{WATCH_INSTALL_ATTEMPTS}); retrying with a fresh streaming_zip_conduit connection in {retry_delay_ms} ms: {error:?}"
                );
                tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
                retry_delay_ms = retry_delay_ms
                    .saturating_mul(2)
                    .min(WATCH_INSTALL_MAX_RETRY_DELAY_MS);
            }
            Err(error) => {
                warn!(
                    "Apple Watch install/update failed for {bundle_id} on attempt {attempt}/{WATCH_INSTALL_ATTEMPTS}; the existing Watch app was not explicitly uninstalled"
                );
                return Err(error);
            }
        }
    }

    unreachable!("bounded Watch install retry loop must return")
}

async fn install_watch_app_with_retry_rsd(
    rsd_provider: &mut impl RsdProvider,
    handshake: &mut RsdHandshake,
    watch_lockdown: &mut LockdownClient,
    watch_pairing: &PairingFile,
    legacy: bool,
    watch_app: &Bundle,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    let bundle_id = watch_app
        .bundle_identifier()
        .ok_or_report()
        .context("Watch app is missing CFBundleIdentifier")?;
    let mut retry_delay_ms = WATCH_INSTALL_INITIAL_RETRY_DELAY_MS;

    for attempt in 1..=WATCH_INSTALL_ATTEMPTS {
        match install_watch_app_zip_conduit_rsd(
            rsd_provider,
            handshake,
            watch_lockdown,
            watch_pairing,
            legacy,
            watch_app,
            progress_callback,
            observer,
        )
        .await
        {
            Ok(()) => {
                if attempt > 1 {
                    info!(
                        "Apple Watch app {bundle_id} installed/updated over RSD after {attempt} attempts"
                    );
                }
                return Ok(());
            }
            Err(error)
                if is_watch_coordinator_conflict(&error) && attempt < WATCH_INSTALL_ATTEMPTS =>
            {
                warn!(
                    "Apple Watch RSD install coordinator still busy for {bundle_id} (attempt {attempt}/{WATCH_INSTALL_ATTEMPTS}); retrying with a fresh streaming_zip_conduit connection in {retry_delay_ms} ms: {error:?}"
                );
                tokio::time::sleep(Duration::from_millis(retry_delay_ms)).await;
                retry_delay_ms = retry_delay_ms
                    .saturating_mul(2)
                    .min(WATCH_INSTALL_MAX_RETRY_DELAY_MS);
            }
            Err(error) => {
                warn!(
                    "Apple Watch RSD install/update failed for {bundle_id} on attempt {attempt}/{WATCH_INSTALL_ATTEMPTS}; the existing Watch app was not explicitly uninstalled"
                );
                return Err(error);
            }
        }
    }

    unreachable!("bounded Watch RSD install retry loop must return")
}

fn is_watch_coordinator_conflict(error: &Report) -> bool {
    let message = format!("{error:?}\n{error}");
    message.contains("IXErrorDomain Code=48")
        || message.contains("A coordinated install exists for this identity")
        || (message.contains("process-scoped") && message.contains("scoped to a different process"))
}

async fn install_watch_app_zip_conduit(
    device_provider: &impl IdeviceProvider,
    watch_lockdown: &mut LockdownClient,
    watch_pairing: &PairingFile,
    legacy: bool,
    watch_app: &Bundle,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    let (remote_port, ssl) = watch_step(watch_lockdown.start_service(WATCH_ZIP_SERVICE))
        .await
        .context("Failed to start Apple Watch streaming_zip_conduit")?;

    let forwarded_port = {
        let mut companion_proxy = watch_step(CompanionProxy::connect(device_provider)).await
            .context("Failed to connect to fresh Apple Watch companion proxy for streaming_zip_conduit forwarding")?;

        watch_step(companion_proxy.start_forwarding_service_port(
            remote_port,
            Some(WATCH_ZIP_SERVICE),
            Some(prefer_wifi()),
        ))
        .await
        .context("Failed to forward Apple Watch streaming_zip_conduit")?
    };

    let result = async {
        let mut connection =
            connect_forwarded_watch_port(device_provider, forwarded_port, "streaming_zip_conduit")
                .await?;

        if ssl {
            watch_step(connection.start_session(watch_pairing, legacy))
                .await
                .context("Failed to secure Apple Watch streaming_zip_conduit connection")?;
        }

        transfer_result(
            stream_watch_app(&mut connection, watch_app, progress_callback, observer).await,
        )
    }
    .await;

    match watch_step(CompanionProxy::connect(device_provider)).await {
        Ok(mut companion_proxy) => {
            if let Err(e) =
                watch_step(companion_proxy.stop_forwarding_service_port(remote_port)).await
            {
                warn!("Failed to stop Apple Watch streaming_zip_conduit forwarding: {e}");
            }
        }
        Err(e) => {
            warn!(
                "Failed to open fresh Apple Watch companion proxy to stop streaming_zip_conduit forwarding: {e}"
            );
        }
    }

    result
}

async fn install_watch_app_zip_conduit_rsd(
    rsd_provider: &mut impl RsdProvider,
    handshake: &mut RsdHandshake,
    watch_lockdown: &mut LockdownClient,
    watch_pairing: &PairingFile,
    legacy: bool,
    watch_app: &Bundle,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    let (remote_port, ssl) = watch_step(watch_lockdown.start_service(WATCH_ZIP_SERVICE))
        .await
        .context("Failed to start Apple Watch streaming_zip_conduit over RSD")?;

    let forwarded_port = {
        let mut companion_proxy = watch_step(CompanionProxy::connect_rsd(rsd_provider, handshake)).await
            .context(
                "Failed to connect to fresh Apple Watch RSD companion proxy for streaming_zip_conduit forwarding",
            )?;

        watch_step(companion_proxy.start_forwarding_service_port(
            remote_port,
            Some(WATCH_ZIP_SERVICE),
            Some(prefer_wifi()),
        ))
        .await
        .context("Failed to forward Apple Watch streaming_zip_conduit over RSD")?
    };

    let result = async {
        let mut connection =
            connect_forwarded_watch_port_rsd(rsd_provider, forwarded_port, "streaming_zip_conduit")
                .await?;

        if ssl {
            watch_step(connection.start_session(watch_pairing, legacy))
                .await
                .context("Failed to secure Apple Watch RSD streaming_zip_conduit connection")?;
        }

        transfer_result(
            stream_watch_app(&mut connection, watch_app, progress_callback, observer).await,
        )
    }
    .await;

    match watch_step(CompanionProxy::connect_rsd(rsd_provider, handshake)).await {
        Ok(mut companion_proxy) => {
            if let Err(e) =
                watch_step(companion_proxy.stop_forwarding_service_port(remote_port)).await
            {
                warn!("Failed to stop Apple Watch RSD streaming_zip_conduit forwarding: {e}");
            }
        }
        Err(e) => {
            warn!(
                "Failed to open fresh Apple Watch RSD companion proxy to stop streaming_zip_conduit forwarding: {e}"
            );
        }
    }

    result
}

async fn stream_watch_app(
    connection: &mut Idevice,
    watch_app: &Bundle,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    let app_name = watch_app
        .bundle_dir
        .file_name()
        .ok_or_report()
        .context("Watch app path has no file name")?
        .to_string_lossy()
        .to_string();

    let mut files = Vec::new();
    collect_files(&watch_app.bundle_dir, &mut files)?;
    files.sort();

    let mut entries = Vec::with_capacity(files.len());
    let mut total_uncompressed = 0u64;

    for source in files {
        let relative = source
            .strip_prefix(&watch_app.bundle_dir)
            .context("Failed to create relative Watch app path")?;
        let relative = relative.to_string_lossy().replace('\\', "/");
        let destination = format!("Payload/{app_name}/{relative}");
        let size = isideload_vfs::fs::metadata(&source)?.len();
        total_uncompressed = total_uncompressed.saturating_add(size);
        entries.push((source, destination));
    }

    let media_subdir = format!("PublicStaging/{app_name}.ipa");
    let mut install_options = Dictionary::new();
    insert_integer(&mut install_options, "DisableDeltaTransfer", 1);
    insert_string(
        &mut install_options,
        "InstallDeltaTypeKey",
        "InstallDeltaTypeSparseIPAFiles",
    );
    insert_integer(&mut install_options, "IsUserInitiated", 1);
    insert_string(&mut install_options, "PackageType", "Customer");
    insert_integer(&mut install_options, "PreferWifi", 1);

    let mut init_transfer = Dictionary::new();
    init_transfer.insert(
        "InstallOptionsDictionary".to_string(),
        Value::Dictionary(install_options),
    );
    insert_integer(&mut init_transfer, "InstallTransferredDirectory", 1);
    insert_string(&mut init_transfer, "MediaSubdir", media_subdir);
    insert_integer(&mut init_transfer, "UserInitiatedTransfer", 0);

    send_prefixed_plist(connection, init_transfer).await?;

    let mut metadata = Dictionary::new();
    insert_integer(&mut metadata, "StandardDirectoryPerms", 16877);
    insert_integer(&mut metadata, "StandardFilePerms", -32348);
    insert_integer(
        &mut metadata,
        "RecordCount",
        i64::try_from(entries.len() + 2).context("Too many Watch app files")?,
    );
    insert_integer(
        &mut metadata,
        "TotalUncompressedBytes",
        i64::try_from(total_uncompressed).context("Watch app is too large")?,
    );
    insert_integer(&mut metadata, "Version", 2);

    let mut metadata_bytes = Vec::new();
    Value::Dictionary(metadata).to_writer_xml(&mut metadata_bytes)?;

    let transfer_progress = TransferProgress::new(total_uncompressed);
    let config = TransferConfig::from_env();
    let mut pacer = TransferPacer::new(config);
    let mut initial = InstallProgress::percent(InstallStage::WatchTransfer, 0);
    initial.bytes_sent = Some(0);
    initial.total_bytes = Some(total_uncompressed);
    pacer.describe(&mut initial);
    observer(initial);
    if config.adaptive {
        info!(
            "Apple Watch adaptive transfer settings: credit refill {} KiB/s, burst allowance {} bytes, peak ceiling {} KiB/s",
            config.kbps, config.unpaced_bytes, config.max_kbps
        );
    } else {
        info!(
            "Apple Watch transfer settings: {} KiB/s (0 = unlimited), {} unpaced bytes",
            config.kbps, config.unpaced_bytes
        );
    }
    info!(
        "Apple Watch transfer mode: {}; adaptive peak ceiling {} KiB/s; credit refill {} KiB/s",
        if config.adaptive { "adaptive" } else { "fixed" },
        config.max_kbps,
        config.kbps
    );
    send_zip_directory(connection, "META-INF/").await?;
    send_zip_file(
        connection,
        "META-INF/com.apple.ZipMetadata.plist",
        &metadata_bytes,
        &mut pacer,
        None,
    )
    .await?;

    let mut sent = 0u64;
    let mut logged = 0u64;
    for (source, destination) in entries {
        let data = isideload_vfs::fs::read(&source)?;
        send_zip_file(
            connection,
            &destination,
            &data,
            &mut pacer,
            Some((&transfer_progress, observer)),
        )
        .await?;
        sent += data.len() as u64;
        let percent = sent * 100 / total_uncompressed.max(1);
        if percent >= logged + 5 {
            logged = percent;
            info!("Sending Apple Watch app: {percent}%");
        }
    }

    watch_step(connection.send_raw(CENTRAL_DIRECTORY_HEADER)).await?;

    observer(InstallProgress::stage(InstallStage::WatchInstall));
    wait_for_watch_install(connection, progress_callback, observer).await
}

async fn wait_for_watch_install(
    connection: &mut Idevice,
    progress_callback: &(impl Fn(u64) + Send + Sync),
    observer: &ProgressObserver<'_>,
) -> Result<(), Report> {
    let mut watchdog = ProgressWatchdog::new(tokio::time::Instant::now());
    loop {
        let response = tokio::time::timeout_at(watchdog.deadline, read_prefixed_plist(connection))
            .await
            .map_err(|_| report!(WatchInstallError::ProgressStalled))??;

        if response
            .get("Status")
            .and_then(Value::as_string)
            .is_some_and(|status| status == "DataComplete")
        {
            progress_callback(100);
            observer(InstallProgress::percent(InstallStage::WatchInstall, 100));
            info!("Apple Watch installation completed");
            return Ok(());
        }

        let progress = response
            .get("InstallProgressDict")
            .and_then(Value::as_dictionary)
            .ok_or_else(|| report!("Unexpected Apple Watch install response: {response:?}"))?;

        if let Some(error) = progress.get("Error").and_then(Value::as_string) {
            let description = progress
                .get("ErrorDescription")
                .and_then(Value::as_string)
                .unwrap_or("No ErrorDescription returned by watchOS");
            bail!("Apple Watch installation failed: {error}: {description}");
        }

        let percent = progress
            .get("PercentComplete")
            .and_then(Value::as_unsigned_integer)
            .unwrap_or(0);
        let status = progress
            .get("Status")
            .and_then(Value::as_string)
            .unwrap_or("Unknown");

        watchdog.observe(percent, status, tokio::time::Instant::now());
        info!("Installing Apple Watch app: {percent}% {status}");
        progress_callback(percent);
        observer(InstallProgress::percent(
            InstallStage::WatchInstall,
            percent,
        ));
    }
}

fn collect_files(dir: &Path, files: &mut Vec<PathBuf>) -> Result<(), Report> {
    for entry in isideload_vfs::fs::read_dir(dir)? {
        let entry = entry?;
        let path = entry.path();
        let metadata = isideload_vfs::fs::metadata(&path)?;

        if metadata.is_dir() {
            collect_files(&path, files)?;
        } else if metadata.is_file() {
            files.push(path);
        }
    }

    Ok(())
}

async fn send_prefixed_plist(
    connection: &mut Idevice,
    dictionary: Dictionary,
) -> Result<(), Report> {
    let mut payload = Vec::new();
    Value::Dictionary(dictionary).to_writer_xml(&mut payload)?;
    let payload_len = u32::try_from(payload.len()).context("Watch plist is too large")?;

    let mut frame = Vec::with_capacity(payload.len() + 4);
    frame.extend_from_slice(&payload_len.to_be_bytes());
    frame.extend_from_slice(&payload);

    watch_step(connection.send_raw(&frame)).await?;
    Ok(())
}

async fn read_prefixed_plist(connection: &mut Idevice) -> Result<Dictionary, Report> {
    let len = connection.read_raw(4).await.map_err(Error::IdeviceError)?;
    let payload_len = u32::from_be_bytes([len[0], len[1], len[2], len[3]]) as usize;
    let payload = connection
        .read_raw(payload_len)
        .await
        .map_err(Error::IdeviceError)?;
    let value: Value = plist::from_bytes(&payload)?;

    match value {
        Value::Dictionary(dictionary) => Ok(dictionary),
        other => bail!("Expected dictionary plist from Apple Watch, got {other:?}"),
    }
}

async fn send_zip_directory(connection: &mut Idevice, name: &str) -> Result<(), Report> {
    let header = zip_header(name, 0, 0)?;
    watch_step(connection.send_raw(&header)).await?;
    Ok(())
}

async fn send_zip_file(
    connection: &mut Idevice,
    name: &str,
    data: &[u8],
    pacer: &mut TransferPacer,
    progress: Option<(&TransferProgress, &ProgressObserver<'_>)>,
) -> Result<(), Report> {
    let size = u32::try_from(data.len()).context("Watch app file is too large")?;
    let header = zip_header(name, size, crc32_ieee(data))?;

    watch_step(connection.send_raw(&header)).await?;
    for chunk in data.chunks(SEND_CHUNK) {
        if let Some(deadline) = pacer.before_write(chunk.len(), tokio::time::Instant::now()) {
            tokio::time::sleep_until(deadline).await;
        }
        let started = tokio::time::Instant::now();
        let deadline = pacer.next_deadline(chunk.len(), started);
        watch_step(connection.send_raw(chunk)).await?;
        pacer.observe_write(started, tokio::time::Instant::now());
        if let Some((tracker, observer)) = progress {
            if let Some(mut update) = tracker.advance(chunk.len()) {
                pacer.describe(&mut update);
                observer(update);
            }
        }
        if let Some(deadline) = deadline {
            tokio::time::sleep_until(deadline).await;
        }
    }
    Ok(())
}

fn zip_header(name: &str, size: u32, crc32: u32) -> Result<Vec<u8>, Report> {
    let name_bytes = name.as_bytes();
    let name_len = u16::try_from(name_bytes.len()).context("Watch zip path is too long")?;
    let extra_len = u16::try_from(ZIP_EXTRA.len()).context("Watch zip extra data is too large")?;

    let mut header = Vec::with_capacity(30 + name_bytes.len() + ZIP_EXTRA.len());
    push_u32_le(&mut header, 0x04034b50);
    push_u16_le(&mut header, 20);
    push_u16_le(&mut header, 0);
    push_u16_le(&mut header, 0);
    push_u16_le(&mut header, 0xbdef);
    push_u16_le(&mut header, 0x52ec);
    push_u32_le(&mut header, crc32);
    push_u32_le(&mut header, size);
    push_u32_le(&mut header, size);
    push_u16_le(&mut header, name_len);
    push_u16_le(&mut header, extra_len);
    header.extend_from_slice(name_bytes);
    header.extend_from_slice(ZIP_EXTRA);
    Ok(header)
}

fn push_u16_le(buffer: &mut Vec<u8>, value: u16) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn push_u32_le(buffer: &mut Vec<u8>, value: u32) {
    buffer.extend_from_slice(&value.to_le_bytes());
}

fn insert_string(dictionary: &mut Dictionary, key: &str, value: impl Into<String>) {
    dictionary.insert(key.to_string(), Value::String(value.into()));
}

fn prefer_wifi() -> Dictionary {
    let mut options = Dictionary::new();
    options.insert("PreferWifi".to_string(), Value::Boolean(true));
    options
}

fn insert_integer(dictionary: &mut Dictionary, key: &str, value: i64) {
    dictionary.insert(key.to_string(), Value::Integer(value.into()));
}

fn crc32_ieee(data: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;

    for byte in data {
        crc ^= u32::from(*byte);
        for _ in 0..8 {
            let mask = (crc & 1).wrapping_neg();
            crc = (crc >> 1) ^ (0xedb8_8320 & mask);
        }
    }

    !crc
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::{AsyncReadExt, AsyncWriteExt};

    #[derive(Debug)]
    struct FailingSetup {
        calls: std::sync::atomic::AtomicUsize,
        transient: bool,
    }

    impl IdeviceProvider for FailingSetup {
        fn connect(
            &self,
            _: u16,
        ) -> std::pin::Pin<
            Box<dyn std::future::Future<Output = Result<Idevice, idevice::IdeviceError>> + Send>,
        > {
            panic!("a failed pairing-file lookup must not open device services");
        }

        fn label(&self) -> &str {
            "test-setup"
        }

        fn get_pairing_file(
            &self,
        ) -> std::pin::Pin<
            Box<
                dyn std::future::Future<Output = Result<PairingFile, idevice::IdeviceError>> + Send,
            >,
        > {
            let attempt = self
                .calls
                .fetch_add(1, std::sync::atomic::Ordering::Relaxed);
            let transient = self.transient;
            Box::pin(async move {
                if transient && attempt == 0 {
                    Err(idevice::IdeviceError::NoEstablishedConnection)
                } else {
                    Err(idevice::IdeviceError::UserDeniedPairing)
                }
            })
        }
    }

    #[tokio::test]
    async fn setup_reconnects_only_on_transient_failure_and_stops_on_denial() {
        let root = std::env::temp_dir().join(format!("watch-retry-{}", uuid::Uuid::new_v4()));
        std::fs::create_dir_all(&root).unwrap();
        let mut info = Dictionary::new();
        insert_string(&mut info, "CFBundleIdentifier", "com.example.retry");
        plist::to_file_xml(root.join("Info.plist"), &info).unwrap();
        let app = Bundle::new(root.clone()).unwrap();
        std::fs::remove_dir_all(root).unwrap();
        for (transient, expected_calls) in [(false, 1), (true, 2)] {
            let provider = FailingSetup {
                calls: std::sync::atomic::AtomicUsize::new(0),
                transient,
            };
            assert!(
                install_watch_apps(&provider, std::slice::from_ref(&app), "test", |_| panic!(
                    "setup failure cannot emit install progress"
                ))
                .await
                .is_err()
            );
            assert_eq!(
                provider.calls.load(std::sync::atomic::Ordering::Relaxed),
                expected_calls
            );
        }
    }

    #[tokio::test]
    async fn chunked_transfer_preserves_zip_headers_and_every_byte() {
        let (writer, mut reader) = tokio::io::duplex(1024);
        let mut connection = Idevice::new(Box::new(writer), "test-watch-transfer");
        let files = [
            ("Payload/Watch.app/Watch", vec![0x91; SEND_CHUNK * 3 + 17]),
            ("Payload/Watch.app/Info.plist", vec![0x23; 77]),
        ];
        let mut expected = Vec::new();
        for (name, data) in &files {
            expected.extend(zip_header(name, data.len() as u32, crc32_ieee(data)).unwrap());
            expected.extend(data);
        }
        let receive = tokio::spawn(async move {
            let mut bytes = Vec::new();
            reader.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        let mut pacer = TransferPacer::new(TransferConfig {
            kbps: 0,
            unpaced_bytes: 0,
            adaptive: false,
            max_kbps: 4096,
        });
        let total = files.iter().map(|(_, data)| data.len() as u64).sum();
        let tracker = TransferProgress::new(total);
        let updates = std::sync::Mutex::new(Vec::new());
        let observe = |update| updates.lock().unwrap().push(update);
        for (name, data) in files {
            send_zip_file(
                &mut connection,
                name,
                &data,
                &mut pacer,
                Some((&tracker, &observe)),
            )
            .await
            .unwrap();
        }
        drop(connection);
        assert_eq!(receive.await.unwrap(), expected);
        let updates = updates.into_inner().unwrap();
        let last = updates.last().unwrap();
        assert_eq!(last.stage, InstallStage::WatchTransfer);
        assert_eq!(last.percent, Some(100));
        assert_eq!(last.bytes_sent, Some(total));
        assert_eq!(last.total_bytes, Some(total));
    }

    #[tokio::test]
    async fn adaptive_pressure_reduces_target_without_replaying_or_changing_wire_bytes() {
        let data = vec![0x42; SEND_CHUNK * 3 + 7];
        let name = "Payload/Watch.app/resource.bin";
        let header = zip_header(name, data.len() as u32, crc32_ieee(&data)).unwrap();
        let mut expected = header.clone();
        expected.extend(&data);
        let (writer, mut reader) = tokio::io::duplex(1024);
        let mut connection = Idevice::new(Box::new(writer), "adaptive-test");
        let receive = tokio::spawn(async move {
            let mut bytes = vec![0; header.len()];
            reader.read_exact(&mut bytes).await.unwrap();
            // Deliberately stop draining the socket during the first body write.
            tokio::time::sleep(Duration::from_millis(200)).await;
            reader.read_to_end(&mut bytes).await.unwrap();
            bytes
        });
        let tracker = TransferProgress::new(data.len() as u64);
        let updates = std::sync::Mutex::new(Vec::new());
        let observe = |update| updates.lock().unwrap().push(update);
        let mut pacer = TransferPacer::new(TransferConfig {
            kbps: 512,
            unpaced_bytes: 32 * 1024 * 1024,
            adaptive: true,
            max_kbps: 4096,
        });
        send_zip_file(
            &mut connection,
            name,
            &data,
            &mut pacer,
            Some((&tracker, &observe)),
        )
        .await
        .unwrap();
        drop(connection);
        assert_eq!(receive.await.unwrap(), expected);
        let updates = updates.into_inner().unwrap();
        assert!(
            updates
                .iter()
                .any(|update| update.target_kbps.is_some_and(|rate| rate < 4096))
        );
        let last = updates.last().unwrap();
        assert_eq!(last.transfer_mode, Some("adaptive"));
        assert_eq!(last.percent, Some(100));
        assert_eq!(last.bytes_sent, Some(data.len() as u64));
    }

    async fn install_response(response: Dictionary) -> (Result<(), Report>, Vec<u64>) {
        let (reader, mut writer) = tokio::io::duplex(4096);
        let mut connection = Idevice::new(Box::new(reader), "test-watch-result");
        let mut payload = Vec::new();
        Value::Dictionary(response)
            .to_writer_xml(&mut payload)
            .unwrap();
        writer
            .write_all(&(payload.len() as u32).to_be_bytes())
            .await
            .unwrap();
        writer.write_all(&payload).await.unwrap();
        drop(writer);
        let progress = std::sync::Mutex::new(Vec::new());
        let result = wait_for_watch_install(
            &mut connection,
            &|percent| progress.lock().unwrap().push(percent),
            &|_| {},
        )
        .await;
        (result, progress.into_inner().unwrap())
    }

    #[tokio::test]
    async fn only_explicit_data_complete_reports_install_success() {
        let mut complete = Dictionary::new();
        insert_string(&mut complete, "Status", "DataComplete");
        let (result, progress) = install_response(complete).await;
        result.unwrap();
        assert_eq!(progress, vec![100]);

        let mut incomplete = Dictionary::new();
        let mut progress = Dictionary::new();
        insert_integer(&mut progress, "PercentComplete", 50);
        insert_string(&mut progress, "Status", "Installing");
        incomplete.insert("InstallProgressDict".into(), Value::Dictionary(progress));
        let (result, progress) = install_response(incomplete).await;
        let error = transfer_result(result).unwrap_err();
        assert!(!progress.contains(&100));
        assert!(!is_transient_watch_error(&error));
        assert!(error.iter_reports().any(|cause| matches!(
            cause.downcast_current_context::<WatchInstallError>(),
            Some(WatchInstallError::TransferUnconfirmed)
        )));
    }

    #[tokio::test]
    async fn explicit_rejection_is_preserved_and_never_replayed() {
        let mut response = Dictionary::new();
        let mut progress = Dictionary::new();
        insert_string(&mut progress, "Error", "ApplicationVerificationFailed");
        insert_string(&mut progress, "ErrorDescription", "invalid test signature");
        response.insert("InstallProgressDict".into(), Value::Dictionary(progress));
        let (result, progress) = install_response(response).await;
        let error = transfer_result(result).unwrap_err();
        assert!(format!("{error:?}").contains("ApplicationVerificationFailed"));
        assert!(!is_transient_watch_error(&error));
        assert!(progress.is_empty());
    }

    #[test]
    fn explicit_coordinator_rejection_keeps_existing_retry_behavior() {
        let error = transfer_result(Err(report!(
            "IXErrorDomain Code=48: A coordinated install exists for this identity"
        )
        .into()))
        .unwrap_err();
        assert!(is_watch_coordinator_conflict(&error));
        assert!(!is_transient_watch_error(&error));
    }
}
