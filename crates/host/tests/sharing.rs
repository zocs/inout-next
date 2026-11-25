use share_host::{
    HostController, HostSession, LinkKind, ProblemAction, ProblemReason, SessionCommand,
    StartOptions, StartReason,
};
use share_http::WebBundle;
use tokio::io::{AsyncReadExt, AsyncWriteExt};

static PAGES: &[(&str, &[u8])] = &[
    ("index.html", b"<!doctype html><title>Embedded</title>"),
    ("_next/app.js", b"embedded()"),
];

async fn get(url: &str, path: &str) -> String {
    let address = url.strip_prefix("http://").unwrap();
    let mut stream = tokio::net::TcpStream::connect(address).await.unwrap();
    stream
        .write_all(
            format!("GET {path} HTTP/1.1\r\nHost: {address}\r\nConnection: close\r\n\r\n")
                .as_bytes(),
        )
        .await
        .unwrap();
    let mut response = String::new();
    stream.read_to_string(&mut response).await.unwrap();
    response
}

#[tokio::test]
async fn an_embedded_host_can_share_the_folder_that_contains_the_application() {
    // Mirrors sharing a home directory that holds the extracted package.
    let home = tempfile::tempdir().unwrap();
    let package = home.path().join("apps/inout");
    std::fs::create_dir_all(package.join("web/_next")).unwrap();
    std::fs::write(package.join("web/index.html"), b"on-disk copy").unwrap();
    let host = HostController::new_embedded(WebBundle::from_static(PAGES).unwrap());
    host.select_root(home.path().to_owned()).await.unwrap();
    let status = host.start_checked(StartOptions::default()).await.unwrap();
    let url = status.manager_url.unwrap();
    let page = get(&url, "/").await;
    assert!(page.starts_with("HTTP/1.1 200"));
    assert!(page.ends_with("<!doctype html><title>Embedded</title>"));
    assert!(get(&url, "/api/v1/entries?path=apps/inout/web")
        .await
        .starts_with("HTTP/1.1 200"));
    host.stop().await.unwrap();
}

#[tokio::test]
async fn start_failures_report_which_setting_needs_attention() {
    let temp = tempfile::tempdir().unwrap();
    let share = temp.path().join("share");
    std::fs::create_dir_all(share.join("web/_next")).unwrap();
    std::fs::write(share.join("web/index.html"), b"page").unwrap();

    let host = HostController::new_embedded(WebBundle::from_static(PAGES).unwrap());
    let missing = host.start_checked(StartOptions::default()).await;
    assert_eq!(missing.err().unwrap().reason, StartReason::NoFolder);
    host.select_root(share.clone()).await.unwrap();
    let blank = host
        .start_checked(StartOptions {
            uploads: true,
            key: Some("  ".into()),
            ..Default::default()
        })
        .await;
    assert_eq!(blank.err().unwrap().reason, StartReason::Key);
    let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let busy = host
        .start_checked(StartOptions {
            port: occupied.local_addr().unwrap().port(),
            ..Default::default()
        })
        .await;
    assert_eq!(busy.err().unwrap().reason, StartReason::PortInUse);
    host.start_checked(StartOptions::default()).await.unwrap();
    let again = host.start_checked(StartOptions::default()).await;
    assert_eq!(again.err().unwrap().reason, StartReason::AlreadyRunning);
    host.stop().await.unwrap();

    // A directory-backed host still refuses resources inside the share, and
    // says so instead of blaming the key or the port.
    let overlapping = HostController::new(share.join("web"));
    overlapping.select_root(share).await.unwrap();
    let failure = overlapping
        .start_checked(StartOptions::default())
        .await
        .err()
        .unwrap();
    assert_eq!(failure.reason, StartReason::Resources);
    assert!(overlapping.start(StartOptions::default()).await.is_err());
}

#[tokio::test]
async fn sessions_publish_labelled_endpoints_and_typed_problems() {
    let temp = tempfile::tempdir().unwrap();
    let share = temp.path().join("share");
    std::fs::create_dir(&share).unwrap();
    let host = HostController::new_embedded(WebBundle::from_static(PAGES).unwrap());
    let mut session = HostSession::new(host);
    assert!(session.snapshot().endpoints.is_empty());

    session
        .apply(SessionCommand::StartWithRoot {
            root: temp.path().join("absent"),
            options: StartOptions::default(),
        })
        .await;
    let problem = session.snapshot().problem.unwrap();
    assert!(problem.action == ProblemAction::SelectRoot);
    assert_eq!(problem.reason, ProblemReason::Folder);

    let occupied = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    session
        .apply(SessionCommand::StartWithRoot {
            root: share.clone(),
            options: StartOptions {
                port: occupied.local_addr().unwrap().port(),
                ..Default::default()
            },
        })
        .await;
    assert_eq!(
        session.snapshot().problem.unwrap().reason,
        ProblemReason::PortInUse
    );

    // Local-only sharing lists this device alone.
    session
        .apply(SessionCommand::Start(StartOptions::default()))
        .await;
    let local = session.snapshot();
    assert!(local.problem.is_none());
    assert_eq!(local.endpoints.len(), 1);
    assert_eq!(local.endpoints[0].kind, LinkKind::Loopback);
    assert_eq!(
        Some(&local.endpoints[0].url),
        local.host.manager_url.as_ref()
    );
    session.apply(SessionCommand::RequestStop).await;
    session.apply(SessionCommand::ConfirmInterrupt).await;
    assert!(session.snapshot().endpoints.is_empty());

    // Network sharing keeps this device last and mirrors the same order in
    // the existing address list.
    session
        .apply(SessionCommand::Start(StartOptions {
            network: true,
            ..Default::default()
        }))
        .await;
    let shared = session.snapshot();
    let (last, lan) = shared.endpoints.split_last().unwrap();
    assert_eq!(last.kind, LinkKind::Loopback);
    assert!(lan
        .iter()
        .all(|endpoint| endpoint.kind != LinkKind::Loopback));
    assert_eq!(
        lan.iter().map(|endpoint| &endpoint.url).collect::<Vec<_>>(),
        shared.host.addresses.iter().collect::<Vec<_>>()
    );
    let port = shared.host.active_settings.unwrap().port;
    assert!(shared
        .endpoints
        .iter()
        .all(|endpoint| endpoint.url.ends_with(&format!(":{port}"))));
    session.shutdown().await;
}
