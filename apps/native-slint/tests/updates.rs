use share_native_slint::updates::Latest;

#[test]
fn a_stalled_view_receives_only_the_latest_state_and_can_schedule_the_next_change() {
    let updates = Latest::default();
    let scheduled = (0..1000).filter(|value| updates.push(*value)).count();
    assert_eq!(scheduled, 1, "a stalled view must not grow its event queue");
    assert_eq!(updates.take(), Some(999));
    assert_eq!(updates.take(), None);
    assert!(updates.push(1000));
    assert!(!updates.push(1001));
    assert_eq!(updates.take(), Some(1001));
}

#[test]
fn a_failed_delivery_drops_retained_data_and_permits_a_fresh_schedule() {
    let updates = Latest::default();
    assert!(updates.push(String::from("old service")));
    updates.discard();
    assert_eq!(updates.take(), None);
    assert!(updates.push(String::from("new service")));
    assert_eq!(updates.take().as_deref(), Some("new service"));
}
