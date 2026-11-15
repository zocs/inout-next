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
    let root = root.canonicalize().unwrap();
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

#[tokio::test]
async fn active_settings_report_the_real_bound_service_without_returning_a_key() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    create_assets(&web);
    let host = HostController::new(web);
    host.select_root(root.clone()).await.unwrap();
    let root = root.canonicalize().unwrap();
    let key = "active-settings-native-secret";
    let status = host
        .start(StartOptions {
            network: true,
            port: 0,
            uploads: true,
            manage: true,
            key: Some(key.into()),
        })
        .await
        .unwrap();
    let serialized = serde_json::to_value(&status).unwrap();
    let url = status.manager_url.as_deref().unwrap();
    let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
    assert_ne!(port, 0);
    assert_eq!(
        serialized["activeSettings"],
        serde_json::json!({"network":true,"port":port,"uploads":true,"manage":true,"authRequired":true})
    );
    assert_eq!(
        serialized["capabilities"],
        serde_json::json!({
            "storageKind":"folder",
            "chooseFolder":true,
            "openManager":true,
            "foregroundOnly":false
        })
    );
    assert_eq!(serialized.as_object().unwrap().len(), 6);
    assert!(!serialized.to_string().contains(key));
    assert!(!serialized.to_string().contains("\"key\""));
    assert_eq!(
        serde_json::to_value(host.status().await).unwrap()["activeSettings"],
        serialized["activeSettings"]
    );
    assert!(status
        .addresses
        .iter()
        .all(|address| address.ends_with(&format!(":{port}"))));
    assert!(request(url, "GET", "/api/v1/status", "", "")
        .await
        .contains("\"auth_required\":true"));
    assert!(request(url, "GET", "/api/v1/context", "", "")
        .await
        .starts_with("HTTP/1.1 401"));
    let auth = format!("Authorization: Bearer {key}\r\n");
    let context = request(url, "GET", "/api/v1/context", &auth, "").await;
    assert!(context.starts_with("HTTP/1.1 200"));
    assert!(context.contains("\"upload\":true"));
    assert!(context.contains("\"manage\":true"));
    let stopped = serde_json::to_value(host.stop().await.unwrap()).unwrap();
    assert_eq!(stopped["activeSettings"], serde_json::Value::Null);
    assert_eq!(stopped["running"], false);
    assert_eq!(stopped["root"], root.to_str().unwrap());
    assert_eq!(stopped["managerUrl"], serde_json::Value::Null);
    assert_eq!(stopped["addresses"], serde_json::json!([]));
    assert!(
        tokio::net::TcpStream::connect(url.strip_prefix("http://").unwrap())
            .await
            .is_err()
    );
}

#[tokio::test]
async fn active_settings_stay_null_after_failed_starts_and_match_readonly_restarts() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    create_assets(&web);
    let host = HostController::new(web);
    let initial = serde_json::to_value(host.status().await).unwrap();
    assert_eq!(
        initial.get("activeSettings"),
        Some(&serde_json::Value::Null)
    );
    assert!(host.start(StartOptions::default()).await.is_err());
    host.select_root(root).await.unwrap();
    assert!(host
        .start(StartOptions {
            uploads: true,
            ..Default::default()
        })
        .await
        .is_err());
    assert_eq!(
        serde_json::to_value(host.status().await).unwrap()["activeSettings"],
        serde_json::Value::Null
    );
    let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    assert!(host
        .start(StartOptions {
            port: occupied.local_addr().unwrap().port(),
            ..Default::default()
        })
        .await
        .is_err());
    assert_eq!(
        serde_json::to_value(host.status().await).unwrap()["activeSettings"],
        serde_json::Value::Null
    );
    for key in [None, Some("readonly-native-secret")] {
        let status = host
            .start(StartOptions {
                key: key.map(str::to_owned),
                ..Default::default()
            })
            .await
            .unwrap();
        let url = status.manager_url.as_deref().unwrap();
        let port: u16 = url.rsplit(':').next().unwrap().parse().unwrap();
        let serialized = serde_json::to_value(&status).unwrap();
        assert_eq!(
            serialized["activeSettings"],
            serde_json::json!({"network":false,"port":port,"uploads":false,"manage":false,"authRequired":key.is_some()})
        );
        assert_eq!(serialized["addresses"], serde_json::json!([]));
        assert!(!serialized.to_string().contains("readonly-native-secret"));
        let auth = key
            .map(|key| format!("Authorization: Bearer {key}\r\n"))
            .unwrap_or_default();
        let context = request(url, "GET", "/api/v1/context", &auth, "").await;
        assert!(context.starts_with("HTTP/1.1 200"));
        assert!(context.contains("\"upload\":false"));
        assert!(context.contains("\"manage\":false"));
        assert_eq!(
            serde_json::to_value(host.stop().await.unwrap()).unwrap()["activeSettings"],
            serde_json::Value::Null
        );
        assert_eq!(
            serde_json::to_value(host.stop().await.unwrap()).unwrap()["activeSettings"],
            serde_json::Value::Null
        );
    }
}

#[tokio::test]
async fn private_host_preselects_only_its_owned_folder_and_preserves_uploaded_files() {
    let temporary = tempfile::tempdir().unwrap();
    let root = temporary.path().join("files/shared");
    let web = temporary.path().join("files/runtime-assets/web");
    let other = temporary.path().join("other-files");
    std::fs::create_dir_all(&root).unwrap();
    std::fs::create_dir(&other).unwrap();
    create_assets(&web);
    std::fs::write(
        temporary.path().join("private-settings.json"),
        "private sibling",
    )
    .unwrap();
    let host = HostController::new_app_private(web, root.clone()).unwrap();
    let initial = serde_json::to_value(host.status().await).unwrap();
    assert_eq!(
        initial["root"],
        root.canonicalize().unwrap().to_str().unwrap()
    );
    assert_eq!(
        initial["capabilities"],
        serde_json::json!({
            "storageKind":"app-private", "chooseFolder":false, "openManager":false, "foregroundOnly":true
        })
    );
    assert!(host.select_root(other).await.is_err());
    let key = "owned-storage-test-key";
    let options = || StartOptions {
        uploads: true,
        manage: true,
        key: Some(key.into()),
        ..Default::default()
    };
    let started = host.start(options()).await.unwrap();
    let url = started.manager_url.as_deref().unwrap();
    assert!(request(
        url,
        "PUT",
        "/api/v1/content?path=owned.txt",
        "",
        "owned upload"
    )
    .await
    .starts_with("HTTP/1.1 401"));
    let authorization = format!("Authorization: Bearer {key}\r\n");
    assert!(request(
        url,
        "PUT",
        "/api/v1/content?path=owned.txt",
        &authorization,
        "owned upload"
    )
    .await
    .starts_with("HTTP/1.1 201"));
    assert_eq!(
        std::fs::read(root.join("owned.txt")).unwrap(),
        b"owned upload"
    );
    let download = request(
        url,
        "GET",
        "/api/v1/content?path=owned.txt",
        &authorization,
        "",
    )
    .await;
    assert!(download.starts_with("HTTP/1.1 200") && download.ends_with("owned upload"));
    let private = request(
        url,
        "GET",
        "/api/v1/content?path=..%2F..%2Fprivate-settings.json",
        &authorization,
        "",
    )
    .await;
    assert!(!private.starts_with("HTTP/1.1 200") && !private.contains("private sibling"));
    assert!(request(url, "GET", "/", "", "")
        .await
        .contains("Host fixture"));
    let address = url.strip_prefix("http://").unwrap().to_owned();
    host.stop().await.unwrap();
    assert!(tokio::net::TcpStream::connect(address).await.is_err());
    assert_eq!(
        std::fs::read(root.join("owned.txt")).unwrap(),
        b"owned upload"
    );
    let restarted = host.start(options()).await.unwrap();
    assert!(request(
        restarted.manager_url.as_deref().unwrap(),
        "GET",
        "/api/v1/content?path=owned.txt",
        &authorization,
        ""
    )
    .await
    .ends_with("owned upload"));
    assert_eq!(
        std::fs::read_to_string(temporary.path().join("private-settings.json")).unwrap(),
        "private sibling"
    );
    host.stop().await.unwrap();
}

#[test]
fn private_host_rejects_a_shared_ancestor_of_its_resources() {
    let temporary = tempfile::tempdir().unwrap();
    let web = temporary.path().join("runtime-assets/web");
    create_assets(&web);
    assert!(HostController::new_app_private(web.clone(), temporary.path().to_path_buf()).is_err());
    assert!(HostController::new_app_private(web.clone(), web.clone()).is_err());
    assert!(HostController::new_app_private(web, temporary.path().join("missing")).is_err());
}
