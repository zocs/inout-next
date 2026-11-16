use share_host::{HostController, HostSession, Interrupt, SessionCommand, StartOptions};
use share_http::TransferState;
use std::{path::PathBuf, time::Duration};
use tokio::io::{AsyncReadExt, AsyncWriteExt};

struct Fixture {
    session: HostSession,
    root: PathBuf,
    _temp: tempfile::TempDir,
}

impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("简体 繁體 files");
        let web = temp.path().join("web");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir_all(web.join("_next")).unwrap();
        std::fs::write(
            web.join("index.html"),
            "<!doctype html><title>Session</title>",
        )
        .unwrap();
        let mut session = HostSession::new(HostController::new(web));
        session
            .apply(SessionCommand::SelectRoot(root.clone()))
            .await;
        Self {
            session,
            root,
            _temp: temp,
        }
    }

    async fn start(&mut self) -> String {
        self.session
            .apply(SessionCommand::Start(StartOptions {
                uploads: true,
                ..Default::default()
            }))
            .await;
        self.session.snapshot().host.manager_url.unwrap()
    }

    async fn paused_upload(&mut self, url: &str) -> tokio::net::TcpStream {
        let address = url.strip_prefix("http://").unwrap();
        let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
        client.write_all(format!(
            "PUT /api/v1/content?path=active.bin HTTP/1.1\r\nHost: {address}\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc"
        ).as_bytes()).await.unwrap();
        tokio::time::timeout(Duration::from_secs(2), async {
            loop {
                self.session.observe().await;
                let snapshot = self.session.snapshot();
                if snapshot.transfers.records.iter().any(|record| {
                    record.path == "active.bin"
                        && record.state == TransferState::Active
                        && record.bytes == 3
                }) {
                    break;
                }
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("real paused upload must become observable");
        client
    }
}

async fn request(url: &str) -> String {
    let address = url.strip_prefix("http://").unwrap();
    let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
    client
        .write_all(
            format!("GET /api/v1/context HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), client.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    response
}

#[tokio::test]
async fn requesting_stop_and_canceling_keep_a_paused_upload_admitted_until_publication() {
    let mut fixture = Fixture::new().await;
    let url = fixture.start().await;
    let mut client = fixture.paused_upload(&url).await;
    fixture.session.apply(SessionCommand::RequestStop).await;
    let requested = fixture.session.snapshot();
    let reachable = request(&url).await;
    fixture.session.apply(SessionCommand::CancelInterrupt).await;
    let canceled = fixture.session.snapshot();
    client.write_all(b"def").await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), client.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    fixture.session.observe().await;
    let published = fixture.session.snapshot();
    fixture.session.shutdown().await;
    assert_eq!(requested.confirmation, Some(Interrupt::Stop));
    assert!(requested.host.running && canceled.host.running);
    assert_eq!(canceled.host.manager_url.as_deref(), Some(url.as_str()));
    assert_eq!(canceled.confirmation, None);
    assert!(reachable.starts_with("HTTP/1.1 200"));
    assert!(response.starts_with("HTTP/1.1 201"));
    assert_eq!(
        std::fs::read(fixture.root.join("active.bin")).unwrap(),
        b"abcdef"
    );
    assert_eq!(published.transfers.active, 0);
    assert!(published.transfers.records.iter().any(|record| {
        record.path == "active.bin" && record.bytes == 6 && record.state == TransferState::Published
    }));
}

#[tokio::test]
async fn requesting_close_requires_confirmation_before_interrupting_owned_work() {
    let mut fixture = Fixture::new().await;
    let url = fixture.start().await;
    let client = fixture.paused_upload(&url).await;
    fixture.session.apply(SessionCommand::RequestClose).await;
    let requested = fixture.session.snapshot();
    let reachable = request(&url).await;
    fixture
        .session
        .apply(SessionCommand::ConfirmInterrupt)
        .await;
    let confirmed = fixture.session.snapshot();
    let listener_released = tokio::net::TcpStream::connect(url.strip_prefix("http://").unwrap())
        .await
        .is_err();
    drop(client);
    fixture.session.shutdown().await;
    assert_eq!(requested.confirmation, Some(Interrupt::Close));
    assert!(requested.host.running && !requested.exit_ready);
    assert!(reachable.starts_with("HTTP/1.1 200"));
    assert!(!confirmed.host.running && confirmed.exit_ready);
    assert!(listener_released);
    assert!(!fixture.root.join("active.bin").exists());
    assert_eq!(
        std::fs::read_dir(fixture.root.join("INOUTTMP"))
            .unwrap()
            .count(),
        0
    );
    assert_eq!(confirmed.transfers.active, 0);
    assert!(confirmed
        .transfers
        .records
        .iter()
        .any(|record| record.state == TransferState::Interrupted));
}

#[tokio::test]
async fn stopping_an_idle_service_still_requires_confirmation_and_keeps_the_window_open() {
    let mut fixture = Fixture::new().await;
    let url = fixture.start().await;
    fixture.session.apply(SessionCommand::RequestStop).await;
    let requested = fixture.session.snapshot();
    fixture
        .session
        .apply(SessionCommand::ConfirmInterrupt)
        .await;
    let stopped = fixture.session.snapshot();
    let listener_released = tokio::net::TcpStream::connect(url.strip_prefix("http://").unwrap())
        .await
        .is_err();
    fixture.session.shutdown().await;
    assert_eq!(requested.confirmation, Some(Interrupt::Stop));
    assert!(requested.host.running);
    assert!(!stopped.host.running && !stopped.exit_ready);
    assert!(listener_released);
}

#[tokio::test]
async fn failed_start_is_recoverable_and_credentials_never_enter_the_view_snapshot() {
    let mut fixture = Fixture::new().await;
    fixture
        .session
        .apply(SessionCommand::Start(StartOptions {
            key: Some("  ".into()),
            uploads: true,
            ..Default::default()
        }))
        .await;
    let failed = fixture.session.snapshot();
    let key = "session-private-access-key";
    fixture
        .session
        .apply(SessionCommand::Start(StartOptions {
            key: Some(key.into()),
            uploads: true,
            ..Default::default()
        }))
        .await;
    let recovered = fixture.session.snapshot();
    let serialized = serde_json::to_string(&recovered).unwrap();
    fixture.session.shutdown().await;
    assert!(!failed.host.running);
    assert!(failed.problem.is_some());
    assert!(recovered.host.running && recovered.problem.is_none());
    assert!(recovered.host.active_settings.unwrap().auth_required);
    assert!(!serialized.contains(key));
}

#[tokio::test]
async fn a_closed_stopped_session_cannot_be_restarted_by_a_queued_callback() {
    let mut fixture = Fixture::new().await;
    fixture.session.apply(SessionCommand::RequestClose).await;
    let closed = fixture.session.snapshot();
    fixture
        .session
        .apply(SessionCommand::Start(StartOptions::default()))
        .await;
    let after = fixture.session.snapshot();
    fixture.session.shutdown().await;
    assert!(closed.exit_ready && !closed.host.running);
    assert!(after.exit_ready && !after.host.running);
}

#[tokio::test]
async fn confirmation_cannot_change_roots_or_downgrade_a_pending_window_close() {
    let mut fixture = Fixture::new().await;
    let url = fixture.start().await;
    fixture
        .session
        .apply(SessionCommand::ConfirmInterrupt)
        .await;
    let unsolicited = fixture.session.snapshot();
    fixture.session.apply(SessionCommand::RequestClose).await;
    fixture.session.apply(SessionCommand::RequestStop).await;
    fixture
        .session
        .apply(SessionCommand::SelectRoot(
            fixture.root.parent().unwrap().to_path_buf(),
        ))
        .await;
    fixture
        .session
        .apply(SessionCommand::Start(StartOptions::default()))
        .await;
    fixture
        .session
        .apply(SessionCommand::StartWithRoot {
            root: fixture.root.parent().unwrap().to_path_buf(),
            options: StartOptions::default(),
        })
        .await;
    let pending = fixture.session.snapshot();
    fixture.session.apply(SessionCommand::CancelInterrupt).await;
    let reachable = request(&url).await;
    fixture.session.shutdown().await;
    assert!(unsolicited.host.running);
    assert_eq!(pending.confirmation, Some(Interrupt::Close));
    assert!(pending.host.running && pending.problem.is_none());
    assert_eq!(pending.host.root, unsolicited.host.root);
    assert!(reachable.starts_with("HTTP/1.1 200"));
}

#[tokio::test]
async fn a_failed_new_folder_cannot_silently_start_sharing_the_previous_folder() {
    let mut fixture = Fixture::new().await;
    fixture
        .session
        .apply(SessionCommand::StartWithRoot {
            root: fixture.root.join("missing-folder"),
            options: StartOptions {
                uploads: true,
                ..Default::default()
            },
        })
        .await;
    let failed = fixture.session.snapshot();
    fixture.session.shutdown().await;
    assert!(!failed.host.running);
    assert!(failed.host.manager_url.is_none());
    assert!(failed.problem.is_some());
}

#[tokio::test]
async fn adopting_a_running_host_requires_close_confirmation_and_retains_its_upload() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir_all(web.join("_next")).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><title>Adopt</title>",
    )
    .unwrap();
    let host = HostController::new(web);
    host.select_root(root.clone()).await.unwrap();
    let running = host
        .start(StartOptions {
            uploads: true,
            ..Default::default()
        })
        .await
        .unwrap();
    let url = running.manager_url.unwrap();
    let address = url.strip_prefix("http://").unwrap();
    let control = host.transfer_control().await.unwrap();
    let mut client = tokio::net::TcpStream::connect(address).await.unwrap();
    client.write_all(format!(
        "PUT /api/v1/content?path=adopted.bin HTTP/1.1\r\nHost: {address}\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc"
    ).as_bytes()).await.unwrap();
    tokio::time::timeout(Duration::from_secs(2), async {
        while !control
            .snapshot()
            .records
            .iter()
            .any(|record| record.bytes == 3)
        {
            tokio::task::yield_now().await;
        }
    })
    .await
    .unwrap();
    let mut session = HostSession::new(host);
    // No caller-side observation should be needed to preserve ownership.
    session.apply(SessionCommand::RequestClose).await;
    let requested = session.snapshot();
    let reachable = request(&url).await;
    session.apply(SessionCommand::CancelInterrupt).await;
    client.write_all(b"def").await.unwrap();
    let mut response = String::new();
    tokio::time::timeout(Duration::from_secs(2), client.read_to_string(&mut response))
        .await
        .unwrap()
        .unwrap();
    session.observe().await;
    let published = session.snapshot();
    session.shutdown().await;
    assert_eq!(requested.confirmation, Some(Interrupt::Close));
    assert!(!requested.exit_ready && requested.host.running);
    assert_eq!(requested.transfers.active, 1);
    assert_eq!(requested.transfers.records[0].bytes, 3);
    assert!(reachable.starts_with("HTTP/1.1 200"));
    assert!(response.starts_with("HTTP/1.1 201"));
    assert_eq!(std::fs::read(root.join("adopted.bin")).unwrap(), b"abcdef");
    assert_eq!(
        published.transfers.records[0].state,
        TransferState::Published
    );
}
