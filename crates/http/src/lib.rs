//! Independent HTTP host. Applications may inject any [`FileStore`] through
//! [`start_with_store`] or [`router_with_store`] without exposing ambient roots.
mod auth;
mod routes;

use axum::{extract::DefaultBodyLimit, Router};
use share_domain::{Error, ModuleRegistry, Product};
use share_storage::{FileStore, FsStore};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio_util::sync::CancellationToken;
use tower_http::services::ServeDir;

pub struct ServerConfig {
    pub root: PathBuf,
    pub listen: SocketAddr,
    pub web_dir: Option<PathBuf>,
    pub key: Option<String>,
    pub writable: bool,
    pub modules: Vec<String>,
    pub max_upload_bytes: u64,
    pub product: Product,
}
impl ServerConfig {
    pub fn new(root: PathBuf, listen: SocketAddr) -> Self {
        Self {
            root,
            listen,
            web_dir: None,
            key: None,
            writable: false,
            modules: vec!["files".into(), "uploads".into(), "manage".into()],
            max_upload_bytes: 2147483648,
            product: Product::default(),
        }
    }
    fn validate(&self) -> Result<ModuleRegistry, ServerError> {
        if self.key.as_ref().is_some_and(|key| key.trim().is_empty())
            || (self.writable && self.key.is_none())
        {
            return Err(ServerError::Policy);
        }
        ModuleRegistry::new(&self.modules).map_err(ServerError::Domain)
    }
}

#[derive(Debug, thiserror::Error)]
pub enum ServerError {
    #[error("Writable mode requires a nonempty access key")]
    Policy,
    #[error("Invalid service configuration: {0}")]
    Domain(Error),
    #[error("Server network operation failed")]
    Io(#[source] std::io::Error),
    #[error("Server task failed")]
    Task,
}

pub struct RunningServer {
    address: SocketAddr,
    cancellation: CancellationToken,
    task: Option<JoinHandle<Result<(), std::io::Error>>>,
}
impl RunningServer {
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn request_stop(&self) {
        self.cancellation.cancel();
    }
    /// Drain active requests for at most two seconds, then release the listener.
    pub async fn stop(mut self) -> Result<(), ServerError> {
        self.request_stop();
        if let Some(mut task) = self.task.take() {
            match tokio::time::timeout(Duration::from_secs(2), &mut task).await {
                Ok(Ok(Ok(()))) => Ok(()),
                Ok(Ok(Err(error))) => Err(ServerError::Io(error)),
                Ok(Err(_)) => Err(ServerError::Task),
                Err(_) => {
                    task.abort();
                    let _ = task.await;
                    Ok(())
                }
            }
        } else {
            Ok(())
        }
    }
}
impl Drop for RunningServer {
    fn drop(&mut self) {
        self.cancellation.cancel();
        if let Some(task) = self.task.take() {
            task.abort();
        }
    }
}

#[derive(Clone)]
struct AppState {
    store: Arc<dyn FileStore>,
    product: Product,
    registry: ModuleRegistry,
    writable: bool,
    max_upload_bytes: u64,
    auth: Arc<auth::Auth>,
    cancellation: CancellationToken,
}

pub async fn start(config: ServerConfig) -> Result<RunningServer, ServerError> {
    config.validate()?;
    let store = Arc::new(FsStore::new(&config.root).map_err(ServerError::Domain)?);
    start_with_store(config, store).await
}

/// Start with a replaceable storage provider; `config.root` is never opened here.
/// A platform host can use document providers or a remote/test backend.
pub async fn start_with_store(
    config: ServerConfig,
    store: Arc<dyn FileStore>,
) -> Result<RunningServer, ServerError> {
    let cancellation = CancellationToken::new();
    let router = build_router(&config, store, cancellation.clone())?;
    let listener = tokio::net::TcpListener::bind(config.listen)
        .await
        .map_err(ServerError::Io)?;
    let address = listener.local_addr().map_err(ServerError::Io)?;
    let stop = cancellation.clone();
    let task = tokio::spawn(async move {
        axum::serve(listener, router)
            .with_graceful_shutdown(stop.cancelled_owned())
            .await
    });
    Ok(RunningServer {
        address,
        cancellation,
        task: Some(task),
    })
}

/// Build the same API without binding a listener. Storage owns all user-file I/O.
pub fn router_with_store(
    config: &ServerConfig,
    store: Arc<dyn FileStore>,
) -> Result<Router, ServerError> {
    build_router(config, store, CancellationToken::new())
}
fn build_router(
    config: &ServerConfig,
    store: Arc<dyn FileStore>,
    cancellation: CancellationToken,
) -> Result<Router, ServerError> {
    let state = AppState {
        registry: config.validate()?,
        store,
        product: config.product.clone(),
        writable: config.writable,
        max_upload_bytes: config.max_upload_bytes,
        auth: Arc::new(auth::Auth::new(config.key.as_deref())),
        cancellation,
    };
    let api = routes::mount(&state)
        .layer(DefaultBodyLimit::max(64 * 1024))
        .with_state(state);
    let router = Router::new()
        .nest("/api/v1", api)
        .fallback(routes::not_found);
    Ok(if let Some(web_dir) = &config.web_dir {
        router.fallback_service(ServeDir::new(web_dir).append_index_html_on_directories(true))
    } else {
        router
    })
}
