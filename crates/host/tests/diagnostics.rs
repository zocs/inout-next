use share_host::{HostController, StartOptions};

struct Fixture {
    _temp: tempfile::TempDir,
    host: HostController,
    root: std::path::PathBuf,
}
impl Fixture {
    async fn new(key: Option<&str>) -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("files");
        let web = temp.path().join("web");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir_all(web.join("_next")).unwrap();
        std::fs::write(
            web.join("index.html"),
            "<!doctype html><title>Diagnostics</title>",
        )
        .unwrap();
        std::fs::write(root.join("sample.bin"), vec![23; 10003]).unwrap();
        std::fs::write(root.join("资料 & #+%.bin"), vec![7; 33]).unwrap();
        std::fs::write(root.join("empty.bin"), []).unwrap();
        let host = HostController::new(web);
        host.select_root(root.clone()).await.unwrap();
        host.start(StartOptions {
            key: key.map(str::to_owned),
            ..Default::default()
        })
        .await
        .unwrap();
        Self {
            _temp: temp,
            host,
            root,
        }
    }
}

#[tokio::test]
async fn settings_require_enabled_mode_validate_and_reset_after_disable_and_stop() {
    let fixture = Fixture::new(None).await;
    assert!(!fixture.host.diagnostic_status().await.enabled);
    assert!(fixture.host.set_download_buffer(16).await.is_err());
    assert!(fixture
        .host
        .run_diagnostic_test("sample.bin".into(), 1)
        .await
        .is_err());
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    for kib in [16, 32, 48, 64, 80, 96, 112, 128] {
        assert_eq!(
            fixture
                .host
                .set_download_buffer(kib)
                .await
                .unwrap()
                .buffer_kib,
            kib
        );
    }
    fixture.host.set_download_buffer(32).await.unwrap();
    for kib in [0, 1, 15, 17, 129, usize::MAX] {
        assert!(fixture.host.set_download_buffer(kib).await.is_err());
        assert_eq!(fixture.host.diagnostic_status().await.buffer_kib, 32);
    }
    assert_eq!(
        fixture
            .host
            .reset_diagnostic_settings()
            .await
            .unwrap()
            .buffer_kib,
        128
    );
    fixture.host.set_download_buffer(16).await.unwrap();
    let disabled = fixture.host.set_diagnostic_mode(false).await.unwrap();
    assert!(!disabled.enabled);
    assert_eq!(disabled.buffer_kib, 128);
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    fixture.host.set_download_buffer(48).await.unwrap();
    fixture.host.stop().await.unwrap();
    let stopped = fixture.host.diagnostic_status().await;
    assert!(!stopped.enabled);
    assert_eq!(stopped.buffer_kib, 128);
    assert!(fixture.host.set_diagnostic_mode(true).await.is_err());
}

#[tokio::test]
async fn authenticated_real_ranges_count_exact_bytes_and_never_expose_secrets_or_paths() {
    let secret = "diagnostics-test-secret";
    let fixture = Fixture::new(Some(secret)).await;
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    fixture.host.set_download_buffer(32).await.unwrap();
    for connections in [1, 2, 4, 8] {
        let report = fixture
            .host
            .run_diagnostic_test("sample.bin".into(), connections)
            .await
            .unwrap();
        assert_eq!(report.bytes, 10003);
        assert_eq!(report.connections, connections);
        assert_eq!(report.buffer_kib, 32);
        assert!(report.duration_ms > 0.0);
        assert!(report.mib_per_second.is_finite() && report.mib_per_second > 0.0);
        let records =
            serde_json::to_string(&(report, fixture.host.diagnostic_status().await)).unwrap();
        assert!(!records.contains(secret));
        assert!(!records.contains(fixture.root.to_str().unwrap()));
        assert!(!records.contains("sample.bin"));
    }
    assert_eq!(
        fixture
            .host
            .run_diagnostic_test("资料 & #+%.bin".into(), 8)
            .await
            .unwrap()
            .bytes,
        33
    );
    fixture.host.stop().await.unwrap();
}

#[tokio::test]
async fn invalid_paths_connections_empty_and_protected_names_are_rejected() {
    let fixture = Fixture::new(Some("private-read-key")).await;
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    for path in [
        "",
        "../sample.bin",
        "/sample.bin",
        "a/../sample.bin",
        "a\\sample.bin",
        "INOUTTMP/secret",
        ".inout-stage-private",
        "missing",
        "empty.bin",
    ] {
        assert!(
            fixture
                .host
                .run_diagnostic_test(path.into(), 1)
                .await
                .is_err(),
            "{path}"
        );
        assert!(!fixture.host.diagnostic_status().await.running);
    }
    for connections in [0, 3, 5, 16] {
        assert!(fixture
            .host
            .run_diagnostic_test("sample.bin".into(), connections)
            .await
            .is_err());
    }
    #[cfg(unix)]
    {
        std::fs::write(fixture._temp.path().join("outside"), "private").unwrap();
        std::os::unix::fs::symlink(
            fixture._temp.path().join("outside"),
            fixture.root.join("escape"),
        )
        .unwrap();
        assert!(fixture
            .host
            .run_diagnostic_test("escape".into(), 1)
            .await
            .is_err());
    }
    fixture.host.stop().await.unwrap();
}

// Poll a real IPC-shaped future once: connection setup is pending at this point.
// No test request task is detached and dropping this future exercises RAII.
async fn begin_pending<F: std::future::Future>(mut future: std::pin::Pin<&mut F>) {
    std::future::poll_fn(|cx| {
        assert!(
            future.as_mut().poll(cx).is_pending(),
            "Probe finished before pending-work check"
        );
        std::task::Poll::Ready(())
    })
    .await;
}

#[tokio::test]
async fn duplicate_settings_cancel_disable_stop_and_future_drop_release_pending_probe() {
    let fixture = Fixture::new(None).await;
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    for action in ["cancel", "disable", "drop", "stop"] {
        let mut probe = Box::pin(fixture.host.run_diagnostic_test("sample.bin".into(), 4));
        begin_pending(probe.as_mut()).await;
        assert!(fixture.host.diagnostic_status().await.running);
        assert!(fixture
            .host
            .run_diagnostic_test("sample.bin".into(), 1)
            .await
            .is_err());
        assert!(fixture.host.set_download_buffer(16).await.is_err());
        assert!(fixture.host.reset_diagnostic_settings().await.is_err());
        match action {
            "cancel" => {
                fixture.host.cancel_diagnostic_test().await.unwrap();
                assert!(probe.await.is_err());
            }
            "disable" => {
                fixture.host.set_diagnostic_mode(false).await.unwrap();
                assert!(probe.await.is_err());
            }
            "drop" => drop(probe),
            "stop" => {
                fixture.host.stop().await.unwrap();
                assert!(probe.await.is_err());
            }
            _ => unreachable!(),
        }
        assert!(!fixture.host.diagnostic_status().await.running);
        if action == "stop" {
            fixture.host.start(StartOptions::default()).await.unwrap();
        }
        fixture.host.set_diagnostic_mode(true).await.unwrap();
        assert_eq!(
            fixture
                .host
                .run_diagnostic_test("sample.bin".into(), 1)
                .await
                .unwrap()
                .bytes,
            10003
        );
    }
    fixture.host.stop().await.unwrap();
}

#[tokio::test]
async fn sample_is_bounded_to_first_64_mib_and_small_files_have_nonzero_ranges() {
    let fixture = Fixture::new(None).await;
    let file = std::fs::File::create(fixture.root.join("large.bin")).unwrap();
    file.set_len(70 * 1024 * 1024).unwrap();
    std::fs::write(fixture.root.join("tiny.bin"), [1]).unwrap();
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    assert_eq!(
        fixture
            .host
            .run_diagnostic_test("large.bin".into(), 8)
            .await
            .unwrap()
            .bytes,
        67_108_864
    );
    assert!(fixture
        .host
        .run_diagnostic_test("tiny.bin".into(), 8)
        .await
        .is_err());
    assert_eq!(
        fixture
            .host
            .run_diagnostic_test("tiny.bin".into(), 1)
            .await
            .unwrap()
            .bytes,
        1
    );
    fixture.host.stop().await.unwrap();
}

#[tokio::test]
async fn restart_cannot_overlap_a_canceled_probe_whose_caller_has_not_resumed() {
    let fixture = Fixture::new(Some("old-session-key")).await;
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    let mut probe = Box::pin(fixture.host.run_diagnostic_test("sample.bin".into(), 1));
    begin_pending(probe.as_mut()).await;
    fixture.host.stop().await.unwrap();
    fixture.host.start(StartOptions::default()).await.unwrap();
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    assert!(
        fixture
            .host
            .run_diagnostic_test("sample.bin".into(), 1)
            .await
            .is_err(),
        "Restart overlapped an owned pending probe"
    );
    drop(probe);
    assert_eq!(
        fixture
            .host
            .run_diagnostic_test("sample.bin".into(), 1)
            .await
            .unwrap()
            .bytes,
        10003
    );
    fixture.host.stop().await.unwrap();
}

#[tokio::test]
async fn real_core_rejects_an_alias_of_a_protected_inflight_upload_inode() {
    use tokio::io::AsyncWriteExt;
    let fixture = Fixture::new(None).await;
    fixture.host.stop().await.unwrap();
    let status = fixture
        .host
        .start(StartOptions {
            uploads: true,
            key: Some("upload-protection-key".into()),
            ..Default::default()
        })
        .await
        .unwrap();
    fixture.host.set_diagnostic_mode(true).await.unwrap();
    let address = status.manager_url.unwrap();
    let address = address.strip_prefix("http://").unwrap();
    let mut upload = tokio::net::TcpStream::connect(address).await.unwrap();
    upload.write_all(format!("PUT /api/v1/content?path=upload.bin HTTP/1.1\r\nHost: {address}\r\nAuthorization: Bearer upload-protection-key\r\nContent-Length: 1048576\r\n\r\n").as_bytes()).await.unwrap();
    upload.write_all(&[5; 10003]).await.unwrap();
    let stage = tokio::time::timeout(std::time::Duration::from_secs(2), async {
        loop {
            if let Ok(entries) = std::fs::read_dir(fixture.root.join("INOUTTMP")) {
                for entry in entries.flatten() {
                    // Windows directory entries can cache zero length while the writer is open.
                    // Query a file handle so the in-flight upload remains incomplete.
                    let file = std::fs::File::open(entry.path()).unwrap();
                    if file.metadata().unwrap().len() > 0 {
                        return entry.path();
                    }
                }
            }
            tokio::task::yield_now().await;
        }
    })
    .await
    .expect("Upload never created a populated protected stage");
    std::fs::hard_link(stage, fixture.root.join("staged-alias.bin")).unwrap();
    let error = fixture
        .host
        .run_diagnostic_test("staged-alias.bin".into(), 1)
        .await
        .err()
        .expect("Diagnostic HEAD bypassed protected inode checks");
    assert!(error.contains("access is denied"));
    drop(upload);
    fixture.host.stop().await.unwrap();
    assert!(!fixture.root.join("upload.bin").exists());
}
