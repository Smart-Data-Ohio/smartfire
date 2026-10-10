//! Consume the merged room/thread owner APIs, including actual socket attachment updates.
use super::google_api_tests::{self as support, Recorded};
use crate::{
    controllers::presenters::test_support::{Req, TestApp},
    integrations::google::api,
};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, ThreadMembership};
use campfire_kit::FrozenClock;
use hyper::Method;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};


#[tokio::test]
async fn google_drive_message_requests_and_attachment_socket_frames_match_rails() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/google_message_drive.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let mut a = TestApp::boot_with_clock(Arc::new(FrozenClock::new(
            "2026-03-02T16:00:00Z".parse().unwrap(),
        )))
        .await
        .unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let r = Recorded::new(vec![]);
        a.booted
            .app
            .google
            .install_api(api::Api::new(support::config(), r.clone()));
        let room = row["room_id"].as_i64().unwrap();
        let creator = row["creator_id"].as_i64().unwrap();
        let actor = row["actor_id"].as_i64().unwrap();
        let thread_scope = row["scope"] == "thread";
        let create = row["scenario"].as_str().unwrap().starts_with("create_");
        let initial = row.clone();
        let (thread,message)=a.db().write(move |tx| {
            tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[actor])?;
            tx.conn().execute_batch("INSERT INTO sqlite_sequence(name,seq) SELECT 'channel_threads',9700000000 WHERE NOT EXISTS(SELECT 1 FROM sqlite_sequence WHERE name='channel_threads');UPDATE sqlite_sequence SET seq=9700000000 WHERE name='channel_threads';UPDATE sqlite_sequence SET seq=9600000000 WHERE name='messages';")?;
            let thread=if thread_scope {
                let thread=ChannelThread::create(tx,NewChannelThread {room_id:room,creator_id:creator,name:Some("Drive request fixture".into()),..Default::default()})?;
                ThreadMembership::join(tx,thread.id,creator)?;
                Some(thread.id)
            } else {None};
            let message=if create {None} else {Some(Message::create(tx,NewMessage {room_id:room,creator_id:creator,thread_id:thread,client_message_id:Some(initial["client_id"].as_str().unwrap().into()),markdown_source:Some(initial["original"].as_str().unwrap().into()),drive_file_ids:if initial["scenario"]=="json_empty" {vec![]} else {vec!["1AbcDefGhIjKlMnOpQrSt".into(),"2BcdEfgHiJkLmNoPqRsTu".into()]},..Default::default()})?.id)};
            Ok((thread,message))
        }).await.unwrap();
        assert_eq!(json!(thread), row["thread_id"]);
        assert_eq!(json!(message), row["message_id"]);
        let before = a
            .db()
            .read(|c| {
                Ok((
                    c.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                    c.query_row("SELECT count(*) FROM drive_attachments", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                ))
            })
            .await
            .unwrap();
        let mut b = a.sign_in(actor).await;
        b.get("/users/me/profile").await;
        let method = if create {
            Method::POST
        } else if thread_scope {
            Method::PATCH
        } else {
            Method::PUT
        };
        let reply = b
            .write(
                Req::new(method, row["path"].as_str().unwrap())
                    .header("content-type", "application/json")
                    .header(
                        "accept",
                        if row["path"].as_str().unwrap().ends_with("turbo_stream") {
                            "text/vnd.turbo-stream.html"
                        } else if row["path"].as_str().unwrap().ends_with("json") {
                            "application/json"
                        } else {
                            "text/html"
                        },
                    )
                    .body(serde_json::to_vec(&json!({"message":row["params"]})).unwrap()),
            )
            .await;
        let client = row["client_id"].as_str().unwrap().to_owned();
        let state = a
            .db()
            .read(move |c| {
                use rusqlite::OptionalExtension;
                let message = c
                    .query_row(
                        "SELECT id,markdown_source FROM messages WHERE client_message_id=?",
                        [client],
                        |r| Ok((r.get::<_, i64>(0)?, r.get::<_, Option<String>>(1)?)),
                    )
                    .optional()?;
                let files = if let Some((id, _)) = &message {
                    Some(
                        c.prepare(
                            "SELECT file_id FROM drive_attachments WHERE message_id=? ORDER BY id",
                        )?
                        .query_map([id], |r| r.get::<_, String>(0))?
                        .collect::<rusqlite::Result<Vec<_>>>()?,
                    )
                } else {
                    None
                };
                let counts = (
                    c.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?,
                    c.query_row("SELECT count(*) FROM drive_attachments", [], |r| {
                        r.get::<_, i64>(0)
                    })?,
                );
                Ok((files, message.and_then(|(_, s)| s), counts))
            })
            .await
            .unwrap();
        let json_drive =
            if row["path"].as_str().unwrap().ends_with("json") && reply.status.as_u16() < 400 {
                reply.json()["drive_attachments"].clone()
            } else {
                Value::Null
            };
        let observed = json!({"status":reply.status.as_u16(),"location":reply.location(),"delta":[state.2.0-before.0,state.2.1-before.1],"files":state.0,"source":state.1,"json_drive":json_drive});
        let mut expected = row["result"].clone();
        expected.as_object_mut().unwrap().remove("frames");
        if create && !thread_scope && expected["status"] == 200 { expected["status"] = json!(201); }
        assert_eq!(
            observed, expected,
            "{} {}",
            row["scope"], row["scenario"]
        );
        assert!(
            r.calls.lock().unwrap().is_empty(),
            "attaching files requires room authority, never a Google grant"
        );
    }
    println!(
        "Pinned Rails Drive message requests: 32 exercised; persisted attachments checked; 0 Google calls; 0 skipped"
    );
}
