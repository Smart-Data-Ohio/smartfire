//! Actual production sink/rendering over a socket, including a rejected after-commit ledger.
use crate::channels::tests::support::{Client, bind_listener};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, DAVID, TestApp, david_cookie,
};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage, Room};
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

struct Listener(tokio::task::JoinHandle<()>);
impl Drop for Listener {
    fn drop(&mut self) {
        self.0.abort();
    }
}

#[tokio::test]
async fn ws11_deletion_indicator_socket_precedes_failed_ledger_without_webhook() {
    deletion_frames(false).await;
}

#[tokio::test]
async fn ws11_deletion_indicator_socket_precedes_failed_ledger_with_webhook() {
    deletion_frames(true).await;
}

async fn deletion_frames(webhook: bool) {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/agents_deletion_indicator_contract.json"
    ))
    .unwrap();
    for reject in [false, true] {
        let (app, _dir) = TestApp::boot_frozen()
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let key = format!("webhook_{webhook}_reject_{reject}");
        let client_id = format!("ws11-deletion-indicator-{key}");
        let target = format!("thread_indicator_message_{client_id}");
        let (parent_id, thread_id) = app.db.write(move |tx| {
            tx.conn().execute("DELETE FROM agent_grants", [])?;
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BENDER])?;
            if !webhook { tx.conn().execute("DELETE FROM webhooks WHERE user_id=?", [BENDER])?; }
            let parent = Message::create(tx, NewMessage {
                room_id: ALL_TALK, creator_id: DAVID, client_message_id: Some(client_id),
                markdown_source: Some("Parent".into()), ..Default::default()
            })?;
            let mut thread = ChannelThread::create(tx, NewChannelThread {
                room_id: ALL_TALK, creator_id: DAVID, parent_message_id: Some(parent.id),
                name: Some("Indicator deletion".into()), work_status: Some("planned".into()),
                ..Default::default()
            })?;
            tx.conn().execute("UPDATE channel_threads SET work_owner_id=? WHERE id=?", rusqlite::params![BENDER,thread.id])?;
            thread.post_message(tx, DAVID, NewMessage {markdown_source: Some("Reply".into()),..Default::default()})?;
            tx.conn().execute("DELETE FROM agent_events", [])?;
            if reject {
                tx.conn().execute_batch(&format!("CREATE TEMP TRIGGER ws11_reject_indicator_ledger BEFORE INSERT ON agent_events WHEN NEW.event_type='work_unassigned' AND json_extract(NEW.metadata,'$.thread_id')={} BEGIN SELECT RAISE(ABORT,'WS11 rejected indicator ledger'); END",thread.id))?;
            }
            Ok((parent.id,thread.id))
        }).await.unwrap();
        let listener = bind_listener().await;
        let addr = listener.local_addr().unwrap();
        let router = app.cable.router::<()>("/cable");
        let _listener = Listener(tokio::spawn(async move {
            axum::serve(listener, router).await.unwrap();
        }));
        let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
        request.headers_mut().insert(
            "origin",
            format!(
                "{}://{addr}",
                if app.cable.config().assume_ssl {
                    "https"
                } else {
                    "http"
                }
            )
            .parse()
            .unwrap(),
        );
        request
            .headers_mut()
            .insert("cookie", david_cookie().parse().unwrap());
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "actioncable-v1-json".parse().unwrap(),
        );
        let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let mut client = Client { socket };
        assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
        let room = app
            .db
            .read(|conn| Room::find(conn, ALL_TALK))
            .await
            .unwrap();
        let gid = crate::channels::room_gid(&room).to_param();
        let stream = format!("{gid}:messages");
        let signed = rails_compat::turbo::signed_stream_name(&app.secrets, &[&gid, "messages"]);
        let identifier =
            json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
        client.confirm(&identifier).await;
        let result = app
            .db
            .write(move |tx| ChannelThread::find(tx.conn(), thread_id)?.destroy(tx))
            .await;
        assert_eq!(result.is_err(), reject);
        let state = app.db.read(move |conn| Ok(json!({
            "error": if result.is_err() {Some("statement_invalid")} else {None},
            "thread_exists": ChannelThread::find_by_id(conn,thread_id)?.is_some(),
            "parent_exists": Message::find_by_id(conn,parent_id)?.is_some(),
            "replies_remaining": conn.query_row("SELECT COUNT(*) FROM messages WHERE thread_id=?",[thread_id],|r|r.get::<_,i64>(0))?,
            "deletion_events": conn.query_row("SELECT COUNT(*) FROM agent_events WHERE event_type='work_unassigned'",[],|r|r.get::<_,i64>(0))?
        }))).await.unwrap();
        // FIFO on this same stream proves all earlier frames arrived without a silent timeout.
        app.broadcasts
            .channel(&stream, &json!({"ws11_indicator_barrier":true}));
        let mut frames = Vec::new();
        loop {
            let envelope: Value = serde_json::from_str(&client.next_text().await).unwrap();
            assert_eq!(envelope["identifier"], identifier);
            if envelope["message"]["ws11_indicator_barrier"] == true {
                break;
            }
            if let Some(html) = envelope["message"]
                .as_str()
                .filter(|html| html.contains(&target))
            {
                frames.push(html.to_owned());
            }
        }
        let mut expected = oracle["results"][&key].clone();
        let expected_frames = expected
            .as_object_mut()
            .unwrap()
            .remove("indicator_frames")
            .unwrap();
        assert_eq!(state, expected, "{key}: committed state");
        assert_eq!(
            json!(frames),
            expected_frames,
            "{key}: rendered frames match pinned Rails before the failed ledger"
        );
    }
}
