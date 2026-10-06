//! `test/models/message_test.rb`, `message/searchable_test.rb`, `message/attachment_test.rb`
//! (the parts that are data, not image processing), and pagination.

use super::*;
use crate::models::active_storage::Blob;
use crate::models::message::PAGE_SIZE;
use crate::rich_text::mention_attachment_for;
use crate::{Message, NewMessage, Room, RoomType, Timeline, Timestamp};

fn create(t: &TestDb, room: &str, creator: &str, body: &str, client_message_id: &str) -> Message {
    let attributes = NewMessage {
        room_id: id(room),
        creator_id: id(creator),
        client_message_id: Some(client_message_id.into()),
        body: Some(body.into()),
        attachment_blob_id: None,
        ..Default::default()
    };
    t.write(move |tx| Message::create(tx, attributes))
}

fn search(t: &TestDb, room: &str, query: &str) -> Vec<i64> {
    let room_id = id(room);
    t.read(|c| Message::search_in_room(c, room_id, query))
        .into_iter()
        .map(|m| m.id)
        .collect()
}

#[test]
fn creating_a_message_enqueues_to_push_later() {
    let t = TestDb::new();
    let message = create(&t, "designers", "jason", "Hello", "123");
    let pushes: Vec<_> = t
        .events()
        .into_iter()
        .filter(|e| matches!(e, Event::PushMessage { .. }))
        .collect();
    assert_eq!(
        pushes,
        vec![Event::PushMessage {
            room_id: id("designers"),
            message_id: message.id
        }]
    );
}

#[test]
fn mentionees() {
    let t = TestDb::new();
    let rich_text = BasicRichText;
    let pets = id("pets");
    let mentioned = |html: String| {
        t.read(|c| {
            crate::models::message::mentionees_in_room(
                c,
                pets,
                &crate::RichText::mentioned_user_ids(&rich_text, c, &html),
            )
        })
    };

    let users = mentioned(format!(
        "<div>Hey {}</div>",
        mention_attachment_for(id("david"))
    ));
    assert_eq!(
        users.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![id("david")]
    );

    let users = mentioned(format!(
        "<div>Hey {} {}</div>",
        mention_attachment_for(id("david")),
        mention_attachment_for(id("david"))
    ));
    assert_eq!(
        users.iter().map(|u| u.id).collect::<Vec<_>>(),
        vec![id("david")]
    );

    // Kevin isn't in All Pets.
    assert!(
        mentioned(format!(
            "<div>Hey {}</div>",
            mention_attachment_for(id("kevin"))
        ))
        .is_empty()
    );
}

#[test]
fn message_body_is_indexed_and_searchable() {
    let t = TestDb::new();
    let mut message = create(
        &t,
        "designers",
        "david",
        "My hovercraft is full of eels",
        "earth",
    );
    assert_eq!(search(&t, "designers", "eel"), vec![message.id]);

    let m = message.clone();
    message = t.write(move |tx| {
        let mut m = m;
        m.update_body(tx, "My hovercraft is full of sharks")?;
        Ok(m)
    });
    assert_eq!(search(&t, "designers", "sharks"), vec![message.id]);

    t.write(move |tx| message.destroy(tx));
    assert!(search(&t, "designers", "sharks").is_empty());
}

#[test]
fn search_words_are_never_query_syntax() {
    let t = TestDb::new();
    let message = create(&t, "designers", "jason", "Do NOT feed the eel OR the shark", "c1");
    for query in ["NOT", "OR", "AND", "NEAR", "eel NOT", "NOT eel", "shark OR", "\"", "*", "", "a\0b", "\0"] {
        search(&t, "designers", query);
    }
    assert_eq!(search(&t, "designers", "NOT eel"), vec![message.id]);
    assert!(search(&t, "designers", "eel dolphin").is_empty());
    assert_eq!(search(&t, "designers", "eel\0shark"), vec![message.id]);
}

#[test]
fn search_results_are_returned_in_message_order() {
    let t = TestDb::new();
    let ids: Vec<i64> = ["first cat", "second cat", "third cat", "cat cat cat"]
        .into_iter()
        .map(|body| {
            t.travel(1);
            create(&t, "designers", "david", body, body).id
        })
        .collect();
    assert_eq!(search(&t, "designers", "cat"), ids);
}

#[test]
fn rich_text_body_is_converted_to_plain_text_for_indexing() {
    let t = TestDb::new();
    let message = create(
        &t,
        "designers",
        "david",
        "<span>My hovercraft is full of eels</span>",
        "earth",
    );
    assert!(search(&t, "designers", "span").is_empty());
    assert_eq!(search(&t, "designers", "eel"), vec![message.id]);
}

#[test]
fn search_reachable_only_finds_messages_in_the_users_rooms() {
    let t = TestDb::new();
    let message = create(&t, "designers", "david", "hovercraft", "earth");
    let found = t.read(|c| Message::search_reachable(c, id("kevin"), "hovercraft"));
    assert_eq!(
        found.iter().map(|m| m.id).collect::<Vec<_>>(),
        vec![message.id]
    );
    assert!(
        t.read(|c| Message::search_reachable(c, id("bender"), "hovercraft"))
            .is_empty()
    );
}

#[test]
fn creating_a_blank_message_with_attachment_uses_filename_as_plain_text_body() {
    let t = TestDb::new();
    let message = t.write(|tx| {
        let blob = Blob::create(
            tx,
            &Blob {
                id: 0,
                key: "abc123".into(),
                filename: "moon.jpg".into(),
                content_type: Some("image/jpeg".into()),
                metadata: Some(r#"{"identified":true}"#.into()),
                service_name: "local".into(),
                byte_size: 1,
                checksum: None,
                created_at: tx.now(),
            },
        )?;
        Message::create(
            tx,
            NewMessage {
                room_id: id("hq"),
                creator_id: id("david"),
                client_message_id: Some("message".into()),
                body: None,
                attachment_blob_id: Some(blob.id),
                ..Default::default()
            },
        )
    });
    assert_eq!(
        t.read(|c| message.plain_text_body(c, &BasicRichText)),
        "moon.jpg"
    );
    assert_eq!(
        t.read(|c| message.content_type(c, &BasicRichText)),
        crate::ContentType::Attachment
    );
    assert_eq!(search(&t, "hq", "moon"), vec![message.id]);

    t.write(move |tx| message.destroy(tx));
    assert!(
        t.events()
            .iter()
            .any(|e| matches!(e, Event::PurgeBlob { .. }))
    );
    assert_eq!(
        t.read(|c| crate::sql::count(c, "SELECT COUNT(*) FROM active_storage_attachments", [])),
        0
    );
}

#[test]
fn sound_messages() {
    let t = TestDb::new();
    let message = create(&t, "designers", "david", "/play trombone", "x");
    assert_eq!(
        t.read(|c| message.content_type(c, &BasicRichText)),
        crate::ContentType::Sound
    );
    assert_eq!(
        t.read(|c| message.sound(c, &BasicRichText)).unwrap().name,
        "trombone"
    );
    let message = create(&t, "designers", "david", "/play nosuchsound", "y");
    assert_eq!(
        t.read(|c| message.content_type(c, &BasicRichText)),
        crate::ContentType::Text
    );
}

#[test]
fn client_message_id_defaults_to_a_uuid() {
    let t = TestDb::new();
    let message = t.write(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("hq"),
                creator_id: id("bender"),
                body: Some("beep".into()),
                ..Default::default()
            },
        )
    });
    assert_eq!(message.client_message_id.len(), 36);
}

#[test]
fn pagination() {
    let t = TestDb::new();
    let watercooler = Timeline::Room(id("watercooler"));
    let all = t.read(|c| Message::last_page(c, watercooler));
    assert_eq!(all.len(), 10, "watercooler fixtures");
    assert!(all.windows(2).all(|w| w[0].created_at <= w[1].created_at));
    assert!(!t.read(|c| Message::paged(c, watercooler)));

    let middle = all[5].clone();
    let before = t.read(|c| Message::page_before(c, watercooler, &middle));
    let after = t.read(|c| Message::page_after(c, watercooler, &middle));
    assert_eq!(before, all[..5]);
    assert_eq!(after, all[6..]);
    let around = t.read(|c| Message::page_around(c, watercooler, &middle));
    assert_eq!(around, all);
    assert!(t.read(|c| Message::exists_before(c, watercooler, &middle)));
    assert!(!t.read(|c| Message::exists_after(c, watercooler, &all[9])));

    for n in 0..PAGE_SIZE {
        t.travel(1);
        create(
            &t,
            "watercooler",
            "david",
            &format!("message {n}"),
            &format!("c{n}"),
        );
    }
    assert!(t.read(|c| Message::paged(c, watercooler)));
    let last = t.read(|c| Message::last_page(c, watercooler));
    assert_eq!(last.len() as i64, PAGE_SIZE);
    assert_eq!(t.read(|c| Message::first_page(c, watercooler))[0], all[0]);

    let since: Timestamp = all[9].created_at;
    let created = t.read(|c| Message::page_created_since(c, watercooler, since));
    assert_eq!(created.len() as i64, PAGE_SIZE);
    let created_ids: Vec<i64> = created.iter().map(|m| m.id).collect();
    let updated = t.read(|c| Message::page_updated_since(c, watercooler, since, &created_ids));
    assert!(updated.iter().all(|m| !created_ids.contains(&m.id)));
}

/// A bare `channel_threads` row (`ChannelThread` isn't ported yet).
pub(super) fn create_thread(tx: &mut Tx<'_>, room_id: i64, creator_id: i64) -> Result<i64> {
    let now = tx.now();
    Ok(tx.conn().query_row(
        r#"INSERT INTO "channel_threads" ("created_at", "creator_id", "last_activity_at", "name", "room_id", "updated_at") VALUES (?, ?, ?, 'Thread', ?, ?) RETURNING "id""#,
        rusqlite::params![now, creator_id, now, room_id, now],
        |r| r.get(0),
    )?)
}

fn create_at_one_instant(t: &TestDb, room: &str, count: usize) -> Vec<Message> {
    t.clock.travel_to(t.now());
    let room_id = id(room);
    t.write(move |tx| {
        (0..count)
            .map(|_| Message::create(tx, NewMessage { room_id, creator_id: id("david"), ..Default::default() }))
            .collect()
    })
}

/// `Message::Pagination`'s `(created_at, id)` cursors: messages sharing the cursor's timestamp
/// are on the right side of it, and page edges inside a tie neither skip nor repeat.
#[test]
fn pagination_cursors_break_created_at_ties_by_id() {
    let t = TestDb::new();
    let tied = create_at_one_instant(&t, "watercooler", 3);
    assert!(tied.windows(2).all(|w| w[0].created_at == w[1].created_at));
    let room = Timeline::Room(id("watercooler"));

    let before = t.read(|c| Message::page_before(c, room, &tied[2]));
    assert_eq!(before[before.len() - 2..], tied[..2]);
    let after = t.read(|c| Message::page_after(c, room, &tied[0]));
    assert_eq!(after, tied[1..]);
    assert!(t.read(|c| Message::exists_after(c, room, &tied[1])));
    assert!(!t.read(|c| Message::exists_after(c, room, &tied[2])));
    assert!(t.read(|c| Message::exists_before(c, room, &tied[1])));
    let around = t.read(|c| Message::page_around(c, room, &tied[1]));
    assert_eq!(around.iter().filter(|m| tied.contains(m)).count(), 3, "no tied message lost or repeated");
}

/// Paging through a tie wider than a page visits every message exactly once, in `(created_at,
/// id)` order.
#[test]
fn paging_back_through_a_tie_wider_than_a_page_visits_each_message_once() {
    let t = TestDb::new();
    let room_id = id("watercooler");
    let tied = create_at_one_instant(&t, "watercooler", PAGE_SIZE as usize + 15);
    let room = Timeline::Room(room_id);

    let mut seen = t.read(|c| Message::last_page(c, room));
    assert_eq!(seen[..], tied[tied.len() - PAGE_SIZE as usize..]);
    loop {
        let page = t.read(|c| Message::page_before(c, room, &seen[0]));
        if page.is_empty() {
            break;
        }
        seen.splice(0..0, page);
    }
    let everything = t.read(|c| {
        crate::sql::query_all(
            c,
            r#"SELECT * FROM "messages" WHERE "room_id" = ? AND "thread_id" IS NULL ORDER BY "created_at", "id""#,
            [room_id],
            Message::from_row,
        )
    });
    assert_eq!(seen, everything);
}

/// `room.root_messages`: thread messages are on the thread's timeline, not the room's.
#[test]
fn room_timeline_is_the_root_messages() {
    let t = TestDb::new();
    let room_id = id("watercooler");
    let (thread_id, reply) = t.write(move |tx| {
        let thread_id = create_thread(tx, room_id, id("david"))?;
        let reply = Message::create(
            tx,
            NewMessage { room_id, creator_id: id("david"), thread_id: Some(thread_id), body: Some("in a thread".into()), ..Default::default() },
        )?;
        Ok((thread_id, reply))
    });
    let room = t.read(|c| Message::last_page(c, Timeline::Room(room_id)));
    assert!(!room.contains(&reply));
    assert_eq!(t.read(|c| Message::last_page(c, Timeline::Thread(thread_id))), vec![reply.clone()]);
    assert!(matches!(
        t.read(|c| Ok(Message::find_in(c, Timeline::Room(room_id), reply.id))),
        Err(crate::Error::RecordNotFound("Message"))
    ));
    assert_eq!(t.read(|c| Message::find_in(c, Timeline::Thread(thread_id), reply.id)), reply);
    // Thread replies are indexed but, until `ChannelThread#receive` is ported, don't mark the room unread or push.
    assert_eq!(search(&t, "watercooler", "thread"), vec![reply.id]);
    assert!(!t.events().iter().any(|e| matches!(e, Event::PushMessage { .. })));
}

/// The root timeline's newest page reads `index_messages_on_room_thread_created` in order: the
/// index ends with the rowid, so `ORDER BY created_at, id` needs no sort. (Upstream added
/// `index_messages_on_room_id_and_created_at` for this; our schema's index serves it instead.)
#[test]
fn root_timeline_pages_read_the_room_thread_created_index_without_sorting() {
    let t = TestDb::new();
    let room_id = id("watercooler");
    let plan = |sql: &str, values: Vec<rusqlite::types::Value>| -> String {
        t.read(|c| {
            let mut stmt = c.prepare(&format!("EXPLAIN QUERY PLAN {sql}"))?;
            let rows = stmt.query_map(rusqlite::params_from_iter(values), |r| r.get::<_, String>(3))?;
            Ok(rows.collect::<rusqlite::Result<Vec<_>>>()?.join("\n"))
        })
    };
    let last_page = plan(
        r#"SELECT "messages".* FROM "messages" WHERE "messages"."room_id" = ? AND "messages"."thread_id" IS NULL ORDER BY "messages"."created_at" DESC, "messages"."id" DESC LIMIT 40"#,
        vec![room_id.into()],
    );
    assert!(last_page.contains("index_messages_on_room_thread_created"), "{last_page}");
    assert!(!last_page.contains("TEMP B-TREE"), "{last_page}");
    let before = plan(
        r#"SELECT "messages".* FROM "messages" WHERE "messages"."room_id" = ? AND "messages"."thread_id" IS NULL AND ((messages.created_at, messages.id) < (?, ?)) ORDER BY "messages"."created_at" DESC, "messages"."id" DESC LIMIT 40"#,
        vec![room_id.into(), "2030-01-01 00:00:00.000000".to_string().into(), 1.into()],
    );
    assert!(before.contains("index_messages_on_room_thread_created"), "{before}");
    assert!(!before.contains("TEMP B-TREE"), "{before}");
}

/// A system note renders in the timeline but marks nobody unread, pushes nothing and stays out
/// of the search index (`Room#receive`, `Message::Searchable#create_in_index`).
#[test]
fn system_notes_are_quiet() {
    let t = TestDb::new();
    let room_id = id("designers");
    let note = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage { room_id, creator_id: id("david"), system_note: true, body: Some("renamed the group".into()), ..Default::default() },
        )
    });
    assert!(note.system_note);
    assert!(t.read(|c| Message::last_page(c, Timeline::Room(room_id))).contains(&note));
    assert!(t.events().is_empty(), "{:?}", t.events());
    assert!(search(&t, "designers", "renamed").is_empty());
    let unread: i64 = t.read(|c| {
        crate::sql::count(c, "SELECT COUNT(*) FROM memberships WHERE room_id = ? AND unread_at = ?", rusqlite::params![room_id, note.created_at])
    });
    assert_eq!(unread, 0);
}

/// A streaming message defers every side effect to its finalize: no index row, no unread, no
/// push; `touch_streaming_activity` stamps `streaming_updated_at`.
#[test]
fn streaming_messages_defer_their_side_effects() {
    let t = TestDb::new();
    let room_id = id("designers");
    let message = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage { room_id, creator_id: id("david"), streaming: true, body: Some("partial answer".into()), ..Default::default() },
        )
    });
    assert!(message.streaming);
    assert_eq!(message.streaming_updated_at, Some(message.created_at));
    assert!(t.events().is_empty(), "{:?}", t.events());
    assert!(search(&t, "designers", "partial").is_empty());
    let indexed: i64 = t.read(|c| crate::sql::count(c, "SELECT COUNT(*) FROM message_search_index WHERE rowid = ?", [message.id]));
    assert_eq!(indexed, 0);
}

/// `reachable_messages` goes through `user.rooms`, so a soft-deleted room's messages are
/// unreachable even while the user's membership row remains.
#[test]
fn messages_in_deleted_rooms_are_unreachable() {
    let t = TestDb::new();
    let message = t.read(|c| Message::find(c, id("first")));
    let creator = message.creator_id;
    assert!(t.read(|c| Ok(Message::find_reachable(c, creator, message.id).is_ok())));
    let room_id = message.room_id;
    t.write(move |tx| {
        tx.conn().execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", rusqlite::params![tx.now(), room_id])?;
        Ok(())
    });
    assert!(matches!(
        t.read(|c| Ok(Message::find_reachable(c, creator, message.id))),
        Err(crate::Error::RecordNotFound("Message"))
    ));
}

fn create_in(t: &TestDb, room_id: i64, thread_id: Option<i64>, system_note: bool) -> crate::Result<Message> {
    t.try_write(move |tx| {
        Message::create(
            tx,
            NewMessage { room_id, creator_id: id("david"), thread_id, system_note, body: Some("hi".into()), ..Default::default() },
        )
    })
}

fn full_messages(result: crate::Result<Message>) -> Vec<String> {
    match result {
        Err(crate::Error::RecordInvalid(errors)) => errors.full_messages(),
        other => panic!("expected RecordInvalid, got {other:?}"),
    }
}

/// `validate_conversation_links` (`app/models/message.rb`): a thread message must be in its
/// thread's room.
#[test]
fn thread_messages_stay_in_their_threads_room() {
    let t = TestDb::new();
    let thread = t.write(|tx| create_thread(tx, id("watercooler"), id("david")));
    let before = t.read(Message::count);

    let error = full_messages(create_in(&t, id("designers"), Some(thread), false));
    assert_eq!(error, ["Thread must belong to the message room and cannot be a direct room thread"]);
    assert_eq!(t.read(Message::count), before);

    assert!(create_in(&t, id("watercooler"), Some(thread), false).is_ok());
}

/// `validate_conversation_links`: direct rooms have no threads, even in their own room.
#[test]
fn direct_room_threads_take_no_messages() {
    let t = TestDb::new();
    let thread = t.write(|tx| create_thread(tx, id("david_and_kevin"), id("david")));
    let error = full_messages(create_in(&t, id("david_and_kevin"), Some(thread), false));
    assert_eq!(error, ["Thread must belong to the message room and cannot be a direct room thread"]);
}

/// `no_root_messages_in_boards`: a board's chat lives in its posts' threads; only quiet system
/// notes may sit at its root.
#[test]
fn boards_take_no_root_messages_except_system_notes() {
    let t = TestDb::new();
    let board = t.write(|tx| Room::create_for(tx, RoomType::Board, Some("Work"), id("david"), &[id("david")]));
    let before = t.read(Message::count);

    assert_eq!(full_messages(create_in(&t, board.id, None, false)), ["Thread must be present in a board"]);
    assert_eq!(t.read(Message::count), before);

    assert!(create_in(&t, board.id, None, true).is_ok());
    let thread = t.write(move |tx| create_thread(tx, board.id, id("david")));
    assert!(create_in(&t, board.id, Some(thread), false).is_ok());
}

/// `before_save :touch_streaming_activity, if: :streaming?`: every save of a streaming
/// message restarts the finalize sweep's clock; a `touch` (a boost) doesn't save.
#[test]
fn saving_a_streaming_message_restarts_its_activity_clock() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let room_id = id("designers");
    let started = t.now();
    let mut message = t.write(move |tx| {
        Message::create(tx, NewMessage { room_id, creator_id: id("david"), streaming: true, body: Some("partial".into()), ..Default::default() })
    });
    assert_eq!(message.streaming_updated_at, Some(started));

    t.travel(120);
    let edited = t.now();
    let mut m = message.clone();
    message = t.write(move |tx| m.update_body(tx, "partial answer").map(|_| m));
    assert_eq!(message.streaming_updated_at, Some(edited));
    assert_eq!(t.read(|c| Message::find(c, message.id)).streaming_updated_at, Some(edited));

    t.travel(60);
    let mut m = message.clone();
    t.write(move |tx| m.update_body(tx, "partial answer"));
    assert_eq!(t.read(|c| Message::find(c, message.id)).streaming_updated_at, Some(t.now()), "a save with the same body still saves");

    let touched_at = t.now();
    t.travel(60);
    let mut m = message.clone();
    t.write(move |tx| m.touch(tx));
    assert_eq!(t.read(|c| Message::find(c, message.id)).streaming_updated_at, Some(touched_at));

    // Finished messages keep no activity clock.
    let mut done = t.read(|c| Message::find(c, id("first")));
    t.write(move |tx| done.update_body(tx, "rewritten"));
    assert_eq!(t.read(|c| Message::find(c, id("first"))).streaming_updated_at, None);
}

/// `update!(attachment: nil)` on a streaming message with no attachment still saves (the
/// activity clock changes), and a saved change touches the room (`belongs_to :room, touch: true`).
#[test]
fn a_streaming_attachment_save_touches_the_room() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let room_id = id("designers");
    let message = t.write(move |tx| {
        Message::create(tx, NewMessage { room_id, creator_id: id("david"), streaming: true, body: Some("partial".into()), ..Default::default() })
    });
    t.travel(60);
    let mut m = message.clone();
    t.write(move |tx| m.replace_attachment(tx, None));
    let saved = t.read(|c| Message::find(c, message.id));
    assert_eq!(saved.updated_at, t.now());
    assert_eq!(saved.streaming_updated_at, Some(t.now()));
    assert_eq!(t.read(|c| Room::find(c, room_id)).updated_at, t.now());
}

fn create_thread_under(tx: &mut Tx<'_>, parent: &Message) -> Result<i64> {
    let thread_id = create_thread(tx, parent.room_id, parent.creator_id)?;
    tx.conn().execute(r#"UPDATE "channel_threads" SET "parent_message_id" = ? WHERE "id" = ?"#, rusqlite::params![parent.id, thread_id])?;
    Ok(thread_id)
}

fn messages_count(t: &TestDb, thread_id: i64) -> i64 {
    t.read(move |c| crate::sql::count(c, r#"SELECT "messages_count" FROM "channel_threads" WHERE "id" = ?"#, [thread_id]))
}

/// `after_create`/`after_destroy :refresh_thread_messages_count, if: :thread_reply?`
/// (`ChannelThread.refresh_messages_count`): the counter counts finished, non-note replies, and
/// the parent message is bumped so its indicator re-renders.
#[test]
fn thread_replies_refresh_the_thread_counter() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let parent = t.read(|c| Message::find(c, id("first")));
    let p = parent.clone();
    let thread = t.write(move |tx| create_thread_under(tx, &p));
    let room_id = parent.room_id;

    t.travel(10);
    let reply = create_in(&t, room_id, Some(thread), false).unwrap();
    assert_eq!(messages_count(&t, thread), 1);
    assert_eq!(t.read(|c| Message::find(c, parent.id)).updated_at, t.now());

    create_in(&t, room_id, Some(thread), true).unwrap();
    t.write(move |tx| Message::create(tx, NewMessage { room_id, creator_id: id("david"), thread_id: Some(thread), streaming: true, ..Default::default() }));
    assert_eq!(messages_count(&t, thread), 1, "notes and streams don't count");

    t.travel(10);
    t.write(move |tx| reply.destroy(tx));
    assert_eq!(messages_count(&t, thread), 0);
    assert_eq!(t.read(|c| Message::find(c, parent.id)).updated_at, t.now());
}

/// `destroy`'s data side (`app/models/message.rb`): `preserve_reply_tombstones` marks each reply
/// before the link is cut; `dependent: :nullify` board stale digests and `dependent: :destroy`
/// activity items and agent steps (none of which have destroy callbacks) go with it. The thread
/// it opened and its forwards are nulled by their foreign keys.
#[test]
fn destroying_a_message_tombstones_replies_and_clears_dependents() {
    let t = TestDb::new();
    t.clock.travel_to(t.now());
    let target = create(&t, "designers", "david", "original", "doomed");
    let reply = create(&t, "designers", "kevin", "answer", "reply");
    let (target_id, reply_id) = (target.id, reply.id);
    let thread = t.write(move |tx| {
        let now = tx.now();
        let c = tx.conn();
        c.execute(r#"UPDATE "messages" SET "reply_to_message_id" = ? WHERE "id" = ?"#, rusqlite::params![target_id, reply_id])?;
        c.execute(
            r#"INSERT INTO "activity_items" ("created_at", "event_type", "source_id", "source_type", "updated_at", "user_id") VALUES (?, 'mention', ?, 'Message', ?, ?)"#,
            rusqlite::params![now, target_id, now, id("kevin")],
        )?;
        c.execute(
            r#"INSERT INTO "agent_steps" ("agent_id", "created_at", "message_id", "name", "updated_at") VALUES (?, ?, ?, 'think', ?)"#,
            rusqlite::params![id("bender_agent"), now, target_id, now],
        )?;
        c.execute(
            r#"INSERT INTO "board_stale_digests" ("created_at", "digest_on", "message_id", "room_id", "updated_at") VALUES (?, '2026-09-29', ?, ?, ?)"#,
            rusqlite::params![now, target_id, id("designers"), now],
        )?;
        let thread = create_thread(tx, id("designers"), id("david"))?;
        tx.conn().execute(r#"UPDATE "channel_threads" SET "parent_message_id" = ? WHERE "id" = ?"#, rusqlite::params![target_id, thread])?;
        Ok(thread)
    });

    t.travel(5);
    t.write(move |tx| target.destroy(tx));

    let reply = t.read(move |c| Message::find(c, reply_id));
    assert_eq!((reply.reply_to_message_id, reply.reply_target_deleted_at), (None, Some(t.now())));
    assert_eq!(reply.updated_at, t.now());
    let remaining = |sql: &'static str| t.read(move |c| crate::sql::count(c, sql, [target_id]));
    assert_eq!(remaining(r#"SELECT COUNT(*) FROM "activity_items" WHERE "source_type" = 'Message' AND "source_id" = ?"#), 0);
    assert_eq!(remaining(r#"SELECT COUNT(*) FROM "agent_steps" WHERE "message_id" = ?"#), 0);
    assert_eq!(remaining(r#"SELECT COUNT(*) FROM "board_stale_digests" WHERE "message_id" = ?"#), 0);
    assert_eq!(t.read(|c| crate::sql::count(c, r#"SELECT COUNT(*) FROM "board_stale_digests""#, [])), 1);
    let parent: Option<i64> = t.read(move |c| crate::sql::query_one(c, r#"SELECT "parent_message_id" FROM "channel_threads" WHERE "id" = ?"#, [thread], |r| r.get(0))).flatten();
    assert_eq!(parent, None);
}
