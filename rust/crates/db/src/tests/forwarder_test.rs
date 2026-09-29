//! `test/services/messages/forwarder_test.rb`

use std::sync::{Arc, Mutex};

use super::channel_thread_test::{create_thread, frozen};
use super::*;
use crate::models::forwarder::{self, BlobCopier, Destination, Forwarded, Refusal};
use crate::sql::count;
use crate::{Blob, Error, Message, NewMessage, Room, RoomType, ThreadMembership, Tx};

/// Stands in for `campfire_storage`'s upload: each copy is a new blob row under a new key, and
/// the copier remembers what it made and what it was told to discard. `fail_on` makes the nth
/// copy fail, as a storage error would.
#[derive(Clone, Default)]
struct Copier {
    fail_on: Option<usize>,
    made: Arc<Mutex<Vec<String>>>,
    discarded: Arc<Mutex<Vec<String>>>,
}

impl BlobCopier for Copier {
    fn copy(&self, tx: &Tx<'_>, blob: &Blob) -> Result<Blob> {
        let mut made = self.made.lock().unwrap();
        if self.fail_on == Some(made.len() + 1) {
            return Err(Error::Other("storage is down".into()));
        }
        let copy = Blob::create(tx, &Blob { key: crate::sql::uuid(), ..blob.clone() })?;
        made.push(copy.key.clone());
        Ok(copy)
    }

    fn discard(&self, blobs: &[Blob]) {
        self.discarded.lock().unwrap().extend(blobs.iter().map(|b| b.key.clone()));
    }
}

/// `setup`: a Markdown source with an attachment in designers.
fn source(t: &TestDb) -> Message {
    t.write(|tx| {
        let blob = Blob::create(
            tx,
            &Blob {
                id: 0,
                key: "forward-source-key".into(),
                filename: "source.txt".into(),
                content_type: Some("text/plain".into()),
                metadata: Some(r#"{"identified":true}"#.into()),
                service_name: "local".into(),
                byte_size: 17,
                checksum: Some("c2hvdWxkIGJlIG1kNQ==".into()),
                created_at: tx.now(),
            },
        )?;
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                markdown_source: Some("# Heading\n\n**Formatted** @[Jason]".into()),
                attachment_blob_id: Some(blob.id),
                client_message_id: Some("forward-source".into()),
                ..Default::default()
            },
        )
    })
}

fn forward(
    t: &TestDb,
    source: &Message,
    destinations: Vec<Destination>,
    note: Option<&'static str>,
    copier: &Copier,
) -> Result<std::result::Result<Vec<Forwarded>, Refusal>> {
    let (source, copier) = (source.clone(), copier.clone());
    t.try_write(move |tx| forwarder::forward(tx, &source, &destinations, note, id("david"), &copier))
}

fn forward_one(t: &TestDb, source: &Message, room: &str, note: Option<&'static str>) -> Message {
    let forwarded = forward(t, source, vec![Destination::room(id(room))], note, &Copier::default()).unwrap().unwrap();
    assert_eq!(forwarded.len(), 1);
    forwarded.into_iter().next().unwrap().message
}

#[test]
fn copies_a_body_and_attachment_snapshot_that_survives_source_edits_and_deletion() {
    let t = frozen();
    let source = source(&t);
    let source_html = t.read(|c| source.body_html(c)).unwrap();
    let copier = Copier::default();
    let forwarded = forward(&t, &source, vec![Destination::room(id("watercooler"))], Some("Context"), &copier).unwrap().unwrap();
    let forwarded = forwarded.into_iter().next().unwrap().message;

    assert!(forwarded.forwarded());
    assert!(!forwarded.markdown());
    assert_eq!(t.read(|c| forwarded.body_html(c)).unwrap(), source_html);
    let (_, copy) = t.read(|c| forwarded.attachment(c)).unwrap();
    let (_, original) = t.read(|c| source.attachment(c)).unwrap();
    assert_ne!(copy.id, original.id);
    assert_eq!((copy.filename.as_str(), copy.byte_size, &copy.metadata), ("source.txt", 17, &original.metadata));
    assert_eq!(*copier.made.lock().unwrap(), vec![copy.key.clone()]);

    let mut edited = source.clone();
    t.write(move |tx| {
        edited.update(tx, crate::MessageChanges { markdown_source: Some("Changed source".into()), ..Default::default() })?;
        edited.destroy(tx)
    });

    let forwarded = t.read(|c| Message::find(c, forwarded.id));
    assert_eq!(forwarded.forwarded_from_message_id, None);
    assert!(forwarded.forwarded(), "forwarded_at outlives the source");
    assert_eq!(t.read(|c| forwarded.body_html(c)).unwrap(), source_html);
    assert_eq!(t.read(|c| forwarded.attachment(c)).unwrap().1.key, copy.key);
    assert!(t.read(|c| forwarded.plain_text_body(c, &BasicRichText)).starts_with("Context\n\n"));
}

#[test]
fn copies_drive_attachment_ids_onto_room_and_thread_forwards() {
    let t = frozen();
    let source = source(&t);
    let source_id = source.id;
    t.write(move |tx| {
        for file_id in ["1AbcDefGhIjKlMnOpQrSt", "2BcdEfgHiJkLmNoPqRsTu"] {
            tx.conn().execute(
                r#"INSERT INTO "drive_attachments" ("created_at", "file_id", "message_id") VALUES (?, ?, ?)"#,
                rusqlite::params![tx.now(), file_id, source_id],
            )?;
        }
        Ok(())
    });
    let thread = create_thread(&t, "designers", "david", None, Some("Forward attachments"));

    let results = forward(
        &t,
        &source,
        vec![Destination::room(id("watercooler")), Destination::thread(id("designers"), thread.id)],
        None,
        &Copier::default(),
    )
    .unwrap()
    .unwrap();

    assert_eq!(results.len(), 2);
    for result in &results {
        assert_eq!(t.read(|c| result.message.drive_file_ids(c)), vec!["1AbcDefGhIjKlMnOpQrSt", "2BcdEfgHiJkLmNoPqRsTu"]);
    }
    let in_thread = &results[1];
    assert_eq!(in_thread.message.thread_id, Some(thread.id));
    assert_eq!(in_thread.thread.as_ref().unwrap().messages_count, 1);
    assert!(t.read(|c| ThreadMembership::find_by_thread_and_user(c, thread.id, id("david"))).is_some());
}

#[test]
fn a_later_thread_destination_failure_rolls_back_prior_messages_and_cloned_blobs() {
    let t = frozen();
    let source = source(&t);
    let thread = create_thread(&t, "designers", "jz", None, Some("Forward destination"));
    let counts = |t: &TestDb| {
        t.read(|c| {
            Ok((
                count(c, "SELECT COUNT(*) FROM messages", [])?,
                count(c, "SELECT COUNT(*) FROM thread_memberships", [])?,
                count(c, "SELECT COUNT(*) FROM active_storage_blobs", [])?,
            ))
        })
    };
    let before = counts(&t);
    let events_before = t.events().len();

    let copier = Copier { fail_on: Some(2), ..Default::default() };
    let error = forward(
        &t,
        &source,
        vec![Destination::room(id("watercooler")), Destination::thread(id("designers"), thread.id)],
        None,
        &copier,
    )
    .unwrap_err();

    assert_eq!(error.to_string(), "storage is down");
    assert_eq!(counts(&t), before);
    let made = copier.made.lock().unwrap().clone();
    assert_eq!(made.len(), 1, "the first destination's copy was made");
    assert_eq!(*copier.discarded.lock().unwrap(), made, "and then discarded");
    assert_eq!(t.events().len(), events_before, "nothing is announced for a rolled-back forward");
}

#[test]
fn marks_forwards_of_markdown_sources_as_markdown_without_making_them_markdown_records() {
    let t = frozen();
    let source = source(&t);
    let forwarded = forward_one(&t, &source, "watercooler", None);
    assert!(source.markdown());
    assert!(forwarded.forwarded_markdown);
    assert!(!forwarded.markdown());
}

#[test]
fn leaves_forwards_of_legacy_sources_on_the_legacy_path() {
    let t = frozen();
    let legacy = t.write(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                body: Some("<div>legacy source</div>".into()),
                client_message_id: Some("legacy-forward-source".into()),
                ..Default::default()
            },
        )
    });
    let forwarded = forward_one(&t, &legacy, "watercooler", None);
    assert!(!forwarded.forwarded_markdown);
    assert_eq!(t.read(|c| forwarded.body_html(c)).unwrap(), "<div>legacy source</div>");
}

#[test]
fn forwarding_a_markdown_forward_keeps_the_markdown_flag() {
    let t = frozen();
    let source = source(&t);
    let first = forward_one(&t, &source, "watercooler", Some("First note\n<b>bold</b>"));
    let second = forward_one(&t, &first, "designers", None);
    assert!(second.forwarded_markdown);
    // The inherited note leads the snapshot, escaped, its newlines as breaks.
    let first_html = t.read(|c| first.body_html(c)).unwrap();
    assert_eq!(
        t.read(|c| second.body_html(c)).unwrap(),
        format!("<p>First note<br>&lt;b&gt;bold&lt;/b&gt;</p>{first_html}")
    );
}

#[test]
fn refuses_board_rooms_as_destinations() {
    let t = frozen();
    let source = source(&t);
    let david = id("david");
    let (board, post) = t.write(move |tx| {
        let board = Room::create_for(tx, RoomType::Board, Some("Launch"), david, &[david])?;
        let post = crate::ChannelThread::create(
            tx,
            crate::NewChannelThread {
                room_id: board.id,
                creator_id: david,
                name: Some("Ship it".into()),
                work_status: Some("planned".into()),
                ..Default::default()
            },
        )?;
        Ok((board, post))
    });
    let refused = |destination| forward(&t, &source, vec![destination], None, &Copier::default()).unwrap().unwrap_err();
    assert_eq!(refused(Destination::room(board.id)), Refusal::InvalidDestination("You cannot forward to a board".into()));
    assert_eq!(refused(Destination::thread(board.id, post.id)), Refusal::InvalidDestination("You cannot forward to a board".into()));
}

#[test]
fn copied_body_mentions_do_not_notify_while_a_new_forward_note_can() {
    let t = frozen();
    let source = source(&t);
    assert_eq!(t.read(|c| source.mentionees(c, &BasicRichText)).iter().map(|u| u.id).collect::<Vec<_>>(), vec![id("jason")]);
    let mut forwarded = forward_one(&t, &source, "watercooler", Some("@[Jason] please review"));
    assert_eq!(t.read(|c| forwarded.mentionees(c, &BasicRichText)).iter().map(|u| u.id).collect::<Vec<_>>(), vec![id("jason")]);

    let forwarded = t.write(move |tx| {
        forwarded.update(tx, crate::MessageChanges { forward_note: Some(None), ..Default::default() })?;
        Ok(forwarded)
    });
    assert!(t.read(|c| forwarded.mentionees(c, &BasicRichText)).is_empty());
    let silent = forward_one(&t, &source, "watercooler", None);
    assert!(t.read(|c| silent.mentionees(c, &BasicRichText)).is_empty(), "the snapshot's own mention never notifies");
}

// The rest of `normalize_destinations`.

#[test]
fn refuses_empty_excess_foreign_duplicate_and_unavailable_destinations() {
    let t = frozen();
    let source = source(&t);
    let refused = |destinations: Vec<Destination>| forward(&t, &source, destinations, None, &Copier::default()).unwrap().unwrap_err();
    let invalid = |message: &str| Refusal::InvalidDestination(message.into());

    let too_many = Refusal::TooManyDestinations("Choose between 1 and 5 destinations".into());
    assert_eq!(refused(vec![]), too_many);
    assert_eq!(refused(vec![Destination::room(id("designers")); 6]), too_many);
    assert_eq!(refused(vec![Destination { room_id: None, thread_id: None }]), invalid("A destination room is required"));
    // David isn't in bender_and_kevin.
    assert_eq!(refused(vec![Destination::room(id("bender_and_kevin"))]), invalid("You cannot forward to that room"));
    assert_eq!(
        refused(vec![Destination::room(id("designers")), Destination::room(id("designers"))]),
        invalid("Destinations must be unique")
    );
    let elsewhere = create_thread(&t, "watercooler", "david", None, Some("Elsewhere"));
    assert_eq!(refused(vec![Destination::thread(id("designers"), elsewhere.id)]), invalid("That thread is unavailable"));
    let locked = create_thread(&t, "designers", "david", None, Some("Locked"));
    let locked_id = locked.id;
    t.write(move |tx| crate::ChannelThread::find(tx.conn(), locked_id)?.lock_conversation(tx));
    assert_eq!(refused(vec![Destination::thread(id("designers"), locked.id)]), invalid("That thread is locked"));
    // A room and one of its threads are different destinations.
    let open = create_thread(&t, "designers", "david", None, Some("Open"));
    let both = forward(&t, &source, vec![Destination::room(id("designers")), Destination::thread(id("designers"), open.id)], None, &Copier::default());
    assert_eq!(both.unwrap().unwrap().len(), 2);
}
