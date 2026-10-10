use super::channel_thread_test::{frozen, post_root};
use super::*;
use crate::models::message_reference as refs;
use crate::models::message_reference::QuoteCardsRefreshJob;
use crate::{Message, MessageChanges, NewMessage};

fn quotes(t: &TestDb, message: &Message) -> Vec<i64> {
    t.read(|c| refs::referenced_ids(c, message.id))
}
fn quote(t: &TestDb, source: &Message) -> Message {
    let attrs = NewMessage {
        room_id: id("designers"),
        creator_id: id("david"),
        body: Some(format!("<p>/rooms/999/@{}</p>", source.id)),
        ..Default::default()
    };
    t.write(move |tx| Message::create(tx, attrs))
}

#[test]
fn reference_helpers_match_rails_vectors() {
    let golden: serde_json::Value =
        serde_json::from_str(include_str!("ws8_reference_vectors.json")).unwrap();
    for row in golden["extraction"].as_array().unwrap() {
        assert_eq!(
            serde_json::to_value(refs::extract_message_ids(
                row["text"].as_str().unwrap_or("")
            ))
            .unwrap(),
            row["ids"],
            "{row}"
        );
    }
    for row in golden["html"].as_array().unwrap() {
        assert_eq!(
            refs::non_code_text(row["html"].as_str().unwrap()).unwrap(),
            row["text"].as_str().unwrap(),
            "{row}"
        );
    }
}

#[test]
fn references_validate_required_associations_uniqueness_and_self() {
    let t = frozen();
    let source = post_root(&t, "watercooler", "david", "source");
    let quoting = post_root(&t, "designers", "david", "quoting");
    let (a, b) = (quoting.id, source.id);
    t.write(move |tx| refs::create(tx, a, b));
    for (a, b) in [(a, b), (a, a), (-1, b), (a, -1)] {
        assert!(matches!(
            t.try_write(move |tx| refs::create(tx, a, b)),
            Err(crate::Error::RecordInvalid(_))
        ));
    }
}

#[test]
fn posting_quotes_syncs_and_edits_reconcile_idempotently() {
    let t = frozen();
    let source = post_root(&t, "watercooler", "david", "source");
    let mut quoting = quote(&t, &source);
    assert_eq!(quotes(&t, &quoting), [source.id]);
    let copy = quoting.clone();
    t.write(move |tx| refs::sync(tx, &copy));
    assert_eq!(quotes(&t, &quoting), [source.id]);
    quoting = t.write(move |tx| {
        quoting.edit(
            tx,
            MessageChanges {
                body: Some("no links".into()),
                ..Default::default()
            },
        )?;
        Ok(quoting)
    });
    assert!(quotes(&t, &quoting).is_empty());
}

#[test]
fn sync_filters_code_missing_self_system_targets_and_caps_at_ten() {
    let t = frozen();
    let sources: Vec<_> = (0..12)
        .map(|n| post_root(&t, "watercooler", "david", &format!("source {n}")))
        .collect();
    let body = sources
        .iter()
        .map(|m| format!("<a href='/rooms/1/@{}'>label</a>", m.id))
        .collect::<String>();
    let attrs = NewMessage {
        room_id: id("designers"),
        creator_id: id("david"),
        body: Some(body),
        ..Default::default()
    };
    let quote = t.write(move |tx| Message::create(tx, attrs));
    let mut expected: Vec<_> = sources[..10].iter().map(|m| m.id).collect();
    expected.sort();
    assert_eq!(quotes(&t, &quote), expected);
    let note = t.write(|tx| {
        Message::create(
            tx,
            NewMessage {
                room_id: id("designers"),
                creator_id: id("david"),
                body: Some("note".into()),
                system_note: true,
                ..Default::default()
            },
        )
    });
    let source = sources[0].id;
    let attrs = NewMessage {
        room_id: id("designers"),
        creator_id: id("david"),
        streaming: true,
        body: Some(format!(
            "<code>/rooms/1/@{source}</code><pre>/rooms/1/@{source}</pre> /rooms/1/@{} /rooms/1/@999999999 /rooms/1/@{}",
            note.id, quote.id
        )),
        ..Default::default()
    };
    let stream = t.write(move |tx| Message::create(tx, attrs));
    assert!(quotes(&t, &stream).is_empty());
    let stream_id = stream.id;
    let final_message = t.write(move |tx| {
        let mut m = Message::find(tx.conn(), stream_id)?;
        m.claim_stream_finalized(tx)?;
        refs::sync(tx, &m)?;
        Ok(m)
    });
    assert_eq!(quotes(&t, &final_message), [quote.id]);
}

#[test]
fn source_edits_enqueue_and_refresh_capped_cards_in_conversation() {
    let t = frozen();
    let mut source = post_root(&t, "watercooler", "david", "source");
    let a = quote(&t, &source);
    let _b = quote(&t, &source);
    let _c = quote(&t, &source);
    source = t.write(move |tx| {
        source.edit(
            tx,
            MessageChanges {
                body: Some("changed source".into()),
                ..Default::default()
            },
        )?;
        Ok(source)
    });
    assert!(
        t.events()
            .iter()
            .filter_map(|e| e.as_job::<QuoteCardsRefreshJob>())
            .any(|job| job.source_message_id == source.id)
    );
    let from = t.events().len();
    let source_id = source.id;
    assert_eq!(
        t.write(move |tx| refs::refresh_quote_cards(tx, source_id, 2, 1)),
        2
    );
    let events = t.events();
    assert_eq!(
        events[from..]
            .iter()
            .filter(|e| e.as_broadcast().is_some())
            .count(),
        2
    );
    assert!(
        events[from..]
            .iter()
            .any(|e| e.as_broadcast().is_some_and(|b| matches!(b, crate::broadcasts::Broadcast::MessageCards { message_id } if message_id == a.id)))
    );
    assert_eq!(t.write(|tx| refs::refresh_quote_cards(tx, -1, 200, 100)), 0);
}

#[test]
fn destroying_a_source_removes_references_stamps_quotes_without_touching_rooms() {
    let t = frozen();
    let source = post_root(&t, "watercooler", "david", "source");
    let quoting = quote(&t, &source);
    assert_eq!(quotes(&t, &quoting), [source.id]);
    let room = t.read(|c| crate::Room::find(c, quoting.room_id));
    t.clock.travel(jiff::SignedDuration::from_secs(60));
    let from = t.events().len();
    t.write(move |tx| source.destroy(tx));
    assert!(quotes(&t, &quoting).is_empty());
    assert_eq!(t.read(|c| Message::find(c, quoting.id)).updated_at, t.now());
    assert_eq!(
        t.read(|c| crate::Room::find(c, quoting.room_id)).updated_at,
        room.updated_at
    );
    assert!(
        t.events()[from..]
            .iter()
            .any(|e| e.as_broadcast().is_some())
    );
}
