//! Replies, edits and the columns around them: `test/models/message_test.rb`'s edit test, the
//! `edited_at` rule the edit endpoints apply (`body_content_will_change?`,
//! `test/controllers/messages_controller_test.rb`'s "(edited)" cases), `client_message_id`
//! dedup (`Message.find_duplicate`), and the `system_note`, `action` and `embeds_suppressed`
//! columns.

use super::channel_thread_test::{frozen, markdown, post_root};
use super::*;
use crate::{Error, Message, MessageChanges, NewMessage};

fn edit(t: &TestDb, message: &Message, changes: MessageChanges) -> Message {
    let mut message = message.clone();
    t.write(move |tx| {
        message.edit(tx, changes)?;
        Ok(message)
    })
}

fn source(text: &str) -> MessageChanges {
    MessageChanges { markdown_source: Some(text.into()), ..Default::default() }
}

fn body(html: &str) -> MessageChanges {
    MessageChanges { body: Some(html.into()), ..Default::default() }
}

/// "plain_text_body memoizes per instance and refreshes after edits"
#[test]
fn plain_text_body_refreshes_after_edits() {
    let t = frozen();
    let message = post_root(&t, "designers", "david", "original");
    assert_eq!(t.read(|c| message.plain_text_body(c, &BasicRichText)), "original");
    let edited = edit(&t, &message, source("edited"));
    assert_eq!(t.read(|c| edited.plain_text_body(c, &BasicRichText)), "edited");
    assert_eq!(t.read(|c| Message::search_in_room(c, id("designers"), "edited")).len(), 1, "reindexed after commit");
}

#[test]
fn a_markdown_edit_stamps_edited_at_and_touches_the_message_and_room() {
    let t = frozen();
    let message = post_root(&t, "designers", "david", "original");
    let room_before = t.read(|c| crate::Room::find(c, id("designers"))).updated_at;
    t.travel(60);
    let edited = edit(&t, &message, source("changed"));
    assert_eq!(edited.edited_at, Some(t.now()));
    assert_eq!(edited.updated_at, t.now());
    assert_eq!(edited.markdown_source.as_deref(), Some("changed"));
    assert!(t.read(|c| crate::Room::find(c, id("designers"))).updated_at > room_before);
}

#[test]
fn an_identical_save_neither_stamps_nor_touches() {
    let t = frozen();
    let message = post_root(&t, "designers", "david", "same");
    t.travel(60);
    let saved = edit(&t, &message, source("same"));
    assert_eq!(saved, message);
    let html = t.read(|c| message.body_html(c)).unwrap();
    assert_eq!(edit(&t, &message, body(&html)), message);
}

#[test]
fn a_legacy_body_edit_stamps_only_when_its_text_changes() {
    let t = frozen();
    let legacy = t.write(|tx| {
        Message::create(tx, NewMessage { room_id: id("designers"), creator_id: id("david"), body: Some("<div>hello</div>".into()), ..Default::default() })
    });
    t.travel(60);
    let changed = edit(&t, &legacy, body("<div>hello there</div>"));
    assert_eq!(changed.edited_at, Some(t.now()));

    // A blank body given to a message that had none (attachment-only) isn't a text change,
    // though the save goes through.
    let bare = t.write(|tx| Message::create(tx, NewMessage { room_id: id("designers"), creator_id: id("david"), ..Default::default() }));
    t.travel(60);
    let blank = edit(&t, &bare, body("<div></div>"));
    assert_eq!(blank.edited_at, None);
    assert_eq!(blank.updated_at, t.now());
}

#[test]
fn non_text_saves_never_stamp_edited_at() {
    let t = frozen();
    let message = post_root(&t, "designers", "david", "embeds https://example.com");
    t.travel(60);
    let mut suppressed = message.clone();
    let suppressed = t.write(move |tx| {
        suppressed.suppress_embeds(tx)?;
        Ok(suppressed)
    });
    assert!(suppressed.embeds_suppressed);
    assert_eq!(suppressed.edited_at, None);
    assert_eq!(suppressed.updated_at, t.now());
    // Suppressing twice is a no-op.
    t.travel(60);
    let mut again = suppressed.clone();
    let again = t.write(move |tx| {
        again.suppress_embeds(tx)?;
        Ok(again)
    });
    assert_eq!(again, suppressed);

    // `update` (not an edit endpoint) never stamps, even for a text change.
    let mut updated = message.clone();
    let updated = t.write(move |tx| {
        updated.update(tx, source("quietly changed"))?;
        Ok(updated)
    });
    assert_eq!(updated.edited_at, None);
}

#[test]
fn edits_are_validated_as_updates() {
    let t = frozen();
    let message = post_root(&t, "designers", "david", "original");
    let mut blank = message.clone();
    let Error::RecordInvalid(errors) = t.try_write(move |tx| blank.edit(tx, source("   "))).unwrap_err() else { panic!() };
    assert_eq!(errors.on("markdown_source"), vec!["can't be blank"]);

    let mut long = message.clone();
    let Error::RecordInvalid(errors) = t.try_write(move |tx| long.edit(tx, source(&"x".repeat(50_001)))).unwrap_err() else { panic!() };
    assert_eq!(errors.on("markdown_source"), vec!["is too long (maximum is 50000 characters)"]);

    // A forward whose source was deleted keeps `forwarded_at` alone, and still saves.
    let source_message = post_root(&t, "designers", "david", "source");
    let forward = t.write(move |tx| {
        Message::create(
            tx,
            NewMessage {
                forwarded_from_message_id: Some(source_message.id),
                forwarded_at: Some(tx.now()),
                forward_note: Some("note".into()),
                body: Some("<p>snapshot</p>".into()),
                room_id: id("watercooler"),
                creator_id: id("david"),
                ..Default::default()
            },
        )
    });
    let source_message = t.read(|c| Message::find(c, source_message.id));
    t.write(move |tx| source_message.destroy(tx));
    let mut orphan = t.read(|c| Message::find(c, forward.id));
    assert_eq!(orphan.forwarded_from_message_id, None);
    t.write(move |tx| orphan.update(tx, MessageChanges { forward_note: Some(Some("new note".into())), ..Default::default() }));
    assert_eq!(t.read(|c| Message::find(c, forward.id)).forward_note.as_deref(), Some("new note"));
}

#[test]
fn a_drive_edit_replaces_the_set_keeping_what_stays() {
    let t = frozen();
    let message = t.write(|tx| {
        Message::create(
            tx,
            NewMessage { drive_file_ids: vec!["1AbcDefGhIjKlMnOpQrSt".into(), "2BcdEfgHiJkLmNoPqRsTu".into()], ..markdown("designers", "david", "files") },
        )
    });
    let kept_row = t.read(|c| {
        crate::sql::count(c, r#"SELECT "id" FROM "drive_attachments" WHERE "file_id" = '2BcdEfgHiJkLmNoPqRsTu'"#, [])
    });
    let edited = edit(
        &t,
        &message,
        MessageChanges { drive_file_ids: Some(vec!["2BcdEfgHiJkLmNoPqRsTu".into(), "3CdeFghIjKlMnOpQrStUv".into()]), ..Default::default() },
    );
    assert_eq!(t.read(|c| edited.drive_file_ids(c)), vec!["2BcdEfgHiJkLmNoPqRsTu", "3CdeFghIjKlMnOpQrStUv"]);
    assert_eq!(
        t.read(|c| crate::sql::count(c, r#"SELECT "id" FROM "drive_attachments" WHERE "file_id" = '2BcdEfgHiJkLmNoPqRsTu'"#, [])),
        kept_row
    );
    assert_eq!(edited.edited_at, None, "files aren't text");

    let mut too_many = edited.clone();
    let ids: Vec<String> = (0..11).map(|n| format!("{n}AbcDefGhIjKlMnOpQrSt")).collect();
    let Error::RecordInvalid(errors) =
        t.try_write(move |tx| too_many.edit(tx, MessageChanges { drive_file_ids: Some(ids), ..Default::default() })).unwrap_err()
    else {
        panic!()
    };
    assert_eq!(errors.on("drive_attachments"), vec!["are limited to 10 per message"]);
}

#[test]
fn replies_store_their_target_and_notify_flag() {
    let t = frozen();
    let target = post_root(&t, "designers", "jason", "Question?");
    let reply = t.write(move |tx| {
        Message::create(tx, NewMessage { reply_to_message_id: Some(target.id), reply_notify_author: Some(false), ..markdown("designers", "david", "Answer") })
    });
    assert!(reply.reply());
    assert_eq!(reply.reply_to_message_id, Some(target.id));
    assert!(!reply.reply_notify_author);
    let default = t.write(move |tx| Message::create(tx, NewMessage { reply_to_message_id: Some(target.id), ..markdown("designers", "david", "Again") }));
    assert!(default.reply_notify_author, "the column defaults to notifying");

    let elsewhere = post_root(&t, "watercooler", "david", "Elsewhere");
    let cross_room = NewMessage { reply_to_message_id: Some(elsewhere.id), ..markdown("designers", "david", "Nope") };
    assert_eq!(t.read(|c| Message::validate(c, &cross_room)).on("reply_to_message"), vec!["must be in the same conversation"]);
}

#[test]
fn a_retried_create_finds_its_duplicate_by_client_message_id() {
    let t = frozen();
    let first = t.write(|tx| Message::create(tx, NewMessage { client_message_id: Some("retry-me".into()), ..markdown("designers", "david", "Once") }));
    let found = |room: &str, creator: &str, cmid: &str| t.read(|c| Message::find_duplicate(c, id(room), id(creator), cmid)).map(|m| m.id);
    assert_eq!(found("designers", "david", "retry-me"), Some(first.id));
    assert_eq!(found("designers", "jason", "retry-me"), None, "another author's id is theirs");
    assert_eq!(found("watercooler", "david", "retry-me"), None, "scoped to the room");
    assert_eq!(found("designers", "david", " "), None, "a blank id never matches");
}

#[test]
fn system_note_action_and_embeds_suppressed_are_stored() {
    let t = frozen();
    let message = t.write(|tx| {
        Message::create(tx, NewMessage { action: true, embeds_suppressed: true, system_note: true, ..markdown("designers", "david", "waves") })
    });
    assert!(message.action && message.embeds_suppressed && message.system_note);
    assert!(t.read(|c| Message::search_in_room(c, id("designers"), "waves")).is_empty(), "system notes aren't indexed");
}

#[test]
fn forward_metadata_comes_in_pairs_on_create() {
    let t = frozen();
    let source_message = post_root(&t, "designers", "david", "source");
    let half = NewMessage { forwarded_from_message_id: Some(source_message.id), ..markdown("designers", "david", "x") };
    assert_eq!(t.read(|c| Message::validate(c, &half)).on("forwarded_at"), vec!["must be present for a forwarded message"]);
    let other_half = NewMessage { forwarded_at: Some(t.now()), ..markdown("designers", "david", "x") };
    assert_eq!(t.read(|c| Message::validate(c, &other_half)).on("forwarded_from_message"), vec!["must be present for a forwarded message"]);
    let note = NewMessage { forward_note: Some("n".repeat(50_001)), ..markdown("designers", "david", "x") };
    assert_eq!(t.read(|c| Message::validate(c, &note)).on("forward_note"), vec!["is too long (maximum is 50000 characters)"]);
}
