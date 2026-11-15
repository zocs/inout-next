use axum::{body::Body, Router};
use http::{Request, StatusCode};
use http_body_util::BodyExt;
use share_domain::{Error, Permissions};
use share_http::{
    router_with_store, router_with_store_and_policy, KeyUpdate, PolicyUpdate, ServerConfig,
    TransferKind, TransferState,
};
use share_storage::FsStore;
use std::sync::Arc;
use tower::ServiceExt;

async fn request(
    router: &Router,
    method: &str,
    uri: &str,
    headers: &[(&str, &str)],
    body: impl Into<Body>,
) -> http::Response<Body> {
    let mut builder = Request::builder()
        .method(method)
        .uri(uri)
        .header("host", "localhost:5000");
    for (name, value) in headers {
        builder = builder.header(*name, *value);
    }
    router
        .clone()
        .oneshot(builder.body(body.into()).unwrap())
        .await
        .unwrap()
}

#[tokio::test]
async fn explicitly_enabled_public_writes_work_but_published_delete_is_denied() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.writable = true;
    let result = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap()));
    assert!(
        result.is_ok(),
        "public writable sharing must be configurable"
    );
    let router = result.unwrap();
    let upload = request(
        &router,
        "PUT",
        "/api/v1/content?path=public.bin",
        &[],
        "abc",
    )
    .await;
    assert_eq!(upload.status(), StatusCode::CREATED);
    let mkdir = request(
        &router,
        "POST",
        "/api/v1/directories",
        &[("content-type", "application/json")],
        r#"{"path":"new-folder"}"#,
    )
    .await;
    assert_eq!(mkdir.status(), StatusCode::CREATED);
    let deletion = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=public.bin",
        &[],
        "",
    )
    .await;
    assert_eq!(deletion.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        std::fs::read(root.path().join("public.bin")).unwrap(),
        b"abc"
    );
    assert!(root.path().join("new-folder").is_dir());
    let context = request(&router, "GET", "/api/v1/context", &[], "").await;
    let context: serde_json::Value =
        serde_json::from_slice(&context.into_body().collect().await.unwrap().to_bytes()).unwrap();
    assert_eq!(context["permissions"]["upload"], true);
    assert_eq!(context["permissions"]["manage"], true);
    assert_eq!(context["permissions"]["delete"], false);
}

#[tokio::test]
async fn public_mutations_reject_a_foreign_browser_origin() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.writable = true;
    let router = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap())).unwrap();
    let response = request(
        &router,
        "PUT",
        "/api/v1/content?path=foreign.bin",
        &[("origin", "http://other-device.invalid")],
        "foreign",
    )
    .await;
    assert_eq!(response.status(), StatusCode::FORBIDDEN);
    assert!(!root.path().join("foreign.bin").exists());
    let response = request(
        &router,
        "PUT",
        "/api/v1/content?path=same-origin.bin",
        &[("origin", "http://localhost:5000")],
        "allowed",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn a_started_upload_finishes_after_permissions_and_the_key_change() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.writable = true;
    config.key = Some("original-key".into());
    let (router, policy) =
        router_with_store_and_policy(&config, Arc::new(FsStore::new(root.path()).unwrap()))
            .unwrap();
    let (sender, receiver) = tokio::sync::mpsc::channel::<Result<bytes::Bytes, std::io::Error>>(2);
    let stream = futures_util::stream::unfold(receiver, |mut receiver| async {
        receiver.recv().await.map(|item| (item, receiver))
    });
    sender
        .send(Ok(bytes::Bytes::from_static(b"abc")))
        .await
        .unwrap();
    let upload_router = router.clone();
    let upload = tokio::spawn(async move {
        request(
            &upload_router,
            "PUT",
            "/api/v1/content?path=started.bin",
            &[("authorization", "Bearer original-key")],
            Body::from_stream(stream),
        )
        .await
    });
    tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            let received = std::fs::read_dir(root.path().join("INOUTTMP"))
                .ok()
                .is_some_and(|mut entries| {
                    entries.any(|item| {
                        item.ok()
                            .and_then(|item| item.metadata().ok())
                            .is_some_and(|meta| meta.len() == 3)
                    })
                });
            if received {
                break;
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("the upload must be admitted and receive its first chunk");
    let initial = policy.status();
    let next = policy
        .update(PolicyUpdate {
            expected_revision: initial.revision,
            permissions: Permissions {
                list: true,
                download: true,
                ..Permissions::default()
            },
            key: KeyUpdate::Replace("replacement-key".into()),
        })
        .unwrap();
    assert!(next.revision > initial.revision);
    for (key, expected) in [
        ("Bearer original-key", StatusCode::UNAUTHORIZED),
        ("Bearer replacement-key", StatusCode::FORBIDDEN),
    ] {
        let response = request(
            &router,
            "PUT",
            "/api/v1/content?path=new.bin",
            &[("authorization", key)],
            "new",
        )
        .await;
        assert_eq!(response.status(), expected);
    }
    sender
        .send(Ok(bytes::Bytes::from_static(b"def")))
        .await
        .unwrap();
    drop(sender);
    let response = upload.await.unwrap();
    assert_eq!(response.status(), StatusCode::CREATED);
    assert_eq!(
        response.headers()["x-policy-revision"],
        initial.revision.to_string()
    );
    assert_eq!(
        std::fs::read(root.path().join("started.bin")).unwrap(),
        b"abcdef"
    );
    assert!(!root.path().join("new.bin").exists());
}

#[tokio::test]
async fn invalid_or_stale_updates_preserve_the_entire_current_policy() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.writable = true;
    config.key = Some("kept-key".into());
    let (router, policy) =
        router_with_store_and_policy(&config, Arc::new(FsStore::new(root.path()).unwrap()))
            .unwrap();
    let before = policy.status();
    for key in [KeyUpdate::Remove, KeyUpdate::Replace("  ".into())] {
        assert!(policy
            .update(PolicyUpdate {
                expected_revision: before.revision,
                permissions: before.permissions.clone(),
                key
            })
            .is_err());
        assert_eq!(policy.status(), before);
    }
    assert!(matches!(
        policy.update(PolicyUpdate {
            expected_revision: before.revision + 1,
            permissions: Permissions::default(),
            key: KeyUpdate::Replace("wrong-generation".into()),
        }),
        Err(Error::Conflict)
    ));
    assert_eq!(policy.status(), before);
    let response = request(
        &router,
        "PUT",
        "/api/v1/content?path=kept.bin",
        &[("authorization", "Bearer kept-key")],
        "kept",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
}

#[tokio::test]
async fn runtime_permission_updates_cannot_enable_unmounted_modules() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.modules = vec!["files".into()];
    let (router, policy) =
        router_with_store_and_policy(&config, Arc::new(FsStore::new(root.path()).unwrap()))
            .unwrap();
    let before = policy.status();
    assert!(matches!(
        policy.update(PolicyUpdate {
            expected_revision: before.revision,
            permissions: Permissions {
                upload: true,
                ..before.permissions.clone()
            },
            key: KeyUpdate::Keep,
        }),
        Err(Error::Unsupported)
    ));
    assert_eq!(policy.status(), before);
    let response = request(
        &router,
        "PUT",
        "/api/v1/content?path=disabled.bin",
        &[],
        "disabled",
    )
    .await;
    assert!(matches!(
        response.status(),
        StatusCode::NOT_FOUND | StatusCode::METHOD_NOT_ALLOWED
    ));
    assert!(!root.path().join("disabled.bin").exists());
}

#[tokio::test]
async fn rename_is_independent_and_cannot_be_used_to_bypass_a_denied_move() {
    let root = tempfile::tempdir().unwrap();
    std::fs::create_dir(root.path().join("first")).unwrap();
    std::fs::create_dir(root.path().join("second")).unwrap();
    std::fs::write(root.path().join("first/original.txt"), b"retained").unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.permissions = Some(Permissions {
        list: true,
        download: true,
        rename: true,
        ..Permissions::default()
    });
    let router = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap())).unwrap();
    let response = request(
        &router,
        "POST",
        "/api/v1/operations",
        &[("content-type", "application/json")],
        r#"{"action":"rename","source":"first/original.txt","destination":"first/renamed.txt"}"#,
    )
    .await;
    #[cfg(unix)]
    assert_eq!(response.status(), StatusCode::OK);
    #[cfg(not(unix))]
    assert_eq!(response.status(), StatusCode::NOT_IMPLEMENTED);
    #[cfg(unix)]
    for action in ["rename", "move", "copy"] {
        let body = serde_json::json!({"action":action,"source":"first/renamed.txt","destination":"second/bypass.txt"}).to_string();
        let response = request(
            &router,
            "POST",
            "/api/v1/operations",
            &[("content-type", "application/json")],
            body,
        )
        .await;
        assert_eq!(response.status(), StatusCode::FORBIDDEN, "{action}");
    }
    #[cfg(unix)]
    assert_eq!(
        std::fs::read(root.path().join("first/renamed.txt")).unwrap(),
        b"retained"
    );
    assert!(!root.path().join("second/bypass.txt").exists());
}

#[tokio::test]
async fn a_download_body_retains_its_admission_after_headers_and_live_updates() {
    let root = tempfile::tempdir().unwrap();
    let bytes: Vec<u8> = (0..384 * 1024 + 17)
        .map(|index| (index % 251) as u8)
        .collect();
    std::fs::write(root.path().join("download.bin"), &bytes).unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.key = Some("download-old-key".into());
    let transfers = config.transfer_control.clone();
    let (router, policy) =
        router_with_store_and_policy(&config, Arc::new(FsStore::new(root.path()).unwrap()))
            .unwrap();
    let response = request(
        &router,
        "GET",
        "/api/v1/content?path=download.bin",
        &[("authorization", "Bearer download-old-key")],
        "",
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    let started = transfers.snapshot();
    assert_eq!(
        started.active, 1,
        "response headers must not finish a download"
    );
    assert_eq!(started.records[0].kind, TransferKind::Download);
    assert_eq!(started.records[0].state, TransferState::Active);
    assert_eq!(started.records[0].bytes, 0);
    let initial = policy.status();
    policy
        .update(PolicyUpdate {
            expected_revision: initial.revision,
            permissions: Permissions::default(),
            key: KeyUpdate::Replace("download-new-key".into()),
        })
        .unwrap();
    let denied = request(
        &router,
        "GET",
        "/api/v1/content?path=download.bin",
        &[("authorization", "Bearer download-new-key")],
        "",
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    let downloaded = response.into_body().collect().await.unwrap().to_bytes();
    assert_eq!(&downloaded[..], bytes);
    let finished = transfers.snapshot();
    assert_eq!(finished.active, 0);
    assert_eq!(finished.records[0].state, TransferState::Served);
    assert_eq!(finished.records[0].bytes, bytes.len() as u64);
    assert_eq!(finished.records[0].policy_revision, initial.revision);
}

#[tokio::test]
async fn head_does_not_start_a_transfer_and_a_dropped_body_is_interrupted() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("partial.bin"), vec![7_u8; 512 * 1024]).unwrap();
    let config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    let transfers = config.transfer_control.clone();
    let router = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap())).unwrap();
    let head = request(&router, "HEAD", "/api/v1/content?path=partial.bin", &[], "").await;
    assert_eq!(head.status(), StatusCode::OK);
    assert!(transfers.snapshot().records.is_empty());
    let response = request(&router, "GET", "/api/v1/content?path=partial.bin", &[], "").await;
    let mut body = response.into_body();
    let frame = body.frame().await.unwrap().unwrap().into_data().unwrap();
    assert!(!frame.is_empty());
    assert!(frame.len() < 512 * 1024);
    assert_eq!(transfers.snapshot().active, 1);
    drop(body);
    let stopped = transfers.snapshot();
    assert_eq!(stopped.active, 0);
    assert_eq!(stopped.records[0].state, TransferState::Interrupted);
    assert_eq!(stopped.records[0].bytes, frame.len() as u64);
}

#[tokio::test]
async fn publication_is_observed_only_after_storage_success() {
    let root = tempfile::tempdir().unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.writable = true;
    let transfers = config.transfer_control.clone();
    let router = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap())).unwrap();
    let response = request(
        &router,
        "PUT",
        "/api/v1/content?path=published.bin",
        &[],
        "published",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CREATED);
    let first = transfers.snapshot();
    assert_eq!(first.active, 0);
    assert_eq!(first.records[0].state, TransferState::Published);
    assert_eq!(first.records[0].bytes, 9);
    let response = request(
        &router,
        "PUT",
        "/api/v1/content?path=published.bin",
        &[],
        "collision",
    )
    .await;
    assert_eq!(response.status(), StatusCode::CONFLICT);
    let second = transfers.snapshot();
    assert_eq!(second.active, 0);
    assert_eq!(second.records[1].state, TransferState::Failed);
    assert_eq!(
        std::fs::read(root.path().join("published.bin")).unwrap(),
        b"published"
    );
}

#[tokio::test]
async fn a_copy_records_its_published_size_without_claiming_pending_progress() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("source.bin"), b"copy-source").unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.writable = true;
    let transfers = config.transfer_control.clone();
    let router = router_with_store(&config, Arc::new(FsStore::new(root.path()).unwrap())).unwrap();
    let response = request(
        &router,
        "POST",
        "/api/v1/operations",
        &[("content-type", "application/json")],
        r#"{"action":"copy","source":"source.bin","destination":"copied.bin"}"#,
    )
    .await;
    assert_eq!(response.status(), StatusCode::OK);
    assert_eq!(
        std::fs::read(root.path().join("copied.bin")).unwrap(),
        b"copy-source"
    );
    assert_eq!(
        std::fs::read(root.path().join("source.bin")).unwrap(),
        b"copy-source"
    );
    let snapshot = transfers.snapshot();
    assert_eq!(snapshot.active, 0);
    assert_eq!(snapshot.records.len(), 1);
    assert_eq!(snapshot.records[0].kind, TransferKind::Copy);
    assert_eq!(snapshot.records[0].state, TransferState::Published);
    assert_eq!(snapshot.records[0].path, "copied.bin");
    assert_eq!(snapshot.records[0].total, Some(11));
    assert_eq!(snapshot.records[0].bytes, 11);
}

#[tokio::test]
async fn sessions_survive_permission_edits_and_deletion_uses_the_current_key() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("retained.bin"), b"retained").unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.key = Some("session-old-key".into());
    let (router, policy) =
        router_with_store_and_policy(&config, Arc::new(FsStore::new(root.path()).unwrap()))
            .unwrap();
    let login = request(
        &router,
        "POST",
        "/api/v1/session",
        &[("content-type", "application/json")],
        r#"{"key":"session-old-key"}"#,
    )
    .await;
    assert_eq!(login.status(), StatusCode::NO_CONTENT);
    let cookie = login.headers()[http::header::SET_COOKIE]
        .to_str()
        .unwrap()
        .split(';')
        .next()
        .unwrap()
        .to_owned();
    let before = policy.status();
    let changed = policy
        .update(PolicyUpdate {
            expected_revision: before.revision,
            permissions: Permissions {
                delete: true,
                ..before.permissions
            },
            key: KeyUpdate::Keep,
        })
        .unwrap();
    let browse = request(
        &router,
        "GET",
        "/api/v1/entries?path=",
        &[("cookie", &cookie)],
        "",
    )
    .await;
    assert_eq!(
        browse.status(),
        StatusCode::OK,
        "permission-only updates retain admitted sessions"
    );
    let unauthenticated = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[],
        "",
    )
    .await;
    assert_eq!(unauthenticated.status(), StatusCode::UNAUTHORIZED);
    let foreign = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[("cookie", &cookie), ("origin", "http://foreign.invalid")],
        "",
    )
    .await;
    assert_eq!(foreign.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        std::fs::read(root.path().join("retained.bin")).unwrap(),
        b"retained"
    );
    assert!(matches!(
        policy.update(PolicyUpdate {
            expected_revision: changed.revision,
            permissions: changed.permissions.clone(),
            key: KeyUpdate::Remove,
        }),
        Err(Error::Forbidden)
    ));
    assert_eq!(
        policy.status(),
        changed,
        "invalid key removal must preserve flags and sessions atomically"
    );
    let deleted = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[("cookie", &cookie), ("origin", "http://localhost:5000")],
        "",
    )
    .await;
    assert_eq!(
        deleted.status(),
        StatusCode::NO_CONTENT,
        "authenticated delete is independent of general management"
    );
    assert!(!root.path().join("retained.bin").exists());
    std::fs::write(root.path().join("retained.bin"), b"new-entry").unwrap();
    policy
        .update(PolicyUpdate {
            expected_revision: changed.revision,
            permissions: changed.permissions,
            key: KeyUpdate::Replace("session-new-key".into()),
        })
        .unwrap();
    let stale = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[("cookie", &cookie), ("origin", "http://localhost:5000")],
        "",
    )
    .await;
    assert_eq!(stale.status(), StatusCode::UNAUTHORIZED);
    assert_eq!(
        std::fs::read(root.path().join("retained.bin")).unwrap(),
        b"new-entry"
    );
    let current = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[("authorization", "Bearer session-new-key")],
        "",
    )
    .await;
    assert_eq!(current.status(), StatusCode::NO_CONTENT);
    assert!(!root.path().join("retained.bin").exists());
}

#[tokio::test]
async fn disabling_delete_allows_key_removal_without_readmitting_delete() {
    let root = tempfile::tempdir().unwrap();
    std::fs::write(root.path().join("retained.bin"), b"retained").unwrap();
    let mut config = ServerConfig::new(root.path().to_owned(), "127.0.0.1:0".parse().unwrap());
    config.key = Some("retained-key".into());
    config.permissions = Some(Permissions {
        list: true,
        download: true,
        upload: false,
        manage: false,
        rename: false,
        delete: true,
    });
    let (router, policy) =
        router_with_store_and_policy(&config, Arc::new(FsStore::new(root.path()).unwrap()))
            .unwrap();
    let enabled = policy.status();
    let disabled = policy
        .update(PolicyUpdate {
            expected_revision: enabled.revision,
            permissions: Permissions {
                delete: false,
                ..enabled.permissions
            },
            key: KeyUpdate::Keep,
        })
        .unwrap();
    assert!(disabled.auth_required);
    let denied = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[("authorization", "Bearer retained-key")],
        "",
    )
    .await;
    assert_eq!(denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        std::fs::read(root.path().join("retained.bin")).unwrap(),
        b"retained"
    );
    let unkeyed = policy
        .update(PolicyUpdate {
            expected_revision: disabled.revision,
            permissions: disabled.permissions,
            key: KeyUpdate::Remove,
        })
        .unwrap();
    assert!(!unkeyed.auth_required);
    let public = request(&router, "GET", "/api/v1/entries?path=", &[], "").await;
    assert_eq!(public.status(), StatusCode::OK);
    assert!(matches!(
        policy.update(PolicyUpdate {
            expected_revision: unkeyed.revision,
            permissions: Permissions {
                delete: true,
                ..unkeyed.permissions.clone()
            },
            key: KeyUpdate::Keep,
        }),
        Err(Error::Forbidden)
    ));
    assert_eq!(policy.status(), unkeyed);
    let still_public = request(&router, "GET", "/api/v1/entries?path=", &[], "").await;
    assert_eq!(still_public.status(), StatusCode::OK);
    let still_denied = request(
        &router,
        "DELETE",
        "/api/v1/entries?path=retained.bin",
        &[],
        "",
    )
    .await;
    assert_eq!(still_denied.status(), StatusCode::FORBIDDEN);
    assert_eq!(
        std::fs::read(root.path().join("retained.bin")).unwrap(),
        b"retained"
    );
}
