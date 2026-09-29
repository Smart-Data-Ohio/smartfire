//! Which timestamps each `Message` save path advances, against `message_save_touches.json`:
//! the reference app's answer for the same cases, from `ruby/save_touches.rb` (regenerated and
//! compared by `reference-tools/db/differential.sh`). Each case gets its own room and message at
//! T0, runs one action at T0 + 60s, and records how far past T0 the message's `updated_at` and
//! `streaming_updated_at` (none once destroyed) and the room's `updated_at` ended up.
//!
//! In Rails a message save touches its room (`belongs_to :room, touch: true`) whenever it saves
//! a change, and a streaming message always does (`before_save :touch_streaming_activity`).

use std::collections::BTreeMap;
use std::sync::atomic::{AtomicUsize, Ordering};

use serde::Deserialize;

use super::*;
use crate::models::active_storage::Blob;
use crate::{Boost, Message, MessageChanges, MessagePin, NewMessage, NewSavedItem, Room, RoomType, SavedItem, Timestamp};

const RUBY: &str = include_str!("message_save_touches.json");
const ACTIONS: [&str; 15] = [
    "attach_nil",
    "attach_blob",
    "body_same",
    "body_new",
    "touch",
    "boost_create",
    "boost_destroy",
    "destroy",
    "markdown_new",
    "embeds_suppress",
    "forward_note",
    "drive_add",
    "pin",
    "unpin",
    "save_item",
];

#[derive(Debug, PartialEq, Deserialize)]
struct Outcome {
    message_updated_at: Option<i64>,
    streaming_updated_at: Option<i64>,
    room_updated_at: Option<i64>,
    attached: bool,
}

fn blob(tx: &Tx<'_>, name: &str) -> Result<i64> {
    static KEYS: AtomicUsize = AtomicUsize::new(0);
    let blob = Blob {
        id: 0,
        key: format!("key-{name}-{}", KEYS.fetch_add(1, Ordering::Relaxed)),
        filename: format!("{name}.txt"),
        content_type: Some("text/plain".into()),
        metadata: Some("{}".into()),
        service_name: "local".into(),
        byte_size: name.len() as i64,
        checksum: None,
        created_at: tx.now(),
    };
    Ok(Blob::create(tx, &blob)?.id)
}

fn act(tx: &mut Tx<'_>, message_id: i64, action: &str) -> Result<()> {
    let mut message = Message::find(tx.conn(), message_id)?;
    match action {
        "attach_nil" => message.replace_attachment(tx, None),
        "attach_blob" => {
            let blob_id = blob(tx, "new")?;
            message.replace_attachment(tx, Some(blob_id))
        }
        "body_same" => message.update_body(tx, "partial"),
        "body_new" => message.update_body(tx, "rewritten"),
        "touch" => message.touch(tx),
        "boost_create" => Boost::create(tx, message_id, id("jason"), "hi").map(|_| ()),
        "boost_destroy" => message.boosts(tx.conn())?[0].destroy(tx),
        "destroy" => message.destroy(tx),
        "markdown_new" => message.update(tx, MessageChanges { markdown_source: Some("rewritten".into()), ..Default::default() }),
        "embeds_suppress" => message.update(tx, MessageChanges { embeds_suppressed: Some(true), ..Default::default() }),
        "forward_note" => message.update(tx, MessageChanges { forward_note: Some(Some("note".into())), ..Default::default() }),
        "drive_add" => {
            let ids = vec!["1AbcDefGhIjKlMnOpQrSt".to_string()];
            message.update(tx, MessageChanges { drive_file_ids: Some(ids), ..Default::default() })
        }
        "pin" => MessagePin::pin(tx, &message, id("jason"))?.map(drop).map_err(|e| crate::Error::Other(e.0)),
        "unpin" => MessagePin::find_by_message(tx.conn(), message.id)?.expect("pinned in setup").unpin(tx),
        "save_item" => {
            let remind_at = Some(tx.now().since(jiff::SignedDuration::from_hours(1)));
            SavedItem::create(tx, NewSavedItem { user_id: id("jason"), message_id: message.id, remind_at, status: None }).map(drop)
        }
        other => unreachable!("{other}"),
    }
}

fn run_case(t: &TestDb, t0: Timestamp, streaming: bool, attached: bool, action: &'static str) -> Outcome {
    let name = format!(
        "{}/{}/{action}",
        if streaming { "streaming" } else { "finished" },
        if attached { "attached" } else { "bare" }
    );
    t.clock.travel_to(t0);
    let room_name = name.clone();
    let (room_id, message_id) = t.write(move |tx| {
        let room = Room::create(tx, RoomType::Open, Some(&room_name), id("david"))?;
        let attachment_blob_id = if attached { Some(blob(tx, "old")?) } else { None };
        let message = Message::create(
            tx,
            NewMessage {
                room_id: room.id,
                creator_id: id("david"),
                client_message_id: Some(room_name.clone()),
                body: Some("partial".into()),
                streaming,
                attachment_blob_id,
                ..Default::default()
            },
        )?;
        if action == "boost_destroy" {
            Boost::create(tx, message.id, id("jason"), "yo")?;
        }
        if action == "unpin" {
            MessagePin::pin(tx, &message, id("jason"))?.map_err(|e| crate::Error::Other(e.0))?;
        }
        Ok((room.id, message.id))
    });
    t.travel(60);
    t.write(move |tx| act(tx, message_id, action));
    let offset = |time: Timestamp| (time.as_microsecond() - t0.as_microsecond()) / 1_000_000;
    let message = t.read(|c| Message::find_by_id(c, message_id));
    Outcome {
        message_updated_at: message.as_ref().map(|m| offset(m.updated_at)),
        streaming_updated_at: message.as_ref().and_then(|m| m.streaming_updated_at).map(offset),
        room_updated_at: Some(offset(t.read(|c| Room::find(c, room_id)).updated_at)),
        attached: message.is_some_and(|m| t.read(|c| m.attachment(c)).is_some()),
    }
}

#[test]
fn message_saves_touch_what_rails_touches() {
    let ruby: BTreeMap<String, Outcome> = serde_json::from_str(RUBY).unwrap();
    let t = TestDb::new();
    let t0 = t.now();
    let mut rust = BTreeMap::new();
    for streaming in [false, true] {
        for attached in [false, true] {
            for action in ACTIONS {
                let name = format!(
                    "{}/{}/{action}",
                    if streaming { "streaming" } else { "finished" },
                    if attached { "attached" } else { "bare" }
                );
                rust.insert(name, run_case(&t, t0, streaming, attached, action));
            }
        }
    }
    assert_eq!(rust.len(), 60);
    let differing: Vec<_> = ruby
        .iter()
        .filter(|(name, outcome)| rust.get(*name) != Some(outcome))
        .map(|(name, outcome)| format!("{name}: ruby {outcome:?}, rust {:?}", rust.get(name)))
        .collect();
    assert!(differing.is_empty(), "{}", differing.join("\n"));
    assert_eq!(ruby.keys().collect::<Vec<_>>(), rust.keys().collect::<Vec<_>>());
}
