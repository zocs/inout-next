mod capabilities;
mod network;
mod session;
pub use network::{LinkKind, ShareEndpoint};
pub use session::{
    HostSession, Interrupt, ProblemAction, ProblemReason, SessionCommand, SessionProblem,
    SessionSnapshot,
};
#[path = "diagnostics.rs"]
pub(crate) mod diagnostics;
pub use capabilities::{HostCapabilities, StorageKind};
use diagnostics::Diagnostics;
pub use diagnostics::{DiagnosticReport, DiagnosticStatus};
use serde::{Deserialize, Serialize};
use share_http::{RunningServer, ServerConfig, ServerError, WebBundle};
use std::sync::Arc;
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};
use tokio::sync::{Mutex, Semaphore};

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartOptions {
    pub network: bool,
    pub port: u16,
    pub uploads: bool,
    pub manage: bool,
    pub key: Option<String>,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSettings {
    pub network: bool,
    pub port: u16,
    pub uploads: bool,
    pub manage: bool,
    pub auth_required: bool,
}

#[derive(Clone, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub root: Option<String>,
    pub running: bool,
    pub manager_url: Option<String>,
    pub addresses: Vec<String>,
    pub active_settings: Option<ActiveSettings>,
    pub capabilities: HostCapabilities,
}

/// Which setting a failed start points at, for hosts that localize messages.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Serialize)]
#[serde(rename_all = "kebab-case")]
pub enum StartReason {
    AlreadyRunning,
    NoFolder,
    Folder,
    Resources,
    Key,
    PortInUse,
    Port,
    Other,
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct StartFailure {
    pub reason: StartReason,
    pub message: &'static str,
}

impl StartFailure {
    fn new(reason: StartReason) -> Self {
        let message = match reason {
            StartReason::AlreadyRunning => "The service is already running",
            StartReason::NoFolder => "Choose a folder before starting the service",
            StartReason::Folder => "The shared folder is unavailable; choose another folder",
            StartReason::Resources => {
                "The web resources are missing or inside the shared folder; choose a folder that does not contain the application"
            }
            StartReason::Key => "The access key must not be blank",
            StartReason::PortInUse => {
                "The port is already in use; choose another port or use 0"
            }
            StartReason::Port => "The port is not available; choose another port or use 0",
            StartReason::Other => {
                "The service could not start; check the folder, assets and port"
            }
        };
        Self { reason, message }
    }
}

enum WebSource {
    Directory(PathBuf),
    Bundle(WebBundle),
}

#[derive(Default)]
struct HostState {
    root: Option<PathBuf>,
    running: Option<RunningServer>,
    endpoints: Vec<ShareEndpoint>,
    addresses: Vec<String>,
    active_settings: Option<ActiveSettings>,
    diagnostics: Option<Arc<Diagnostics>>,
}

impl HostState {
    fn status(&self, capabilities: HostCapabilities) -> HostStatus {
        HostStatus {
            root: self
                .root
                .as_ref()
                .map(|path| path.to_string_lossy().into_owned()),
            running: self.running.is_some(),
            manager_url: self
                .running
                .as_ref()
                .map(|server| format!("http://127.0.0.1:{}", server.address().port())),
            addresses: self.addresses.clone(),
            active_settings: self.active_settings.clone(),
            capabilities,
        }
    }
}

/// Owns the service lifecycle. Serializing transitions prevents two starts or a
/// root change from racing a listener that is still starting or shutting down.
pub struct HostController {
    web: WebSource,
    capabilities: HostCapabilities,
    state: Mutex<HostState>,
    probe_permit: Arc<Semaphore>,
}

impl HostController {
    pub fn new(web_dir: PathBuf) -> Self {
        Self::with_source(WebSource::Directory(web_dir))
    }

    /// Serve pages compiled into the host. No resource directory is read, so
    /// the shared folder may contain the application itself.
    pub fn new_embedded(bundle: WebBundle) -> Self {
        Self::with_source(WebSource::Bundle(bundle))
    }

    fn with_source(web: WebSource) -> Self {
        Self {
            web,
            capabilities: HostCapabilities::default(),
            state: Mutex::new(HostState::default()),
            probe_permit: Arc::new(Semaphore::new(1)),
        }
    }

    pub fn new_app_private(web_dir: PathBuf, root: PathBuf) -> Result<Self, String> {
        let root = checked_root(root)?;
        let web_dir = web_dir
            .canonicalize()
            .map_err(|_| "The packaged web resources are unavailable")?;
        if !checked_assets(&web_dir) {
            return Err("The packaged web assets are missing or incomplete".into());
        }
        if root.starts_with(&web_dir) || web_dir.starts_with(&root) {
            return Err("App storage must be separate from the application resources".into());
        }
        Ok(Self {
            web: WebSource::Directory(web_dir),
            capabilities: HostCapabilities::app_private(),
            state: Mutex::new(HostState {
                root: Some(root),
                ..HostState::default()
            }),
            probe_permit: Arc::new(Semaphore::new(1)),
        })
    }

    pub fn capabilities(&self) -> HostCapabilities {
        self.capabilities
    }

    pub async fn select_root(&self, root: PathBuf) -> Result<HostStatus, String> {
        if !self.capabilities.choose_folder {
            return Err("This preview shares only its own app storage".into());
        }
        let mut state = self.state.lock().await;
        if state.running.is_some() {
            return Err("Stop the service before choosing another folder".into());
        }
        let root = checked_root(root)?;
        state.root = Some(root);
        Ok(state.status(self.capabilities))
    }

    pub async fn start(&self, options: StartOptions) -> Result<HostStatus, String> {
        self.start_checked(options)
            .await
            .map_err(|failure| failure.message.to_owned())
    }

    /// Start, reporting which setting a failure points at.
    pub async fn start_checked(&self, options: StartOptions) -> Result<HostStatus, StartFailure> {
        let mut state = self.state.lock().await;
        if state.running.is_some() {
            return Err(StartFailure::new(StartReason::AlreadyRunning));
        }
        let root = state
            .root
            .clone()
            .ok_or(StartFailure::new(StartReason::NoFolder))?;
        let listen = SocketAddr::from((
            if options.network {
                Ipv4Addr::UNSPECIFIED
            } else {
                Ipv4Addr::LOCALHOST
            },
            options.port,
        ));
        let mut config = ServerConfig::new(root, listen);
        match &self.web {
            WebSource::Directory(path) => config.web_dir = Some(path.clone()),
            WebSource::Bundle(bundle) => config.web_bundle = Some(bundle.clone()),
        }
        let auth_required = options.key.is_some();
        let diagnostic_key = options.key.clone();
        config.key = options.key;
        config.writable = options.uploads || options.manage;
        config.modules = vec!["files".into()];
        if options.uploads {
            config.modules.push("uploads".into());
        }
        if options.manage {
            config.modules.push("manage".into());
        }
        let running = share_http::start(config).await.map_err(|error| {
            StartFailure::new(match error {
                ServerError::Policy => StartReason::Key,
                ServerError::Root(_) => StartReason::Folder,
                ServerError::Assets(_) => StartReason::Resources,
                ServerError::Io(error) => match error.kind() {
                    std::io::ErrorKind::AddrInUse => StartReason::PortInUse,
                    std::io::ErrorKind::AddrNotAvailable | std::io::ErrorKind::PermissionDenied => {
                        StartReason::Port
                    }
                    _ => StartReason::Other,
                },
                _ => StartReason::Other,
            })
        })?;
        let port = running.address().port();
        state.endpoints = network::endpoints(port, options.network);
        state.addresses = state
            .endpoints
            .iter()
            .filter(|endpoint| endpoint.kind != LinkKind::Loopback)
            .map(|endpoint| endpoint.url.clone())
            .collect();
        state.active_settings = Some(ActiveSettings {
            network: options.network,
            port,
            uploads: options.uploads,
            manage: options.manage,
            auth_required,
        });
        state.diagnostics = Some(Diagnostics::new(
            port,
            diagnostic_key,
            running.download_control(),
            self.probe_permit.clone(),
        ));
        state.running = Some(running);
        Ok(state.status(self.capabilities))
    }

    pub async fn stop(&self) -> Result<HostStatus, String> {
        let mut state = self.state.lock().await;
        if let Some(diagnostics) = state.diagnostics.take() {
            diagnostics.stop();
        }
        state.addresses.clear();
        state.endpoints.clear();
        state.active_settings = None;
        if let Some(running) = state.running.take() {
            running.request_stop();
            running
                .stop()
                .await
                .map_err(|_| "The service could not shut down cleanly")?;
        }
        Ok(state.status(self.capabilities))
    }

    pub async fn diagnostic_status(&self) -> DiagnosticStatus {
        self.state
            .lock()
            .await
            .diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.status())
            .unwrap_or_default()
    }

    pub async fn set_diagnostic_mode(&self, enabled: bool) -> Result<DiagnosticStatus, String> {
        let state = self.state.lock().await;
        match &state.diagnostics {
            Some(diagnostics) => Ok(diagnostics.set_enabled(enabled)),
            None if !enabled => Ok(DiagnosticStatus::default()),
            None => Err("Start sharing before enabling local diagnostics".into()),
        }
    }

    pub async fn set_download_buffer(&self, kib: usize) -> Result<DiagnosticStatus, String> {
        let state = self.state.lock().await;
        state
            .diagnostics
            .as_ref()
            .ok_or("Start sharing before changing local diagnostics")?
            .set_buffer(Some(kib))
    }

    pub async fn reset_diagnostic_settings(&self) -> Result<DiagnosticStatus, String> {
        let state = self.state.lock().await;
        state
            .diagnostics
            .as_ref()
            .ok_or("Start sharing before changing local diagnostics")?
            .set_buffer(None)
    }

    pub async fn cancel_diagnostic_test(&self) -> Result<DiagnosticStatus, String> {
        Ok(self
            .state
            .lock()
            .await
            .diagnostics
            .as_ref()
            .map(|diagnostics| diagnostics.cancel())
            .unwrap_or_default())
    }

    pub async fn run_diagnostic_test(
        &self,
        path: String,
        connections: usize,
    ) -> Result<DiagnosticReport, String> {
        let probe = {
            let state = self.state.lock().await;
            state
                .diagnostics
                .as_ref()
                .ok_or("Start sharing before running a local test")?
                .begin(path, connections)?
        };
        probe.run().await
    }

    pub async fn status(&self) -> HostStatus {
        self.state.lock().await.status(self.capabilities)
    }

    /// Labelled share addresses in recommended order while running; this
    /// device is last. `HostStatus::addresses` lists the same LAN URLs.
    pub async fn endpoints(&self) -> Vec<ShareEndpoint> {
        self.state.lock().await.endpoints.clone()
    }

    pub async fn transfer_control(&self) -> Option<share_http::TransferControl> {
        self.state
            .lock()
            .await
            .running
            .as_ref()
            .map(RunningServer::transfer_control)
    }
}

fn checked_root(root: PathBuf) -> Result<PathBuf, String> {
    let root = root
        .canonicalize()
        .map_err(|_| "The selected folder is unavailable")?;
    if !root.is_dir() {
        return Err("Choose a folder to share".into());
    }
    Ok(root)
}

/// Packaged resources are authoritative. A checked build-time repository path
/// is permitted only with debug assertions, never as a release cwd fallback.
pub fn resolve_web_dir(resources: &Path, debug_fallback: Option<&Path>) -> Result<PathBuf, String> {
    let packaged = resources.join("web");
    if checked_assets(&packaged) {
        return Ok(packaged);
    }
    if cfg!(debug_assertions) {
        if let Some(fallback) = debug_fallback.filter(|path| checked_assets(path)) {
            return Ok(fallback.to_path_buf());
        }
    }
    Err("The packaged web assets are missing or incomplete".into())
}

fn checked_assets(path: &Path) -> bool {
    path.join("index.html").is_file() && path.join("_next").is_dir()
}
