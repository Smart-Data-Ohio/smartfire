//! test/models/event/reference_sync_test.rb: exact reference lifecycle assertions.
use super::*;
use crate::{Message, MessageChanges, NewMessage};
fn message(t: &TestDb, text: &str, client: &str) -> Message {
    let text = text.to_owned();
    let client = client.to_owned();
    t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                markdown_source: Some(text),
                client_message_id: Some(client),
                ..Default::default()
            },
        )
    })
}
fn references(t: &TestDb, mid: i64) -> Vec<i64> {
    t.read(|c| {
        crate::sql::query_all(
            c,
            "SELECT event_id FROM event_references WHERE message_id=? ORDER BY event_id",
            [mid],
            |r| r.get(0),
        )
    })
}
fn event_ids(t: &TestDb, mid: i64) -> Vec<i64> {
    t.read(|c| {
        Ok(CalendarEvent::for_message_ids(c, &[mid])?
            .remove(&mid)
            .unwrap_or_default()
            .into_iter()
            .map(|e| e.id)
            .collect())
    })
}
#[test]
fn cutover_reference_message_without_link_has_no_events() {
    let t = frozen();
    let m = message(&t, "just chatting", "evt-ref-none");
    assert!(references(&t, m.id).is_empty());
    assert!(event_ids(&t, m.id).is_empty());
}
#[test]
fn cutover_reference_missing_event_creates_no_references() {
    let t = frozen();
    let m = message(
        &t,
        &format!("see /rooms/{}/events/999999", id("designers")),
        "evt-ref-missing",
    );
    assert!(references(&t, m.id).is_empty());
    assert!(event_ids(&t, m.id).is_empty());
}
#[test]
fn cutover_reference_edit_adds_the_exact_event() {
    let t = frozen();
    let mut m = message(&t, "just chatting", "evt-ref-edit");
    assert!(references(&t, m.id).is_empty());
    assert!(event_ids(&t, m.id).is_empty());
    let mid = m.id;
    t.write(move |tx| {
        m.edit(
            tx,
            MessageChanges {
                markdown_source: Some(format!(
                    "now with /rooms/{}/events/{}",
                    id("designers"),
                    id("launch_party")
                )),
                ..Default::default()
            },
        )
    });
    assert_eq!(references(&t, mid), vec![id("launch_party")]);
    assert_eq!(event_ids(&t, mid), vec![id("launch_party")]);
}
#[test]
fn cutover_reference_message_destroy_removes_join_and_preserves_event() {
    let t = frozen();
    let m = message(
        &t,
        &format!(
            "see /rooms/{}/events/{}",
            id("designers"),
            id("launch_party")
        ),
        "evt-ref-msg-delete",
    );
    assert_eq!(references(&t, m.id), vec![id("launch_party")]);
    let mid = m.id;
    t.write(move |tx| m.destroy(tx));
    assert!(references(&t, mid).is_empty());
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, id("launch_party"))).id,
        id("launch_party")
    );
}
