//! Named WebhookTest cases use the real app, HTTP parser and reply writers.
use super::super::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use super::*;
use crate::controllers::presenters::test_support::{ALL_TALK, BENDER, DAVID, TestApp};
use campfire_db::{ChannelThread, NewChannelThread, NewMessage, RoomType};
use std::{
    collections::HashSet,
    sync::{Arc, Mutex},
};
async fn setup(shape: &str, legacy: bool, locked: bool) -> (TestApp, AgentEvent, i64) {
    let t = TestApp::boot().await.expect("default seed");
    let shape = shape.to_owned();
    let (e,bot)=t.db().write(move|tx| {
        let bot=if legacy {User::create_bot(tx,"Legacy Sync",Some("http://bots.example:8080/hook"))?.id} else {BENDER};
        let room=if shape=="board" {Room::create_for(tx,RoomType::Board,Some("Sync Board"),DAVID,&[DAVID,bot])?} else {let r=Room::find(tx.conn(),ALL_TALK)?;r.grant_to(tx,&[bot])?;r};
        tx.conn().execute("UPDATE webhooks SET url='http://bots.example:8080/hook',signing_secret=NULL WHERE user_id=?",[bot])?;
        let m=if shape=="root" {Message::create(tx,NewMessage {room_id:room.id,creator_id:DAVID,body:Some("First post!".into()),..Default::default()})?}
        else {
            let mut thread=ChannelThread::create(tx,NewChannelThread {room_id:room.id,creator_id:DAVID,name:Some("Sync chat".into()),work_status:(shape=="board").then(||"in_progress".into()),..Default::default()})?;
            let m=thread.post_message(tx,DAVID,NewMessage {body:Some("First post!".into()),..Default::default()})?;
            if locked {thread.lock_conversation(tx)?;}
            m
        };
        let aid=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        let e=AgentEvent::create(tx,domain::NewEvent {agent_id:aid,message_id:Some(m.id),room_id:Some(m.room_id),event_type:"mention".into(),outcome:Some("delivered".into()),..Default::default()})?;
        Ok((e,bot))
    }).await.unwrap();
    (t, e, bot)
}
async fn respond(route: Route) -> (FakeServer, Network) {
    let server = FakeServer::start(vec![route]).await;
    let net = network(
        Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        Arc::new(MappingDialer {
            public: HashSet::from(["93.184.216.34".parse().unwrap()]),
            to: server.addr,
            dialed: Mutex::new(vec![]),
        }),
    );
    (server, net)
}
async fn count(t: &TestApp) -> i64 {
    t.db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap()
}
async fn last(t: &TestApp) -> Message {
    t.db()
        .read(|c| {
            Message::find(
                c,
                c.query_row("SELECT MAX(id) FROM messages", [], |r| r.get(0))?,
            )
        })
        .await
        .unwrap()
}
async fn legacy(t: &TestApp, e: &AgentEvent, bot: i64, net: &Network) -> JobResult {
    jobs::deliver_webhook_with_network(
        &t.booted.app,
        crate::jobs::WebhookJob {
            bot_id: bot,
            message_id: e.message_id.unwrap(),
        },
        net,
    )
    .await
}
async fn text(shape: &str, is_legacy: bool, locked: bool) {
    let (t, e, bot) = setup(shape, is_legacy, locked).await;
    let before = count(&t).await;
    let (server, net) = respond(
        Route::new("POST", "*", "/hook", 200)
            .header("Content-Type", "text/plain")
            .body(b"Hello back!"),
    )
    .await;
    if is_legacy {
        let result = legacy(&t, &e, bot, &net).await;
        if locked {
            assert!(format!("{:?}", result.unwrap_err()).contains("locked"));
        } else {
            result.unwrap();
        }
    } else {
        assert!(matches!(
            post_event(&t.booted.app, &e, &net).await,
            AttemptOutcome::Delivered
        ));
    }
    assert_eq!(server.received().len(), 1);
    if locked {
        assert_eq!(count(&t).await, before);
        return;
    }
    let m = last(&t).await;
    assert_eq!(m.creator_id, bot);
    let mid = e.message_id.unwrap();
    let trigger = t.db().read(move |c| Message::find(c, mid)).await.unwrap();
    assert_eq!(m.thread_id, trigger.thread_id);
    assert_eq!(m.reply_to_message_id, Some(trigger.id));
    let rich = t.booted.app.db.env().rich_text.clone();
    t.db()
        .read(move |c| {
            assert_eq!(m.plain_text_body(c, &*rich)?, "Hello back!");
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_webhook_case_payload() {
    let (t, e, bot) = setup("root", false, false).await;
    let (server, net) = respond(Route::new("POST", "*", "/hook", 200)).await;
    legacy(&t, &e, bot, &net).await.unwrap();
    let r = server.received();
    assert_eq!(r.len(), 1);
    let p: serde_json::Value = serde_json::from_slice(&r[0].body).unwrap();
    assert_eq!(p["user"]["id"], DAVID);
    assert_eq!(p["message"]["id"], e.message_id.unwrap());
    assert_eq!(p["message"]["body"]["plain"], "First post!");
    assert_eq!(p["message"]["body"]["html"], "First post!");
    assert_eq!(p["room"]["path"], campfire_routes::room(ALL_TALK));
    assert_eq!(
        p["message"]["path"],
        campfire_routes::room_at_message(ALL_TALK, e.message_id.unwrap())
    );
    let key = t
        .db()
        .read(move |c| Ok(User::find(c, bot)?.bot_key()))
        .await
        .unwrap();
    assert!(!String::from_utf8_lossy(&r[0].body).contains(&key));
}
#[tokio::test]
async fn ws11_webhook_case_delivery() {
    let (t, e, bot) = setup("root", false, false).await;
    let before = count(&t).await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    legacy(&t, &e, bot, &n).await.unwrap();
    assert_eq!(s.received().len(), 1);
    assert_eq!(count(&t).await, before);
}
#[tokio::test]
async fn ws11_webhook_case_ok_text_reply() {
    text("root", true, false).await;
}
#[tokio::test]
async fn ws11_webhook_case_agent_thread_text_reply() {
    text("thread", false, false).await;
}
#[tokio::test]
async fn ws11_webhook_case_legacy_thread_text_reply() {
    text("thread", true, false).await;
}
#[tokio::test]
async fn ws11_webhook_case_board_text_reply() {
    text("board", false, false).await;
}
#[tokio::test]
async fn ws11_webhook_case_root_references_trigger() {
    text("root", false, false).await;
}
async fn attachment(shape: &str, is_legacy: bool) {
    let (t, e, bot) = setup(shape, is_legacy, false).await;
    let bytes = include_bytes!("../../../../../vectors/storage/moon-thumb.jpg").to_vec();
    let (s, n) = respond(
        Route::new("POST", "*", "/hook", 200)
            .header("Content-Type", "image/jpeg")
            .body(bytes),
    )
    .await;
    if is_legacy {
        legacy(&t, &e, bot, &n).await.unwrap();
    } else {
        assert!(matches!(
            post_event(&t.booted.app, &e, &n).await,
            AttemptOutcome::Delivered
        ));
    }
    assert_eq!(s.received().len(), 1);
    let m = last(&t).await;
    let mid = e.message_id.unwrap();
    let trigger = t.db().read(move |c| Message::find(c, mid)).await.unwrap();
    assert_eq!(m.thread_id, trigger.thread_id);
    assert_eq!(m.reply_to_message_id, Some(trigger.id));
    t.db()
        .read(move |c| {
            assert!(m.attachment(c)?.is_some());
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws11_webhook_case_agent_thread_attachment_reply() {
    attachment("thread", false).await;
}
struct HangingDialer;
impl super::super::net::Dialer for HangingDialer {
    fn connect(
        &self,
        _: std::net::SocketAddr,
    ) -> super::super::net::BoxFuture<'_, std::io::Result<tokio::net::TcpStream>> {
        Box::pin(std::future::pending())
    }
}
fn hanging() -> Network {
    Network {
        resolver: Arc::new(FakeResolver::new([("bots.example", vec!["93.184.216.34"])])),
        dialer: Arc::new(HangingDialer),
        tls: super::super::net::tls_config(super::super::test_support::test_tls_roots()),
    }
}
#[tokio::test]
async fn ws11_webhook_case_agent_timeout_raises_without_posting() {
    let (t, _, _) = setup("root", false, false).await;
    let before = count(&t).await;
    assert!(matches!(
        webhook::deliver_signed(
            &hanging(),
            "http://bots.example:8080/hook",
            "{}".into(),
            None,
            || t.booted.app.clock.now(),
            true
        )
        .await,
        Err(webhook::WebhookError::Http(
            super::super::net::http::HttpError::OpenTimeout
        ))
    ));
    assert_eq!(count(&t).await, before);
}
#[tokio::test]
async fn ws11_webhook_case_agent_locked_reply_logged() {
    text("thread", false, true).await;
}
#[tokio::test]
async fn ws11_webhook_case_legacy_without_secret_unsigned() {
    let (t, e, bot) = setup("root", false, false).await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    legacy(&t, &e, bot, &n).await.unwrap();
    let r = s.received();
    assert_eq!(r.len(), 1);
    assert!(r[0].header("x-smartfire-signature").is_none());
    assert!(
        r[0].header("x-smartfire-timestamp")
            .unwrap()
            .bytes()
            .all(|b| b.is_ascii_digit())
    );
}
#[tokio::test]
async fn ws11_webhook_case_legacy_with_secret_signs_body() {
    let (t, e, bot) = setup("root", false, false).await;
    let crypto = t.booted.app.ar_encryption.clone();
    let secret = t
        .db()
        .write(move |tx| {
            Webhook::find_by_user(tx.conn(), bot)?
                .unwrap()
                .reset_signing_secret(tx, &crypto)
        })
        .await
        .unwrap();
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    legacy(&t, &e, bot, &n).await.unwrap();
    let r = s.received();
    assert_eq!(r.len(), 1);
    let timestamp = r[0].header("x-smartfire-timestamp").unwrap();
    assert_eq!(
        r[0].header("x-smartfire-signature"),
        Some(rails_compat::webhook::smartfire_signature(&secret, timestamp, &r[0].body).as_str())
    );
}
#[tokio::test]
async fn ws11_webhook_case_legacy_locked_reply_raises() {
    text("thread", true, true).await;
}
#[tokio::test]
async fn ws11_webhook_case_ok_attachment_reply() {
    attachment("root", true).await;
}
#[tokio::test]
async fn ws11_webhook_case_error_reply_posts_nothing() {
    let (t, e, bot) = setup("root", true, false).await;
    let before = count(&t).await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 500).body(b"Internal Error!")).await;
    legacy(&t, &e, bot, &n).await.unwrap();
    assert_eq!(s.received().len(), 1);
    assert_eq!(count(&t).await, before);
}
#[tokio::test]
async fn ws11_webhook_case_legacy_timeout_posts_failure() {
    let (t, e, bot) = setup("root", true, false).await;
    let before = count(&t).await;
    legacy(&t, &e, bot, &hanging()).await.unwrap();
    assert_eq!(count(&t).await, before + 1);
    let m = last(&t).await;
    assert!(m.thread_id.is_none());
    assert!(m.reply_to_message_id.is_none());
    let rich = t.booted.app.db.env().rich_text.clone();
    t.db()
        .read(move |c| {
            assert_eq!(
                m.plain_text_body(c, &*rich)?,
                "Failed to respond within 7 seconds"
            );
            Ok(())
        })
        .await
        .unwrap();
}
async fn refused(url: &str, answers: Vec<&str>, agent: bool) -> webhook::WebhookError {
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    let dialer = n.dialer.clone();
    let net = Network {
        resolver: Arc::new(FakeResolver::new([("bots.example", answers)])),
        dialer,
        tls: n.tls,
    };
    let (t, _, _) = setup("root", false, false).await;
    let before = count(&t).await;
    let error = webhook::deliver_signed(
        &net,
        url,
        "{}".into(),
        None,
        || t.booted.app.clock.now(),
        agent,
    )
    .await
    .unwrap_err();
    assert!(s.received().is_empty());
    assert_eq!(count(&t).await, before);
    error
}
#[tokio::test]
async fn ws11_webhook_case_loopback_refused() {
    assert!(matches!(
        refused("http://127.0.0.1:9999/hook", vec![], false).await,
        webhook::WebhookError::Guard(super::super::net::guard::GuardError::Violation(_))
    ));
}
#[tokio::test]
async fn ws11_webhook_case_dns_private_refused() {
    assert!(matches!(
        refused("http://bots.example:8080/hook", vec!["10.0.0.5"], false).await,
        webhook::WebhookError::Guard(super::super::net::guard::GuardError::Violation(_))
    ));
}
#[tokio::test]
async fn ws11_webhook_case_agent_private_refused_without_reply() {
    assert!(matches!(
        refused("http://192.168.1.10/hook", vec![], true).await,
        webhook::WebhookError::Guard(super::super::net::guard::GuardError::Violation(_))
    ));
}
#[tokio::test]
async fn ws11_webhook_case_approval_private_refused() {
    let (t, e, _) = setup("root", false, false).await;
    let (s, n) = respond(Route::new("POST", "*", "/hook", 200)).await;
    let a = t
        .db()
        .write(move |tx| {
            let a = campfire_db::AgentApproval::create(
                tx,
                campfire_db::NewApproval {
                    agent_id: e.agent_id,
                    action: "deploy".into(),
                    summary: "Ship it".into(),
                    ..Default::default()
                },
            )?;
            tx.conn().execute(
                "UPDATE webhooks SET url='http://10.1.2.3/hook' WHERE user_id=?",
                [BENDER],
            )?;
            Ok(a)
        })
        .await
        .unwrap();
    let event = t
        .db()
        .write(move |tx| {
            AgentEvent::create(
                tx,
                domain::NewEvent {
                    agent_id: a.agent_id,
                    agent_approval_id: Some(a.id),
                    event_type: "approval_decided".into(),
                    outcome: Some("delivered".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    assert!(
        matches!(post_event(&t.booted.app,&event,&n).await,AttemptOutcome::Permanent(error) if error.contains("RestrictedHTTP::Violation"))
    );
    assert!(s.received().is_empty());
}
#[tokio::test]
async fn ws11_webhook_case_dns_unresolvable_refused() {
    assert!(matches!(
        refused("http://bots.example:8080/hook", vec![], false).await,
        webhook::WebhookError::Guard(super::super::net::guard::GuardError::Unresolvable)
    ));
}
