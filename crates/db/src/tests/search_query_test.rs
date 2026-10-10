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
fn review_unicode_connector_phrases_match_rails_in_sqlite() {
    let t = super::channel_thread_test::frozen();
    t.write(|tx| {
        for (client, body) in [("adjacent", "a b"), ("separated", "a x b")] {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some(body.into()),
                    client_message_id: Some(client.into()),
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    });
    let cases: Value = serde_json::from_str(include_str!("ws8_review_vectors.json")).unwrap();
    for row in cases["search"].as_array().unwrap() {
        let q = SearchQuery::parse(row["raw"].as_str().unwrap());
        assert_eq!(json!(q.text_tokens()), row["tokens"], "{row}");
        assert_eq!(json!(q.match_expression()), row["expression"], "{row}");
        let page =
            t.read(|conn| q.messages_for_user(conn, id("david"), jiff::tz::TimeZone::UTC, None));
        assert_eq!(
            json!(
                page.messages
                    .iter()
                    .map(|m| &m.client_message_id)
                    .collect::<Vec<_>>()
            ),
            row["clients"],
            "{row}"
        );
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

#[test]
fn stable_ids_survive_renames_and_keep_visibility() {
    let t = fixture();
    let query = format!("from_id:{} in_id:{}", id("david"), id("designers"));
    let found = t.read(|c| {
        SearchQuery::parse(&query).messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, None)
    });
    assert!(!found.messages.is_empty(), "IDs find messages without text");
    t.write(|tx| {
        tx.conn().execute(
            "UPDATE users SET name='Renamed author' WHERE id=?",
            [id("david")],
        )?;
        tx.conn().execute(
            "UPDATE rooms SET name='Renamed channel' WHERE id=?",
            [id("designers")],
        )?;
        Ok(())
    });
    let renamed = t.read(|c| {
        SearchQuery::parse(&query).messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, None)
    });
    assert_eq!(
        found.messages.iter().map(|m| m.id).collect::<Vec<_>>(),
        renamed.messages.iter().map(|m| m.id).collect::<Vec<_>>()
    );
    let hidden = t.read(|c| {
        SearchQuery::parse(&format!("from_id:{} in_id:{}", id("david"), id("all_talk")))
            .messages_for_user(c, id("kevin"), jiff::tz::TimeZone::UTC, None)
    });
    assert!(hidden.messages.is_empty());
}

#[test]
fn mentions_filter_current_body_and_viewer_id() {
    let t = fixture();
    let (mine, other) = t.write(|tx| {
        let mut make = |target, client: &str| {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some(crate::rich_text::mention_attachment_for(target)),
                    client_message_id: Some(client.into()),
                    ..Default::default()
                },
            )
        };
        Ok((
            make(id("david"), "mention-me")?.id,
            make(id("kevin"), "mention-other")?.id,
        ))
    });
    let matches = |q| {
        t.read(|c| {
            SearchQuery::parse(q).messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, None)
        })
        .messages
        .iter()
        .map(|m| m.id)
        .collect::<Vec<_>>()
    };
    assert_eq!(matches("has:mention"), vec![mine, other]);
    assert_eq!(matches("mentions:me"), vec![mine]);
    t.write(move |tx| {
        Message::find(tx.conn(), mine)?.update(
            tx,
            crate::MessageChanges {
                body: Some("Mention removed".into()),
                ..Default::default()
            },
        )?;
        Ok(())
    });
    assert!(
        matches("mentions:me").is_empty(),
        "does not search stale notification rows"
    );
}

#[test]
fn media_filters_use_current_blob_content_types() {
    let t = fixture();
    t.write(|tx| {
        for (n, content_type) in [(0, "audio/ogg"), (1, "video/mp4"), (2, "application/pdf")] {
            let blob = Blob::create(
                tx,
                &Blob {
                    id: 0,
                    key: format!("search-media-{n}"),
                    filename: format!("media-{n}"),
                    content_type: Some(content_type.into()),
                    metadata: None,
                    service_name: "local".into(),
                    byte_size: 1,
                    checksum: None,
                    created_at: tx.now(),
                },
            )?;
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    attachment_blob_id: Some(blob.id),
                    client_message_id: Some(format!("media-{n}")),
                    ..Default::default()
                },
            )?;
        }
        Ok(())
    });
    for (q, expected) in [("has:audio", "media-0"), ("has:video", "media-1")] {
        let page = t.read(|c| {
            SearchQuery::parse(q).messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, None)
        });
        assert_eq!(
            page.messages
                .iter()
                .map(|m| m.client_message_id.as_str())
                .collect::<Vec<_>>(),
            [expected]
        );
    }
}

#[test]
fn oldest_and_relevance_change_display_order() {
    let t = fixture();
    let (strong, weak) = t.write(|tx| {
        let mut make = |body: &str, client: &str| {
            Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some(body.into()),
                    client_message_id: Some(client.into()),
                    ..Default::default()
                },
            )
        };
        Ok((
            make("rankneedle rankneedle rankneedle", "rank-strong")?.id,
            make(
                "rankneedle surrounded by many unrelated words that dilute this result",
                "rank-weak",
            )?
            .id,
        ))
    });
    let matches = |q| {
        t.read(|c| {
            SearchQuery::parse(q).messages_for_user_sorted(
                c,
                id("david"),
                jiff::tz::TimeZone::UTC,
                None,
            )
        })
        .page
        .messages
        .iter()
        .map(|m| m.id)
        .collect::<Vec<_>>()
    };
    assert_eq!(matches("rankneedle sort:oldest"), [weak, strong]);
    assert_eq!(matches("rankneedle sort:relevance"), [weak, strong]);
    assert_eq!(matches("rankneedle sort:newest"), [strong, weak]);
}

#[test]
fn relevance_pages_survive_unrelated_activity_in_visible_and_private_rooms() {
    for private in [false, true] {
        let t = TestDb::new();
        let expected = t.write(|tx| {
            let mut ids = Vec::new();
            for n in 0..85 {
                ids.push(
                    Message::create(
                        tx,
                        NewMessage {
                            room_id: id("designers"),
                            creator_id: id("david"),
                            body: Some("needle".into()),
                            client_message_id: Some(format!("relevance-{n}")),
                            ..Default::default()
                        },
                    )?
                    .id,
                );
            }
            Ok(ids.into_iter().rev().collect::<Vec<_>>())
        });
        let q = SearchQuery::parse("needle sort:relevance");
        let first =
            t.read(|c| q.messages_for_user_sorted(c, id("david"), jiff::tz::TimeZone::UTC, None));
        assert_eq!(first.page.messages.len(), 40);
        t.write(move |tx| {
            let room = if private {
                Room::create_for(
                    tx,
                    RoomType::Closed,
                    Some("Private activity"),
                    id("kevin"),
                    &[id("kevin")],
                )?
                .id
            } else {
                id("pets")
            };
            Message::create(
                tx,
                NewMessage {
                    room_id: room,
                    creator_id: id("kevin"),
                    body: Some("unrelated words".into()),
                    client_message_id: Some("unrelated-activity".into()),
                    ..Default::default()
                },
            )?;
            Ok(())
        });
        let second = t.read(|c| {
            q.messages_for_user_sorted(c, id("david"), jiff::tz::TimeZone::UTC, first.next)
        });
        assert_eq!(second.page.messages.len(), 40, "private={private}");
        let third = t.read(|c| {
            q.messages_for_user_sorted(c, id("david"), jiff::tz::TimeZone::UTC, second.next)
        });
        assert_eq!(third.page.messages.len(), 5);
        assert!(third.next.is_none());
        let found = [first.page, second.page, third.page]
            .into_iter()
            .flat_map(|page| page.messages.into_iter().rev().map(|m| m.id))
            .collect::<Vec<_>>();
        assert_eq!(found, expected, "every hit exactly once, private={private}");
    }
}

#[test]
fn relevance_id_adapter_falls_back_to_newest_without_error_or_missing_results() {
    let t = TestDb::new();
    let expected = t.write(|tx| {
        let mut ids = Vec::new();
        for n in 0..85 {
            ids.push(
                Message::create(
                    tx,
                    NewMessage {
                        room_id: id("designers"),
                        creator_id: id("david"),
                        body: Some(format!("adapterneedle {}", "padding ".repeat(n % 3))),
                        client_message_id: Some(format!("adapter-{n}")),
                        ..Default::default()
                    },
                )?
                .id,
            );
        }
        Ok(ids.into_iter().rev().collect::<Vec<_>>())
    });
    let q = SearchQuery::parse("adapterneedle sort:relevance");
    let mut before = None;
    let mut found = Vec::new();
    loop {
        let page = t.read(|c| q.messages_for_user(c, id("david"), jiff::tz::TimeZone::UTC, before));
        before = page.messages.first().map(|m| m.id);
        found.extend(page.messages.into_iter().rev().map(|m| m.id));
        if !page.has_more {
            break;
        }
    }
    assert_eq!(found, expected);
}

#[test]
fn sorted_pages_do_not_repeat_ties_and_survive_cursor_deletion() {
    use crate::models::search_query::SearchSort;
    let t = fixture();
    let expected = t.write(|tx| {
        let mut ids = Vec::new();
        for n in 0..85 {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: id("designers"),
                    creator_id: id("david"),
                    body: Some(format!("pagingrank {}", "padding ".repeat(n % 3))),
                    client_message_id: Some(format!("sorted-{n}")),
                    ..Default::default()
                },
            )?;
            ids.push(message.id);
        }
        Ok(ids)
    });
    for sort in [
        SearchSort::Oldest,
        SearchSort::Newest,
        SearchSort::Relevance,
    ] {
        let mut q = SearchQuery::parse("pagingrank");
        q.sort = sort;
        let mut cursor = None;
        let mut found = Vec::new();
        loop {
            let page = t.read(|c| {
                q.messages_for_user_sorted(c, id("david"), jiff::tz::TimeZone::UTC, cursor)
            });
            found.extend(page.page.messages.iter().rev().map(|m| m.id));
            cursor = page.next;
            if cursor.is_none() {
                break;
            }
        }
        if sort == SearchSort::Oldest {
            assert_eq!(found, expected);
        }
        if sort == SearchSort::Newest {
            assert_eq!(found, expected.iter().rev().copied().collect::<Vec<_>>());
        }
        found.sort();
        assert_eq!(found, expected, "every hit exactly once with {sort:?}");
    }
    let mut q = SearchQuery::parse("pagingrank");
    q.sort = SearchSort::Oldest;
    let first =
        t.read(|c| q.messages_for_user_sorted(c, id("david"), jiff::tz::TimeZone::UTC, None));
    let cursor = first.next.unwrap();
    let crate::models::search_query::SearchCursor::Message { id: cursor_id, .. } = cursor else {
        panic!("oldest uses a message cursor")
    };
    t.write(move |tx| Message::find(tx.conn(), cursor_id)?.destroy(tx));
    let next = t.read(|c| {
        q.messages_for_user_sorted(c, id("david"), jiff::tz::TimeZone::UTC, Some(cursor))
    });
    assert_eq!(
        next.page
            .messages
            .iter()
            .rev()
            .map(|m| m.id)
            .collect::<Vec<_>>(),
        expected[40..80]
    );
}

#[test]
fn every_new_filter_keeps_room_and_thread_visibility() {
    let t = fixture();
    t.write(|tx| {
        let hidden = Room::create_for(
            tx,
            RoomType::Closed,
            Some("Filter private"),
            id("kevin"),
            &[id("kevin")],
        )?;
        let deleted = Room::create_for(
            tx,
            RoomType::Closed,
            Some("Filter deleted"),
            id("david"),
            &[id("david")],
        )?;
        for room in [id("designers"), hidden.id, deleted.id] {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: room,
                    creator_id: id("david"),
                    name: Some("Visibility thread".into()),
                    ..Default::default()
                },
            )?;
            for (n, content_type) in [(0, "image/png"), (1, "audio/ogg"), (2, "video/mp4")] {
                let blob = Blob::create(
                    tx,
                    &Blob {
                        id: 0,
                        key: format!("filter-{room}-{n}"),
                        filename: "filter".into(),
                        content_type: Some(content_type.into()),
                        metadata: None,
                        service_name: "local".into(),
                        byte_size: 1,
                        checksum: None,
                        created_at: tx.now(),
                    },
                )?;
                Message::create(
                    tx,
                    NewMessage {
                        room_id: room,
                        creator_id: id("david"),
                        thread_id: Some(thread.id),
                        body: Some(format!(
                            "visibilityneedle <a href=\"https://example.com\">link</a>{}",
                            crate::rich_text::mention_attachment_for(id("david"))
                        )),
                        attachment_blob_id: Some(blob.id),
                        client_message_id: Some(format!("filter-{room}-{n}")),
                        ..Default::default()
                    },
                )?;
            }
        }
        tx.conn().execute(
            "UPDATE rooms SET deleted_at=? WHERE id=?",
            rusqlite::params![tx.now(), deleted.id],
        )?;
        Ok(())
    });
    for filter in [
        format!("from_id:{}", id("david")),
        format!("in_id:{}", id("designers")),
        "mentions:me".into(),
        "has:mention".into(),
        "has:file".into(),
        "has:image".into(),
        "has:link".into(),
        "has:audio".into(),
        "has:video".into(),
        "sort:oldest".into(),
        "sort:relevance".into(),
    ] {
        let q = SearchQuery::parse(&format!("visibilityneedle {filter}"));
        let page =
            t.read(|c| q.messages_for_user_after(c, id("david"), jiff::tz::TimeZone::UTC, None));
        assert!(!page.messages.is_empty(), "positive witness for {filter}");
        assert!(
            page.messages
                .iter()
                .all(|m| m.room_id == id("designers") && m.thread_id.is_some()),
            "{filter}"
        );
    }
}
