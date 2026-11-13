use axum::{body::Body, Router};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use serde_json::{json, Value};
use share_http::{router_with_store, start, ServerConfig};
use share_storage::FsStore;
use std::{sync::Arc, time::Duration};
use tower::ServiceExt;

struct Fixture {
    root: tempfile::TempDir,
    router: Router,
}
impl Fixture {
    fn new(writable: bool, modules: &[&str], limit: u64) -> Self {
        let root = tempfile::tempdir().unwrap();
        let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
        config.key = Some("test-access-key".into());
        config.writable = writable;
        config.modules = modules.iter().map(|id| (*id).into()).collect();
        config.max_upload_bytes = limit;
        let store = Arc::new(FsStore::new(root.path()).unwrap());
        let router = router_with_store(&config, store).unwrap();
        Self { root, router }
    }
    async fn request(
        &self,
        method: &str,
        uri: &str,
        headers: &[(&str, &str)],
        body: impl Into<Body>,
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
            .oneshot(request.body(body.into()).unwrap())
            .await
            .unwrap()
    }
}
const BEARER: [(&str, &str); 1] = [("authorization", "Bearer test-access-key")];
async fn json_body(response: http::Response<Body>) -> Value {
    serde_json::from_slice(&response.into_body().collect().await.unwrap().to_bytes()).unwrap()
}

#[tokio::test]
async fn authenticated_upload_collision_and_exact_ranges() {
    let fixture = Fixture::new(true, &["files", "uploads", "manage"], 100);
    let response = fixture
        .request("PUT", "/api/v1/content?path=a.bin", &BEARER, "abcdef")
        .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(json_body(response).await["entry"]["size"], 6);
    let collision = fixture
        .request("PUT", "/api/v1/content?path=a.bin", &BEARER, "replace")
        .await;
    assert_eq!(collision.status(), StatusCode::CONFLICT);
    assert_eq!(json_body(collision).await["error"]["code"], "target_exists");
    assert_eq!(
        std::fs::read(fixture.root.path().join("a.bin")).unwrap(),
        b"abcdef"
    );
    for (range, expected, content_range) in [
        ("bytes=2-4", "cde", "bytes 2-4/6"),
        ("bytes=-2", "ef", "bytes 4-5/6"),
        ("bytes=4-", "ef", "bytes 4-5/6"),
        ("bytes=0-999", "abcdef", "bytes 0-5/6"),
    ] {
        let response = fixture
            .request(
                "GET",
                "/api/v1/content?path=a.bin",
                &[("authorization", BEARER[0].1), ("range", range)],
                "",
            )
            .await;
        assert_eq!(response.status(), StatusCode::PARTIAL_CONTENT);
        assert_eq!(response.headers()["content-range"], content_range);
        assert_eq!(
            response.headers()["content-length"],
            expected.len().to_string()
        );
        assert_eq!(response.headers()["x-content-type-options"], "nosniff");
        assert!(response.headers()["content-disposition"]
            .to_str()
            .unwrap()
            .starts_with("attachment"));
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            expected
        );
    }
    let invalid = fixture
        .request(
            "GET",
            "/api/v1/content?path=a.bin",
            &[("authorization", BEARER[0].1), ("range", "bytes=6-")],
            "",
        )
        .await;
    assert_eq!(invalid.status(), StatusCode::RANGE_NOT_SATISFIABLE);
    assert_eq!(invalid.headers()["content-range"], "bytes */6");
    let head = fixture
        .request(
            "HEAD",
            "/api/v1/content?path=a.bin",
            &[("authorization", BEARER[0].1), ("range", "bytes=2-4")],
            "",
        )
        .await;
    assert_eq!(head.status(), StatusCode::OK);
    assert_eq!(head.headers()["content-length"], "6");
    assert!(head
        .into_body()
        .collect()
        .await
        .unwrap()
        .to_bytes()
        .is_empty());
}

#[tokio::test]
async fn empty_files_and_zero_suffix_have_correct_range_behavior() {
    let fixture = Fixture::new(true, &["files", "uploads"], 10);
    assert_eq!(
        fixture
            .request("PUT", "/api/v1/content?path=empty", &BEARER, "")
            .await
            .status(),
        201
    );
    let full = fixture
        .request("GET", "/api/v1/content?path=empty", &BEARER, "")
        .await;
    assert_eq!(full.status(), 200);
    assert_eq!(full.headers()["content-length"], "0");
    for range in ["bytes=0-", "bytes=-0", "bytes=-1"] {
        let response = fixture
            .request(
                "GET",
                "/api/v1/content?path=empty",
                &[("authorization", BEARER[0].1), ("range", range)],
                "",
            )
            .await;
        assert_eq!(response.status(), 416);
        assert_eq!(response.headers()["content-range"], "bytes */0");
    }
}

#[tokio::test]
async fn public_status_private_context_and_server_side_policy() {
    let fixture = Fixture::new(false, &["files", "uploads", "manage"], 10);
    let status = fixture.request("GET", "/api/v1/status", &[], "").await;
    assert_eq!(status.status(), 200);
    assert_eq!(json_body(status).await["auth_required"], true);
    let unauthorized = fixture.request("GET", "/api/v1/context", &[], "").await;
    assert_eq!(unauthorized.status(), 401);
    assert_eq!(
        json_body(unauthorized).await["error"]["code"],
        "unauthorized"
    );
    let context = json_body(fixture.request("GET", "/api/v1/context", &BEARER, "").await).await;
    assert_eq!(
        context["permissions"],
        json!({"list":true,"download":true,"upload":false,"manage":false})
    );
    assert_eq!(context["capabilities"]["directory_copy"], false);
    assert_eq!(context["capabilities"]["directory_move"], false);
    assert!(!context
        .to_string()
        .contains(fixture.root.path().to_str().unwrap()));
    assert_eq!(
        fixture
            .request("PUT", "/api/v1/content?path=forbidden", &BEARER, "x")
            .await
            .status(),
        403
    );
    assert_eq!(
        fixture
            .request(
                "POST",
                "/api/v1/directories",
                &[
                    ("authorization", BEARER[0].1),
                    ("content-type", "application/json")
                ],
                r#"{"path":"forbidden"}"#
            )
            .await
            .status(),
        403
    );
}

#[tokio::test]
async fn disabled_modules_have_no_routes_or_advertised_capabilities() {
    let fixture = Fixture::new(true, &["files"], 10);
    let context = json_body(fixture.request("GET", "/api/v1/context", &BEARER, "").await).await;
    assert_eq!(context["modules"], json!(["files"]));
    assert_eq!(context["permissions"]["upload"], false);
    assert_eq!(
        fixture
            .request("PUT", "/api/v1/content?path=disabled", &BEARER, "x")
            .await
            .status(),
        405
    );
    assert_eq!(
        fixture
            .request("POST", "/api/v1/directories", &BEARER, "{}")
            .await
            .status(),
        404
    );
    let empty = Fixture::new(false, &[], 10);
    assert_eq!(
        empty
            .request("GET", "/api/v1/entries?path=", &BEARER, "")
            .await
            .status(),
        404
    );
}

#[tokio::test]
async fn session_cookie_login_origin_checks_and_revocation() {
    let fixture = Fixture::new(true, &["files", "uploads", "manage"], 100);
    let json_headers = [
        ("content-type", "application/json"),
        ("origin", "http://localhost:5000"),
    ];
    let wrong = fixture
        .request(
            "POST",
            "/api/v1/session",
            &json_headers,
            r#"{"key":"wrong"}"#,
        )
        .await;
    assert_eq!(wrong.status(), 401);
    let cross_origin = fixture
        .request(
            "POST",
            "/api/v1/session",
            &[
                ("content-type", "application/json"),
                ("origin", "https://evil.test"),
            ],
            r#"{"key":"test-access-key"}"#,
        )
        .await;
    assert_eq!(cross_origin.status(), 403);
    let login = fixture
        .request(
            "POST",
            "/api/v1/session",
            &json_headers,
            r#"{"key":"test-access-key"}"#,
        )
        .await;
    assert_eq!(login.status(), 204);
    let cookie_header = login.headers()["set-cookie"].to_str().unwrap();
    assert!(cookie_header.contains("HttpOnly"));
    assert!(cookie_header.contains("SameSite=Strict"));
    let cookie = cookie_header.split(';').next().unwrap().to_owned();
    assert_eq!(
        fixture
            .request("GET", "/api/v1/context", &[("cookie", &cookie)], "")
            .await
            .status(),
        200
    );
    assert_eq!(
        fixture
            .request(
                "PUT",
                "/api/v1/content?path=csrf",
                &[("cookie", &cookie)],
                "x"
            )
            .await
            .status(),
        403
    );
    assert_eq!(
        fixture
            .request(
                "PUT",
                "/api/v1/content?path=csrf",
                &[("cookie", &cookie), ("origin", "https://evil.test")],
                "x"
            )
            .await
            .status(),
        403
    );
    assert_eq!(
        fixture
            .request(
                "PUT",
                "/api/v1/content?path=allowed",
                &[("cookie", &cookie), ("origin", "http://localhost:5000")],
                "x"
            )
            .await
            .status(),
        201
    );
    let logout = fixture
        .request(
            "DELETE",
            "/api/v1/session",
            &[("cookie", &cookie), ("origin", "http://localhost:5000")],
            "",
        )
        .await;
    assert_eq!(logout.status(), 204);
    assert!(logout.headers()["set-cookie"]
        .to_str()
        .unwrap()
        .contains("Max-Age=0"));
    assert_eq!(
        fixture
            .request("GET", "/api/v1/context", &[("cookie", &cookie)], "")
            .await
            .status(),
        401
    );
}

#[tokio::test]
async fn paths_decode_once_and_upload_limits_and_manage_operations_work() {
    let fixture = Fixture::new(true, &["files", "uploads", "manage"], 4);
    for path in [
        "../outside",
        "%2e%2e%2foutside",
        "%2Foutside",
        "C%3Aoutside",
        "a%5Cb",
    ] {
        let response = fixture
            .request("PUT", &format!("/api/v1/content?path={path}"), &BEARER, "x")
            .await;
        assert_eq!(response.status(), 400, "{path}");
        assert_eq!(json_body(response).await["error"]["code"], "invalid_path");
    }
    let too_large = fixture
        .request("PUT", "/api/v1/content?path=huge", &BEARER, "12345")
        .await;
    assert_eq!(too_large.status(), 413);
    assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 0);
    assert_eq!(
        fixture
            .request("PUT", "/api/v1/content?path=%252e%252e", &BEARER, "x")
            .await
            .status(),
        201
    );
    assert!(fixture.root.path().join("%2e%2e").exists());
    let headers = [
        ("authorization", BEARER[0].1),
        ("content-type", "application/json"),
    ];
    assert_eq!(
        fixture
            .request("POST", "/api/v1/directories", &headers, r#"{"path":"dir"}"#)
            .await
            .status(),
        201
    );
    let copy = fixture
        .request(
            "POST",
            "/api/v1/operations",
            &headers,
            r#"{"action":"copy","source":"%2e%2e","destination":"dir/copied"}"#,
        )
        .await;
    assert_eq!(copy.status(), 200);
    let moved = fixture
        .request(
            "POST",
            "/api/v1/operations",
            &headers,
            r#"{"action":"rename","source":"dir/copied","destination":"dir/renamed"}"#,
        )
        .await;
    assert_eq!(moved.status(), 200);
    assert_eq!(
        fixture
            .request("DELETE", "/api/v1/entries?path=dir", &BEARER, "")
            .await
            .status(),
        409
    );
    assert_eq!(
        fixture
            .request(
                "DELETE",
                "/api/v1/entries?path=dir&recursive=true",
                &BEARER,
                ""
            )
            .await
            .status(),
        204
    );
    assert_eq!(
        fixture
            .request(
                "DELETE",
                "/api/v1/entries?path=&recursive=true",
                &BEARER,
                ""
            )
            .await
            .status(),
        400
    );
}

#[tokio::test]
async fn writable_without_key_is_rejected_and_stop_releases_listener() {
    let root = tempfile::tempdir().unwrap();
    let mut invalid = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    invalid.writable = true;
    assert!(start(invalid).await.is_err());
    let server = start(ServerConfig::new(
        root.path().to_owned(),
        "127.0.0.1:0".parse().unwrap(),
    ))
    .await
    .unwrap();
    let address = server.address();
    assert!(tokio::net::TcpStream::connect(address).await.is_ok());
    tokio::time::timeout(Duration::from_secs(3), server.stop())
        .await
        .unwrap()
        .unwrap();
    let restarted = start(ServerConfig::new(root.path().to_owned(), address))
        .await
        .unwrap();
    drop(restarted);
    tokio::task::yield_now().await;
    tokio::time::timeout(Duration::from_secs(2), async {
        loop {
            if let Ok(listener) = tokio::net::TcpListener::bind(address).await {
                drop(listener);
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
}

#[tokio::test]
async fn injected_provider_controls_storage_and_product_without_opening_config_root() {
    let selected = tempfile::tempdir().unwrap();
    std::fs::write(selected.path().join("provider-file"), b"chosen").unwrap();
    let mut config = ServerConfig::new(
        selected.path().join("not-an-ambient-root"),
        "127.0.0.1:0".parse().unwrap(),
    );
    config.product.name = "Provider product".into();
    let store = Arc::new(FsStore::new(selected.path()).unwrap());
    let router = router_with_store(&config, store.clone()).unwrap();
    let context = router
        .clone()
        .oneshot(
            Request::builder()
                .uri("/api/v1/context")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    let context = json_body(context).await;
    assert_eq!(context["product"]["name"], "Provider product");
    let entries = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/entries?path=")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(
        json_body(entries).await["entries"][0]["name"],
        "provider-file"
    );
    let server = share_http::start_with_store(config, store).await.unwrap();
    assert!(tokio::net::TcpStream::connect(server.address())
        .await
        .is_ok());
    server.stop().await.unwrap();
}

#[tokio::test]
async fn unknown_length_oversize_failed_and_cancelled_bodies_leave_no_staging() {
    use bytes::Bytes;
    use futures_util::StreamExt;
    let fixture = Fixture::new(true, &["files", "uploads"], 4);
    let oversized = Body::from_stream(futures_util::stream::iter([Ok::<_, std::io::Error>(
        Bytes::from_static(b"12345"),
    )]));
    assert_eq!(
        fixture
            .request("PUT", "/api/v1/content?path=big", &BEARER, oversized)
            .await
            .status(),
        413
    );
    let failure = Body::from_stream(futures_util::stream::iter([
        Ok(Bytes::from_static(b"123")),
        Err(std::io::Error::other("private host path must not appear")),
    ]));
    let response = fixture
        .request("PUT", "/api/v1/content?path=failed", &BEARER, failure)
        .await;
    assert_eq!(response.status(), 500);
    let error = json_body(response).await;
    assert_eq!(error["error"]["code"], "io_error");
    assert!(!error.to_string().contains("private host path"));
    let pending = Body::from_stream(
        futures_util::stream::iter([Ok::<_, std::io::Error>(Bytes::from_static(b"123"))])
            .chain(futures_util::stream::pending()),
    );
    let request = Request::builder()
        .method("PUT")
        .uri("/api/v1/content?path=cancelled")
        .header("authorization", BEARER[0].1)
        .body(pending)
        .unwrap();
    assert!(tokio::time::timeout(
        Duration::from_millis(40),
        fixture.router.clone().oneshot(request)
    )
    .await
    .is_err());
    assert_eq!(std::fs::read_dir(fixture.root.path()).unwrap().count(), 0);
}

#[tokio::test]
async fn stopping_active_upload_cleans_stage_and_releases_port() {
    use tokio::io::AsyncWriteExt;
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.key = Some("test-access-key".into());
    config.writable = true;
    let server = start(config).await.unwrap();
    let address = server.address();
    let mut connection = tokio::net::TcpStream::connect(address).await.unwrap();
    connection.write_all(b"PUT /api/v1/content?path=incomplete HTTP/1.1\r\nHost: localhost\r\nAuthorization: Bearer test-access-key\r\nContent-Length: 1000\r\n\r\npartial").await.unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if std::fs::read_dir(root.path()).unwrap().count() > 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    server.request_stop();
    tokio::time::timeout(Duration::from_secs(3), server.stop())
        .await
        .unwrap()
        .unwrap();
    tokio::time::timeout(Duration::from_secs(1), async {
        loop {
            if std::fs::read_dir(root.path()).unwrap().count() == 0 {
                break;
            }
            tokio::time::sleep(Duration::from_millis(5)).await;
        }
    })
    .await
    .unwrap();
    let listener = tokio::net::TcpListener::bind(address).await.unwrap();
    drop(listener);
}

#[cfg(unix)]
#[tokio::test]
async fn api_rejects_symlink_paths_with_sanitized_errors() {
    let fixture = Fixture::new(true, &["files", "uploads", "manage"], 100);
    let outside = tempfile::tempdir().unwrap();
    std::fs::write(outside.path().join("secret"), b"secret").unwrap();
    std::os::unix::fs::symlink(outside.path(), fixture.root.path().join("link")).unwrap();
    for (method, uri) in [
        ("GET", "/api/v1/content?path=link/secret"),
        ("PUT", "/api/v1/content?path=link/new"),
        ("DELETE", "/api/v1/entries?path=link&recursive=true"),
    ] {
        let response = fixture.request(method, uri, &BEARER, "x").await;
        assert_eq!(response.status(), 403);
        let body = json_body(response).await;
        assert_eq!(body["error"]["code"], "forbidden");
        assert!(!body.to_string().contains(outside.path().to_str().unwrap()));
    }
    let listed = json_body(
        fixture
            .request("GET", "/api/v1/entries?path=", &BEARER, "")
            .await,
    )
    .await;
    assert_eq!(listed["entries"], json!([]));
    assert_eq!(
        std::fs::read(outside.path().join("secret")).unwrap(),
        b"secret"
    );
}

#[tokio::test]
async fn malformed_requests_use_error_envelopes_and_multiranges_are_ignored() {
    let fixture = Fixture::new(true, &["files", "uploads", "manage"], 100);
    assert_eq!(
        fixture
            .request("PUT", "/api/v1/content?path=a", &BEARER, "abcdef")
            .await
            .status(),
        201
    );
    for range in [
        "bytes=4-2",
        "bytes=garbage",
        "bytes=-0",
        "bytes=18446744073709551616-",
    ] {
        let response = fixture
            .request(
                "GET",
                "/api/v1/content?path=a",
                &[("authorization", BEARER[0].1), ("range", range)],
                "",
            )
            .await;
        assert_eq!(response.status(), 416);
        assert_eq!(json_body(response).await["error"]["code"], "invalid_range");
    }
    let full = fixture
        .request(
            "GET",
            "/api/v1/content?path=a",
            &[("authorization", BEARER[0].1), ("range", "bytes=0-1,4-5")],
            "",
        )
        .await;
    assert_eq!(full.status(), 200);
    assert_eq!(
        full.into_body().collect().await.unwrap().to_bytes(),
        "abcdef"
    );
    let missing = fixture.request("GET", "/api/v1/entries", &BEARER, "").await;
    assert_eq!(missing.status(), 400);
    assert_eq!(json_body(missing).await["error"]["code"], "invalid_path");
    let invalid_json = fixture
        .request(
            "POST",
            "/api/v1/directories",
            &[
                ("authorization", BEARER[0].1),
                ("content-type", "application/json"),
            ],
            "bad-json",
        )
        .await;
    assert_eq!(invalid_json.status(), 400);
    assert!(json_body(invalid_json).await["error"]["code"].is_string());
}
