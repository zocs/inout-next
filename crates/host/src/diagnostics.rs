//! Application-only, read-only timing samples against the active real service.
//! All network futures belong to the caller; no connection driver is spawned.
use bytes::Bytes;
use futures_util::{stream::FuturesUnordered, StreamExt};
use http_body_util::{BodyExt, Empty};
use hyper::{header, HeaderMap, Method, Request, StatusCode};
use hyper_util::rt::TokioIo;
use serde::Serialize;
use share_http::{DownloadControl, DEFAULT_DOWNLOAD_BUFFER_KIB};
use std::{
    net::Ipv4Addr,
    sync::{Arc, Mutex},
    time::{Duration, Instant},
};
use tokio::{
    net::TcpStream,
    sync::{OwnedSemaphorePermit, Semaphore},
};
use tokio_util::sync::CancellationToken;

const MAX_BYTES: u64 = 64 * 1024 * 1024;
const TIMEOUT: Duration = Duration::from_secs(30);
const DEFAULT_KIB: usize = DEFAULT_DOWNLOAD_BUFFER_KIB;

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticReport {
    pub bytes: u64,
    pub duration_ms: f64,
    pub mib_per_second: f64,
    pub buffer_kib: usize,
    pub connections: usize,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct DiagnosticStatus {
    pub enabled: bool,
    pub running: bool,
    pub buffer_kib: usize,
    pub default_buffer_kib: usize,
    pub probe_state: &'static str,
    pub result: Option<DiagnosticReport>,
}
impl Default for DiagnosticStatus {
    fn default() -> Self {
        Self {
            enabled: false,
            running: false,
            buffer_kib: DEFAULT_KIB,
            default_buffer_kib: DEFAULT_KIB,
            probe_state: "idle",
            result: None,
        }
    }
}

#[derive(Default)]
struct Settings {
    enabled: bool,
    active: Option<CancellationToken>,
    result: Option<DiagnosticReport>,
}

pub(crate) struct Diagnostics {
    port: u16,
    // This credential never enters status, reports, errors or logs.
    key: Mutex<Option<String>>,
    control: DownloadControl,
    settings: Mutex<Settings>,
    permit: Arc<Semaphore>,
}
impl Diagnostics {
    pub(crate) fn new(
        port: u16,
        key: Option<String>,
        control: DownloadControl,
        permit: Arc<Semaphore>,
    ) -> Arc<Self> {
        Arc::new(Self {
            port,
            key: Mutex::new(key),
            control,
            settings: Mutex::new(Settings::default()),
            permit,
        })
    }
    pub(crate) fn stop(&self) {
        self.set_enabled(false);
        self.key.lock().expect("diagnostic credential lock").take();
    }
    pub(crate) fn status(&self) -> DiagnosticStatus {
        let settings = self.settings.lock().expect("diagnostic settings lock");
        DiagnosticStatus {
            enabled: settings.enabled,
            running: settings.active.is_some(),
            buffer_kib: self.control.buffer_kib(),
            default_buffer_kib: DEFAULT_KIB,
            probe_state: if settings.active.is_some() {
                "running"
            } else {
                "idle"
            },
            result: settings.result.clone(),
        }
    }
    pub(crate) fn set_enabled(&self, enabled: bool) -> DiagnosticStatus {
        {
            let mut settings = self.settings.lock().expect("diagnostic settings lock");
            settings.enabled = enabled;
            if !enabled {
                if let Some(token) = &settings.active {
                    token.cancel();
                }
                self.control.reset();
                settings.result = None;
            }
        }
        self.status()
    }
    pub(crate) fn set_buffer(&self, kib: Option<usize>) -> Result<DiagnosticStatus, String> {
        {
            let settings = self.settings.lock().expect("diagnostic settings lock");
            if !settings.enabled {
                return Err("Enable local diagnostics first".into());
            }
            if settings.active.is_some() {
                return Err("Wait for the test or cancel it before changing settings".into());
            }
            match kib {
                Some(kib) => self
                    .control
                    .set_buffer_kib(kib)
                    .map_err(|_| "Choose a buffer from 16 to 128 KiB in 16 KiB steps")?,
                None => self.control.reset(),
            }
        }
        Ok(self.status())
    }
    pub(crate) fn cancel(&self) -> DiagnosticStatus {
        {
            let settings = self.settings.lock().expect("diagnostic settings lock");
            if let Some(token) = &settings.active {
                token.cancel();
            }
        }
        self.status()
    }
    pub(crate) fn begin(
        self: &Arc<Self>,
        path: String,
        connections: usize,
    ) -> Result<Probe, String> {
        if !matches!(connections, 1 | 2 | 4 | 8) {
            return Err("Choose 1, 2, 4 or 8 connections".into());
        }
        if path.len() > 4096 || share_domain::RelativePath::parse(&path, false).is_err() {
            return Err("Enter a valid relative shared-file path".into());
        }
        let mut settings = self.settings.lock().expect("diagnostic settings lock");
        if !settings.enabled {
            return Err("Enable local diagnostics first".into());
        }
        let permit = self
            .permit
            .clone()
            .try_acquire_owned()
            .map_err(|_| "A local test is already running")?;
        let token = CancellationToken::new();
        settings.active = Some(token.clone());
        settings.result = None;
        Ok(Probe {
            owner: self.clone(),
            _permit: permit,
            token,
            path,
            connections,
            buffer_kib: self.control.buffer_kib(),
        })
    }
}
impl Drop for Diagnostics {
    fn drop(&mut self) {
        self.control.reset();
        if let Ok(settings) = self.settings.lock() {
            if let Some(token) = &settings.active {
                token.cancel();
            }
        }
    }
}

pub(crate) struct Probe {
    owner: Arc<Diagnostics>,
    _permit: OwnedSemaphorePermit,
    token: CancellationToken,
    path: String,
    connections: usize,
    buffer_kib: usize,
}
impl Drop for Probe {
    fn drop(&mut self) {
        self.token.cancel();
        self.owner
            .settings
            .lock()
            .expect("diagnostic settings lock")
            .active = None;
    }
}
impl Probe {
    pub(crate) async fn run(self) -> Result<DiagnosticReport, String> {
        let result = tokio::select! {
            biased;
            _ = self.token.cancelled() => Err("Local test canceled".into()),
            result = tokio::time::timeout(TIMEOUT, self.measure()) => result.unwrap_or_else(|_| Err("Local test timed out after 30 seconds".into())),
        };
        if let Ok(report) = &result {
            let mut settings = self
                .owner
                .settings
                .lock()
                .expect("diagnostic settings lock");
            if settings.enabled && !self.token.is_cancelled() {
                settings.result = Some(report.clone());
            }
        }
        result
    }
    async fn measure(&self) -> Result<DiagnosticReport, String> {
        let started = Instant::now();
        let query =
            percent_encoding::utf8_percent_encode(&self.path, percent_encoding::NON_ALPHANUMERIC);
        let uri = format!(
            "http://127.0.0.1:{}/api/v1/content?path={query}",
            self.owner.port
        );
        let (head, _) = self.exchange(&uri, None).await?;
        let size = number(&head, header::CONTENT_LENGTH)?;
        let bytes = size.min(MAX_BYTES);
        if bytes < self.connections as u64 {
            return Err("Choose a nonempty file with at least one byte per connection".into());
        }
        let mut requests = FuturesUnordered::new();
        for index in 0..self.connections as u64 {
            let start = bytes * index / self.connections as u64;
            let end = bytes * (index + 1) / self.connections as u64 - 1;
            requests.push(self.exchange(&uri, Some((start, end, size))));
        }
        let mut measured = 0;
        while let Some(result) = requests.next().await {
            measured += result?.1;
        }
        if measured != bytes {
            return Err("The local test received an incomplete sample".into());
        }
        let seconds = started.elapsed().as_secs_f64().max(f64::MIN_POSITIVE);
        Ok(DiagnosticReport {
            bytes: measured,
            duration_ms: seconds * 1000.0,
            mib_per_second: measured as f64 / (1024.0 * 1024.0) / seconds,
            buffer_kib: self.buffer_kib,
            connections: self.connections,
        })
    }
    async fn exchange(
        &self,
        uri: &str,
        range: Option<(u64, u64, u64)>,
    ) -> Result<(HeaderMap, u64), String> {
        let stream = TcpStream::connect((Ipv4Addr::LOCALHOST, self.owner.port))
            .await
            .map_err(|_| "The active local service could not be reached")?;
        let (mut sender, connection) = hyper::client::conn::http1::handshake(TokioIo::new(stream))
            .await
            .map_err(|_| "The local HTTP connection failed")?;
        let mut request = Request::builder()
            .method(if range.is_some() {
                Method::GET
            } else {
                Method::HEAD
            })
            .uri(uri)
            .header(header::HOST, format!("127.0.0.1:{}", self.owner.port))
            .header(header::CONNECTION, "close");
        {
            let key = self.owner.key.lock().expect("diagnostic credential lock");
            if let Some(key) = key.as_ref() {
                request = request.header(header::AUTHORIZATION, format!("Bearer {key}"));
            }
        }
        if let Some((start, end, _)) = range {
            request = request.header(header::RANGE, format!("bytes={start}-{end}"));
        }
        let request = request
            .body(Empty::<Bytes>::new())
            .map_err(|_| "The local request could not be created")?;
        let exchange = async {
            let response = sender
                .send_request(request)
                .await
                .map_err(|_| "The local read-only request failed")?;
            let want = if range.is_some() {
                StatusCode::PARTIAL_CONTENT
            } else {
                StatusCode::OK
            };
            if response.status() != want {
                return Err("The shared file is unavailable or access is denied".into());
            }
            let headers = response.headers().clone();
            let expected = if let Some((start, end, size)) = range {
                let value = format!("bytes {start}-{end}/{size}");
                if headers
                    .get(header::CONTENT_RANGE)
                    .and_then(|value| value.to_str().ok())
                    != Some(value.as_str())
                    || number(&headers, header::CONTENT_LENGTH)? != end - start + 1
                {
                    return Err(
                        "The shared file changed or its range response is inconsistent".into(),
                    );
                }
                end - start + 1
            } else {
                0
            };
            let mut body = response.into_body();
            let mut count = 0;
            while let Some(frame) = body.frame().await {
                let frame =
                    frame.map_err(|_| "The local transfer ended before the sample completed")?;
                if let Ok(data) = frame.into_data() {
                    count += data.len() as u64;
                    if count > expected {
                        return Err("The local transfer exceeded its requested range".into());
                    }
                }
            }
            if count != expected {
                return Err("The local transfer ended before the sample completed".into());
            }
            Ok((headers, count))
        };
        // Driver and exchange are polled together and dropped together on any
        // error/cancel/timeout/caller drop. No socket work outlives this future.
        tokio::pin!(connection);
        tokio::pin!(exchange);
        tokio::select! {
            biased;
            result = &mut exchange => result,
            result = &mut connection => {
                result.map_err(|_| "The local HTTP connection failed")?;
                exchange.await
            }
        }
    }
}
fn number(headers: &HeaderMap, name: header::HeaderName) -> Result<u64, String> {
    headers
        .get(name)
        .and_then(|value| value.to_str().ok())
        .and_then(|value| value.parse().ok())
        .ok_or_else(|| "The local response is missing a valid content length".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::io::AsyncReadExt;

    // Only the unresponsive HTTP peer is controlled. The real client, timeout,
    // request driver, permit and cancellation/drop behavior execute unchanged.
    #[tokio::test]
    async fn stalled_peer_closes_on_cancel_stop_timeout_and_caller_drop() {
        for action in ["cancel", "stop", "timeout", "drop"] {
            let listener = tokio::net::TcpListener::bind((Ipv4Addr::LOCALHOST, 0))
                .await
                .unwrap();
            let address = listener.local_addr().unwrap();
            let (entered, received) = tokio::sync::oneshot::channel();
            let mut peers = tokio::task::JoinSet::new();
            peers.spawn(async move {
                let (mut stream, _) = listener.accept().await.unwrap();
                let mut headers = Vec::new();
                loop {
                    let byte = stream.read_u8().await.unwrap();
                    headers.push(byte);
                    if headers.ends_with(b"\r\n\r\n") {
                        break;
                    }
                }
                entered.send(()).unwrap();
                let mut byte = [0];
                stream.read(&mut byte).await.unwrap()
            });
            let diagnostics = Diagnostics::new(
                address.port(),
                Some("private-key".into()),
                DownloadControl::default(),
                Arc::new(Semaphore::new(1)),
            );
            diagnostics.set_enabled(true);
            let mut callers = tokio::task::JoinSet::new();
            callers.spawn(diagnostics.begin("sample.bin".into(), 1).unwrap().run());
            tokio::time::timeout(Duration::from_secs(1), received)
                .await
                .unwrap()
                .unwrap();
            match action {
                "cancel" => {
                    diagnostics.cancel();
                }
                "stop" => {
                    diagnostics.stop();
                }
                "timeout" => {
                    tokio::time::pause();
                    tokio::time::advance(Duration::from_secs(31)).await;
                }
                "drop" => callers.abort_all(),
                _ => unreachable!(),
            }
            let result = callers.join_next().await.unwrap();
            if action == "drop" {
                assert!(result.err().unwrap().is_cancelled());
            } else {
                let error = result.unwrap().err().unwrap();
                assert!(error.contains(if action == "timeout" {
                    "timed out after 30 seconds"
                } else {
                    "canceled"
                }));
            }
            if action == "timeout" {
                tokio::time::resume();
            }
            assert_eq!(
                tokio::time::timeout(Duration::from_secs(1), peers.join_next())
                    .await
                    .unwrap()
                    .unwrap()
                    .unwrap(),
                0,
                "Socket survived {action}"
            );
            assert!(!diagnostics.status().running);
            diagnostics.set_enabled(true);
            assert!(diagnostics.begin("sample.bin".into(), 1).is_ok());
        }
    }
}
