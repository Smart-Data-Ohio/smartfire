//! The merged registry must consume real ring intents and deliver exact Rails frames.
use crate::controllers::presenters::test_support::TestApp;
use campfire_db::models::huddle_invitations::RingRequest;
use campfire_db::{Event, Session};
use campfire_kit::Crypto;
use futures_util::{SinkExt, StreamExt};
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};
use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};

#[tokio::test]
async fn registered_ring_worker_uses_current_ws17_policy_and_exact_rails_frames() {
    let fixture: Value = serde_json::from_str(include_str!(
        "../../../db/src/tests/huddle_ring_policy_seam.json"
    )).unwrap();
    let now = fixture["cases"][0]["outcomes"][0]["context"]["now"].as_i64().unwrap();
    let test = TestApp::boot_with_clock(Arc::new(campfire_kit::clock::FrozenClock::new(
        jiff::Timestamp::from_second(now).unwrap(),
    ))).await.expect("WS13b requires the parity seed");
    let app = test.booted.app.clone();
    let user = campfire_db::fixtures::identify("jason");
    let grant_id = app.db.write(|tx| {
        let caller = campfire_db::fixtures::identify("david");
        let room = campfire_db::fixtures::identify("david_and_jason");
        let session = Session::start(tx, caller, None, None)?;
        let member = campfire_db::Membership::find_by_room_and_user(tx.conn(), room, caller)?.unwrap();
        Ok(tx.conn().query_row("INSERT INTO huddle_grants(identity,room_name,session_id,user_id,membership_id,room_id,last_issued_at,created_at,updated_at) VALUES('ws13b-policy-source','ws13b-policy-room',?,?,?,?,?,?,?) RETURNING id",rusqlite::params![session.id,caller,member.id,room,tx.now(),tx.now(),tx.now()],|r|r.get::<_,i64>(0))?)
    }).await.unwrap();
    let session = app.db.write(move |tx| Session::start_with(tx, user, campfire_db::NewSession {
        two_factor_verified: true, ..Default::default()
    })).await.unwrap();
    let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &session.token, None);
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.cable.router::<()>("/cable");
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert("origin", format!("http://{addr}").parse().unwrap());
    request.headers_mut().insert("cookie", format!("session_token={}", campfire_kit::cookies::escape(&signed)).parse().unwrap());
    request.headers_mut().insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
    let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    macro_rules! next {
        () => {
            tokio::time::timeout(Duration::from_secs(3), async {
                loop {
                    if let Message::Text(text) = socket.next().await.expect("socket closed").unwrap() {
                        let value: Value = serde_json::from_str(&text).unwrap();
                        if value["type"] != "ping" { break value; }
                    }
                }
            }).await.expect("ring worker did not deliver a frame")
        };
    }
    assert_eq!(next!()["type"], "welcome");
    let identifier = json!({"channel":"ActivityChannel"}).to_string();
    socket.send(Message::Text(json!({"command":"subscribe","identifier":identifier}).to_string().into())).await.unwrap();
    assert_eq!(next!()["type"], "confirm_subscription");
    let mut outcomes = 0;
    for case in fixture["cases"].as_array().unwrap() {
        // The per-call quiet-check override is exercised by both persisted recipients
        // in the domain replay. Production has no global test override.
        if case["name"] == "quiet_override" { continue; }
        for outcome in case["outcomes"].as_array().unwrap() {
            let sql = outcome["setup_sql"].as_str().unwrap().to_owned();
            let mut invitation = outcome["broadcast"]["payload"]["huddleInvitation"].clone();
            invitation.as_object_mut().unwrap().remove("silent");
            let sender = outcome["sender_id"].as_i64().unwrap();
            let intent = RingRequest { recipient_id: user, sender_id: sender, grant_id:Some(grant_id), invitation };
            app.db.write(move |tx| {
                tx.conn().execute_batch(&sql)?;
                tx.conn().execute("UPDATE huddle_grants SET user_id=? WHERE id=?", rusqlite::params![sender, grant_id])?;
                tx.emit_after_commit(Event::job(&intent));
                Ok(())
            }).await.unwrap();
            let frame = next!();
            assert_eq!(frame["identifier"], identifier);
            assert_eq!(frame["message"], outcome["broadcast"]["payload"], "{}", case["title"]);
            outcomes += 1;
        }
    }
    assert_eq!(outcomes, 11);
    test.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let rows = app.db.read(campfire_jobs::inspect::all).await.unwrap();
    assert!(!rows.iter().any(|row| row.class == "Notifications::HuddleRingJob"), "ring intents were not acknowledged: {rows:?}");
    socket.close(None).await.unwrap();
    server.abort();
    let _ = server.await;
}
