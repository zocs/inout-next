use share_desktop_lib::{resolve_web_dir, HostController, StartOptions};
use std::path::Path;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

fn create_assets(path: &Path) {
    std::fs::create_dir_all(path.join("_next")).unwrap();
    std::fs::write(
        path.join("index.html"),
        "<!doctype html><title>Host fixture</title>",
    )
    .unwrap();
}

#[test]
fn asset_lookup_prefers_packaged_mapping_and_rejects_partial_folders() {
    let temp = tempfile::tempdir().unwrap();
    let resources = temp.path().join("resources");
    let fallback = temp.path().join("debug-web");
    create_assets(&fallback);
    if cfg!(debug_assertions) {
        assert_eq!(
            resolve_web_dir(&resources, Some(&fallback)).unwrap(),
            fallback
        );
    } else {
        assert!(resolve_web_dir(&resources, Some(&fallback)).is_err());
    }
    create_assets(&resources.join("web"));
    assert_eq!(
        resolve_web_dir(&resources, Some(&fallback)).unwrap(),
        resources.join("web")
    );
    std::fs::remove_file(resources.join("web/index.html")).unwrap();
    assert!(resolve_web_dir(&resources, None).is_err());
    create_assets(&resources.join("web"));
    std::fs::remove_dir(resources.join("web/_next")).unwrap();
    assert!(resolve_web_dir(&resources, None).is_err());
    std::fs::write(resources.join("web/_next"), "not an asset directory").unwrap();
    assert!(resolve_web_dir(&resources, None).is_err());
}

#[tokio::test]
async fn native_controller_uses_the_real_service_and_stops_it() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    create_assets(&web);
    let host = HostController::new(web);
    host.select_root(root).await.unwrap();
    let status = host.start(StartOptions::default()).await.unwrap();
    assert!(status.running);
    let url = status.manager_url.unwrap();
    assert!(url.starts_with("http://127.0.0.1:"));
    let address = url.strip_prefix("http://").unwrap();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream
        .write_all(
            format!("GET /api/v1/status HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    assert!(response.starts_with("HTTP/1.1 200"));
    assert!(response.contains("\"auth_required\":false"));
    assert!(host.start(StartOptions::default()).await.is_err());
    let stopped = host.stop().await.unwrap();
    assert!(!stopped.running);
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
}

#[tokio::test]
async fn native_controller_requires_a_selected_root_and_a_key_for_writes() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    create_assets(&web);
    let host = HostController::new(web);
    assert!(host.start(StartOptions::default()).await.is_err());
    host.select_root(root).await.unwrap();
    assert!(host
        .start(StartOptions {
            uploads: true,
            ..Default::default()
        })
        .await
        .is_err());
    assert!(host
        .start(StartOptions {
            manage: true,
            ..Default::default()
        })
        .await
        .is_err());
}

async fn request(url: &str, method: &str, path: &str, extra: &str, body: &str) -> String {
    let address = url.strip_prefix("http://").unwrap();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream.write_all(format!("{method} {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\nContent-Length: {}\r\n{extra}\r\n{body}", body.len()).as_bytes()).await.unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

#[tokio::test]
async fn root_selection_is_checked_locked_while_running_and_retained_for_restart() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let other_root = temp.path().join("other-files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir(&other_root).unwrap();
    create_assets(&web);
    let host = HostController::new(web);
    assert!(host.status().await.root.is_none());
    assert!(host.select_root(temp.path().join("missing")).await.is_err());
    let selected = host.select_root(root.clone()).await.unwrap();
    assert_eq!(selected.root.as_deref(), root.to_str());
    assert!(!selected.running);
    let started = host.start(StartOptions::default()).await.unwrap();
    assert!(started.addresses.is_empty());
    assert!(host.select_root(other_root).await.is_err());
    assert_eq!(host.status().await.root.as_deref(), root.to_str());
    let url = started.manager_url.unwrap();
    let context = request(&url, "GET", "/api/v1/context", "", "").await;
    assert!(context.contains("\"modules\":[\"files\"]"));
    assert!(!context.contains(root.to_str().unwrap()));
    let stopped = host.stop().await.unwrap();
    assert!(stopped.manager_url.is_none());
    assert!(stopped.addresses.is_empty());
    assert_eq!(stopped.root.as_deref(), root.to_str());
    assert!(!host.stop().await.unwrap().running);
    let port = url.rsplit(':').next().unwrap().parse().unwrap();
    assert_eq!(
        host.start(StartOptions {
            port,
            ..Default::default()
        })
        .await
        .unwrap()
        .manager_url
        .as_deref(),
        Some(url.as_str())
    );
    host.stop().await.unwrap();
}

#[tokio::test]
async fn keyed_upload_and_manage_modes_enforce_auth_and_enable_only_requested_modules() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    create_assets(&web);
    let host = HostController::new(web);
    host.select_root(root.clone()).await.unwrap();
    assert!(host
        .start(StartOptions {
            uploads: true,
            key: Some("  ".into()),
            ..Default::default()
        })
        .await
        .is_err());
    let key = "native-test-access-key";
    let status = host
        .start(StartOptions {
            uploads: true,
            key: Some(key.into()),
            ..Default::default()
        })
        .await
        .unwrap();
    assert!(!serde_json::to_string(&status).unwrap().contains(key));
    let url = status.manager_url.unwrap();
    let unauthorized = request(&url, "PUT", "/api/v1/content?path=test.txt", "", "hello").await;
    assert!(unauthorized.starts_with("HTTP/1.1 401"));
    assert!(!root.join("test.txt").exists());
    let auth = format!("Authorization: Bearer {key}\r\n");
    assert!(
        request(&url, "PUT", "/api/v1/content?path=test.txt", &auth, "hello")
            .await
            .starts_with("HTTP/1.1 201")
    );
    assert_eq!(std::fs::read(root.join("test.txt")).unwrap(), b"hello");
    assert!(request(
        &url,
        "POST",
        "/api/v1/directories",
        &format!("{auth}Content-Type: application/json\r\n"),
        "{\"path\":\"blocked\"}"
    )
    .await
    .starts_with("HTTP/1.1 404"));
    host.stop().await.unwrap();
    let status = host
        .start(StartOptions {
            manage: true,
            key: Some(key.into()),
            ..Default::default()
        })
        .await
        .unwrap();
    let url = status.manager_url.unwrap();
    assert!(request(
        &url,
        "POST",
        "/api/v1/directories",
        &format!("{auth}Content-Type: application/json\r\n"),
        "{\"path\":\"created\"}"
    )
    .await
    .starts_with("HTTP/1.1 201"));
    assert!(root.join("created").is_dir());
    assert!(request(
        &url,
        "PUT",
        "/api/v1/content?path=blocked.txt",
        &auth,
        "hello"
    )
    .await
    .starts_with("HTTP/1.1 405"));
    assert!(!root.join("blocked.txt").exists());
    host.stop().await.unwrap();
}
