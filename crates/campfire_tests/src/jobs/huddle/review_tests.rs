use super::*;
use campfire_jobs::Execution;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::huddle_invitations::RingRequest;
use campfire_db::{Job, Membership, Session};
use crate::controllers::presenters::test_support::{TestApp, DAVID, JASON, DIRECT_DAVID_JASON};

#[tokio::test]
async fn ws13b_review_queued_and_already_claimed_rings_cannot_follow_explicit_leave() {
    for suppressed in [false, true] {
        let test = TestApp::boot().await.expect("parity seed required");
        test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
        let app = test.booted.app.clone();
        let grant = app.db.write(move |tx| {
            if suppressed { tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [JASON])?; }
            let session = Session::start(tx, DAVID, None, None)?;
            let member = Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
            HuddleGrant::issue(tx, session.id, member.id, member.room_id, &campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13b-review-fixture-value".into()), admin_configured:false})
        }).await.unwrap();
        app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
        let rows = app.db.read(campfire_jobs::inspect::all).await.unwrap();
        let queued = rows.iter().find(|j| j.class == RingRequest::CLASS).unwrap();
        let retained = serde_json::from_value::<RingRequest>(queued.arguments.clone()).unwrap();
        let execution = Execution { id: queued.id, executions:1, enqueued_at:queued.created_at, scheduled_at:queued.run_at };
        let id = grant.id;
        app.db.write(move |tx| {
            let mut grant = HuddleGrant::find_by_id(tx.conn(), id)?.unwrap();
            grant.record_seen(tx)?;
            grant.mark_out_of_call(tx, None)?;
            Ok(())
        }).await.unwrap();
        // Subscribe after the ended frame. A marker detects extra frames
        // deterministically, including a worker retaining its pre-leave payload.
        use campfire_kit::Crypto;
        use tokio_tungstenite::tungstenite::client::IntoClientRequest;
        let session = app.db.write(|tx| Session::start_with(tx, JASON, campfire_db::NewSession { two_factor_verified:true, ..Default::default() })).await.unwrap();
        let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &session.token, None);
        let listener = crate::channels::tests::support::bind_listener().await;
        let addr = listener.local_addr().unwrap();
        let router = app.cable.router::<()>("/cable");
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
        request.headers_mut().insert("origin", format!("http://{addr}").parse().unwrap());
        request.headers_mut().insert("cookie", format!("session_token={}", campfire_kit::cookies::escape(&signed)).parse().unwrap());
        request.headers_mut().insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
        let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let mut client = crate::channels::tests::support::Client { socket };
        assert_eq!(serde_json::from_str::<serde_json::Value>(&client.next_text().await).unwrap()["type"], "welcome");
        let identifier = serde_json::json!({"channel":"ActivityChannel"}).to_string();
        client.confirm(&identifier).await;
        let stream = format!("user_{JASON}_activity");
        ring(app.clone(), Ring(retained), execution.clone()).await.unwrap();
        app.cable.broadcast(&stream, &serde_json::json!({"reviewMarker":true}));
        let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
        assert_eq!(frame["message"], serde_json::json!({"reviewMarker":true}), "suppressed={suppressed}: claimed ring restarted an ended call");
        let execution_id = execution.id;
        let queued = app.db.read(move |conn| campfire_jobs::inspect::find(conn, execution_id)).await.unwrap().unwrap();
        ring(app.clone(), Ring(serde_json::from_value(queued.arguments).unwrap()), execution).await.unwrap();
        app.cable.broadcast(&stream, &serde_json::json!({"reviewMarker":true}));
        let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
        assert_eq!(frame["message"], serde_json::json!({"reviewMarker":true}));
        client.socket.close(None).await.unwrap();
        server.abort();
        let _ = server.await;
    }
}
