//! Legacy pre-upgrade queued envelopes: actual synchronous emission is covered
//! by the observed Rails differential; retained workers must not replay stale rings.
use super::*;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::huddle_invitations::RingRequest;
use campfire_db::models::room_delete::HuddleConfig;
use campfire_db::{ActivityItem, Job, Membership, Session};
use crate::controllers::presenters::test_support::{TestApp, DAVID, JASON, DIRECT_DAVID_JASON};
use campfire_kit::Crypto;
use campfire_kit::clock::Clock;
use serde_json::{Value, json};
use tokio_tungstenite::tungstenite::client::IntoClientRequest;

fn config() -> HuddleConfig {
    HuddleConfig {api_secret:Some("ws13b-review-fixture-value".into()), admin_configured:false}
}

async fn frames(app: &App, client: &mut crate::channels::tests::support::Client) -> Vec<Value> {
    app.cable.broadcast(&format!("user_{JASON}_activity"), &json!({"matrixMarker":true}));
    let mut result = Vec::new();
    loop {
        let frame: Value = serde_json::from_str(&client.next_text().await).unwrap();
        let message = &frame["message"];
        if message["matrixMarker"] == true { break; }
        if !message["huddleInvitation"].is_null() { result.push(message.clone()); }
    }
    result
}

async fn activity_client(addr: std::net::SocketAddr, signed: &str) -> crate::channels::tests::support::Client {
    let mut request = format!("ws://{addr}/cable").into_client_request().unwrap();
    request.headers_mut().insert("origin", format!("http://{addr}").parse().unwrap());
    request.headers_mut().insert("cookie", format!("session_token={}", campfire_kit::cookies::escape(signed)).parse().unwrap());
    request.headers_mut().insert("sec-websocket-protocol", "actioncable-v1-json".parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = crate::channels::tests::support::Client {socket};
    assert_eq!(serde_json::from_str::<Value>(&client.next_text().await).unwrap()["type"], "welcome");
    client.confirm(&json!({"channel":"ActivityChannel"}).to_string()).await;
    client
}

#[tokio::test]
async fn ring_supersession_and_replacement_enqueue_roll_back_together() {
    for banner in [false, true] {
        let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new("2026-01-01T12:00:00Z".parse().unwrap()));
        let test = TestApp::boot_with_huddle_and_clock(crate::huddle::Config::default(), clock.clone()).await.expect("parity seed required");
        test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
        let app = test.booted.app.clone();
        let grant = app.db.write(move |tx| {
            if banner {tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [JASON])?;}
            let session = Session::start(tx, DAVID, None, None)?;
            let member = Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
            HuddleGrant::issue(tx, session.id, member.id, member.room_id, &config())
        }).await.unwrap();
        app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
        let old = app.db.read(|conn| Ok(campfire_jobs::inspect::all(conn)?.into_iter().find(|row| row.class == RingRequest::CLASS && row.arguments["recipient_id"] == JASON).unwrap())).await.unwrap();
        app.db.write(|tx| {
            tx.conn().execute_batch("CREATE TRIGGER reject_replacement_ring BEFORE INSERT ON background_jobs WHEN NEW.job_class='Notifications::HuddleRingJob' BEGIN SELECT RAISE(ABORT,'reject replacement ring'); END")?;
            Ok(())
        }).await.unwrap();
        clock.advance(jiff::SignedDuration::from_secs(181));
        let result = app.db.write(move |tx| HuddleGrant::issue(tx, grant.session_id, grant.membership_id, grant.room_id, &config())).await;
        assert!(result.is_err(), "replacement enqueue unexpectedly succeeded");
        let id = old.id;
        let current = app.db.read(move |conn| campfire_jobs::inspect::find(conn, id)).await.unwrap().unwrap();
        assert_eq!(current.arguments, old.arguments, "failed replacement invalidated the only pending emission");
        assert_eq!(app.db.read(move |conn| Ok(HuddleGrant::find_by_id(conn, grant.id)?.unwrap().last_issued_at)).await.unwrap(), Some(campfire_db::Timestamp::from_jiff(clock.now())), "issuance must retain its Rails primary commit");
        let viewer = app.db.write(|tx| Session::start_with(tx, JASON, campfire_db::NewSession {two_factor_verified:true,..Default::default()})).await.unwrap();
        let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &viewer.token, None);
        let listener = crate::channels::tests::support::bind_listener().await;
        let addr = listener.local_addr().unwrap();
        let router = app.cable.router::<()>("/cable");
        let server = tokio::spawn(async move {axum::serve(listener, router).await.unwrap()});
        let mut client = activity_client(addr, &signed).await;
        let execution = Execution {id:old.id, executions:1, enqueued_at:old.created_at, scheduled_at:old.run_at};
        ring(app.clone(), Ring(serde_json::from_value(old.arguments.clone()).unwrap()), execution).await.unwrap();
        let mut invitation = old.arguments["invitation"].clone();
        invitation["silent"] = json!(false);
        assert_eq!(frames(&app, &mut client).await, vec![json!({"activityItemId":invitation["activityItemId"],"huddleInvitation":invitation})]);
        client.socket.close(None).await.unwrap();
        server.abort();
        let _ = server.await;
    }
}

async fn reviewer_r4_cross_form_case(banner: bool, newest_first: bool, dismiss_retry: bool) -> Vec<Value> {
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new("2026-01-01T12:00:00Z".parse().unwrap()));
    let test = TestApp::boot_with_huddle_and_clock(crate::huddle::Config::default(), clock.clone()).await.expect("parity seed required");
    test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
    let app = test.booted.app.clone();
    let grant = app.db.write(move |tx| {
        tx.conn().execute("DELETE FROM activity_items", [])?;
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?", rusqlite::params![if banner {r#"{"huddle_invitations":false}"#} else {r#"{"huddle_invitations":true}"#}, JASON])?;
        let session = Session::start(tx, DAVID, None, None)?;
        let member = Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
        HuddleGrant::issue(tx, session.id, member.id, member.room_id, &config())
    }).await.unwrap();
    app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
    let retained = app.db.read(|conn| Ok(campfire_jobs::inspect::all(conn)?.into_iter().find(|row| row.class == RingRequest::CLASS && row.arguments["recipient_id"] == JASON).unwrap())).await.unwrap();
    clock.advance(jiff::SignedDuration::from_secs(181));
    app.db.write(move |tx| {
        tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?", rusqlite::params![if banner {r#"{"huddle_invitations":true}"#} else {r#"{"huddle_invitations":false}"#}, JASON])?;
        HuddleGrant::issue(tx, grant.session_id, grant.membership_id, grant.room_id, &config())
    }).await.unwrap();
    if dismiss_retry {
        assert!(banner);
        clock.advance(jiff::SignedDuration::from_secs(1));
        app.db.write(move |tx| {
            ActivityItem::find_by_user_and_source(tx.conn(), JASON, "HuddleGrant", grant.id)?.unwrap().mark_read(tx)?;
            Ok(())
        }).await.unwrap();
    }
    app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
    let mut pending = app.db.read(|conn| Ok(campfire_jobs::inspect::all(conn)?.into_iter().filter(|row| row.class == RingRequest::CLASS && row.arguments["recipient_id"] == JASON).collect::<Vec<_>>())).await.unwrap();
    // Use arguments captured before later updates, as an already-claimed worker does.
    pending.iter_mut().find(|row| row.id == retained.id).unwrap().arguments = retained.arguments;
    pending.sort_by_key(|row| row.id);
    if newest_first {pending.reverse();}
    let viewer = app.db.write(|tx| Session::start_with(tx, JASON, campfire_db::NewSession {two_factor_verified:true,..Default::default()})).await.unwrap();
    let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &viewer.token, None);
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.cable.router::<()>("/cable");
    let server = tokio::spawn(async move {axum::serve(listener, router).await.unwrap()});
    let mut client = activity_client(addr, &signed).await;
    for row in pending {
        let execution = Execution {id:row.id, executions:1, enqueued_at:row.created_at, scheduled_at:row.run_at};
        ring(app.clone(), Ring(serde_json::from_value(row.arguments).unwrap()), execution).await.unwrap();
        app.db.write(move |tx| {tx.conn().execute("DELETE FROM background_jobs WHERE id=?", [row.id])?; Ok(())}).await.unwrap();
    }
    let delivered = frames(&app, &mut client).await;
    client.socket.close(None).await.unwrap();
    server.abort();
    let _ = server.await;
    println!("cross_form banner={banner} newest_first={newest_first} dismiss_retry={dismiss_retry}: {}", json!(delivered));
    delivered
}

#[tokio::test]
async fn reviewer_r4_cross_form_retry_must_keep_only_the_current_invitation() {
    let mut failures = Vec::new();
    for banner in [false, true] {for newest_first in [false, true] {
        let delivered = reviewer_r4_cross_form_case(banner, newest_first, false).await;
        if delivered.len() != 1 {failures.push((banner,newest_first,delivered.len()));}
    }}
    assert!(failures.is_empty(), "same caller/grant retry produced multiple current rings: {failures:?}");
}

#[tokio::test]
async fn reviewer_r4_dismissed_retry_must_not_revive_its_old_banner() {
    let mut failures = Vec::new();
    for newest_first in [false, true] {
        let delivered = reviewer_r4_cross_form_case(true, newest_first, true).await;
        let starts = delivered.iter().filter(|f| f["huddleInvitation"]["state"] == "unread").count();
        if starts != 0 {failures.push((newest_first, starts));}
    }
    assert!(failures.is_empty(), "dismissed retry still emits old unread banner: {failures:?}");
}

#[tokio::test]
async fn reviewer_r4_call_end_must_preserve_pending_handled_notification() {
    let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new("2026-01-01T12:00:00Z".parse().unwrap()));
    let test = TestApp::boot_with_huddle_and_clock(crate::huddle::Config::default(), clock.clone()).await.expect("parity seed required");
    test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
    let app = test.booted.app.clone();
    let grant = app.db.write(|tx| {
        tx.conn().execute("DELETE FROM activity_items", [])?;
        let session = Session::start(tx, DAVID, None, None)?;
        let member = Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
        HuddleGrant::issue(tx, session.id, member.id, member.room_id, &config())
    }).await.unwrap();
    let viewer = app.db.write(|tx| Session::start_with(tx, JASON, campfire_db::NewSession {two_factor_verified:true,..Default::default()})).await.unwrap();
    let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &viewer.token, None);
    let listener = crate::channels::tests::support::bind_listener().await;
    let addr = listener.local_addr().unwrap();
    let router = app.cable.router::<()>("/cable");
    let server = tokio::spawn(async move {axum::serve(listener, router).await.unwrap()});
    let mut client = activity_client(addr, &signed).await;
    app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
    let initial = app.db.read(|conn| Ok(campfire_jobs::inspect::all(conn)?.into_iter().find(|row| row.class == RingRequest::CLASS && row.arguments["recipient_id"] == JASON).unwrap())).await.unwrap();
    let execution = Execution {id:initial.id, executions:1, enqueued_at:initial.created_at, scheduled_at:initial.run_at};
    ring(app.clone(), Ring(serde_json::from_value(initial.arguments).unwrap()), execution).await.unwrap();
    app.db.write(move |tx| {tx.conn().execute("DELETE FROM background_jobs WHERE id=?", [initial.id])?; Ok(())}).await.unwrap();
    let first_frames = frames(&app, &mut client).await;
    println!("handled_then_end initial: {}", json!(first_frames));
    assert_eq!(first_frames.len(), 1);
    clock.advance(jiff::SignedDuration::from_secs(1));
    app.db.write(move |tx| {
        ActivityItem::find_by_user_and_source(tx.conn(), JASON, "HuddleGrant", grant.id)?.unwrap().mark_handled(tx)?;
        Ok(())
    }).await.unwrap();
    app.db.write(|tx| {tx.conn().execute("UPDATE background_jobs SET arguments=json_remove(arguments,'$.delivered') WHERE job_class='Notifications::HuddleRingJob'", [])?; Ok(())}).await.unwrap();
    let _synchronous_handled = frames(&app, &mut client).await;
    let handled = app.db.read(|conn| Ok(campfire_jobs::inspect::all(conn)?.into_iter().find(|row| row.class == RingRequest::CLASS && row.arguments["recipient_id"] == JASON).unwrap())).await.unwrap();
    assert_eq!(handled.arguments["invitation"]["state"], "handled");
    clock.advance(jiff::SignedDuration::from_secs(1));
    app.db.write(move |tx| {
        let mut current = HuddleGrant::find_by_id(tx.conn(), grant.id)?.unwrap();
        current.record_seen(tx)?;
        current.mark_out_of_call(tx, None)?;
        Ok(())
    }).await.unwrap();
    assert!(frames(&app, &mut client).await.is_empty(), "handled item receives no ended notification");
    let execution = Execution {id:handled.id, executions:1, enqueued_at:handled.created_at, scheduled_at:handled.run_at};
    ring(app.clone(), Ring(serde_json::from_value(handled.arguments).unwrap()), execution).await.unwrap();
    let delivered = frames(&app, &mut client).await;
    client.socket.close(None).await.unwrap();
    server.abort();
    let _ = server.await;
    println!("handled_then_end delivered: {}", json!(delivered));
    assert_eq!(delivered.len(), 1, "call end discarded the only handled activity notification");
    assert_eq!(delivered[0]["huddleInvitation"]["state"], "handled");
}
