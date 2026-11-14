use super::*;
use std::{path::PathBuf, time::Duration};
use tokio::sync::oneshot;

struct Fixture {
    _temp: tempfile::TempDir,
    state: Arc<NativeState>,
    root: PathBuf,
    other: PathBuf,
}

impl Fixture {
    async fn new() -> Self {
        let temp = tempfile::tempdir().unwrap();
        let root = temp.path().join("files");
        let other = temp.path().join("other-files");
        let web = temp.path().join("web");
        std::fs::create_dir(&root).unwrap();
        std::fs::create_dir(&other).unwrap();
        std::fs::create_dir_all(web.join("_next")).unwrap();
        std::fs::write(
            web.join("index.html"),
            "<!doctype html><title>Picker fixture</title>",
        )
        .unwrap();
        let state = Arc::new(NativeState {
            host: HostController::new(web),
            transition: Mutex::new(()),
            exiting: AtomicBool::new(false),
            shutdown_completed: AtomicBool::new(false),
        });
        state.host.select_root(root.clone()).await.unwrap();
        let root = root.canonicalize().unwrap();
        let other = other.canonicalize().unwrap();
        Self {
            _temp: temp,
            state,
            root,
            other,
        }
    }
}

// Only the OS dialog completion is controlled here. Selection, lifecycle locks,
// shutdown and service behavior execute the actual production state methods.
async fn pending_picker(
    state: &Arc<NativeState>,
) -> (
    oneshot::Sender<Option<PathBuf>>,
    tokio::task::JoinHandle<Result<HostStatus, String>>,
) {
    let (result_sender, result_receiver) = oneshot::channel();
    let (entered_sender, entered_receiver) = oneshot::channel();
    let state = Arc::clone(state);
    let task = tokio::spawn(async move {
        state
            .pick_folder(|| async {
                entered_sender.send(()).unwrap();
                result_receiver
                    .await
                    .map_err(|_| "Picker receiver closed".to_owned())
            })
            .await
    });
    entered_receiver.await.unwrap();
    (result_sender, task)
}

#[tokio::test]
async fn pending_picker_does_not_delay_shutdown_cleanup() {
    let fixture = Fixture::new().await;
    let (reply, picker) = pending_picker(&fixture.state).await;
    fixture.state.exiting.store(true, Ordering::Release);
    let stopped = tokio::time::timeout(Duration::from_secs(1), fixture.state.stop_for_shutdown())
        .await
        .expect("Shutdown waited for unresolved native picker")
        .unwrap();
    assert!(!stopped.running);
    assert!(!picker.is_finished());
    reply.send(None).unwrap();
    assert!(picker.await.unwrap().is_err());
}

#[tokio::test]
async fn late_picker_selection_is_rejected_after_shutdown_begins() {
    let fixture = Fixture::new().await;
    let (reply, picker) = pending_picker(&fixture.state).await;
    fixture.state.exiting.store(true, Ordering::Release);
    reply.send(Some(fixture.other)).unwrap();
    assert!(picker.await.unwrap().is_err());
    assert_eq!(
        fixture.state.host.status().await.root.as_deref(),
        fixture.root.to_str()
    );
}

#[tokio::test]
async fn picker_selection_and_cancellation_preserve_their_normal_results() {
    let fixture = Fixture::new().await;
    let selected = fixture
        .state
        .pick_folder(|| async { Ok(Some(fixture.other.clone())) })
        .await
        .unwrap();
    assert_eq!(selected.root.as_deref(), fixture.other.to_str());
    let cancelled = fixture
        .state
        .pick_folder(|| async { Ok(None) })
        .await
        .unwrap();
    assert_eq!(cancelled.root.as_deref(), fixture.other.to_str());
    assert!(!cancelled.running);
}

#[tokio::test]
async fn running_service_rejects_picker_before_opening_dialog() {
    let fixture = Fixture::new().await;
    fixture
        .state
        .host
        .start(StartOptions::default())
        .await
        .unwrap();
    assert!(fixture
        .state
        .pick_folder(|| async { panic!("Dialog opened while service was running") })
        .await
        .is_err());
    assert_eq!(
        fixture.state.host.status().await.root.as_deref(),
        fixture.root.to_str()
    );
    fixture.state.host.stop().await.unwrap();
}

#[tokio::test]
async fn picker_reply_cannot_change_root_when_service_starts_during_wait() {
    let fixture = Fixture::new().await;
    let (reply, picker) = pending_picker(&fixture.state).await;
    {
        let _transition =
            tokio::time::timeout(Duration::from_secs(1), fixture.state.transition.lock())
                .await
                .expect("Pending picker blocked native service transition");
        fixture
            .state
            .host
            .start(StartOptions::default())
            .await
            .unwrap();
    }
    reply.send(Some(fixture.other)).unwrap();
    assert!(picker.await.unwrap().is_err());
    let status = fixture.state.host.status().await;
    assert!(status.running);
    assert_eq!(status.root.as_deref(), fixture.root.to_str());
    fixture.state.host.stop().await.unwrap();
}
