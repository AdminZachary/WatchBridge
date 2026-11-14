mod state;
mod websocket;

use std::sync::Arc;
use web_time::SystemTime;

use base64::prelude::*;
use plist_macro::plist;
use reqwest::{
    ClientBuilder,
    header::{CONTENT_TYPE, HeaderMap, HeaderValue},
};
use reqwest_middleware::ClientBuilder as MwClientBuilder;
use rootcause::option_ext::OptionExt;
use rootcause::prelude::*;
use serde::Deserialize;
use tracing::{debug, info, warn};

use crate::anisette::remote_v3::{state::AnisetteState, websocket::WsMessage};
use crate::anisette::{AnisetteClientInfo, AnisetteData, AnisetteProvider};
use crate::auth::grandslam::GrandSlam;
use crate::util::plist::PlistDataExtract;
use crate::util::storage::{SideloadingStorage, new_storage};
use crate::{SideloadError, anisette::remote_v3::websocket::AppWebSocket};

pub const DEFAULT_ANISETTE_V3_URL: &str = "https://ani.jkcoxson.com";

/// Fixed service endpoints; URL construction performs no network access.
#[derive(Clone, Copy)]
pub enum Endpoint {
    ClientInfo,
    Headers,
    Provisioning,
}
pub fn endpoint_url(base: &str, endpoint: Endpoint) -> Result<reqwest::Url, Report> {
    let suffix = match endpoint {
        Endpoint::ClientInfo => "client_info",
        Endpoint::Headers => "get_headers",
        Endpoint::Provisioning => "provisioning_session",
    };
    let mut url = reqwest::Url::parse(base)?;
    if !matches!(url.scheme(), "https" | "http")
        || !url.username().is_empty()
        || url.password().is_some()
        || url.query().is_some()
        || url.fragment().is_some()
    {
        bail!("Anisette base URL must be HTTP(S) without credentials, query or fragment");
    }
    let path = format!("{}/v3/{suffix}", url.path().trim_end_matches('/'));
    url.set_path(&path);
    if matches!(endpoint, Endpoint::Provisioning) {
        let scheme = if url.scheme() == "https" { "wss" } else { "ws" };
        url.set_scheme(scheme)
            .map_err(|_| report!("Invalid anisette scheme"))?;
    }
    Ok(url)
}

pub struct RemoteV3AnisetteProvider {
    pub state: Option<AnisetteState>,
    url: String,
    storage: Box<dyn SideloadingStorage>,
    serial_number: String,
    client: reqwest_middleware::ClientWithMiddleware,
    websocket_proxy: Option<String>,
    client_info_override: Option<AnisetteClientInfo>,
}

impl RemoteV3AnisetteProvider {
    /// Create a new RemoteV3AnisetteProvider with the given URL and config path
    ///
    /// # Arguments
    /// - `url`: The URL of the remote anisette service
    /// - `storage`: The storage backend for anisette data
    /// - `serial_number`: The serial number of the device
    pub fn new(
        url: &str,
        storage: Box<dyn SideloadingStorage>,
        serial_number: String,
    ) -> Result<Self, Report> {
        Ok(Self {
            state: None,
            url: url.to_string(),
            storage,
            serial_number,
            client: Self::build_reqwest_client(None)?,
            websocket_proxy: None,
            client_info_override: None,
        })
    }

    /// Override the client identity (X-Mme-Client-Info / User-Agent) reported
    /// by the anisette server. Apple rejects sign-in when the reported identity
    /// claims an OS version it no longer accepts, so callers can force a known
    /// modern Mac identity until servers start reporting one themselves.
    ///
    /// # Arguments
    /// - `client_info_override`: Identity to use instead of the server's, or
    ///   `None` to keep fetching the identity from the server
    pub fn set_client_info_override(
        mut self,
        client_info_override: Option<AnisetteClientInfo>,
    ) -> Self {
        self.client_info_override = client_info_override;
        self
    }

    pub fn set_websocket_proxy(mut self, websocket_proxy: Option<String>) -> Result<Self, Report> {
        self.websocket_proxy = websocket_proxy;
        self.client = Self::build_reqwest_client(self.websocket_proxy.clone())?;
        Ok(self)
    }

    fn build_reqwest_client(
        websocket_proxy: Option<String>,
    ) -> Result<reqwest_middleware::ClientWithMiddleware, Report> {
        if let Some(websocket_proxy) = websocket_proxy {
            use crate::auth::middleware::WasmProxyMiddleware;

            let client = ClientBuilder::new().build()?;

            Ok(MwClientBuilder::new(client)
                .with(WasmProxyMiddleware::new(websocket_proxy))
                .build())
        } else {
            let client = ClientBuilder::new().build()?;

            Ok(MwClientBuilder::new(client).build())
        }
    }

    pub fn default() -> Result<Self, Report> {
        Self::new(
            DEFAULT_ANISETTE_V3_URL,
            Box::new(new_storage()),
            "0".to_string(),
        )
    }

    pub fn set_url(mut self, url: &str) -> RemoteV3AnisetteProvider {
        self.url = url.to_string();
        self
    }

    pub fn set_storage(mut self, storage: Box<dyn SideloadingStorage>) -> RemoteV3AnisetteProvider {
        self.storage = storage;
        self
    }

    pub fn set_serial_number(mut self, serial_number: String) -> RemoteV3AnisetteProvider {
        self.serial_number = serial_number;
        self
    }
}

#[cfg_attr(feature = "wasm", async_trait::async_trait(?Send))]
#[cfg_attr(not(feature = "wasm"), async_trait::async_trait)]
impl AnisetteProvider for RemoteV3AnisetteProvider {
    async fn get_anisette_data(&self) -> Result<AnisetteData, Report> {
        let state = self
            .state
            .as_ref()
            .ok_or(SideloadError::AnisetteNotProvisioned)?;
        let adi_pb = state
            .adi_pb
            .as_ref()
            .ok_or(SideloadError::AnisetteNotProvisioned)?;
        let client_info = self.get_client_info().await?;

        let headers = self
            .client
            .post(endpoint_url(&self.url, Endpoint::Headers)?)
            .header(CONTENT_TYPE, "application/json")
            .body(
                serde_json::json!({
                "identifier": BASE64_STANDARD.encode(state.keychain_identifier),
                "adi_pb": BASE64_STANDARD.encode(adi_pb)
                })
                .to_string(),
            )
            .send()
            .await?
            .error_for_status()?
            .json::<AnisetteHeaders>()
            .await?;

        match headers {
            AnisetteHeaders::Headers {
                machine_id,
                one_time_password,
                routing_info,
            } => {
                let data = AnisetteData {
                    machine_id,
                    one_time_password,
                    routing_info,
                    _device_description: client_info.client_info.clone(),
                    device_unique_identifier: state.get_device_id(),
                    _local_user_id: hex::encode(state.get_md_lu()),
                    generated_at: SystemTime::now(),
                };

                Ok(data)
            }
            AnisetteHeaders::GetHeadersError { message } => {
                Err(report!("Failed to get anisette headers").attach(message))
            }
        }
    }

    async fn get_client_info(&self) -> Result<AnisetteClientInfo, Report> {
        if let Some(client_info_override) = self.client_info_override.as_ref() {
            return Ok(client_info_override.clone());
        }
        // Read public identity metadata from the already selected server.
        // This GET sends no account, provisioning or device data and does not
        // change the selected server or the existing provisioning data flow.
        let info = self
            .client
            .get(endpoint_url(&self.url, Endpoint::ClientInfo)?)
            .timeout(std::time::Duration::from_secs(15))
            .send()
            .await
            .context("Failed to get anisette client info")?
            .error_for_status()
            .context("Failed to get anisette client info")?
            .json::<AnisetteClientInfo>()
            .await
            .context("Failed to decode anisette client info")?;
        if info.client_info.trim().is_empty() || info.user_agent.trim().is_empty() {
            bail!("Anisette server returned empty client info or user agent");
        }
        HeaderValue::from_str(&info.client_info).context("Invalid anisette client info header")?;
        HeaderValue::from_str(&info.user_agent).context("Invalid anisette user agent header")?;
        Ok(info)
    }

    fn needs_provisioning(&self) -> Result<bool, Report> {
        if let Some(state) = &self.state {
            Ok(!state.is_provisioned())
        } else {
            Ok(true)
        }
    }

    async fn provision(&mut self, gs: Arc<GrandSlam>) -> Result<(), Report> {
        self.get_client_info().await?;
        self.get_state(gs).await?;
        Ok(())
    }
}

impl RemoteV3AnisetteProvider {
    async fn get_state(&mut self, gs: Arc<GrandSlam>) -> Result<&mut AnisetteState, Report> {
        if self.state.is_none() {
            if let Ok(Some(state)) = &self.storage.retrieve_data("anisette_state") {
                if let Ok(state) = plist::from_bytes(state) {
                    info!("Loaded existing anisette state");
                    self.state = Some(state);
                } else {
                    warn!("Failed to parse existing anisette state, starting fresh");
                    self.state = Some(AnisetteState::new());
                }
            } else {
                info!("No existing anisette state found");
                self.state = Some(AnisetteState::new());
            }
        }

        let state = self.state.as_mut().ok_or_report()?;
        if !state.is_provisioned() {
            info!("Provisioning required...");
            Self::provision(state, gs, &self.url, self.websocket_proxy.as_deref())
                .await
                .context("Failed to provision")?;
        }
        let buf = Vec::new();
        let mut writer = std::io::BufWriter::new(buf);
        plist::to_writer_xml(&mut writer, &state)?;
        self.storage
            .store_data("anisette_state", &writer.into_inner()?)?;

        Ok(state)
    }

    async fn provisioning_headers(state: &AnisetteState) -> Result<HeaderMap, Report> {
        let mut headers = HeaderMap::new();
        headers.insert(
            "X-Apple-I-MD-LU",
            HeaderValue::from_str(&hex::encode(state.get_md_lu()))?,
        );
        // headers.insert(
        //     "X-Apple-I-Client-Time",
        //     HeaderValue::from_str(
        //         &Utc::now()
        //             .round_subsecs(0)
        //             .format("%+")
        //             .to_string()
        //             .replace("+00:00", "Z"),
        //     )?,
        // );
        // headers.insert("X-Apple-I-TimeZone", HeaderValue::from_static("UTC"));
        // headers.insert("X-Apple-Locale", HeaderValue::from_static("en_US"));
        headers.insert(
            "X-Mme-Device-Id",
            HeaderValue::from_str(&state.get_device_id())?,
        );

        Ok(headers)
    }
    async fn provision(
        state: &mut AnisetteState,
        gs: Arc<GrandSlam>,
        url: &str,
        proxy_url: Option<&str>,
    ) -> Result<(), Report> {
        let start_provisioning = gs.get_url("midStartProvisioning")?;
        let end_provisioning = gs.get_url("midFinishProvisioning")?;

        let websocket_url = endpoint_url(url, Endpoint::Provisioning)?.to_string();

        debug!("Starting provisioning at {}", websocket_url);
        // let (mut ws_stream, _) = timeout(
        //     Duration::from_secs(30),
        //     tokio_tungstenite::connect_async(&websocket_url),
        // )
        // .await
        // .map_err(|_| {
        //     report!("Timed out connecting to provisioning socket. Try a different anisette server.")
        // })
        // .context("Failed to connect to provisioning socket")?
        // .context("Failed to connect to provisioning socket")?;

        let mut ws = AppWebSocket::connect(&websocket_url, proxy_url.as_deref())
            .await
            .context("Failed to connect to provisioning socket")?;

        debug!("Connected to provisioning socket");

        loop {
            let Some(msg) = ws.next().await else { continue };
            let msg = msg?;

            let text = match msg {
                WsMessage::Close => bail!("Provisioning socket closed unexpectedly"),
                WsMessage::Text(t) => t,
            };

            debug!("Received provisioning message: {}", text);
            let provision_msg: ProvisioningMessage = serde_json::from_str(&text)?;

            match provision_msg {
                ProvisioningMessage::GiveIdentifier => {
                    ws.send_text(
                        serde_json::json!({
                            "identifier": BASE64_STANDARD.encode(state.keychain_identifier),
                        })
                        .to_string()
                        .into(),
                    )
                    .await
                    .context("Failed to send identifier")?;
                }
                ProvisioningMessage::GiveStartProvisioningData => {
                    let body = plist!(dict {
                        "Header": {},
                        "Request": {}
                    });

                    let response = gs
                        .plist_request(
                            &start_provisioning,
                            &body,
                            Some(Self::provisioning_headers(state).await?),
                            None,
                        )
                        .await
                        .context("Failed to send start provisioning request")?;

                    let spim = response
                        .get_str("spim")
                        .context("Start provisioning response missing spim")?;

                    ws.send_text(
                        serde_json::json!({
                            "spim": spim,
                        })
                        .to_string()
                        .into(),
                    )
                    .await
                    .context("Failed to send start provisioning data")?;
                }
                ProvisioningMessage::GiveEndProvisioningData { cpim } => {
                    let body = plist!(dict {
                        "Header": {},
                        "Request": {
                            "cpim": cpim,
                        }
                    });

                    let response = gs
                        .plist_request(
                            &end_provisioning,
                            &body,
                            Some(Self::provisioning_headers(state).await?),
                            None,
                        )
                        .await
                        .context("Failed to send end provisioning request")?;

                    ws.send_text(
                        serde_json::json!({
                            "ptm": response
                                .get_str("ptm")
                                .context("End provisioning response missing ptm")?,
                            "tk": response
                                .get_str("tk")
                                .context("End provisioning response missing tk")?,
                        })
                        .to_string()
                        .into(),
                    )
                    .await
                    .context("Failed to send start provisioning data")?;
                }
                ProvisioningMessage::ProvisioningSuccess { adi_pb } => {
                    state.adi_pb = Some(BASE64_STANDARD.decode(adi_pb)?);
                    ws.close().await?;
                    info!("Provisioning successful");
                    break;
                }
                ProvisioningMessage::Timeout => bail!("Anisette provisioning timed out"),
                ProvisioningMessage::InvalidIdentifier => {
                    bail!("Anisette provisioning failed: invalid identifier")
                }
                ProvisioningMessage::StartProvisioningError { message } => {
                    return Err(
                        report!("Anisette provisioning failed: start provisioning error")
                            .attach(message),
                    );
                }
                ProvisioningMessage::EndProvisioningError { message } => {
                    return Err(
                        report!("Anisette provisioning failed: end provisioning error")
                            .attach(message),
                    );
                }
            }
        }

        Ok(())
    }
}

#[derive(Deserialize)]
#[serde(tag = "result")]
enum ProvisioningMessage {
    GiveIdentifier,
    GiveStartProvisioningData,
    GiveEndProvisioningData { cpim: String },
    ProvisioningSuccess { adi_pb: String },
    Timeout,
    InvalidIdentifier,
    StartProvisioningError { message: String },
    EndProvisioningError { message: String },
}

#[derive(Deserialize)]
#[serde(tag = "result")]
enum AnisetteHeaders {
    GetHeadersError {
        message: String,
    },
    Headers {
        #[serde(rename = "X-Apple-I-MD-M")]
        machine_id: String,
        #[serde(rename = "X-Apple-I-MD")]
        one_time_password: String,
        #[serde(rename = "X-Apple-I-MD-RINFO")]
        routing_info: String,
    },
}

#[cfg(test)]
mod endpoint_tests {
    use super::*;
    #[test]
    fn exact_endpoints_preserve_prefix_and_reject_ambiguous_bases_without_network() {
        for (base, expected) in [
            (
                "https://example.invalid",
                "https://example.invalid/v3/get_headers",
            ),
            (
                "https://example.invalid///",
                "https://example.invalid/v3/get_headers",
            ),
            (
                "https://example.invalid/proxy%20path/",
                "https://example.invalid/proxy%20path/v3/get_headers",
            ),
            (
                "http://example.invalid/prefix",
                "http://example.invalid/prefix/v3/get_headers",
            ),
        ] {
            assert_eq!(
                endpoint_url(base, Endpoint::Headers).unwrap().as_str(),
                expected
            );
        }
        assert_eq!(
            endpoint_url("http://example.invalid/prefix/", Endpoint::Provisioning)
                .unwrap()
                .as_str(),
            "ws://example.invalid/prefix/v3/provisioning_session"
        );
        for base in [
            "https://example.invalid/?q=1",
            "https://example.invalid/#fragment",
            "https://user:secret@example.invalid",
            "ftp://example.invalid",
        ] {
            for endpoint in [
                Endpoint::ClientInfo,
                Endpoint::Headers,
                Endpoint::Provisioning,
            ] {
                assert!(endpoint_url(base, endpoint).is_err());
            }
        }
    }
}

#[cfg(test)]
mod client_info_tests {
    use super::*;
    use tokio::{
        io::{AsyncReadExt, AsyncWriteExt},
        net::TcpListener,
    };

    async fn server(
        responses: Vec<(&'static str, &'static str)>,
    ) -> (RemoteV3AnisetteProvider, tokio::task::JoinHandle<()>) {
        let listener = TcpListener::bind("127.0.0.1:0").await.unwrap();
        let url = format!("http://{}/test-prefix/", listener.local_addr().unwrap());
        let task = tokio::spawn(async move {
            for (status, body) in responses {
                let (mut socket, _) = listener.accept().await.unwrap();
                let mut request = Vec::new();
                while !request.ends_with(b"\r\n\r\n") {
                    let mut part = [0; 1024];
                    let count = socket.read(&mut part).await.unwrap();
                    assert!(count > 0);
                    request.extend_from_slice(&part[..count]);
                    assert!(request.len() < 8192);
                }
                let request = String::from_utf8(request).unwrap();
                assert!(request.starts_with("GET /test-prefix/v3/client_info HTTP/1.1"));
                assert!(!request.to_ascii_lowercase().contains("authorization:"));
                assert!(!request.to_ascii_lowercase().contains("content-length:"));
                let response = format!(
                    "HTTP/1.1 {status}\r\nContent-Type: application/json\r\nContent-Length: {}\r\nConnection: close\r\n\r\n{body}",
                    body.len()
                );
                socket.write_all(response.as_bytes()).await.unwrap();
            }
        });
        (
            RemoteV3AnisetteProvider::new(
                &url,
                Box::new(crate::util::storage::InMemoryStorage::new()),
                "test".into(),
            )
            .unwrap(),
            task,
        )
    }

    #[tokio::test]
    async fn identity_is_fetched_from_selected_server_and_refreshed() {
        let (provider, task) = server(vec![
            (
                "200 OK",
                r#"{"client_info":"server-one","user_agent":"agent-one"}"#,
            ),
            (
                "200 OK",
                r#"{"client_info":"server-two","user_agent":"agent-two"}"#,
            ),
        ])
        .await;
        let first = provider.get_client_info().await.unwrap();
        let second = provider.get_client_info().await.unwrap();
        assert_eq!(first.client_info, "server-one");
        assert_eq!(first.user_agent, "agent-one");
        assert_eq!(second.client_info, "server-two");
        assert_eq!(second.user_agent, "agent-two");
        task.await.unwrap();
    }

    #[tokio::test]
    async fn explicit_client_info_override_wins_without_server_roundtrip() {
        // Port 1 is never contacted: if get_client_info tried the server it
        // would fail instead of returning the caller-supplied identity.
        let provider = RemoteV3AnisetteProvider::new(
            "http://127.0.0.1:1/test-prefix/",
            Box::new(crate::util::storage::InMemoryStorage::new()),
            "test".into(),
        )
        .unwrap()
        .set_client_info_override(Some(AnisetteClientInfo {
            client_info: "<Mac15,7> <macOS;27.0;26A5378j> <com.apple.AuthKit/1 (com.apple.akd/1.0)>"
                .into(),
            user_agent: "akd/1.0 CFNetwork/811.5.4".into(),
        }));
        let info = provider.get_client_info().await.unwrap();
        assert_eq!(
            info.client_info,
            "<Mac15,7> <macOS;27.0;26A5378j> <com.apple.AuthKit/1 (com.apple.akd/1.0)>"
        );
        assert_eq!(info.user_agent, "akd/1.0 CFNetwork/811.5.4");
    }

    #[tokio::test]
    async fn unavailable_or_invalid_identity_has_no_hardcoded_fallback() {
        for response in [
            ("503 Service Unavailable", "{}"),
            ("200 OK", "not json"),
            ("200 OK", r#"{"client_info":"","user_agent":"agent"}"#),
            ("200 OK", r#"{"client_info":"server","user_agent":""}"#),
            (
                "200 OK",
                r#"{"client_info":"server","user_agent":"bad\r\nheader"}"#,
            ),
        ] {
            let (provider, task) = server(vec![response]).await;
            assert!(provider.get_client_info().await.is_err());
            task.await.unwrap();
        }
    }
}
