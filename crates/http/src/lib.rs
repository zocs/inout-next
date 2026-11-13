//! Independent HTTP host. Applications may inject any [`FileStore`] through
//! [`start_with_store`] or [`router_with_store`] without exposing ambient roots.
mod assets;
mod auth;
mod routes;

use axum::{
    extract::{DefaultBodyLimit, Request, State},
    middleware::{self, Next},
    response::{IntoResponse, Response},
    Router,
};
use share_domain::{Error, ModuleRegistry, Product};
use share_storage::{FileStore, FsStore};
use std::{net::SocketAddr, path::PathBuf, sync::Arc, time::Duration};
use tokio::task::JoinHandle;
use tokio_util::{sync::CancellationToken, task::TaskTracker};

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
    #[error("Shutdown did not drain pending requests")]
    Shutdown,
}

pub struct RunningServer {
    address: SocketAddr,
    cancellation: CancellationToken,
    task: Option<JoinHandle<Result<(), std::io::Error>>>,
    requests: TaskTracker,
}
impl RunningServer {
    pub fn address(&self) -> SocketAddr {
        self.address
    }
    pub fn request_stop(&self) {
        self.cancellation.cancel();
        self.requests.close();
    }
    /// Cancel and drain requests for at most two seconds, then release the
    /// listener. A missed deadline returns Shutdown rather than false success.
    pub async fn stop(mut self) -> Result<(), ServerError> {
        self.request_stop();
        if let Some(mut task) = self.task.take() {
            let drained = async {
                self.requests.wait().await;
                (&mut task).await
            };
            match tokio::time::timeout(Duration::from_secs(2), drained).await {
                Ok(Ok(Ok(()))) => Ok(()),
                Ok(Ok(Err(error))) => Err(ServerError::Io(error)),
                Ok(Err(_)) => Err(ServerError::Task),
                Err(_) => {
                    task.abort();
                    let _ = task.await;
                    Err(ServerError::Shutdown)
                }
            }
        } else {
            Ok(())
        }
    }
}
impl Drop for RunningServer {
    fn drop(&mut self) {
        self.request_stop();
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
    let requests = TaskTracker::new();
    let router = build_router(&config, store, cancellation.clone(), requests.clone())?;
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
        requests,
    })
}

/// Build the same API without binding a listener. Storage owns all user-file I/O.
pub fn router_with_store(
    config: &ServerConfig,
    store: Arc<dyn FileStore>,
) -> Result<Router, ServerError> {
    build_router(config, store, CancellationToken::new(), TaskTracker::new())
}
fn build_router(
    config: &ServerConfig,
    store: Arc<dyn FileStore>,
    cancellation: CancellationToken,
    requests: TaskTracker,
) -> Result<Router, ServerError> {
    let registry = config.validate()?;
    let public_assets = if let Some(web_dir) = &config.web_dir {
        Some(assets::Assets::new(
            store.web_assets(web_dir).map_err(ServerError::Domain)?,
        ))
    } else {
        None
    };
    let request_control = RequestControl {
        cancellation: cancellation.clone(),
        requests,
    };
    let state = AppState {
        registry,
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
    let router = if let Some(assets) = public_assets {
        router.fallback(move |request: Request| {
            let assets = assets.clone();
            async move { assets.serve(request) }
        })
    } else {
        router
    };
    Ok(router.layer(middleware::from_fn_with_state(
        request_control,
        cancel_requests,
    )))
}

#[derive(Clone)]
struct RequestControl {
    cancellation: CancellationToken,
    requests: TaskTracker,
}

async fn cancel_requests(
    State(control): State<RequestControl>,
    request: Request,
    next: Next,
) -> Response {
    let _tracked = control.requests.token();
    // Dropping next.run drops body extractors, provider futures and any staging
    // guards they own. Bias cancellation so stopped requests cannot start work.
    tokio::select! {
        biased;
        _ = control.cancellation.cancelled() => routes::ApiError::from(Error::Conflict).into_response(),
        response = next.run(request) => response,
    }
}
