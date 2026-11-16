use share_host::{HostController, Interrupt, SessionCommand, SessionSnapshot, StartOptions};
use share_http::TransferState;
use share_native_slint::bridge::{Bridge, BridgeUpdate, CommandError};
use share_native_slint::updates::{CommandCompletion, Latest};
use std::{
    io::{Read, Write},
    sync::mpsc,
    time::{Duration, Instant},
};

fn next_matching(
    receiver: &mpsc::Receiver<BridgeUpdate>,
    matches: impl Fn(&SessionSnapshot) -> bool,
) -> SessionSnapshot {
    next_update_matching(receiver, matches).snapshot
}

fn next_update_matching(
    receiver: &mpsc::Receiver<BridgeUpdate>,
    matches: impl Fn(&SessionSnapshot) -> bool,
) -> BridgeUpdate {
    let deadline = Instant::now() + Duration::from_secs(3);
    loop {
        let snapshot = receiver
            .recv_timeout(deadline.saturating_duration_since(Instant::now()))
            .expect("native bridge must deliver the real service transition");
        if matches(&snapshot.snapshot) {
            return snapshot;
        }
    }
}

#[test]
fn background_bridge_delivers_live_transfers_and_owns_listener_cleanup() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir_all(web.join("_next")).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><title>Bridge</title>",
    )
    .unwrap();
    let (sender, receiver) = mpsc::channel();
    let bridge = Bridge::spawn(HostController::new(web), move |snapshot| {
        let _ = sender.send(snapshot);
    })
    .unwrap();
    let handle = bridge.handle();
    handle
        .send(SessionCommand::SelectRoot(root.clone()))
        .unwrap();
    handle
        .send(SessionCommand::Start(StartOptions {
            uploads: true,
            ..Default::default()
        }))
        .unwrap();
    let running = next_matching(&receiver, |snapshot| snapshot.host.running);
    let url = running.host.manager_url.unwrap();
    let address = url.strip_prefix("http://").unwrap();
    let mut client = std::net::TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client.write_all(format!("PUT /api/v1/content?path=bridge.bin HTTP/1.1\r\nHost: {address}\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc").as_bytes()).unwrap();
    let active_update = next_update_matching(&receiver, |snapshot| {
        snapshot.transfers.records.iter().any(|record| {
            record.path == "bridge.bin"
                && record.bytes == 3
                && record.state == TransferState::Active
        })
    });
    handle.send(SessionCommand::RequestStop).unwrap();
    let requested = next_matching(&receiver, |snapshot| {
        snapshot.confirmation == Some(Interrupt::Stop)
    });
    let cancel = handle.send(SessionCommand::CancelInterrupt).unwrap();
    let mut pending = CommandCompletion::default();
    pending.begin(cancel, "cancel");
    assert_eq!(
        pending.finish_through(active_update.completed_command),
        None
    );
    assert!(pending.is_pending());
    let canceled_update =
        next_update_matching(&receiver, |snapshot| snapshot.confirmation.is_none());
    client.write_all(b"def").unwrap();
    let mut response = String::new();
    client.read_to_string(&mut response).unwrap();
    let published_update = next_update_matching(&receiver, |snapshot| {
        snapshot.transfers.records.iter().any(|record| {
            record.path == "bridge.bin"
                && record.bytes == 6
                && record.state == TransferState::Published
        })
    });
    let updates = Latest::default();
    assert!(updates.push(canceled_update.clone()));
    assert!(!updates.push(published_update.clone()));
    let coalesced = updates.take().unwrap();
    assert_eq!(coalesced.completed_command, cancel);
    assert_eq!(
        pending.finish_through(coalesced.completed_command),
        Some("cancel")
    );
    assert_eq!(pending.finish_through(coalesced.completed_command), None);
    let active = active_update.snapshot;
    let canceled = canceled_update.snapshot;
    let published = published_update.snapshot;
    drop(bridge);
    assert_eq!(active.transfers.active, 1);
    assert!(requested.host.running && canceled.host.running);
    assert_eq!(canceled.host.manager_url.as_deref(), Some(url.as_str()));
    assert_eq!(published.transfers.active, 0);
    assert!(response.starts_with("HTTP/1.1 201"));
    assert_eq!(std::fs::read(root.join("bridge.bin")).unwrap(), b"abcdef");
    assert!(std::net::TcpStream::connect(address).is_err());
    assert_eq!(
        handle.send(SessionCommand::Start(StartOptions::default())),
        Err(CommandError::Closed)
    );
}

#[test]
fn confirmed_close_finishes_the_worker_and_rejects_late_ui_callbacks() {
    let temp = tempfile::tempdir().unwrap();
    let (sender, receiver) = mpsc::channel();
    let bridge = Bridge::spawn(HostController::new(temp.path().into()), move |snapshot| {
        let _ = sender.send(snapshot);
    })
    .unwrap();
    let handle = bridge.handle();
    let initial = next_matching(&receiver, |snapshot| !snapshot.host.running);
    assert!(!initial.exit_ready);
    handle.send(SessionCommand::RequestClose).unwrap();
    let closed = next_matching(&receiver, |snapshot| snapshot.exit_ready);
    drop(bridge);
    assert!(!closed.host.running);
    assert_eq!(
        handle.send(SessionCommand::SelectRoot(temp.path().into())),
        Err(CommandError::Closed)
    );
}

#[test]
fn repeated_failed_starts_each_deliver_completion_so_the_view_can_retry() {
    let temp = tempfile::tempdir().unwrap();
    let root = temp.path().join("files");
    let web = temp.path().join("web");
    std::fs::create_dir(&root).unwrap();
    std::fs::create_dir_all(web.join("_next")).unwrap();
    std::fs::write(
        web.join("index.html"),
        "<!doctype html><title>Retry</title>",
    )
    .unwrap();
    let occupied = std::net::TcpListener::bind("127.0.0.1:0").unwrap();
    let (sender, receiver) = mpsc::channel();
    let bridge = Bridge::spawn(HostController::new(web), move |snapshot| {
        let _ = sender.send(snapshot);
    })
    .unwrap();
    let handle = bridge.handle();
    next_matching(&receiver, |snapshot| snapshot.problem.is_none());
    for _ in 0..2 {
        let command = handle
            .send(SessionCommand::StartWithRoot {
                root: root.clone(),
                options: StartOptions {
                    port: occupied.local_addr().unwrap().port(),
                    ..Default::default()
                },
            })
            .unwrap();
        let failure = next_update_matching(&receiver, |snapshot| snapshot.problem.is_some());
        assert_eq!(failure.completed_command, command);
        assert!(!failure.snapshot.host.running);
    }
    drop(occupied);
    handle
        .send(SessionCommand::StartWithRoot {
            root,
            options: StartOptions::default(),
        })
        .unwrap();
    let recovered = next_matching(&receiver, |snapshot| snapshot.host.running);
    assert!(recovered.problem.is_none());
}

fn prestarted_host() -> (
    tempfile::TempDir,
    tokio::runtime::Runtime,
    HostController,
    String,
    std::path::PathBuf,
) {
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
    let runtime = tokio::runtime::Builder::new_multi_thread()
        .worker_threads(2)
        .enable_all()
        .build()
        .unwrap();
    let host = HostController::new(web);
    let url = runtime.block_on(async {
        host.select_root(root.clone()).await.unwrap();
        host.start(StartOptions {
            uploads: true,
            ..Default::default()
        })
        .await
        .unwrap()
        .manager_url
        .unwrap()
    });
    (temp, runtime, host, url, root)
}

fn begin_upload(address: &str, name: &str) -> std::net::TcpStream {
    let mut client = std::net::TcpStream::connect(address).unwrap();
    client
        .set_read_timeout(Some(Duration::from_secs(3)))
        .unwrap();
    client.write_all(format!(
        "PUT /api/v1/content?path={name} HTTP/1.1\r\nHost: {address}\r\nContent-Length: 6\r\nConnection: close\r\n\r\nabc"
    ).as_bytes()).unwrap();
    client
}

#[test]
fn an_adopted_idle_host_observes_admissions_again_after_its_first_transfer_finishes() {
    let (_temp, _runtime, host, url, root) = prestarted_host();
    let address = url.strip_prefix("http://").unwrap();
    let (sender, receiver) = mpsc::channel();
    let bridge = Bridge::spawn(host, move |snapshot| {
        let _ = sender.send(snapshot);
    })
    .unwrap();
    let initial = next_matching(&receiver, |snapshot| snapshot.host.running);
    assert_eq!(initial.transfers.active, 0);
    let mut first = begin_upload(address, "first.bin");
    let admitted = next_matching(&receiver, |snapshot| {
        snapshot.transfers.active == 1 && snapshot.transfers.records[0].bytes == 3
    });
    first.write_all(b"def").unwrap();
    let mut response = String::new();
    first.read_to_string(&mut response).unwrap();
    let finished = next_matching(&receiver, |snapshot| {
        snapshot.transfers.active == 0 && !snapshot.transfers.records.is_empty()
    });
    let second = begin_upload(address, "second.bin");
    let readmitted = next_matching(&receiver, |snapshot| {
        snapshot.transfers.records.iter().any(|record| {
            record.path == "second.bin"
                && record.state == TransferState::Active
                && record.bytes == 3
        })
    });
    drop(bridge);
    drop(second);
    assert_eq!(admitted.transfers.records[0].path, "first.bin");
    assert_eq!(
        finished.transfers.records[0].state,
        TransferState::Published
    );
    assert_eq!(readmitted.transfers.active, 1);
    assert!(response.starts_with("HTTP/1.1 201"));
    assert_eq!(std::fs::read(root.join("first.bin")).unwrap(), b"abcdef");
    assert!(!root.join("second.bin").exists());
    assert_eq!(std::fs::read_dir(root.join("INOUTTMP")).unwrap().count(), 0);
    assert!(std::net::TcpStream::connect(address).is_err());
}

#[test]
fn adopting_a_host_with_an_active_upload_delivers_its_initial_bytes_and_cleans_up() {
    let (_temp, runtime, host, url, root) = prestarted_host();
    let address = url.strip_prefix("http://").unwrap();
    let control = runtime.block_on(host.transfer_control()).unwrap();
    let client = begin_upload(address, "inherited.bin");
    runtime.block_on(async {
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
    });
    let (sender, receiver) = mpsc::channel();
    let bridge = Bridge::spawn(host, move |snapshot| {
        let _ = sender.send(snapshot);
    })
    .unwrap();
    let initial = next_matching(&receiver, |snapshot| snapshot.host.running);
    drop(bridge);
    drop(client);
    assert_eq!(initial.transfers.active, 1);
    assert_eq!(initial.transfers.records[0].path, "inherited.bin");
    assert_eq!(initial.transfers.records[0].bytes, 3);
    assert!(std::net::TcpStream::connect(address).is_err());
    assert!(!root.join("inherited.bin").exists());
    assert_eq!(std::fs::read_dir(root.join("INOUTTMP")).unwrap().count(), 0);
}
