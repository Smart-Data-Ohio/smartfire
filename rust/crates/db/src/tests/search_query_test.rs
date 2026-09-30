use super::*;
use crate::models::search_query::SearchQuery;
use crate::{
    Blob, ChannelThread, Message, NewChannelThread, NewMessage, Room, RoomType, Timestamp,
};
use serde_json::{Value, json};
fn golden() -> Value {
    serde_json::from_str(include_str!("ws8_search_vectors.json")).unwrap()
}
fn parsed(q: &SearchQuery) -> Value {
    json!({"raw":q.raw,"text":q.text,"from_names":q.from_names,"in_rooms":q.in_rooms,"has_values":q.has_values,"before_date":q.before_date.map(|d|d.to_string()),"after_date":q.after_date.map(|d|d.to_string()),"on_date":q.on_date.map(|d|d.to_string()),"thread_only":q.thread_only,"filters":q.filters(),"blank_query":q.blank_query(),"text_tokens":q.text_tokens(),"match_expression":q.match_expression(),"chips":q.chips})
}
fn fixture() -> TestDb {
    let t = TestDb::with_clock(
        TestClock::frozen_at(Timestamp::parse_db("2026-03-10 12:00:00").unwrap()),
        4,
    );
    let rows = golden()["rows"].clone();
    t.write(move |tx| {
        let hidden = Room::create_for(
            tx,
            RoomType::Closed,
            Some("WS8 hidden"),
            id("kevin"),
            &[id("kevin")],
        )?;
        let deleted = Room::create_for(
            tx,
            RoomType::Closed,
            Some("WS8 deleted"),
            id("david"),
            &[id("david")],
        )?;
        let thread = ChannelThread::create(
            tx,
            NewChannelThread {
                room_id: id("designers"),
                creator_id: id("david"),
                name: Some("WS8 search thread".into()),
                ..Default::default()
            },
        )?;
        for row in rows.as_array().unwrap() {
            let room = match row["room"].as_str().unwrap() {
                "hidden" => hidden.id,
                "deleted" => deleted.id,
                other => id(other),
            };
            let blob = if row["image"] == true {
                Some(
                    Blob::create(
                        tx,
                        &Blob {
                            id: 0,
                            key: "ws8-search-image".into(),
                            filename: "test.png".into(),
                            content_type: Some("image/png".into()),
                            metadata: None,
                            service_name: "local".into(),
                            byte_size: 1,
                            checksum: Some("ndTkYSaMgDT1yFZOFVxnpg==".into()),
                            created_at: tx.now(),
                        },
                    )?
                    .id,
                )
            } else {
                None
            };
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: id(row["creator"].as_str().unwrap()),
                    body: Some(row["body"].as_str().unwrap().into()),
                    client_message_id: Some(row["client"].as_str().unwrap().into()),
                    thread_id: (row["thread"] == true).then_some(thread.id),
                    system_note: row["system_note"] == true,
                    streaming: row["streaming"] == true,
                    attachment_blob_id: blob,
                    drive_file_ids: if row["drive"] == true {
                        vec!["1a2b3c4d5e6f7g8h9i0j".into()]
                    } else {
                        vec![]
                    },
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE messages SET created_at=? WHERE id=?",
                rusqlite::params![row["date"].as_str().unwrap(), m.id],
            )?;
            if row["pin"] == true {
                crate::MessagePin::pin(tx, &m, id("david"))?.unwrap();
            }
        }
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            rusqlite::params![tx.now(), deleted.id],
        )?;
        Ok(())
    });
    t
}
#[test]
fn grammar_matches_rails_including_invalid_tokens_unicode_and_chips() {
    for row in golden()["parser"].as_array().unwrap() {
        let raw = row["raw"].as_str().unwrap_or("");
        assert_eq!(parsed(&SearchQuery::parse(raw)), row["parsed"], "{raw}");
    }
}
#[test]
fn sqlite_operators_match_rails_and_hide_inaccessible_deleted_rooms() {
    let t = fixture();
    for row in golden()["results"].as_array().unwrap() {
        let q = SearchQuery::parse(row["raw"].as_str().unwrap());
        let page = t.read(|c| q.messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, None));
        let clients: Vec<_> = page
            .messages
            .iter()
            .map(|m| m.client_message_id.as_str())
            .collect();
        assert_eq!(json!(clients), row["clients"], "{row}");
    }
}
#[test]
fn cursor_windows_match_rails_without_repeating_same_timestamp_rows() {
    let t = fixture();
    t.write(|tx| {
        for i in 0..45 {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some(format!("ws8paging {i}")),
                    client_message_id: Some(format!("ws8-page-{i}")),
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    });
    let q = SearchQuery::parse("ws8paging");
    let mut before = None;
    for row in golden()["pages"].as_array().unwrap() {
        let page = t.read(|c| q.messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, before));
        let clients: Vec<_> = page
            .messages
            .iter()
            .map(|m| m.client_message_id.as_str())
            .collect();
        assert_eq!(json!({"clients":clients,"has_more":page.has_more}), *row);
        before = page.messages.first().map(|m| m.id);
    }
}
