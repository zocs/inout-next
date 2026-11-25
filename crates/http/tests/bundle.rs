use axum::body::Body;
use http::Request;
use http_body_util::BodyExt;
use share_domain::Error;
use share_http::{router_with_store, start, ServerConfig, ServerError, WebBundle};
use share_storage::FsStore;
use std::sync::Arc;
use tower::ServiceExt;

static PAGES: &[(&str, &[u8])] = &[
    ("index.html", b"trusted page"),
    ("_next/static/app.js", b"trusted script"),
];

#[tokio::test]
async fn an_embedded_bundle_serves_pages_for_a_share_that_contains_the_application() {
    // The whole temporary tree is shared, including a stale on-disk resource
    // folder beside the "application". No directory isolation proof is needed
    // because nothing on disk is ever served as a trusted page.
    let everything = tempfile::tempdir().unwrap();
    let stale = everything.path().join("app/web");
    std::fs::create_dir_all(stale.join("_next")).unwrap();
    std::fs::write(stale.join("index.html"), b"<script>planted()</script>").unwrap();
    let mut config = ServerConfig::new(
        everything.path().join("must-not-be-opened"),
        "127.0.0.1:0".parse().unwrap(),
    );
    config.web_bundle = Some(WebBundle::from_static(PAGES).unwrap());
    // A bundle wins over a configured directory, which is then never read.
    config.web_dir = Some(stale);
    let store = Arc::new(FsStore::new(everything.path()).unwrap());
    let router = router_with_store(&config, store).unwrap();
    for (path, expected) in [
        ("/", "trusted page"),
        ("/_next/static/app.js", "trusted script"),
    ] {
        let response = router
            .clone()
            .oneshot(Request::builder().uri(path).body(Body::empty()).unwrap())
            .await
            .unwrap();
        assert_eq!(response.status(), 200, "{path}");
        assert_eq!(
            response.into_body().collect().await.unwrap().to_bytes(),
            expected
        );
    }
    let listing = router
        .oneshot(
            Request::builder()
                .uri("/api/v1/entries?path=app/web")
                .body(Body::empty())
                .unwrap(),
        )
        .await
        .unwrap();
    assert_eq!(listing.status(), 200, "shared files stay ordinary files");
}

#[test]
fn bundles_reject_unsafe_incomplete_or_ambiguous_tables() {
    static TRAVERSAL: &[(&str, &[u8])] = &[("index.html", b""), ("../escape.js", b"")];
    static NO_INDEX: &[(&str, &[u8])] = &[("_next/app.js", b"")];
    static DUPLICATE: &[(&str, &[u8])] = &[("index.html", b"a"), ("index.html", b"b")];
    assert!(matches!(
        WebBundle::from_static(TRAVERSAL),
        Err(Error::InvalidPath)
    ));
    assert!(matches!(
        WebBundle::from_static(NO_INDEX),
        Err(Error::NotFound)
    ));
    assert!(matches!(
        WebBundle::from_static(DUPLICATE),
        Err(Error::Conflict)
    ));
    let bundle = WebBundle::from_static(PAGES).unwrap();
    assert_eq!(bundle.files(), 2);
    assert_eq!(bundle.bytes(), 26);
}

#[tokio::test]
async fn start_failures_name_the_folder_or_the_resources_instead_of_one_generic_error() {
    let parent = tempfile::tempdir().unwrap();
    let share = parent.path().join("share");
    std::fs::create_dir(&share).unwrap();
    let missing = ServerConfig::new(parent.path().join("absent"), "127.0.0.1:0".parse().unwrap());
    assert!(matches!(
        start(missing).await,
        Err(ServerError::Root(Error::NotFound))
    ));
    let web = share.join("web");
    std::fs::create_dir(&web).unwrap();
    std::fs::write(web.join("index.html"), b"page").unwrap();
    let mut overlapping = ServerConfig::new(share, "127.0.0.1:0".parse().unwrap());
    overlapping.web_dir = Some(web);
    assert!(matches!(
        start(overlapping).await,
        Err(ServerError::Assets(Error::Forbidden))
    ));
}
