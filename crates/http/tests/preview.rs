//! Inline preview is an opt-in presentation of the same authorized download.
//! Only an extension allow-list receives a specific inert media type; active
//! documents keep the octet-stream attachment response.
use axum::{body::Body, Router};
use http::Request;
use http_body_util::BodyExt;
use serde_json::Value;
use share_http::{router_with_store, ServerConfig};
use share_storage::FsStore;
use std::sync::Arc;
use tower::ServiceExt;

const SANDBOX: &str = "sandbox; default-src 'none'";
const BEARER: [(&str, &str); 1] = [("authorization", "Bearer test-access-key")];

struct Fixture {
    root: tempfile::TempDir,
    router: Router,
}

impl Fixture {
    fn new(modules: &[&str]) -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
        config.key = Some("test-access-key".into());
        config.modules = modules.iter().map(|id| (*id).into()).collect();
        let store = Arc::new(FsStore::new(root.path()).unwrap());
        let router = router_with_store(&config, store).unwrap();
        Self { root, router }
    }

    fn write(&self, name: &str, bytes: &[u8]) {
        std::fs::write(self.root.path().join(name), bytes).unwrap();
    }

    async fn request(
        &self,
        method: &str,
        uri: &str,
        headers: &[(&str, &str)],
    ) -> http::Response<Body> {
        let mut request = Request::builder()
            .method(method)
            .uri(uri)
            .header("host", "localhost:5000");
        for (name, value) in headers {
            request = request.header(*name, *value);
        }
        self.router
            .clone()
            .oneshot(request.body(Body::empty()).unwrap())
            .await
            .unwrap()
    }

    async fn inline(&self, name: &str) -> http::Response<Body> {
        self.request(
            "GET",
            &format!("/api/v1/content?path={name}&inline=1"),
            &BEARER,
        )
        .await
    }
}

fn header<'a>(response: &'a http::Response<Body>, name: &str) -> &'a str {
    response
        .headers()
        .get(name)
        .unwrap_or_else(|| panic!("missing {name}"))
        .to_str()
        .unwrap()
}

fn assert_attachment(response: &http::Response<Body>, name: &str) {
    assert_eq!(response.status(), 200, "{name}");
    assert_eq!(
        header(response, "content-type"),
        "application/octet-stream",
        "{name}"
    );
    assert!(
        header(response, "content-disposition").starts_with("attachment; "),
        "{name} must remain a download"
    );
    assert_eq!(header(response, "x-content-type-options"), "nosniff");
    assert!(
        response.headers().get("content-security-policy").is_none(),
        "{name} attachment headers must remain unchanged"
    );
}

#[tokio::test]
async fn allow_listed_extensions_are_served_inline_with_inert_types() {
    let fixture = Fixture::new(&["files"]);
    for (name, expected) in [
        ("photo.png", "image/png"),
        ("SCAN.JPG", "image/jpeg"),
        ("scan.jpeg", "image/jpeg"),
        ("loop.gif", "image/gif"),
        ("modern.webp", "image/webp"),
        ("modern.avif", "image/avif"),
        ("old.bmp", "image/bmp"),
        ("favicon.ico", "image/x-icon"),
        ("song.mp3", "audio/mpeg"),
        ("voice.m4a", "audio/mp4"),
        ("voice.aac", "audio/aac"),
        ("voice.ogg", "audio/ogg"),
        ("voice.opus", "audio/ogg"),
        ("voice.wav", "audio/wav"),
        ("voice.flac", "audio/flac"),
        ("clip.mp4", "video/mp4"),
        ("clip.m4v", "video/mp4"),
        ("clip.webm", "video/webm"),
        ("clip.ogv", "video/ogg"),
        ("clip.MOV", "video/quicktime"),
        ("notes.txt", "text/plain; charset=utf-8"),
        ("README.md", "text/plain; charset=utf-8"),
        ("server.log", "text/plain; charset=utf-8"),
        ("data.json", "text/plain; charset=utf-8"),
        ("table.csv", "text/plain; charset=utf-8"),
        ("table.tsv", "text/plain; charset=utf-8"),
        ("feed.xml", "text/plain; charset=utf-8"),
        ("compose.yaml", "text/plain; charset=utf-8"),
        ("compose.yml", "text/plain; charset=utf-8"),
        ("Cargo.toml", "text/plain; charset=utf-8"),
        ("settings.ini", "text/plain; charset=utf-8"),
        ("nginx.conf", "text/plain; charset=utf-8"),
        ("main.rs", "text/plain; charset=utf-8"),
        ("app.js", "text/plain; charset=utf-8"),
        ("style.css", "text/plain; charset=utf-8"),
        ("page.html.txt", "text/plain; charset=utf-8"),
    ] {
        fixture.write(name, b"sample bytes");
        let response = fixture.inline(name).await;
        assert_eq!(response.status(), 200, "{name}");
        assert_eq!(header(&response, "content-type"), expected, "{name}");
        let disposition = header(&response, "content-disposition");
        assert!(
            disposition.starts_with("inline; filename=\"download\"; filename*=UTF-8''"),
            "{name}: {disposition}"
        );
        assert_eq!(header(&response, "x-content-type-options"), "nosniff");
        assert_eq!(header(&response, "content-security-policy"), SANDBOX);
        assert_eq!(header(&response, "accept-ranges"), "bytes");
        assert_eq!(header(&response, "content-length"), "12");
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            "sample bytes"
        );
    }
}

#[tokio::test]
async fn active_and_unknown_documents_remain_octet_stream_attachments() {
    let fixture = Fixture::new(&["files"]);
    for name in [
        "page.html",
        "page.HTM",
        "page.xhtml",
        "logo.svg",
        "logo.SVGZ",
        "paper.pdf",
        "archive.zip",
        "program.exe",
        "photo.png.html",
        "no-extension",
        "trailing.png.bin",
    ] {
        fixture.write(name, b"<script>attack()</script>");
        assert_attachment(&fixture.inline(name).await, name);
    }
}

#[tokio::test]
async fn the_default_and_disabled_flag_keep_previewable_files_as_attachments() {
    let fixture = Fixture::new(&["files"]);
    fixture.write("photo.png", b"image bytes");
    for query in [
        "path=photo.png",
        "path=photo.png&inline=0",
        "path=photo.png&inline=false",
        "path=photo.png&inline=",
        "path=photo.png&inline=yes",
    ] {
        let response = fixture
            .request("GET", &format!("/api/v1/content?{query}"), &BEARER)
            .await;
        assert_attachment(&response, query);
    }
    let accepted = fixture
        .request("GET", "/api/v1/content?path=photo.png&inline=true", &BEARER)
        .await;
    assert_eq!(header(&accepted, "content-type"), "image/png");
}

#[tokio::test]
async fn inline_ranges_and_head_keep_download_semantics() {
    let fixture = Fixture::new(&["files"]);
    fixture.write("notes.txt", b"0123456789");
    let range = fixture
        .request(
            "GET",
            "/api/v1/content?path=notes.txt&inline=1",
            &[BEARER[0], ("range", "bytes=2-5")],
        )
        .await;
    assert_eq!(range.status(), 206);
    assert_eq!(header(&range, "content-range"), "bytes 2-5/10");
    assert_eq!(header(&range, "content-length"), "4");
    assert_eq!(header(&range, "content-type"), "text/plain; charset=utf-8");
    assert_eq!(header(&range, "content-security-policy"), SANDBOX);
    assert!(header(&range, "content-disposition").starts_with("inline; "));
    assert_eq!(
        range.into_body().collect().await.unwrap().to_bytes(),
        "2345"
    );

    let beyond = fixture
        .request(
            "GET",
            "/api/v1/content?path=notes.txt&inline=1",
            &[BEARER[0], ("range", "bytes=10-")],
        )
        .await;
    assert_eq!(beyond.status(), 416);
    assert_eq!(header(&beyond, "content-range"), "bytes */10");

    let head = fixture
        .request("HEAD", "/api/v1/content?path=notes.txt&inline=1", &BEARER)
        .await;
    assert_eq!(head.status(), 200);
    assert_eq!(header(&head, "content-type"), "text/plain; charset=utf-8");
    assert!(header(&head, "content-disposition").starts_with("inline; "));
    assert_eq!(header(&head, "content-security-policy"), SANDBOX);
    assert_eq!(header(&head, "content-length"), "10");
    assert!(head
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .is_empty());

    let attachment_head = fixture
        .request("HEAD", "/api/v1/content?path=notes.txt", &BEARER)
        .await;
    assert_eq!(
        header(&attachment_head, "content-type"),
        "application/octet-stream"
    );
}

#[tokio::test]
async fn inline_requests_need_the_same_authorization_and_valid_paths() {
    let fixture = Fixture::new(&["files"]);
    fixture.write("photo.png", b"private image");
    for method in ["GET", "HEAD"] {
        let response = fixture
            .request(method, "/api/v1/content?path=photo.png&inline=1", &[])
            .await;
        assert_eq!(response.status(), 401, "{method}");
        assert!(response.headers().get("content-disposition").is_none());
    }
    let wrong = fixture
        .request(
            "GET",
            "/api/v1/content?path=photo.png&inline=1",
            &[("authorization", "Bearer wrong")],
        )
        .await;
    assert_eq!(wrong.status(), 401);
    for path in ["../photo.png", "missing.png", ""] {
        let response = fixture
            .request(
                "GET",
                &format!("/api/v1/content?path={path}&inline=1"),
                &BEARER,
            )
            .await;
        assert!(
            matches!(response.status().as_u16(), 400 | 404),
            "{path}: {}",
            response.status()
        );
    }
}

#[tokio::test]
async fn unicode_names_keep_the_encoded_filename_when_inline() {
    let fixture = Fixture::new(&["files"]);
    fixture.write("资料 照片.png", b"image");
    let response = fixture
        .request(
            "GET",
            "/api/v1/content?path=%E8%B5%84%E6%96%99%20%E7%85%A7%E7%89%87.png&inline=1",
            &BEARER,
        )
        .await;
    assert_eq!(response.status(), 200);
    assert_eq!(
        header(&response, "content-disposition"),
        "inline; filename=\"download\"; filename*=UTF-8''%E8%B5%84%E6%96%99%20%E7%85%A7%E7%89%87%2E%70%6E%67"
    );
}

#[tokio::test]
async fn context_advertises_inline_preview_as_an_additive_capability() {
    let fixture = Fixture::new(&["files"]);
    let response = fixture.request("GET", "/api/v1/context", &BEARER).await;
    let context: Value =
        serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(context["capabilities"]["inline_preview"], true);
    assert_eq!(context["capabilities"]["directory_copy"], false);
    assert!(context["capabilities"]["file_move"].is_boolean());
    assert!(context["capabilities"]["directory_move"].is_boolean());
}
