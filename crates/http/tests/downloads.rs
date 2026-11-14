use async_trait::async_trait;
use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use share_domain::{Entry, EntryKind, Error, RelativePath};
use share_http::{router_with_store, ServerConfig};
use share_storage::{FileReader, FileStore, FsStore, OpenFile, Reader};
use std::{
    io::Cursor,
    pin::Pin,
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    },
    task::{Context, Poll},
};
use tokio::io::{AsyncRead, AsyncSeek, ReadBuf};
use tower::ServiceExt;

struct ObservedReader {
    inner: Cursor<Vec<u8>>,
    read_bytes: Arc<AtomicUsize>,
}

impl AsyncRead for ObservedReader {
    fn poll_read(
        mut self: Pin<&mut Self>,
        cx: &mut Context<'_>,
        buf: &mut ReadBuf<'_>,
    ) -> Poll<std::io::Result<()>> {
        let previous = buf.filled().len();
        let result = Pin::new(&mut self.inner).poll_read(cx, buf);
        self.read_bytes
            .fetch_add(buf.filled().len() - previous, Ordering::Relaxed);
        result
    }
}

impl AsyncSeek for ObservedReader {
    fn start_seek(mut self: Pin<&mut Self>, position: std::io::SeekFrom) -> std::io::Result<()> {
        Pin::new(&mut self.inner).start_seek(position)
    }
    fn poll_complete(mut self: Pin<&mut Self>, cx: &mut Context<'_>) -> Poll<std::io::Result<u64>> {
        Pin::new(&mut self.inner).poll_complete(cx)
    }
}

struct ObservedStore {
    data: Vec<u8>,
    read_bytes: Arc<AtomicUsize>,
    opens: AtomicUsize,
}

#[async_trait]
impl FileStore for ObservedStore {
    async fn list(&self, _: &RelativePath) -> Result<Vec<Entry>, Error> {
        Err(Error::Unsupported)
    }
    async fn open(&self, path: &RelativePath, offset: u64) -> Result<OpenFile, Error> {
        let opened = self.opens.fetch_add(1, Ordering::Relaxed);
        let data = if opened == 0 {
            self.data.clone()
        } else {
            b"replacement".to_vec()
        };
        let entry = Entry {
            name: "archive.bin".into(),
            path: path.as_str().into(),
            kind: EntryKind::File,
            size: Some(data.len() as u64),
            modified: None,
        };
        let mut inner = Cursor::new(data);
        inner.set_position(offset);
        Ok(OpenFile {
            entry,
            reader: FileReader::seekable(ObservedReader {
                inner,
                read_bytes: self.read_bytes.clone(),
            }),
        })
    }
    async fn upload(&self, _: &RelativePath, _: Reader, _: u64) -> Result<Entry, Error> {
        Err(Error::Unsupported)
    }
    async fn mkdir(&self, _: &RelativePath) -> Result<Entry, Error> {
        Err(Error::Unsupported)
    }
    async fn move_entry(&self, _: &RelativePath, _: &RelativePath) -> Result<Entry, Error> {
        Err(Error::Unsupported)
    }
    async fn copy(&self, _: &RelativePath, _: &RelativePath) -> Result<Entry, Error> {
        Err(Error::Unsupported)
    }
    async fn remove(&self, _: &RelativePath, _: bool) -> Result<(), Error> {
        Err(Error::Unsupported)
    }
}

#[tokio::test]
async fn high_offset_ranges_seek_on_the_original_handle_without_reading_the_prefix() {
    let data: Vec<u8> = (0..8 * 1024 * 1024 + 13)
        .map(|index| (index % 251) as u8)
        .collect();
    let expected = data[data.len() - 37..].to_vec();
    let bytes_read = Arc::new(AtomicUsize::new(0));
    let store = Arc::new(ObservedStore {
        data,
        read_bytes: bytes_read.clone(),
        opens: AtomicUsize::new(0),
    });
    let mut config = ServerConfig::new(
        "unused-provider-root".into(),
        "127.0.0.1:0".parse().unwrap(),
    );
    config.key = Some("download-test-key".into());
    config.modules = vec!["files".into()];
    let router = router_with_store(&config, store.clone()).unwrap();
    let response = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/content?path=archive.bin")
                .header("authorization", "Bearer download-test-key")
                .header("range", "bytes=8388584-8388620")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 206);
    assert_eq!(
        response.headers()["content-range"],
        "bytes 8388584-8388620/8388621"
    );
    assert_eq!(response.headers()["content-length"], "37");
    assert_eq!(
        response.into_body().collect().await.unwrap().to_bytes(),
        expected
    );
    assert_eq!(
        store.opens.load(Ordering::Relaxed),
        1,
        "reopening can replace the object after its metadata was read"
    );
    assert!(
        bytes_read.load(Ordering::Relaxed) <= 128 * 1024,
        "high-offset Range read {} bytes to return only 37",
        bytes_read.load(Ordering::Relaxed)
    );
}

async fn count_batches(mut body: Body) -> usize {
    let mut batches = 0;
    let mut bytes = 0;
    while let Some(frame) = body.frame().await {
        bytes += frame.unwrap().into_data().unwrap().len();
        batches += 1;
    }
    assert_eq!(bytes, 1024 * 1024 + 17);
    batches
}

async fn download(router: &axum::Router) -> Body {
    let response = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/content?path=sample.bin")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(response.status(), 200);
    response.into_body()
}

fn tunable_router() -> (tempfile::TempDir, ServerConfig, axum::Router) {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("sample.bin"), vec![42; 1024 * 1024 + 17]).unwrap();
    let config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    let router = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap())).unwrap();
    (root, config, router)
}

#[tokio::test]
async fn tuning_changes_new_downloads_and_preserves_an_existing_streams_buffer() {
    let (_root, config, router) = tunable_router();
    let existing = download(&router).await;
    config.download_control.set_buffer_kib(16).unwrap();
    let subsequent = download(&router).await;
    assert!(
        count_batches(existing).await <= 17,
        "an existing stream lost its initial buffer"
    );
    assert!(
        count_batches(subsequent).await >= 33,
        "the next request ignored the smaller buffer"
    );
}

#[tokio::test]
async fn invalid_tuning_preserves_the_effective_download_setting() {
    let (_root, config, router) = tunable_router();
    config.download_control.set_buffer_kib(16).unwrap();
    for invalid in [0, 4, 15, 17, 129, usize::MAX] {
        assert!(config.download_control.set_buffer_kib(invalid).is_err());
    }
    assert!(count_batches(download(&router).await).await >= 33);
}

#[tokio::test]
async fn reset_restores_batched_downloads_for_the_next_request() {
    let (_root, config, router) = tunable_router();
    config.download_control.set_buffer_kib(16).unwrap();
    assert!(count_batches(download(&router).await).await >= 33);
    config.download_control.reset();
    assert!(count_batches(download(&router).await).await <= 17);
}
