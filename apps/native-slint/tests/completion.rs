use share_host::{HostController, HostSession};
use share_native_slint::{
    bridge::BridgeUpdate,
    updates::{CommandCompletion, Latest},
};

#[test]
fn old_progress_keeps_an_action_pending_until_its_own_completion_arrives() {
    let mut pending = CommandCompletion::default();
    pending.begin(7, "confirm");
    assert_eq!(pending.finish_through(6), None);
    assert!(pending.is_pending());
    assert_eq!(pending.finish_through(7), Some("confirm"));
    assert!(!pending.is_pending());
    assert_eq!(pending.finish_through(7), None, "focus handoff runs once");
}

#[test]
fn an_earlier_completion_cannot_unlock_a_later_action() {
    let mut pending = CommandCompletion::default();
    pending.begin(3, "start");
    pending.begin(4, "close");
    assert_eq!(pending.finish_through(3), None);
    assert!(pending.is_pending());
    assert_eq!(pending.finish_through(4), Some("close"));
}

#[test]
fn progress_coalesced_after_completion_retains_the_command_acknowledgement() {
    let snapshot = HostSession::new(HostController::new("unopened-web".into())).snapshot();
    let updates = Latest::default();
    let mut pending = CommandCompletion::default();
    pending.begin(2, "cancel");
    assert!(updates.push(BridgeUpdate {
        snapshot: snapshot.clone(),
        completed_command: 2,
    }));
    let mut later_progress = snapshot;
    later_progress.transfers.active = 1;
    assert!(!updates.push(BridgeUpdate {
        snapshot: later_progress,
        completed_command: 2,
    }));
    let latest = updates.take().unwrap();
    assert_eq!(latest.snapshot.transfers.active, 1);
    assert_eq!(
        pending.finish_through(latest.completed_command),
        Some("cancel")
    );
    assert!(!pending.is_pending());
}
