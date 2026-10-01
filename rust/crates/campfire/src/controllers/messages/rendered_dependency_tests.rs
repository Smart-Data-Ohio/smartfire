//! Real room reloads, with fixture rows and before/after bytes recorded from pinned Rails.
use crate::controllers::presenters::{Presenter, test_support::*};
use axum::http::StatusCode;
use campfire_db::{Message, User, UserChanges};
use campfire_kit::clock::FrozenClock;
use rusqlite::{params_from_iter, types::Value as SqlValue};
use serde_json::Value;
use std::sync::Arc;

fn sql_value(value: &Value) -> SqlValue {
    match value {
        Value::Null => SqlValue::Null,
        Value::Number(n) => SqlValue::Integer(n.as_i64().unwrap()),
        Value::String(s) => SqlValue::Text(s.clone()),
        _ => panic!("unexpected fixture value {value}"),
    }
}

async fn reload(name: &str) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/rendered-dependencies.json"
    ))
    .unwrap();
    let row = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()
        .clone();
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    app.db()
        .write(move |tx| {
            tx.conn().execute_batch("PRAGMA defer_foreign_keys=ON")?;
            // Import only the oracle's scenario rows; defer forward reply FKs until commit.
            for table in [
                "messages",
                "action_text_rich_texts",
                "boosts",
                "polls",
                "poll_options",
                "poll_votes",
                "active_storage_blobs",
                "active_storage_attachments",
                "message_references",
                "workspace_icons",
                "link_embeds",
                "link_embed_references",
            ] {
                for record in oracle["database"][table].as_array().unwrap() {
                    let record = record.as_object().unwrap();
                    let columns = record
                        .keys()
                        .map(|k| format!("\"{k}\""))
                        .collect::<Vec<_>>()
                        .join(",");
                    let placeholders = std::iter::repeat_n("?", record.len())
                        .collect::<Vec<_>>()
                        .join(",");
                    tx.conn().execute(
                        &format!("INSERT INTO {table} ({columns}) VALUES ({placeholders})"),
                        params_from_iter(record.values().map(sql_value)),
                    )?;
                }
            }
            for room in [ALL_TALK, DIRECT_DAVID_JASON] {
                campfire_db::Room::touch(tx, room)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut viewer = app.david();
    let room = row["room_id"].as_i64().unwrap();
    let message = row["message_id"].as_i64().unwrap();
    let mut previous_key = None;
    for state in ["before", "after"] {
        if state == "after" {
            clock.set("2026-03-02T16:00:00.125Z".parse().unwrap());
            let mutation = row["mutation"].clone();
            app.db()
                .write(move |tx| {
                    let table = mutation["table"].as_str().unwrap();
                    let id = mutation["id"].as_i64().unwrap();
                    let attrs = mutation["attributes"].as_object().unwrap();
                    if table == "users" {
                        User::find(tx.conn(), id)?.update(
                            tx,
                            UserChanges {
                                name: Some(attrs["name"].as_str().unwrap().into()),
                                bio: Some(Some(attrs["bio"].as_str().unwrap().into())),
                                ..Default::default()
                            },
                        )?;
                    } else if table != "clock" {
                        assert!(
                            [
                                "rooms",
                                "poll_options",
                                "active_storage_blobs",
                                "workspace_icons",
                                "link_embed_references"
                            ]
                            .contains(&table)
                        );
                        let sets = attrs
                            .keys()
                            .map(|k| format!("\"{k}\"=?"))
                            .collect::<Vec<_>>()
                            .join(",");
                        let values = attrs
                            .iter()
                            .map(|(k, v)| {
                                if k == "updated_at" {
                                    SqlValue::Text(campfire_storage::blob::format_timestamp(
                                        v.as_str().unwrap().parse().unwrap(),
                                    ))
                                } else {
                                    sql_value(v)
                                }
                            })
                            .chain(std::iter::once(SqlValue::Integer(id)));
                        tx.conn().execute(
                            &format!("UPDATE {table} SET {sets} WHERE id=?"),
                            params_from_iter(values),
                        )?;
                        // Blob#after_update touches the owning Message through Attachment#record.
                        if let Some(id) = mutation["touched_message_id"].as_i64() {
                            tx.conn().execute(
                                "UPDATE messages SET updated_at=? WHERE id=?",
                                rusqlite::params!["2026-03-02 16:00:00.125000", id],
                            )?;
                            campfire_db::Room::touch(tx, Message::find(tx.conn(), id)?.room_id)?;
                        }
                    }
                    Ok(())
                })
                .await
                .unwrap();
        }
        for _ in 0..2 {
            let response = viewer.get(&format!("/rooms/{room}")).await;
            assert_eq!(response.status, StatusCode::OK, "{}", response.text());
            let expected = row[state].as_str().unwrap();
            if !response.text().contains(expected) {
                let marker = format!("id=\"message_dependency-{name}\"");
                let actual = response.text();
                let start = actual
                    .find(&marker)
                    .map(|i| actual[..i].rfind('<').unwrap())
                    .unwrap_or(0);
                rails_mismatch(
                    &actual[start..actual.len().min(start + expected.len())],
                    expected,
                    &format!("{name} {state} warm room"),
                );
            }
        }
        let runtime = app.booted.app.clone();
        let expected = row[state].as_str().unwrap().to_owned();
        let expected_records = row[format!("{state}_key")].as_str().unwrap().to_owned();
        let key = app
            .db()
            .read(move |conn| {
                let p = Presenter::new(conn, &runtime, None);
                let record = Message::find(conn, message)?;
                assert_eq!(
                    p.message_rendered_cache_key(&record)?,
                    expected_records,
                    "individual Rails record versions"
                );
                let key = p.message_fragment_cache_key(&record, "http://campfire.test")?;
                campfire_views::fragment_cache::with(&runtime.fragment_cache, || {
                    assert_eq!(
                        campfire_views::fragment_cache::read(&key).unwrap().as_str(),
                        expected
                    );
                });
                Ok(key)
            })
            .await
            .unwrap();
        if let Some(before) = previous_key {
            assert_ne!(key, before, "{name} dependency must expire the fragment");
        }
        previous_key = Some(key);
    }
}

macro_rules! regression {
    ($($name:ident),+ $(,)?) => { $(#[tokio::test] async fn $name() { reload(stringify!($name)).await; })+ };
}
regression!(
    reactor,
    legacy_booster,
    mention,
    reply_mention,
    quote_mention,
    poll_voter,
    poll_option,
    room,
    direct_room_member,
    attachment,
    reply_attachment,
    icon,
    link_reference,
    poll_clock
);
