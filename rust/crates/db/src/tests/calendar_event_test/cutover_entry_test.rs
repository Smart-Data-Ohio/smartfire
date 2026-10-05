//! test/models/event_calendar_entry_test.rb, also inventoried by WS14g.
use super::*;
use crate::models::google_entry::{self, Entry};
fn entry(t: &TestDb, user: i64, remote: &str) -> Entry {
    let remote = remote.to_owned();
    t.write(move |tx| {
        let mut e = google_entry::reserve(tx, id("launch_party"), user)?;
        tx.conn().execute(
            "UPDATE event_calendar_entries SET google_event_id=? WHERE id=?",
            params![remote, e.id],
        )?;
        e.google_event_id = remote;
        Ok(e)
    })
}
fn deletes(t: &TestDb) -> Vec<serde_json::Value> {
    t.events()
        .iter()
        .filter_map(|e| match e {
            Event::Job(j) if j.class == "Calendar::RemoteDeleteJob" => Some(j.arguments.clone()),
            _ => None,
        })
        .collect()
}
#[test]
fn cutover_entry_destroy_enqueues_captured_remote_identity_after_commit() {
    let t = frozen();
    let e = entry(&t, id("david"), "orphan-id");
    t.sink.take();
    let sink = t.sink.clone();
    t.write(move |tx| {
        google_entry::destroy(tx, &e)?;
        assert!(sink.take().is_empty());
        Ok(())
    });
    assert!(
        t.read(|c| google_entry::find(c, id("launch_party"), id("david")))
            .is_none()
    );
    assert_eq!(
        deletes(&t),
        vec![serde_json::json!([id("david"), "orphan-id"])]
    );
}
#[test]
fn cutover_entry_delete_skips_remote_delete() {
    let t = frozen();
    let e = entry(&t, id("david"), "reconciled-id");
    t.sink.take();
    t.write(move |tx| google_entry::delete(tx, &e));
    assert!(deletes(&t).is_empty());
    assert!(
        t.read(|c| google_entry::find(c, id("launch_party"), id("david")))
            .is_none()
    );
}
#[test]
fn cutover_entry_delete_all_skips_remote_delete_and_preserves_other_users() {
    let t = frozen();
    entry(&t, id("david"), "disconnect-id");
    entry(&t, id("jason"), "other-member-id");
    t.sink.take();
    assert_eq!(
        t.write(|tx| google_entry::delete_all_for_user(tx, id("david"))),
        1
    );
    assert!(deletes(&t).is_empty());
    assert!(
        t.read(|c| google_entry::find(c, id("launch_party"), id("david")))
            .is_none()
    );
    assert!(
        t.read(|c| google_entry::find(c, id("launch_party"), id("jason")))
            .is_some()
    );
}
