//! Each operation's enqueue decisions, immediate frames, durable item state and
//! delayed real-handler output are compared with independently executed Rails.
use super::*;
use campfire_db::models::huddle_grant::HuddleGrant;
use campfire_db::models::huddle_invitations::{self, RingRequest};
use campfire_db::models::huddle_notices::PushInvitationJob;
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

async fn run_matrix(banner: bool, newest_first: bool) {
    let oracle: Value = serde_json::from_str(include_str!("ring_matrix.json")).unwrap();
    assert_eq!(oracle["reference_pin"], "d7c7de92");
    assert_eq!(oracle["cases"].as_array().unwrap().len(), 180);
    let cases = oracle["cases"].as_array().unwrap().iter().filter(|case| case["spec"]["banner"] == banner && case["spec"]["newest_first"] == newest_first);
    let mut count = 0;
    for case in cases {
        let name = case["spec"]["name"].as_str().unwrap();
        let clock = std::sync::Arc::new(campfire_kit::clock::FrozenClock::new("2026-01-01T12:00:00Z".parse().unwrap()));
        let test = TestApp::boot_with_huddle_and_clock(crate::huddle::Config::default(), clock.clone()).await.expect("parity seed required");
        // Pause only this private application's runner to interleave its jobs.
        test.booted.jobs.shutdown(std::time::Duration::from_secs(2)).await;
        let app = test.booted.app.clone();
        let (mut session, member) = app.db.write(move |tx| {
            // Match Rails fresh-schema fixtures, rather than seeded demo inbox rows.
            tx.conn().execute("DELETE FROM activity_items", [])?;
            tx.conn().execute("UPDATE users SET inbox_preferences='{}' WHERE id=?", [DAVID])?;
            tx.conn().execute("DELETE FROM sqlite_sequence WHERE name='activity_items'", [])?;
            if banner { tx.conn().execute(r#"UPDATE users SET inbox_preferences='{"huddle_invitations":false}' WHERE id=?"#, [JASON])?; }
            let session = Session::start(tx, DAVID, None, None)?;
            let member = Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, DAVID)?.unwrap();
            Ok((session.id, member.id))
        }).await.unwrap();
        let viewer = app.db.write(|tx| Session::start_with(tx, JASON, campfire_db::NewSession {two_factor_verified:true,..Default::default()})).await.unwrap();
        let signed = campfire_kit::RailsCrypto::new(app.secrets.clone()).sign_cookie("session_token", &viewer.token, None);
        let listener = crate::channels::tests::support::bind_listener().await;
        let addr = listener.local_addr().unwrap();
        let router = app.cable.router::<()>("/cable");
        let server = tokio::spawn(async move {axum::serve(listener, router).await.unwrap()});
        let mut client = activity_client(addr, &signed).await;
        let mut grant_id = 0;
        let mut latest_job = 0;
        // Preserve worker arguments at enqueue, including those later marked
        // cancelled/superseded: the handler must read its authoritative row.
        let mut pending: Vec<campfire_jobs::inspect::JobRow> = Vec::new();
        for (index, step) in case["spec"]["steps"].as_array().unwrap().iter().enumerate() {
            clock.advance(jiff::SignedDuration::from_secs(step["seconds"].as_i64().unwrap()));
            let action = step["action"].as_str().unwrap();
            let mut delivered = Vec::new();
            if action == "drain" {
                pending.sort_by_key(|row| row.id);
                if newest_first { pending.reverse(); }
                for row in pending.drain(..) {
                    let execution = Execution {id:row.id, executions:1, enqueued_at:row.created_at, scheduled_at:row.run_at};
                    ring(app.clone(), Ring(serde_json::from_value(row.arguments).unwrap()), execution).await.unwrap();
                    app.db.write(move |tx| {tx.conn().execute("DELETE FROM background_jobs WHERE id=?", [row.id])?; Ok(())}).await.unwrap();
                }
                delivered = frames(&app, &mut client).await;
            } else if action == "new_session" {
                session = app.db.write(|tx| Ok(Session::start(tx, DAVID, None, None)?.id)).await.unwrap();
                grant_id = app.db.write(move |tx| HuddleGrant::issue(tx, session, member, DIRECT_DAVID_JASON, &config())).await.unwrap().id;
            } else if action == "group_continues" {
                let live_id = app.db.write(|tx| {
                    let other = campfire_db::fixtures::identify("kevin");
                    let membership = Membership::create_default(tx, DIRECT_DAVID_JASON, other)?;
                    let session = Session::start(tx, other, None, None)?;
                    Ok(HuddleGrant::issue(tx, session.id, membership.id, DIRECT_DAVID_JASON, &config())?.id)
                }).await.unwrap();
                // Match Rails issue!'s completed post-commit invitation before
                // the next record_seen!/revoke! operations change call membership.
                app.db.write(move |tx| {
                    HuddleGrant::find_by_id(tx.conn(), live_id)?.unwrap().record_seen(tx)?;
                    let mut grant = HuddleGrant::find_by_id(tx.conn(), grant_id)?.unwrap();
                    grant.record_seen(tx)?;
                    grant.revoke(tx, false, &config())?;
                    Ok(())
                }).await.unwrap();
            } else {
                let operation = action.to_owned();
                let next = app.db.write(move |tx| {
                    match operation.as_str() {
                        "issue" => return Ok(Some(HuddleGrant::issue(tx, session, member, DIRECT_DAVID_JASON, &config())?.id)),
                        "quiet_revoke" => {HuddleGrant::find_by_id(tx.conn(), grant_id)?.unwrap().revoke(tx, false, &config())?;}
                        "live_revoke" | "end" | "remove_caller" | "sign_out" => {
                            let mut grant = HuddleGrant::find_by_id(tx.conn(), grant_id)?.unwrap();
                            grant.record_seen(tx)?;
                            match operation.as_str() {
                                "live_revoke" => {grant.revoke(tx, false, &config())?;}
                                "end" => {grant.mark_out_of_call(tx, None)?;}
                                "remove_caller" => {Membership::find(tx.conn(), member)?.destroy(tx)?;}
                                "sign_out" => {Session::find(tx.conn(), session)?.destroy(tx)?;}
                                _ => unreachable!(),
                            }
                        }
                        "remove_recipient" => {Membership::find_by_room_and_user(tx.conn(), DIRECT_DAVID_JASON, JASON)?.unwrap().destroy(tx)?;}
                        "read" | "handled" | "missed" | "unread_cycle" => {
                            let item = ActivityItem::find_by_user_and_source(tx.conn(), JASON, "HuddleGrant", grant_id)?.unwrap();
                            match operation.as_str() {
                                "read" => {item.mark_read(tx)?;}
                                "handled" => {item.mark_handled(tx)?;}
                                "missed" => {huddle_invitations::resolve_overdue(tx, Some(JASON))?;}
                                "unread_cycle" => {
                                    item.mark_handled(tx)?;
                                    ActivityItem::refresh_unread(tx, JASON, "HuddleGrant", grant_id, "huddle_started")?;
                                }
                                _ => unreachable!(),
                            }
                        }
                        _ => panic!("unknown matrix operation {operation}"),
                    }
                    Ok(None)
                }).await.unwrap();
                if let Some(next) = next {grant_id = next;}
            }
            let mut immediate = Vec::new();
            if action == "remove_recipient" {
                // Membership destruction really resets this user's sockets.
                // Observe that boundary, then reconnect to detect any leaked ring.
                loop {
                    let frame: Value = serde_json::from_str(&client.next_text().await).unwrap();
                    if frame["type"] == "disconnect" {break;}
                    if !frame["message"]["huddleInvitation"].is_null() {immediate.push(frame["message"].clone());}
                }
                client = activity_client(addr, &signed).await;
            }
            immediate.extend(frames(&app, &mut client).await);
            let new_rows = app.db.read(move |conn| Ok(campfire_jobs::inspect::all(conn)?.into_iter().filter(|row| row.id > latest_job).collect::<Vec<_>>())).await.unwrap();
            let mut emissions = Vec::new();
            let mut pushes = Vec::new();
            for row in new_rows {
                latest_job = latest_job.max(row.id);
                if row.class == RingRequest::CLASS && row.arguments["recipient_id"] == JASON {
                    let request: RingRequest = serde_json::from_value(row.arguments.clone()).unwrap();
                    let now = campfire_db::Timestamp::from_jiff(clock.now());
                    let sound = app.db.read(move |conn| huddle_invitations::ring_allowed(conn, JASON, Some(request.sender_id), now, None)).await.unwrap();
                    let mut invitation = row.arguments["invitation"].clone();
                    invitation["silent"] = json!(!sound);
                    emissions.push(json!({"activityItemId":invitation["activityItemId"], "huddleInvitation":invitation}));
                    pending.push(row);
                } else if row.class == PushInvitationJob::CLASS {
                    pushes.push(row.arguments["activity_item_id"].clone());
                }
            }
            let items = app.db.read(|conn| {
                let mut statement = conn.prepare("SELECT id,source_id,event_type,read_at,handled_at,created_at FROM activity_items WHERE user_id=? AND source_type='HuddleGrant' ORDER BY id")?;
                let items = statement.query_map([JASON], |row| {
                    let read: Option<campfire_db::Timestamp> = row.get(3)?;
                    let handled: Option<campfire_db::Timestamp> = row.get(4)?;
                    let created: campfire_db::Timestamp = row.get(5)?;
                    Ok(json!({"id":row.get::<_,i64>(0)?,"source_id":row.get::<_,i64>(1)?,"event_type":row.get::<_,String>(2)?,
                        "state":if handled.is_some(){"handled"}else if read.is_some(){"read"}else{"unread"},
                        "created_at":format!("{:.6}", created.jiff())}))
                })?.collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(items)
            }).await.unwrap();
            let actual = json!({"emissions":emissions,"immediate":immediate,"pushes":pushes,"delivered":delivered,"items":items});
            assert_eq!(actual, case["phases"][index], "{name} phase {index} {action}");
        }
        client.socket.close(None).await.unwrap();
        server.abort();
        let _ = server.await;
        count += 1;
    }
    assert_eq!(count, if banner {39} else {51});
    println!("Rails ring sequence matrix: {count} cases passed; banner={banner} newest_first={newest_first}");
}

#[tokio::test]
async fn rails_ring_matrix_items_oldest_first() {run_matrix(false, false).await;}
#[tokio::test]
async fn rails_ring_matrix_items_newest_first() {run_matrix(false, true).await;}
#[tokio::test]
async fn rails_ring_matrix_banners_oldest_first() {run_matrix(true, false).await;}
#[tokio::test]
async fn rails_ring_matrix_banners_newest_first() {run_matrix(true, true).await;}

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
