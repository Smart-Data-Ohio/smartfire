//! `test/models/message_pin_test.rb`, plus the pin validations.

use super::channel_thread_test::{create_thread, frozen, post_reply, post_root};
use super::*;
use crate::broadcasts::{Broadcast, Partial, TurboAction};
use crate::models::message_pin::{CapReached, MAX_PER_ROOM, message_path};
use crate::{Error, Membership, Message, MessagePin, Room, User};

fn pin(t: &TestDb, message: &Message, pinner: &str) -> MessagePin {
    try_pin(t, message, pinner).unwrap()
}

fn try_pin(t: &TestDb, message: &Message, pinner: &str) -> std::result::Result<MessagePin, CapReached> {
    let (message, pinner) = (message.clone(), id(pinner));
    t.write(move |tx| MessagePin::pin(tx, &message, pinner))
}

fn unpin(t: &TestDb, pin: &MessagePin) {
    let pin = pin.clone();
    t.write(move |tx| pin.unpin(tx));
}

fn first(t: &TestDb) -> Message {
    t.read(|c| Message::find(c, id("first")))
}

fn room_message_count(t: &TestDb, room: &str) -> i64 {
    t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "messages" WHERE "room_id" = ?"#, [id(room)]))
}

fn pin_count(t: &TestDb) -> i64 {
    t.read(|c| MessagePin::count_for_room(c, id("designers")))
}

/// The broadcasts to `[designers, :messages]` since event `from`.
fn room_broadcasts(t: &TestDb, from: usize) -> Vec<Broadcast> {
    let stream = format!("{}:messages", rails_compat::global_id::GlobalId::new("Rooms::Closed", id("designers")).to_param());
    t.events()[from..].iter().filter_map(|e| e.as_broadcast()).filter(|b| b.stream_name() == stream).collect()
}

fn newest_root(t: &TestDb, room: &str) -> Message {
    t.read(|c| {
        crate::sql::query_one(
            c,
            r#"SELECT "id" FROM "messages" WHERE "room_id" = ? AND "thread_id" IS NULL ORDER BY "created_at" DESC, "id" DESC LIMIT 1"#,
            [id(room)],
            |r| r.get::<_, i64>(0),
        )?
        .map(|id| Message::find(c, id))
        .transpose()
    })
    .unwrap()
}

#[test]
fn pinning_records_the_pin_and_posts_a_channel_note_as_the_pinner() {
    let t = frozen();
    let message = first(&t);
    let before = room_message_count(&t, "designers");
    let pin = pin(&t, &message, "david");
    assert_eq!(pin_count(&t), 1);
    assert_eq!(room_message_count(&t, "designers"), before + 1);
    assert_eq!((pin.message_id, pin.room_id, pin.pinner_id), (message.id, id("designers"), id("david")));
    assert!(t.read(|c| MessagePin::pinned(c, message.id)));

    let note = newest_root(&t, "designers");
    assert_eq!(note.creator_id, id("david"));
    assert!(note.system_note);
    let path = format!("/rooms/{}/@{}", id("designers"), message.id);
    assert_eq!(note.markdown_source.as_deref(), Some(format!("pinned a message: [jump to message]({path})").as_str()));
    assert!(t.read(|c| note.plain_text_body(c, &BasicRichText)).contains("pinned a message"));
}

#[test]
fn a_thread_message_pin_links_into_its_thread() {
    let t = frozen();
    let thread = create_thread(&t, "designers", "jason", None, Some("Deep dive"));
    let reply = post_reply(&t, thread.id, "jason", "Threaded thought");
    assert_eq!(message_path(&reply), format!("/rooms/{}?message_id={}&thread={}", id("designers"), reply.id, thread.id));
    pin(&t, &reply, "david");
    let note = newest_root(&t, "designers");
    assert!(note.markdown_source.unwrap().ends_with(&format!("(/rooms/{}?message_id={}&thread={})", id("designers"), reply.id, thread.id)));
}

#[test]
fn pinning_broadcasts_the_badge_count_and_panel_list() {
    let t = frozen();
    let message = first(&t);
    let from = t.events().len();
    pin(&t, &message, "david");
    let broadcasts = room_broadcasts(&t, from);
    assert_eq!(broadcasts.len(), 4, "{broadcasts:#?}");
    let room_id = id("designers");
    let replaces: Vec<(String, Partial, bool)> = broadcasts
        .iter()
        .filter_map(|b| match b {
            Broadcast::Turbo(s) if s.action == TurboAction::Replace => Some((s.target.clone(), s.partial.clone().unwrap(), s.maintain_scroll)),
            _ => None,
        })
        .collect();
    assert_eq!(
        replaces,
        vec![
            ("pin_badge_message_0001".to_string(), Partial::PinBadge { message_id: message.id }, true),
            (format!("pins_count_rooms_closed_{room_id}"), Partial::PinsCount { room_id }, true),
            (format!("pins_list_rooms_closed_{room_id}"), Partial::PinsList { room_id }, true),
        ]
    );
    let note = newest_root(&t, "designers");
    // `note&.broadcast_create` runs after the transaction, so after the pin's commit callbacks.
    let Broadcast::Turbo(append) = &broadcasts[3] else { panic!() };
    assert_eq!(append.action, TurboAction::Append);
    assert_eq!(append.target, format!("messages_rooms_closed_{room_id}"));
    assert_eq!(append.partial, Some(Partial::Message { message_id: note.id }));
}

#[test]
fn pin_notes_are_quiet_system_notes() {
    let t = frozen();
    let message = first(&t);
    let unread = |t: &TestDb| t.read(|c| Membership::find_by_room_and_user(c, id("designers"), id("jason"))).unwrap().unread_at;
    let unread_before = unread(&t);
    let items_before = t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "activity_items""#, []));
    let from = t.events().len();
    pin(&t, &message, "david");
    let events = &t.events()[from..];
    assert!(!events.iter().any(|e| matches!(e, Event::PushMessage { .. } | Event::Job(_))), "{events:#?}");
    assert!(!events.iter().filter_map(|e| e.as_broadcast()).any(|b| b.stream_name().contains("unread")), "no unread broadcast");
    assert_eq!(unread(&t), unread_before);
    assert_eq!(t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "activity_items""#, [])), items_before);
    let note = newest_root(&t, "designers");
    assert!(note.system_note);
    assert!(!t.read(|c| Message::search_in_room(c, id("designers"), "pinned")).iter().any(|m| m.id == note.id));
}

/// "pin notes by agents record no delivery events": no job (agent events are WS11's).
#[test]
fn pin_notes_by_agents_enqueue_nothing() {
    let t = frozen();
    let message = first(&t);
    let from = t.events().len();
    pin(&t, &message, "bender");
    assert!(!t.events()[from..].iter().any(|e| matches!(e, Event::PushMessage { .. } | Event::Job(_))));
}

#[test]
fn repeated_pin_toggles_post_at_most_one_note_per_message_per_10_minutes() {
    let t = frozen();
    let message = first(&t);
    let before = room_message_count(&t, "designers");
    for _ in 0..5 {
        let pin = pin(&t, &message, "david");
        unpin(&t, &pin);
    }
    assert_eq!(room_message_count(&t, "designers"), before + 1);
    t.travel(11 * 60);
    pin(&t, &message, "david");
    assert_eq!(room_message_count(&t, "designers"), before + 2);
}

#[test]
fn unpinning_stamps_the_message_for_refresh_without_reordering_the_room() {
    let t = frozen();
    let message = first(&t);
    let pin = pin(&t, &message, "david");
    let room_updated_at = t.read(|c| Room::find(c, id("designers"))).updated_at;
    t.travel(60);
    unpin(&t, &pin);
    let room = t.read(|c| Room::find(c, id("designers")));
    assert_eq!(room.updated_at, room_updated_at);
    assert_eq!(room.pins_changed_at, Some(t.now()));
    assert_eq!(t.read(|c| Message::find(c, message.id)).updated_at, t.now());
}

#[test]
fn re_pinning_returns_the_existing_pin_without_posting_another_note() {
    let t = frozen();
    let message = first(&t);
    let pin = pin(&t, &message, "david");
    let before = room_message_count(&t, "designers");
    assert_eq!(self::pin(&t, &message, "jason"), pin);
    assert_eq!(pin_count(&t), 1);
    assert_eq!(room_message_count(&t, "designers"), before);
}

fn fill_room(t: &TestDb, prefix: &str) {
    t.write({
        let prefix = prefix.to_string();
        move |tx| {
            for n in 0..MAX_PER_ROOM {
                let message = Message::create(
                    tx,
                    crate::NewMessage {
                        client_message_id: Some(format!("{prefix}-{n}")),
                        ..super::channel_thread_test::markdown("designers", "david", &format!("Pinnable {n}"))
                    },
                )?;
                MessagePin::pin(tx, &message, message.creator_id)?.map_err(|e| Error::Other(e.0))?;
            }
            Ok(())
        }
    });
    assert_eq!(pin_count(t), MAX_PER_ROOM);
}

#[test]
fn re_pinning_in_a_full_room_returns_success() {
    let t = frozen();
    fill_room(&t, "repin-cap");
    let repinned = t.read(|c| Message::find_duplicate(c, id("designers"), id("david"), "repin-cap-0")).unwrap();
    let pin = pin(&t, &repinned, "david");
    assert_eq!(pin.message_id, repinned.id);
    assert_eq!(pin_count(&t), MAX_PER_ROOM);
}

#[test]
fn pins_cap_at_50_per_room() {
    let t = frozen();
    fill_room(&t, "cap");
    let extra = post_root(&t, "designers", "david", "One too many");
    let before = room_message_count(&t, "designers");
    let from = t.events().len();
    assert_eq!(try_pin(&t, &extra, "david"), Err(CapReached("This channel already has 50 pinned messages".into())));
    assert_eq!(pin_count(&t), MAX_PER_ROOM);
    assert_eq!(room_message_count(&t, "designers"), before);
    assert!(room_broadcasts(&t, from).is_empty());
}

#[test]
fn pin_change_broadcasts_are_skipped_when_the_pin_transaction_rolls_back() {
    let t = frozen();
    let message = first(&t);
    let from = t.events().len();
    let result: Result<()> = t.try_write(move |tx| {
        MessagePin::pin(tx, &message, id("david"))?.unwrap();
        Err(Error::Other("rollback".into()))
    });
    assert!(result.is_err());
    assert!(!t.read(|c| MessagePin::pinned(c, id("first"))));
    assert!(room_broadcasts(&t, from).is_empty());
    assert_eq!(t.read(|c| Room::find(c, id("designers"))).pins_changed_at, None, "no stamp either");
}

#[test]
fn unpin_change_broadcasts_are_skipped_when_the_unpin_transaction_rolls_back() {
    let t = frozen();
    let pin = pin(&t, &first(&t), "david");
    let from = t.events().len();
    let result: Result<()> = t.try_write(move |tx| {
        pin.unpin(tx)?;
        Err(Error::Other("rollback".into()))
    });
    assert!(result.is_err());
    assert!(room_broadcasts(&t, from).is_empty());
    assert!(t.read(|c| MessagePin::pinned(c, id("first"))));
}

#[test]
fn deactivating_a_user_keeps_their_pins_attributed() {
    let t = frozen();
    let message = first(&t);
    pin(&t, &message, "kevin");
    t.write(|tx| User::find(tx.conn(), id("kevin"))?.deactivate(tx));
    assert_eq!(t.read(|c| MessagePin::find_by_message(c, message.id)).unwrap().pinner_id, id("kevin"));
}

#[test]
fn unpinning_removes_the_pin_and_broadcasts_without_a_note() {
    let t = frozen();
    let message = first(&t);
    let pin = pin(&t, &message, "david");
    let before = room_message_count(&t, "designers");
    let from = t.events().len();
    unpin(&t, &pin);
    let broadcasts = room_broadcasts(&t, from);
    assert_eq!(broadcasts.len(), 3);
    assert!(broadcasts.iter().all(|b| matches!(b, Broadcast::Turbo(s) if s.action == TurboAction::Replace && s.maintain_scroll)));
    assert_eq!(room_message_count(&t, "designers"), before);
    assert_eq!(pin_count(&t), 0);
    assert!(!t.read(|c| MessagePin::pinned(c, message.id)));
}

#[test]
fn pins_order_newest_first() {
    let t = frozen();
    let first_pin = pin(&t, &first(&t), "david");
    t.travel(1);
    let second_pin = pin(&t, &t.read(|c| Message::find(c, id("second"))), "david");
    assert_eq!(t.read(|c| MessagePin::ordered_for_room(c, id("designers"))), vec![second_pin, first_pin]);
}

#[test]
fn destroying_the_message_destroys_its_pin() {
    let t = frozen();
    let message = first(&t);
    pin(&t, &message, "david");
    let from = t.events().len();
    t.travel(60);
    t.write(move |tx| message.destroy(tx));
    assert_eq!(t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "message_pins""#, [])), 0);
    assert_eq!(room_broadcasts(&t, from).iter().filter(|b| b.target().is_some_and(|t| t.starts_with("pin"))).count(), 3);
    assert_eq!(t.read(|c| Room::find(c, id("designers"))).pins_changed_at, Some(t.now()));
}

#[test]
fn pins_validate_their_message_room_and_uniqueness() {
    let t = frozen();
    let message = first(&t);
    let errors = t.read(|c| MessagePin::validate(c, message.id, id("watercooler"), id("david")));
    assert_eq!(errors.on("room"), vec!["must be the message's room"]);
    let errors = t.read(|c| MessagePin::validate(c, message.id, id("designers"), 0));
    assert_eq!(errors.on("pinner"), vec!["must exist"]);
    pin(&t, &message, "david");
    let Error::RecordInvalid(errors) = t
        .try_write(move |tx| MessagePin::create(tx, &message, message.room_id, id("jason")))
        .unwrap_err()
    else {
        panic!()
    };
    assert_eq!(errors.on("message_id"), vec!["has already been taken"]);
}
