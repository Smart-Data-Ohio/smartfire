use crate::controllers::presenters::{Presenter, test_support::*};
use campfire_db::{Boost, ChannelThread, Message, MessagePin, NewChannelThread, NewMessage};
use campfire_kit::clock::FrozenClock;
use serde_json::Value;
use std::sync::Arc;

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/collection.json"
    ))
    .unwrap()
}

#[test]
fn nullable_quote_name_digest_matches_ruby_inspect_and_comparison_errors() {
    for row in oracle()["quote_names"].as_array().unwrap() {
        let names = row["names"]
            .as_array()
            .unwrap()
            .iter()
            .map(|pair| {
                (
                    pair[0].as_str().unwrap().to_owned(),
                    pair[1].as_str().map(str::to_owned),
                )
            })
            .collect::<Vec<_>>();
        match campfire_views::fragment_cache::keys::quote_names_digest_nullable(&names) {
            Ok(digest) => assert_eq!(digest, row["digest"].as_str().unwrap()),
            Err(error) => assert_eq!(error, row["error"].as_str().unwrap()),
        }
    }
}

#[tokio::test]
async fn collection_cache_tracks_pin_thread_stream_edit_drive_and_reaction_states() {
    let clock = Arc::new(FrozenClock::new(SEED_NOW.parse().unwrap()));
    let app = TestApp::boot_with_test_clock(clock.clone()).await.unwrap();
    let id = app
        .db()
        .write(|tx| {
            Ok(Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Collection".into()),
                    client_message_id: Some("collection-target".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    assert_eq!(id, oracle()["message_id"].as_i64().unwrap());
    let mut pin_id = None;
    let mut thread_id = None;
    for state in oracle()["states"].as_array().unwrap() {
        clock.set(state["time"].as_str().unwrap().parse().unwrap());
        let name = state["name"].as_str().unwrap().to_owned();
        let (pin, thread) = app.db().write(move |tx| {
            let mut message = Message::find(tx.conn(), id)?;
            match name.as_str() {
                "initial" => {},
                "pinned" => pin_id = Some(MessagePin::create(tx, &message, ALL_TALK, DAVID)?.id),
                "unpinned" => MessagePin::find(tx.conn(), pin_id.unwrap())?.unpin(tx)?,
                "thread" => thread_id = Some(ChannelThread::create(tx, NewChannelThread { room_id: ALL_TALK, creator_id: DAVID,
                    parent_message_id: Some(id), name: Some("Collection thread".into()), ..Default::default() })?.id),
                "thread_reply" => { Message::create(tx, NewMessage { room_id: ALL_TALK, creator_id: DAVID, thread_id,
                    markdown_source: Some("Child".into()), client_message_id: Some("collection-child".into()), ..Default::default() })?; },
                "streaming" | "final" => { tx.conn().execute("UPDATE messages SET streaming = ? WHERE id = ?", (name == "streaming", id))?; },
                "edited" => message.edit(tx, campfire_db::MessageChanges { markdown_source: Some("## Edited".into()), ..Default::default() })?,
                "drive" => { tx.conn().execute("INSERT INTO drive_attachments (message_id, file_id, created_at) VALUES (?, 'abcdefghij', ?)", (id, tx.now()))?; message.touch(tx)?; },
                "reaction" => { Boost::create(tx, id, DAVID, "👍")?; },
                _ => unreachable!()
            }
            Ok((pin_id, thread_id))
        }).await.unwrap();
        pin_id = pin;
        thread_id = thread;
        for _ in 0..2 {
            let runtime = app.booted.app.clone();
            let expected_key = state["key"].as_str().unwrap().to_owned();
            let html = app
                .db()
                .read(move |conn| {
                    let mut presenter = Presenter::new(conn, &runtime, None);
                    presenter.cache_base_url = Some("http://campfire.test".into());
                    let message = Message::find(conn, id)?;
                    campfire_views::fragment_cache::with(&runtime.fragment_cache, || {
                        assert_eq!(
                            presenter.message_collection_cache_key(&message)?,
                            expected_key
                        );
                        let item = presenter.message_item(&message)?;
                        let account = campfire_db::Account::first(conn)?;
                        Ok(crate::controllers::presenters::page::render_detached_at(
                            &runtime,
                            account.as_ref(),
                            "http://campfire.test",
                            |ctx| {
                                campfire_views::messages::cached_message_item(ctx, &item)
                                    .to_string()
                            },
                        ))
                    })
                })
                .await
                .unwrap();
            if html != state["html"].as_str().unwrap() {
                rails_mismatch(&html, state["html"].as_str().unwrap(), state["name"].as_str().unwrap());
            }
            assert_eq!(campfire_cable::turbo::session_bound(&html), None);
        }
    }
}
