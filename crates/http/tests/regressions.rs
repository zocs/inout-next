use async_trait::async_trait;
use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use share_domain::{Entry, EntryKind, Error, RelativePath};
use share_http::{router_with_store, start, start_with_store, ServerConfig};
use share_storage::{FileStore, FsStore, OpenFile, Reader};
use std::{
    path::Path,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
    time::Duration,
};
use tokio::{
    io::{AsyncRead, AsyncWriteExt, ReadBuf},
    sync::Notify,
};
use tower::ServiceExt;

fn config(root: &Path) -> ServerConfig {
    let mut config = ServerConfig::new(root.to_owned(), "127.0.0.1:0".parse().unwrap());
    config.key = Some("test-key".into());
    config.writable = true;
    config
}

#[tokio::test]
async fn asset_trees_must_not_overlap_actual_shared_storage_in_either_direction() {
    let parent = tempfile::tempdir().unwrap();
    let share = parent.path().join("share");
    let assets = parent.path().join("assets");
    std::fs::create_dir(&share).unwrap();
    std::fs::create_dir(&assets).unwrap();
    std::fs::write(share.join("attack.html"), b"<script>attack()</script>").unwrap();
    for web in [&share, parent.path()] {
        let mut config = config(&share);
        config.web_dir = Some(web.to_owned());
        assert!(start(config).await.is_err(), "overlapping assets accepted");
    }
    std::fs::create_dir(share.join("inside")).unwrap();
    let mut inside = config(&share);
    inside.web_dir = Some(share.join("inside"));
    assert!(start(inside).await.is_err());
    let mut isolated = config(&share);
    isolated.root = parent.path().join("must-not-be-opened");
    isolated.web_dir = Some(assets);
    let router = router_with_store(&isolated, Arc::new(FsStore::new(&share).unwrap())).unwrap();
    for path in ["/attack.html", "/api/v1/content?path=attack.html"] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(
            response.status(),
            if path.starts_with("/api") { 401 } else { 404 }
        );
    }
}

#[cfg(unix)]
#[tokio::test]
async fn asset_aliases_are_rejected_and_post_start_filesystem_changes_cannot_expose_shared_html() {
    let parent = tempfile::tempdir().unwrap();
    let share = parent.path().join("share");
    let assets = parent.path().join("assets");
    std::fs::create_dir(&share).unwrap();
    std::fs::create_dir(&assets).unwrap();
    std::fs::write(share.join("secret.html"), b"private HTML").unwrap();
    let alias = parent.path().join("alias");
    std::os::unix::fs::symlink(&share, &alias).unwrap();
    let mut aliased = config(&share);
    aliased.web_dir = Some(alias);
    assert!(start(aliased).await.is_err());
    std::fs::hard_link(share.join("secret.html"), assets.join("linked.html")).unwrap();
    let mut linked = config(&share);
    linked.web_dir = Some(assets.clone());
    assert!(start(linked).await.is_err());
    std::fs::remove_file(assets.join("linked.html")).unwrap();
    std::fs::write(assets.join("index.html"), b"trusted HTML").unwrap();
    let mut isolated = config(&share);
    isolated.web_dir = Some(assets.clone());
    let router = router_with_store(&isolated, Arc::new(FsStore::new(&share).unwrap())).unwrap();
    std::fs::remove_file(assets.join("index.html")).unwrap();
    std::os::unix::fs::symlink(share.join("secret.html"), assets.join("index.html")).unwrap();
    let response = router
        .oneshot(Request::builder().uri("/").body(Body::empty()).unwrap())
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        "trusted HTML"
    );
}

#[derive(Clone, Copy, PartialEq)]
enum Mode {
    Open,
    Range,
    Copy,
    List,
    Manage,
}
struct PendingStore {
    actual: FsStore,
    mode: Mode,
    reader: tokio::sync::Mutex<Option<Reader>>,
    started: Arc<Notify>,
    released: Arc<AtomicUsize>,
}
struct ReaderGuard {
    reader: tokio::io::DuplexStream,
    started: Arc<Notify>,
    released: Arc<AtomicUsize>,
}
impl AsyncRead for ReaderGuard {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        self.started.notify_one();
        Pin::new(&mut self.reader).poll_read(cx, buf)
    }
}
impl Drop for ReaderGuard {
    fn drop(&mut self) {
        self.released.fetch_add(1, Ordering::SeqCst);
    }
}
struct PendingGuard(Arc<AtomicUsize>);
impl Drop for PendingGuard {
    fn drop(&mut self) {
        self.0.fetch_add(1, Ordering::SeqCst);
    }
}
impl PendingStore {
    async fn wait<T>(&self) -> T {
        let _guard = PendingGuard(self.released.clone());
        self.started.notify_one();
        std::future::pending().await
    }
}
#[async_trait]
impl FileStore for PendingStore {
    async fn list(&self, path: &RelativePath) -> Result<Vec<Entry>, Error> {
        if self.mode == Mode::List {
            self.wait().await
        } else {
            self.actual.list(path).await
        }
    }
    async fn open(&self, path: &RelativePath, offset: u64) -> Result<OpenFile, Error> {
        if self.mode == Mode::Open {
            return self.wait().await;
        }
        if self.mode == Mode::Range {
            return Ok(OpenFile {
                entry: Entry {
                    name: "source".into(),
                    path: path.as_str().into(),
                    kind: EntryKind::File,
                    size: Some(6),
                    modified: None,
                },
                reader: self.reader.lock().await.take().unwrap(),
            });
        }
        self.actual.open(path, offset).await
    }
    async fn upload(
        &self,
        path: &RelativePath,
        reader: Reader,
        limit: u64,
    ) -> Result<Entry, Error> {
        self.actual.upload(path, reader, limit).await
    }
    async fn mkdir(&self, path: &RelativePath) -> Result<Entry, Error> {
        if self.mode == Mode::Manage {
            self.wait().await
        } else {
            self.actual.mkdir(path).await
        }
    }
    async fn move_entry(&self, from: &RelativePath, to: &RelativePath) -> Result<Entry, Error> {
        self.actual.move_entry(from, to).await
    }
    async fn copy(&self, _: &RelativePath, to: &RelativePath) -> Result<Entry, Error> {
        self.actual
            .upload(to, self.reader.lock().await.take().unwrap(), 100)
            .await
    }
    async fn remove(&self, path: &RelativePath, recursive: bool) -> Result<(), Error> {
        self.actual.remove(path, recursive).await
    }
}

#[tokio::test]
async fn unknown_injected_providers_reject_assets_without_opening_configured_root() {
    let root = tempfile::tempdir().unwrap();
    let assets = tempfile::tempdir().unwrap();
    let (_, reader) = tokio::io::duplex(1);
    let provider = Arc::new(PendingStore {
        actual: FsStore::new(root.path()).unwrap(),
        mode: Mode::List,
        reader: tokio::sync::Mutex::new(Some(Box::pin(reader))),
        started: Arc::new(Notify::new()),
        released: Arc::new(AtomicUsize::new(0)),
    });
    let mut config = config(&root.path().join("must-not-be-opened"));
    config.web_dir = Some(assets.path().to_owned());
    assert!(router_with_store(&config, provider).is_err());
}

async fn check_pending_shutdown(mode: Mode, drop_server: bool) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source"), b"abcdef").unwrap();
    let started = Arc::new(Notify::new());
    let released = Arc::new(AtomicUsize::new(0));
    let (mut writer, reader) = tokio::io::duplex(16);
    writer.write_all(b"a").await.unwrap();
    let reader = ReaderGuard {
        reader,
        started: started.clone(),
        released: released.clone(),
    };
    let provider = Arc::new(PendingStore {
        actual: FsStore::new(root.path()).unwrap(),
        mode,
        reader: tokio::sync::Mutex::new(Some(Box::pin(reader))),
        started: started.clone(),
        released: released.clone(),
    });
    let server = start_with_store(config(&root.path().join("must-not-be-opened")), provider)
        .await
        .unwrap();
    let mut connection = tokio::net::TcpStream::connect(server.address())
        .await
        .unwrap();
    let request = match mode {
        Mode::Open => "GET /api/v1/content?path=source HTTP/1.1\r\n",
        Mode::Range => "GET /api/v1/content?path=source HTTP/1.1\r\nRange: bytes=2-4\r\n",
        Mode::Copy => "POST /api/v1/operations HTTP/1.1\r\nContent-Type: application/json\r\n",
        Mode::List => "GET /api/v1/entries?path= HTTP/1.1\r\n",
        Mode::Manage => "POST /api/v1/directories HTTP/1.1\r\nContent-Type: application/json\r\n",
    };
    let body = match mode {
        Mode::Copy => r#"{"action":"copy","source":"source","destination":"copied"}"#,
        Mode::Manage => r#"{"path":"made"}"#,
        _ => "",
    };
    let length = if body.is_empty() {
        String::new()
    } else {
        format!("Content-Length: {}\r\n", body.len())
    };
    let request =
        format!("{request}{length}Host: localhost\r\nAuthorization: Bearer test-key\r\n\r\n{body}");
    connection.write_all(request.as_bytes()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), started.notified())
        .await
        .unwrap();
    if drop_server {
        drop(server);
    } else {
        tokio::time::timeout(Duration::from_secs(3), server.stop())
            .await
            .unwrap()
            .unwrap();
        // stop success promises that pending provider/request work has drained.
        assert!(
            released.load(Ordering::SeqCst) > 0,
            "stop left a pending storage future"
        );
    }
    tokio::time::timeout(Duration::from_secs(1), async {
        while released.load(Ordering::SeqCst) == 0 {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    if matches!(mode, Mode::Range | Mode::Copy) {
        assert!(writer.write_all(b"bcdef").await.is_err());
    }
    drop(writer);
    assert!(!root.path().join("copied").exists());
    assert!(!root.path().join("made").exists());
    assert_eq!(std::fs::read_dir(root.path()).unwrap().count(), 1);
}

#[tokio::test]
async fn stop_cancels_file_copy_before_publication_and_cleans_its_real_stage() {
    check_pending_shutdown(Mode::Copy, false).await;
}
#[tokio::test]
async fn drop_cancels_range_prefix_reader() {
    check_pending_shutdown(Mode::Range, true).await;
}
#[tokio::test]
async fn stop_cancels_pending_provider_open() {
    check_pending_shutdown(Mode::Open, false).await;
}
#[tokio::test]
async fn stop_cancels_pending_list_and_manage_provider_calls() {
    check_pending_shutdown(Mode::List, false).await;
    check_pending_shutdown(Mode::Manage, false).await;
}

#[tokio::test]
async fn stopping_pending_json_body_prevents_later_manage_publication() {
    let root = tempfile::tempdir().unwrap();
    let server = start(config(root.path())).await.unwrap();
    let address = server.address();
    let mut connection = tokio::net::TcpStream::connect(address).await.unwrap();
    connection.write_all(b"POST /api/v1/directories HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test-key\r\nContent-Type: application/json\r\nContent-Length: 15\r\n\r\n{\"path\":").await.unwrap();
    tokio::time::sleep(Duration::from_millis(10)).await;
    tokio::time::timeout(Duration::from_secs(3), server.stop())
        .await
        .unwrap()
        .unwrap();
    let _ = connection.write_all(b"\"made\"}").await;
    tokio::task::yield_now().await;
    assert!(!root.path().join("made").exists());
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    drop(listener);
}
