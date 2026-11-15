use super::*;
use std::{path::PathBuf, time::Duration};

struct Fixture {
    _temporary: tempfile::TempDir,
    state: Arc<NativeState>,
    root: PathBuf,
}

impl Fixture {
    fn new() -> Self {
        let temporary = tempfile::tempdir().unwrap();
        let root = temporary.path().join("files/shared");
        let web = temporary.path().join("files/runtime-assets/web");
        std::fs::create_dir_all(&root).unwrap();
        std::fs::create_dir_all(web.join("_next")).unwrap();
        std::fs::write(
            web.join("index.html"),
            "<!doctype html><title>Mobile fixture</title>",
        )
        .unwrap();
        std::fs::write(
            root.join("retained.txt"),
            "private upload retained across pause",
        )
        .unwrap();
        let host = HostController::new_app_private(web, root.clone()).unwrap();
        Self {
            _temporary: temporary,
            state: Arc::new(NativeState::new(host)),
            root,
        }
    }
}

#[test]
fn android_framework_exit_stays_deferred_after_owned_service_cleanup() {
    // The service worker may publish completion, then be descheduled before
    // cleanup_before_exit/_exit. Ordinary framework exit must stay vetoed.
    assert!(framework_exit_is_deferred(true, Some(true)));
    assert!(framework_exit_is_deferred(true, Some(false)));
    assert!(framework_exit_is_deferred(true, None));
}

#[test]
fn desktop_framework_exit_is_released_after_owned_service_cleanup() {
    assert!(framework_exit_is_deferred(false, Some(false)));
    assert!(!framework_exit_is_deferred(false, Some(true)));
    assert!(!framework_exit_is_deferred(false, None));
}

#[tokio::test]
async fn suspension_closes_the_real_listener_and_explicit_resume_preserves_private_files() {
    let fixture = Fixture::new();
    let started = fixture.state.start(StartOptions::default()).await.unwrap();
    let address = started
        .manager_url
        .unwrap()
        .trim_start_matches("http://")
        .to_owned();
    assert!(tokio::net::TcpStream::connect(&address).await.is_ok());
    fixture.state.mark_suspended();
    let stopped = fixture.state.stop_for_background().await.unwrap();
    assert!(!stopped.running);
    assert!(tokio::net::TcpStream::connect(&address).await.is_err());
    assert!(fixture.state.start(StartOptions::default()).await.is_err());
    assert!(!fixture.state.exiting.load(Ordering::Acquire));
    fixture.state.mark_resumed();
    assert!(!fixture.state.host.status().await.running);
    assert!(
        fixture
            .state
            .start(StartOptions::default())
            .await
            .unwrap()
            .running
    );
    assert_eq!(
        std::fs::read_to_string(fixture.root.join("retained.txt")).unwrap(),
        "private upload retained across pause"
    );
    fixture.state.host.stop().await.unwrap();
}

#[tokio::test]
async fn early_resume_cannot_start_until_the_pending_pause_has_drained() {
    let fixture = Fixture::new();
    fixture.state.mark_suspended();
    fixture.state.mark_resumed();
    assert!(fixture.state.start(StartOptions::default()).await.is_err());
    assert!(!fixture.state.host.status().await.running);
    fixture.state.stop_for_background().await.unwrap();
    assert!(
        fixture
            .state
            .start(StartOptions::default())
            .await
            .unwrap()
            .running
    );
    fixture.state.host.stop().await.unwrap();
}

#[tokio::test]
async fn queued_start_cannot_bind_after_the_main_activity_suspends() {
    let fixture = Fixture::new();
    let transition = fixture.state.transition.lock().await;
    let state = fixture.state.clone();
    let (entered, receiver) = tokio::sync::oneshot::channel();
    let start = tokio::spawn(async move {
        entered.send(()).unwrap();
        state.start(StartOptions::default()).await
    });
    receiver.await.unwrap();
    fixture.state.mark_suspended();
    drop(transition);
    assert!(tokio::time::timeout(Duration::from_secs(1), start)
        .await
        .unwrap()
        .unwrap()
        .is_err());
    assert!(!fixture.state.host.status().await.running);
    fixture.state.stop_for_background().await.unwrap();
    fixture.state.mark_resumed();
    assert!(
        fixture
            .state
            .start(StartOptions::default())
            .await
            .unwrap()
            .running
    );
    fixture.state.host.stop().await.unwrap();
}

#[tokio::test]
async fn repeated_suspensions_require_every_pending_stop_before_restart() {
    let fixture = Fixture::new();
    fixture.state.mark_suspended();
    fixture.state.mark_suspended();
    fixture.state.mark_resumed();
    fixture.state.stop_for_background().await.unwrap();
    assert!(fixture.state.start(StartOptions::default()).await.is_err());
    fixture.state.stop_for_background().await.unwrap();
    assert!(
        fixture
            .state
            .start(StartOptions::default())
            .await
            .unwrap()
            .running
    );
    fixture.state.host.stop().await.unwrap();
}
