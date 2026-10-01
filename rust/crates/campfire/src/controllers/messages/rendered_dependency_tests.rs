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

async fn cached_message(app: &TestApp, id: i64) -> (String, String, Arc<String>) {
    let state = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let p = Presenter::new(conn, &state, None);
            let message = Message::find(conn, id)?;
            let helper = p.message_collection_cache_key(&message)?;
            let key = p.message_fragment_cache_key(&message, "http://campfire.test")?;
            let html = state
                .fragment_cache
                .get::<Arc<String>>(&key)
                .expect("mounted shared fragment");
            Ok((helper, key, html))
        })
        .await
        .unwrap()
}

async fn unrelated_write(upload_icon: bool) {
    use axum::http::Method;
    use campfire_db::NewMessage;
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let id = app
        .db()
        .write(|tx| {
            let m = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Stable fragment :github:".into()),
                    client_message_id: Some("cache-stability".into()),
                    ..Default::default()
                },
            )?;
            campfire_db::Boost::create(tx, m.id, JASON, "👍")?;
            Ok(m.id)
        })
        .await
        .unwrap();
    let mut viewer = app.david();
    for path in [
        format!("/rooms/{ALL_TALK}"),
        format!("/rooms/{ALL_TALK}/messages"),
    ] {
        assert_eq!(viewer.get(&path).await.status, StatusCode::OK);
    }
    let before = cached_message(&app, id).await;
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/cache-stability.json"
    ))
    .unwrap();
    let name = if upload_icon {
        "unused_icon_upload"
    } else {
        "unrelated_post"
    };
    let expected = oracle["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|row| row["name"] == name)
        .unwrap();
    assert_eq!(
        before.0,
        expected["before"]["key"].as_str().unwrap(),
        "pinned Rails helper key"
    );
    assert_eq!(
        before.2.as_str(),
        expected["before"]["html"].as_str().unwrap(),
        "pinned Rails partial"
    );
    clock.set("2026-03-02T16:00:02Z".parse().unwrap());
    if upload_icon {
        let svg = std::fs::read(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../vectors/workspace_icons/clean.svg"),
        )
        .unwrap();
        let response = viewer
            .write(Req::new(Method::POST, "/account/icons").multipart(
                &[
                    ("workspace_icon[name]", "review_unused_icon"),
                    ("workspace_icon[title]", "Unused review icon"),
                ],
                ("workspace_icon[image]", "clean.svg", "image/svg+xml", &svg),
            ))
            .await;
        assert_eq!(response.status, StatusCode::FOUND, "{}", response.text());
        assert!(
            app.db()
                .read(|c| Ok(
                    campfire_db::models::workspace_icon::WorkspaceIcon::find_by_name(
                        c,
                        "review_unused_icon"
                    )?
                    .is_some()
                ))
                .await
                .unwrap()
        );
    } else {
        let response = viewer
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                    .header("accept", "text/vnd.turbo-stream.html")
                    .form(&[("message[markdown_source]", "Unrelated post")]),
            )
            .await;
        assert!(
            response.status.is_success() || response.status.is_redirection(),
            "{} {}",
            response.status,
            response.text()
        );
        assert!(
            app.db()
                .read(|c| Ok(c.query_row(
                    "SELECT EXISTS(SELECT 1 FROM messages WHERE markdown_source='Unrelated post')",
                    [],
                    |r| r.get::<_, bool>(0)
                )?))
                .await
                .unwrap()
        );
    }
    for path in [
        format!("/rooms/{ALL_TALK}"),
        format!("/rooms/{ALL_TALK}/messages"),
    ] {
        assert_eq!(viewer.get(&path).await.status, StatusCode::OK);
        let after = cached_message(&app, id).await;
        assert_eq!(before.0, after.0, "Rails collection helper remains stable");
        assert_eq!(
            before.2.as_str(),
            after.2.as_str(),
            "identical rendered HTML"
        );
        assert_eq!(
            before.1, after.1,
            "unrelated writes preserve the fragment key"
        );
        assert!(
            Arc::ptr_eq(&before.2, &after.2),
            "reuse the shared allocation"
        );
    }
}

#[tokio::test]
async fn unrelated_post_preserves_shared_fragment() {
    unrelated_write(false).await;
}

#[tokio::test]
async fn unused_icon_upload_preserves_shared_fragment() {
    unrelated_write(true).await;
}

fn thread_oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/thread-cache-stability.json"
    ))
    .unwrap()
}

async fn thread_fixture() -> (TestApp, Arc<FrozenClock>, i64, Vec<i64>) {
    use campfire_db::{ChannelThread, NewChannelThread, NewMessage};
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone())
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let (thread_id, ids) = app
        .db()
        .write(|tx| {
            let parent = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Thread parent".into()),
                    client_message_id: Some("thread-cache-parent".into()),
                    ..Default::default()
                },
            )?;
            let mut thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    parent_message_id: Some(parent.id),
                    name: Some("Cache thread".into()),
                    ..Default::default()
                },
            )?;
            let mut ids = vec![parent.id];
            for i in 0..2 {
                ids.push(
                    thread
                        .post_message(
                            tx,
                            DAVID,
                            NewMessage {
                                markdown_source: Some(format!("Existing reply {i}")),
                                client_message_id: Some(format!("thread-cache-reply-{i}")),
                                ..Default::default()
                            },
                        )?
                        .id,
                );
            }
            Ok((thread.id, ids))
        })
        .await
        .unwrap();
    assert_eq!(thread_oracle()["thread_id"], thread_id);
    (app, clock, thread_id, ids)
}

async fn thread_snapshot(
    app: &TestApp,
    thread: i64,
    ids: &[i64],
    state: usize,
) -> Vec<(String, String, Arc<String>)> {
    let mut viewer = app.david();
    for path in [
        format!("/rooms/{ALL_TALK}"),
        format!("/rooms/{ALL_TALK}/threads/{thread}/messages"),
    ] {
        let response = viewer.get(&path).await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    }
    let oracle = thread_oracle();
    let expected = oracle["states"][state]["messages"].as_array().unwrap();
    let mut snapshots = Vec::new();
    for (id, expected) in ids.iter().zip(expected) {
        assert_eq!(expected["id"], *id);
        let snapshot = cached_message(app, *id).await;
        assert_eq!(
            snapshot.0,
            expected["key"].as_str().unwrap(),
            "Rails helper key"
        );
        assert_eq!(
            snapshot.2.as_str(),
            expected["html"].as_str().unwrap(),
            "Rails partial HTML"
        );
        snapshots.push(snapshot);
    }
    snapshots
}

async fn post_another_reply(app: &TestApp, clock: &FrozenClock, thread: i64) {
    use axum::http::Method;
    clock.set("2026-03-02T16:00:02Z".parse().unwrap());
    let old_stamp = app
        .db()
        .read(move |conn| Ok(campfire_db::ChannelThread::find(conn, thread)?.updated_at))
        .await
        .unwrap();
    let response = app
        .david()
        .write(
            Req::new(
                Method::POST,
                &format!("/rooms/{ALL_TALK}/threads/{thread}/messages.json"),
            )
            .form(&[
                ("message[markdown_source]", "Another reply"),
                ("message[client_message_id]", "thread-cache-another"),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    app.db()
        .read(move |conn| {
            let thread = campfire_db::ChannelThread::find(conn, thread)?;
            assert_ne!(
                thread.updated_at, old_stamp,
                "exercise the changing thread version"
            );
            assert_eq!(thread.messages_count, 3, "the post persisted");
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn thread_post_preserves_existing_reply_fragments() {
    let (app, clock, thread, ids) = thread_fixture().await;
    let before = thread_snapshot(&app, thread, &ids, 0).await;
    post_another_reply(&app, &clock, thread).await;
    let after = thread_snapshot(&app, thread, &ids, 1).await;
    for (before, after) in before.iter().zip(&after).skip(1) {
        let reused = Arc::ptr_eq(&before.2, &after.2);
        println!(
            "Thread post: helper_key_unchanged={}; html_unchanged={}; fragment_key_unchanged={}; allocation_reused={reused}",
            before.0 == after.0,
            before.2 == after.2,
            before.1 == after.1
        );
        assert_eq!(
            before.1, after.1,
            "another reply preserves the fragment key"
        );
        assert!(reused, "another reply reuses the shared allocation");
    }
}

#[tokio::test]
async fn thread_post_refreshes_parent_reply_count_fragment() {
    let (app, clock, thread, ids) = thread_fixture().await;
    let before = thread_snapshot(&app, thread, &ids, 0).await.remove(0);
    post_another_reply(&app, &clock, thread).await;
    let after = thread_snapshot(&app, thread, &ids, 1).await.remove(0);
    assert!(before.2.contains("2 replies"));
    assert!(after.2.contains("3 replies"));
    assert_ne!(
        before.0, after.0,
        "Rails' literal count and parent version change"
    );
    assert_ne!(
        before.1, after.1,
        "the rendered count invalidates the parent key"
    );
    assert!(
        !Arc::ptr_eq(&before.2, &after.2),
        "render the new parent indicator"
    );
}

#[tokio::test]
async fn thread_rename_preserves_parent_and_reply_fragments() {
    let (app, clock, thread, ids) = thread_fixture().await;
    post_another_reply(&app, &clock, thread).await;
    let before = thread_snapshot(&app, thread, &ids, 1).await;
    clock.set("2026-03-02T16:00:04Z".parse().unwrap());
    app.db()
        .write(move |tx| {
            let mut thread = campfire_db::ChannelThread::find(tx.conn(), thread)?;
            let old_stamp = thread.updated_at;
            thread.update_metadata(tx, Some("Renamed cache thread"), None, None)?;
            assert_ne!(thread.updated_at, old_stamp);
            Ok(())
        })
        .await
        .unwrap();
    let after = thread_snapshot(&app, thread, &ids, 2).await;
    for (before, after) in before.iter().zip(&after) {
        assert_eq!(
            before.1, after.1,
            "thread name is not rendered by the message partial"
        );
        assert!(
            Arc::ptr_eq(&before.2, &after.2),
            "rename reuses the shared allocation"
        );
    }
}
