use serde::{Deserialize, Serialize};
use share_http::{RunningServer, ServerConfig};
use std::{
    net::{Ipv4Addr, SocketAddr},
    path::{Path, PathBuf},
};
use tokio::sync::Mutex;

#[derive(Default, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct StartOptions {
    pub network: bool,
    pub port: u16,
    pub uploads: bool,
    pub manage: bool,
    pub key: Option<String>,
}

#[derive(Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ActiveSettings {
    pub network: bool,
    pub port: u16,
    pub uploads: bool,
    pub manage: bool,
    pub auth_required: bool,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
pub struct HostStatus {
    pub root: Option<String>,
    pub running: bool,
    pub manager_url: Option<String>,
    pub addresses: Vec<String>,
    pub active_settings: Option<ActiveSettings>,
}

#[derive(Default)]
struct HostState {
    root: Option<PathBuf>,
    running: Option<RunningServer>,
    addresses: Vec<String>,
    active_settings: Option<ActiveSettings>,
}

impl HostState {
    fn status(&self) -> HostStatus {
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
        }
    }
}

/// Owns the service lifecycle. Serializing transitions prevents two starts or a
/// root change from racing a listener that is still starting or shutting down.
pub struct HostController {
    web_dir: PathBuf,
    state: Mutex<HostState>,
}

impl HostController {
    pub fn new(web_dir: PathBuf) -> Self {
        Self {
            web_dir,
            state: Mutex::new(HostState::default()),
        }
    }

    pub async fn select_root(&self, root: PathBuf) -> Result<HostStatus, String> {
        let mut state = self.state.lock().await;
        if state.running.is_some() {
            return Err("Stop the service before choosing another folder".into());
        }
        let root = root
            .canonicalize()
            .map_err(|_| "The selected folder is unavailable")?;
        if !root.is_dir() {
            return Err("Choose a folder to share".into());
        }
        state.root = Some(root);
        Ok(state.status())
    }

    pub async fn start(&self, options: StartOptions) -> Result<HostStatus, String> {
        let mut state = self.state.lock().await;
        if state.running.is_some() {
            return Err("The service is already running".into());
        }
        let root = state
            .root
            .clone()
            .ok_or("Choose a folder before starting the service")?;
        let listen = SocketAddr::from((
            if options.network {
                Ipv4Addr::UNSPECIFIED
            } else {
                Ipv4Addr::LOCALHOST
            },
            options.port,
        ));
        let mut config = ServerConfig::new(root, listen);
        config.web_dir = Some(self.web_dir.clone());
        let auth_required = options.key.is_some();
        config.key = options.key;
        config.writable = options.uploads || options.manage;
        config.modules = vec!["files".into()];
        if options.uploads {
            config.modules.push("uploads".into());
        }
        if options.manage {
            config.modules.push("manage".into());
        }
        let running = share_http::start(config)
            .await
            .map_err(|error| match error {
                share_http::ServerError::Policy => "Enter a nonempty access key to enable writes",
                _ => "The service could not start; check the folder, assets and port",
            })?;
        state.addresses = if options.network {
            lan_addresses(running.address().port())
        } else {
            Vec::new()
        };
        state.active_settings = Some(ActiveSettings {
            network: options.network,
            port: running.address().port(),
            uploads: options.uploads,
            manage: options.manage,
            auth_required,
        });
        state.running = Some(running);
        Ok(state.status())
    }

    pub async fn stop(&self) -> Result<HostStatus, String> {
        let mut state = self.state.lock().await;
        state.addresses.clear();
        state.active_settings = None;
        if let Some(running) = state.running.take() {
            running.request_stop();
            running
                .stop()
                .await
                .map_err(|_| "The service could not shut down cleanly")?;
        }
        Ok(state.status())
    }

    pub async fn status(&self) -> HostStatus {
        self.state.lock().await.status()
    }
}

fn lan_addresses(port: u16) -> Vec<String> {
    let mut addresses: Vec<_> = if_addrs::get_if_addrs()
        .unwrap_or_default()
        .into_iter()
        .filter_map(|interface| match interface.ip() {
            std::net::IpAddr::V4(ip)
                if !ip.is_loopback()
                    && !ip.is_unspecified()
                    && !ip.is_link_local()
                    && !ip.is_multicast()
                    && !ip.is_broadcast() =>
            {
                Some(format!("http://{ip}:{port}"))
            }
            _ => None,
        })
        .collect();
    addresses.sort();
    addresses.dedup();
    addresses
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
