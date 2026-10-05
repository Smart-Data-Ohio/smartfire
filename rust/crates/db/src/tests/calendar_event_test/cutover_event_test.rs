//! Remaining declarations from test/models/event_test.rb, in Rails declaration order.
use super::*;
use crate::models::{
    calendar_event::changes::EventChanges,
    room_delete::{self, HuddleConfig},
};
use crate::{Involvement, Membership, NewUser, PasswordDigest};

pub(super) fn scheduled(t: &TestDb) -> CalendarEvent {
    let mut a = attrs(t);
    a.starts_at = Some(t.now().since(SignedDuration::from_hours(48)));
    t.write(move |tx| CalendarEvent::create(tx, a))
}
pub(super) fn involvement(t: &TestDb, who: &str, value: Involvement) {
    let user = id(who);
    t.write(move |tx| {
        Membership::find_by_room_and_user(tx.conn(), id("designers"), user)?
            .unwrap()
            .update_involvement(tx, value)
    });
}
pub(super) fn shift(t: &TestDb, event_id: i64) {
    let starts = t.now().since(SignedDuration::from_hours(72));
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            event_id,
            EventChanges {
                starts_at: Some(Some(starts)),
                ..Default::default()
            },
            "this_event",
            Some(id("david")),
        )
    });
}
pub(super) fn cancel(t: &TestDb, event_id: i64) {
    assert!(t.write(move |tx| CalendarEvent::cancel_with_scope(
        tx,
        event_id,
        "this_event",
        Some(id("david"))
    )));
}
pub(super) fn event_item_count(t: &TestDb, event_id: i64, user: i64) -> i64 {
    t.read(|c| Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=? AND user_id=?",params![event_id,user],|r|r.get(0))?))
}

#[test]
fn cutover_event_end_must_follow_start() {
    let t = frozen();
    for delta in [Some(-3600), Some(0), None] {
        let mut a = attrs(&t);
        a.starts_at = Some(t.now().since(SignedDuration::from_hours(24)));
        a.ends_at = delta.map(|secs| a.starts_at.unwrap().since(SignedDuration::from_secs(secs)));
        assert_eq!(
            t.try_write(move |tx| CalendarEvent::create(tx, a)).is_ok(),
            delta.is_none()
        );
    }
}
#[test]
fn cutover_event_rejects_bot_and_nonmember_organizers() {
    let t = frozen();
    let outsider = t.write(|tx| {
        User::create(
            tx,
            NewUser {
                name: "Outsider".into(),
                email_address: Some("outsider@example.test".into()),
                password_digest: Some(PasswordDigest::create("secret123456", 4)?),
                ..Default::default()
            },
        )
    });
    for user in [id("bender"), outsider.id] {
        let mut a = attrs(&t);
        a.organizer_id = user;
        assert!(matches!(
            t.try_write(move |tx| CalendarEvent::create(tx, a)),
            Err(Error::RecordInvalid(ref errors)) if errors.on("organizer") == vec!["must be an active human member of the room"]
        ));
    }
    t.write(|tx| Membership::create_default(tx, id("designers"), id("bender")));
    let mut a = attrs(&t);
    a.organizer_id = id("bender");
    assert!(
        matches!(t.try_write(move |tx| CalendarEvent::create(tx, a)),
        Err(Error::RecordInvalid(ref errors)) if errors.on("organizer") == vec!["must be an active human member of the room"])
    );
}
#[test]
fn cutover_event_rejects_soft_deleted_venue() {
    let t = frozen();
    let room = t.write(|tx| {
        Room::create_for(
            tx,
            RoomType::Voice,
            Some("Lounge"),
            id("david"),
            &[id("david")],
        )
    });
    t.write(move |tx| {
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            params![tx.now(), room.id],
        )?;
        Ok(())
    });
    let mut a = attrs(&t);
    a.venue_room_id = Some(room.id);
    let result = t.try_write(move |tx| CalendarEvent::create(tx, a));
    assert!(matches!(&result, Err(Error::RecordInvalid(_))));
    let error = result.unwrap_err();
    let Error::RecordInvalid(errors) = error else {
        panic!("{error:?}")
    };
    assert_eq!(
        errors.on("venue"),
        vec!["must be a voice or Stage channel you belong to"]
    );
}
#[test]
fn cutover_event_off_and_invisible_members_get_no_invitation() {
    let t = frozen();
    involvement(&t, "jason", Involvement::Nothing);
    involvement(&t, "jz", Involvement::Invisible);
    let e = scheduled(&t);
    assert!(item(&t, e.id, "jason").is_none());
    assert!(item(&t, e.id, "jz").is_none());
    assert_eq!(
        item(&t, e.id, "kevin").unwrap().event_type,
        "event_invitation"
    );
}
#[test]
fn cutover_event_off_member_keeps_invitation_on_update_and_cancel() {
    let t = frozen();
    let e = scheduled(&t);
    respond(&t, e.id, "jason", "going", false);
    let original = item(&t, e.id, "jason").unwrap();
    involvement(&t, "jason", Involvement::Nothing);
    shift(&t, e.id);
    assert_eq!(
        item(&t, e.id, "jason").unwrap().event_type,
        "event_invitation"
    );
    cancel(&t, e.id);
    let retained = item(&t, e.id, "jason").unwrap();
    assert_eq!(retained.id, original.id);
    assert_eq!(retained.event_type, "event_invitation");
    assert_eq!(t.read(|c|Ok(c.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=? AND user_id=? AND event_type='event_cancelled'",params![e.id,id("jason")],|r|r.get::<_,i64>(0))?)),0);
}
#[test]
fn cutover_event_mentions_member_receives_update_and_cancel() {
    let t = frozen();
    let e = scheduled(&t);
    respond(&t, e.id, "jason", "going", false);
    involvement(&t, "jason", Involvement::Mentions);
    shift(&t, e.id);
    assert_eq!(item(&t, e.id, "jason").unwrap().event_type, "event_update");
    cancel(&t, e.id);
    assert_eq!(
        item(&t, e.id, "jason").unwrap().event_type,
        "event_cancelled"
    );
}
#[test]
fn cutover_event_invitations_exclude_bots() {
    let t = frozen();
    let mut a = attrs(&t);
    a.room_id = id("watercooler");
    a.starts_at = Some(t.now().since(SignedDuration::from_hours(24)));
    let e = t.write(move |tx| CalendarEvent::create(tx, a));
    assert_eq!(
        item(&t, e.id, "jason").unwrap().event_type,
        "event_invitation"
    );
    assert!(item(&t, e.id, "bender").is_none());
}
#[test]
fn cutover_event_time_change_notifies_going_and_maybe_once() {
    let t = frozen();
    let e = scheduled(&t);
    for (who, response) in [("jason", "going"), ("jz", "maybe"), ("kevin", "declined")] {
        respond(&t, e.id, who, response, false);
    }
    shift(&t, e.id);
    for who in ["jason", "jz"] {
        assert_eq!(event_item_count(&t, e.id, id(who)), 1);
        let i = item(&t, e.id, who).unwrap();
        assert_eq!(i.event_type, "event_update");
        assert!(i.unread());
    }
    assert_eq!(
        item(&t, e.id, "kevin").unwrap().event_type,
        "event_invitation"
    );
    assert!(item(&t, e.id, "david").is_none());
}
#[test]
fn cutover_event_title_only_edit_creates_no_items() {
    let t = frozen();
    let e = scheduled(&t);
    let event_count = || {
        t.read(|c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE source_type='Event' AND source_id=?",
                [e.id],
                |r| r.get::<_, i64>(0),
            )?)
        })
    };
    let before = event_count();
    let eid = e.id;
    t.write(move |tx| {
        CalendarEvent::update_with_scope(
            tx,
            eid,
            EventChanges {
                title: Some("Renamed".into()),
                ..Default::default()
            },
            "this_event",
            Some(id("david")),
        )
    });
    assert_eq!(event_count(), before);
    assert_eq!(t.read(|c| CalendarEvent::find(c, e.id)).title, "Renamed");
}
#[test]
fn cutover_event_cancel_notifies_going_maybe_and_handles_declined() {
    let t = frozen();
    let e = scheduled(&t);
    for (who, response) in [("jason", "going"), ("jz", "maybe"), ("kevin", "declined")] {
        respond(&t, e.id, who, response, false);
    }
    cancel(&t, e.id);
    assert!(t.read(|c| CalendarEvent::find(c, e.id)).cancelled());
    for who in ["jason", "jz"] {
        assert_eq!(event_item_count(&t, e.id, id(who)), 1);
        let i = item(&t, e.id, who).unwrap();
        assert_eq!(i.event_type, "event_cancelled");
        assert!(i.unread());
    }
    assert!(item(&t, e.id, "kevin").unwrap().handled());
}
#[test]
fn cutover_event_items_inaccessible_after_member_leaves() {
    let t = frozen();
    let e = scheduled(&t);
    let original = item(&t, e.id, "jason").unwrap();
    assert!(
        t.read(|c| ActivityItem::accessible_to(c, &User::find(c, id("jason"))?))
            .iter()
            .any(|i| i.id == original.id)
    );
    t.write(|tx| {
        Membership::find_by_room_and_user(tx.conn(), id("designers"), id("jason"))?
            .unwrap()
            .destroy(tx)
    });
    assert!(
        !t.read(|c| ActivityItem::accessible_to(c, &User::find(c, id("jason"))?))
            .iter()
            .any(|i| i.id == original.id)
    );
}
#[test]
fn cutover_event_room_deletion_removes_events_attendances_and_items() {
    let t = frozen();
    let e = scheduled(&t);
    let eid = e.id;
    respond(&t, eid, "jason", "going", false);
    let attendance_ids: Vec<i64> = t.read(|c| {
        crate::sql::query_all(
            c,
            "SELECT id FROM event_attendances WHERE event_id=?",
            [eid],
            |r| r.get(0),
        )
    });
    assert!(!attendance_ids.is_empty());
    let ids: Vec<i64> = t.read(|c| {
        Ok(
            c.prepare("SELECT id FROM activity_items WHERE source_type='Event' AND source_id=?")?
                .query_map([eid], |r| r.get(0))?
                .collect::<rusqlite::Result<_>>()?,
        )
    });
    assert!(!ids.is_empty());
    t.write(move |tx| {
        room_delete::begin_destroy(
            tx,
            &Room::find(tx.conn(), e.room_id)?,
            &HuddleConfig::default(),
        )
    });
    tokio::runtime::Runtime::new()
        .unwrap()
        .block_on(room_delete::perform_with_config(
            &t.db,
            e.room_id,
            HuddleConfig::default(),
        ))
        .unwrap();
    assert!(t.db.read_blocking(|c| CalendarEvent::find(c, eid)).is_err());
    assert_eq!(
        t.read(|c| Ok(c.query_row(
            "SELECT COUNT(*) FROM event_attendances WHERE event_id=?",
            [eid],
            |r| r.get::<_, i64>(0)
        )?)),
        0
    );
    for aid in attendance_ids {
        assert_eq!(
            t.read(|c| Ok(c.query_row(
                "SELECT COUNT(*) FROM event_attendances WHERE id=?",
                [aid],
                |r| r.get::<_, i64>(0)
            )?)),
            0
        );
    }
    for iid in ids {
        assert!(t.db.read_blocking(|c| ActivityItem::find(c, iid)).is_err());
    }
}
