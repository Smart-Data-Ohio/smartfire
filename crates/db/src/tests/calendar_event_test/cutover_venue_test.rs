//! test/models/event/venue_test.rb, through real Room/Membership/Event operations.
use super::*;
use crate::Membership;
use crate::models::{
    calendar_event::changes::EventChanges,
    room_delete::{self, HuddleConfig},
};
fn venue(t: &TestDb, kind: RoomType, name: &str) -> i64 {
    let name = name.to_owned();
    t.write(move |tx| {
        Room::create_for(
            tx,
            kind,
            Some(&name),
            id("david"),
            &[id("david"), id("jason")],
        )
    })
    .id
}
fn event(t: &TestDb, venue: Option<i64>, series: bool) -> CalendarEvent {
    let mut a = attrs(t);
    a.starts_at = Some(t.now().since(SignedDuration::from_hours(48)));
    a.venue_room_id = venue;
    if series {
        a.title = "Weekly planning".into();
        a.recurrence_rule = Some("weekly".into());
        a.recurrence_until = Some("2026-10-08".parse().unwrap());
    }
    t.write(move |tx| CalendarEvent::create(tx, a))
}
fn change(t: &TestDb, eid: i64, venue: Option<i64>, scope: &str) -> bool {
    let scope = scope.to_owned();
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            eid,
            EventChanges {
                venue_room_id: Some(venue),
                ..Default::default()
            },
            &scope,
            Some(id("david")),
        )
    })
}
fn venues(t: &TestDb, head: &CalendarEvent) -> Vec<Option<i64>> {
    t.read(|c| {
        Ok(head
            .series_events(c)?
            .into_iter()
            .map(|e| e.venue_room_id)
            .collect())
    })
}
fn invalid(t: &TestDb, vid: i64) {
    let mut a = attrs(t);
    a.venue_room_id = Some(vid);
    let result = t.try_write(move |tx| CalendarEvent::create(tx, a));
    assert!(matches!(&result, Err(Error::RecordInvalid(_))));
    let Error::RecordInvalid(e) = result.unwrap_err() else {
        panic!("expected validation error")
    };
    assert_eq!(
        e.on("venue"),
        vec!["must be a voice or Stage channel you belong to"]
    );
}
#[test]
fn cutover_venue_optional_is_valid_and_absent() {
    let t = frozen();
    assert_eq!(event(&t, None, false).venue_room_id, None);
}
#[test]
fn cutover_venue_voice_and_stage_are_valid() {
    let t = frozen();
    for (kind, name) in [(RoomType::Voice, "Lounge"), (RoomType::Stage, "Town Hall")] {
        let vid = venue(&t, kind, name);
        assert_eq!(event(&t, Some(vid), false).venue_room_id, Some(vid));
    }
}
#[test]
fn cutover_venue_text_and_direct_are_rejected() {
    let t = frozen();
    for vid in [id("designers"), id("david_and_jason")] {
        invalid(&t, vid);
    }
}
#[test]
fn cutover_venue_organizer_membership_is_required() {
    let t = frozen();
    let vid = t
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Voice,
                Some("Outsiders"),
                id("jason"),
                &[id("jason")],
            )
        })
        .id;
    invalid(&t, vid);
}
#[test]
fn cutover_venue_voice_event_can_use_own_room() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let mut a = attrs(&t);
    a.room_id = vid;
    a.venue_room_id = Some(vid);
    assert_eq!(
        t.write(move |tx| CalendarEvent::create(tx, a))
            .venue_room_id,
        Some(vid)
    );
}
#[test]
fn cutover_venue_unrelated_edits_remain_valid_after_organizer_leaves() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let e = event(&t, Some(vid), false);
    let eid = e.id;
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), vid, id("david"))?
            .unwrap()
            .destroy(tx)
    });
    assert!(!t.write(move |tx| CalendarEvent::update_with_scope(
        tx,
        eid,
        EventChanges {
            title: Some("Renamed".into()),
            ..Default::default()
        },
        "this_event",
        Some(id("david"))
    )));
    let saved = t.read(|c| CalendarEvent::find(c, eid));
    assert_eq!(saved.title, "Renamed");
    assert_eq!(saved.venue_room_id, Some(vid));
}
#[test]
fn cutover_venue_destroy_clears_link_and_preserves_event() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let e = event(&t, Some(vid), false);
    t.write(move |tx| {
        room_delete::begin_destroy(tx, &Room::find(tx.conn(), vid)?, &HuddleConfig::default())
    });
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(room_delete::perform_with_config(
            &t.db,
            vid,
            HuddleConfig::default(),
        ))
        .unwrap();
    let saved = t.read(|c| CalendarEvent::find(c, e.id));
    assert_eq!(saved.venue_room_id, None);
    assert_eq!(saved.title, "Planning session");
}
// Deleting a room clears the venue of events elsewhere that met there in one statement, past
// the event callbacks, so it tells those events' rooms itself.
#[test]
fn deleting_a_venue_room_tells_the_rooms_of_events_that_met_there() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    event(&t, Some(vid), false);
    t.sink.take();
    t.write(move |tx| {
        room_delete::begin_destroy(tx, &Room::find(tx.conn(), vid)?, &HuddleConfig::default())
    });
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(room_delete::perform_with_config(
            &t.db,
            vid,
            HuddleConfig::default(),
        ))
        .unwrap();
    assert_eq!(rooms_told(&t), [id("designers")]);
}
#[test]
fn cutover_venue_series_copies_to_all_occurrences() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let head = event(&t, Some(vid), true);
    assert_eq!(venues(&t, &head), vec![Some(vid); 3]);
}
#[test]
fn cutover_venue_following_propagates_change() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let stage = venue(&t, RoomType::Stage, "Town Hall");
    let head = event(&t, Some(vid), true);
    change(&t, head.id, Some(stage), "this_and_following");
    assert_eq!(venues(&t, &head), vec![Some(stage); 3]);
}
#[test]
fn cutover_venue_following_propagates_clear() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let head = event(&t, Some(vid), true);
    change(&t, head.id, None, "this_and_following");
    assert_eq!(venues(&t, &head), vec![None; 3]);
}
#[test]
fn cutover_venue_local_edit_changes_only_one_occurrence() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let stage = venue(&t, RoomType::Stage, "Town Hall");
    let head = event(&t, Some(vid), true);
    let rows = t.read(|c| head.series_events(c));
    change(&t, rows[1].id, Some(stage), "this_event");
    assert_eq!(venues(&t, &head), vec![Some(vid), Some(stage), Some(vid)]);
}
#[test]
fn cutover_venue_only_edit_creates_no_inbox_items() {
    let t = frozen();
    let vid = venue(&t, RoomType::Voice, "Lounge");
    let stage = venue(&t, RoomType::Stage, "Town Hall");
    let e = event(&t, Some(vid), false);
    let n = t.read(|c| {
        Ok(c.query_row(
            "SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=?",
            [e.id],
            |r| r.get::<_, i64>(0),
        )?)
    });
    assert!(!change(&t, e.id, Some(stage), "this_event"));
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=?",
            [e.id],
            |r| r.get::<_, i64>(0)
        )?)),
        n
    );
    assert_eq!(
        t.read(|c| CalendarEvent::find(c, e.id)).venue_room_id,
        Some(stage)
    );
}
