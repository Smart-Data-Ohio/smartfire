//! The last assertion of HuddleInvitationTest's disabled-items declaration:
//! neighboring real Action Text mentions must still record an inbox item.
use crate::controllers::presenters::test_support::TestApp;
use campfire_db::models::{huddle_grant::HuddleGrant, room_delete::HuddleConfig};
use campfire_db::{ActivityItem, Membership, Message, NewMessage, Timestamp};
use serde_json::{Value, json};
use std::sync::Arc;

#[tokio::test]
async fn huddle_items_disabled_still_records_the_neighboring_message_mention() {
    let fixture: Value =
        serde_json::from_str(include_str!("huddle_neighbor_mention.json")).unwrap();
    assert_eq!(fixture["reference_pin"], "d7c7de92");
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        jiff::Timestamp::from_second(fixture["now"].as_i64().unwrap()).unwrap(),
    ));
    let test = TestApp::boot_with_huddle_and_clock(
        crate::huddle::Config::from_lookup(|_| None),
        clock.clone(),
    )
    .await
    .expect("WS13b requires the parity seed");
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(2))
        .await;
    let app = test.booted.app;
    let david = campfire_db::fixtures::identify("david");
    let jason = campfire_db::fixtures::identify("jason");
    let room_id = campfire_db::fixtures::identify("david_and_jason");
    app.db.write(move |tx| {
        tx.conn().execute_batch("DELETE FROM huddle_cleanups; DELETE FROM huddle_grants; DELETE FROM activity_items;")?;
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",rusqlite::params![json!({"huddle_invitations":false}).to_string(),jason])?;
        let member = Membership::find_by_room_and_user(tx.conn(),room_id,david)?.unwrap();
        HuddleGrant::issue(tx,campfire_db::fixtures::identify("david_safari"),member.id,room_id,&HuddleConfig{api_secret:Some("ws13b-fixture-api-secret".into()),admin_configured:false})?;
        Ok(())
    }).await.unwrap();
    let count = || {
        app.db.read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM activity_items WHERE user_id=?",
                [jason],
                |r| r.get::<_, i64>(0),
            )?)
        })
    };
    assert_eq!(json!(count().await.unwrap()), fixture["before"]);
    clock.advance(jiff::SignedDuration::from_secs(46));
    app.db
        .write(|tx| campfire_db::models::huddle_invitations::resolve_overdue(tx, None))
        .await
        .unwrap();
    assert_eq!(json!(count().await.unwrap()), fixture["after_timeout"]);
    let input = fixture["message"].clone();
    let body = fixture["body"].as_str().unwrap().to_owned();
    let message = app
        .db
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: input["room_id"].as_i64().unwrap(),
                    creator_id: input["creator_id"].as_i64().unwrap(),
                    client_message_id: Some(
                        input["client_message_id"].as_str().unwrap().to_owned(),
                    ),
                    body: Some(body),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let id = message.id;
    let rich_text = app.db.env().rich_text.clone();
    let text = app
        .db
        .read(move |conn| Message::find(conn, id)?.plain_text_body(conn, rich_text.as_ref()))
        .await
        .unwrap();
    assert_eq!(json!(text), fixture["plain_text"]);
    let item = app
        .db
        .read(move |conn| ActivityItem::find_by_user_and_source(conn, jason, "Message", id))
        .await
        .unwrap()
        .expect("the ordinary mention must survive the huddle-only preference");
    assert_eq!(
        json!({"user_id":item.user_id,"source_type":item.source_type,"event_type":item.event_type}),
        fixture["mention"]
    );
    assert_eq!(
        item.created_at,
        Timestamp::from_second(fixture["now"].as_i64().unwrap() + 46)
    );
    let item_id = item.id;
    app.db
        .write(move |tx| {
            ActivityItem::find(tx.conn(), item_id)?.mark_handled(tx)?;
            let message = Message::find(tx.conn(), id)?;
            campfire_db::models::activity_mentions::record(tx, &message)
        })
        .await
        .unwrap();
    let item = app
        .db
        .read(move |conn| ActivityItem::find(conn, item_id))
        .await
        .unwrap();
    assert_eq!(
        json!({"read_at":item.read_at.map(|t|t.as_second()),"handled_at":item.handled_at.map(|t|t.as_second()),"event_type":item.event_type}),
        fixture["idempotent"]
    );
    for guard in fixture["guards"].as_array().unwrap() {
        let name = guard["name"].as_str().unwrap().to_owned();
        let body = fixture["body"].as_str().unwrap().to_owned();
        let room_id = fixture["message"]["room_id"].as_i64().unwrap();
        let id = app
            .db
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE memberships SET involvement=? WHERE room_id=? AND user_id=?",
                    rusqlite::params![
                        match name.as_str() {
                            "nothing" | "muted" | "invisible" => name.as_str(),
                            _ => "everything",
                        },
                        room_id,
                        jason
                    ],
                )?;
                tx.conn().execute(
                    "UPDATE users SET role=?,status=? WHERE id=?",
                    rusqlite::params![
                        if name == "bot" {
                            campfire_db::Role::Bot
                        } else {
                            campfire_db::Role::Member
                        },
                        if name == "inactive" {
                            campfire_db::Status::Deactivated
                        } else {
                            campfire_db::Status::Active
                        },
                        jason
                    ],
                )?;
                let thread_id = if name.starts_with("thread_") {
                    let thread = campfire_db::ChannelThread::create(
                        tx,
                        campfire_db::NewChannelThread {
                            room_id,
                            creator_id: david,
                            name: Some("Mention guard".into()),
                            ..Default::default()
                        },
                    )?;
                    campfire_db::ThreadMembership::create(
                        tx,
                        thread.id,
                        jason,
                        if name == "thread_nothing" {
                            campfire_db::ThreadInvolvement::Nothing
                        } else {
                            campfire_db::ThreadInvolvement::Mentions
                        },
                    )?;
                    Some(thread.id)
                } else {
                    None
                };
                Message::create(
                    tx,
                    NewMessage {
                        room_id,
                        creator_id: if name == "self" { jason } else { david },
                        client_message_id: Some(format!("ws13b-mention-{name}")),
                        body: Some(body),
                        thread_id,
                        system_note: name == "system_note",
                        streaming: name == "streaming",
                        ..Default::default()
                    },
                )
                .map(|m| m.id)
            })
            .await
            .unwrap();
        let exists = app
            .db
            .read(move |conn| {
                ActivityItem::find_by_user_and_source(conn, jason, "Message", id)
                    .map(|v| v.is_some())
            })
            .await
            .unwrap();
        assert_eq!(json!(exists), guard["recorded"], "mention guard {guard}");
    }
}
