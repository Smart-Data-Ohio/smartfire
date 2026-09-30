use super::*;
use crate::integrations::test_support::TestDb;
use campfire_db::{Env, Event, EventSink, TestClock, Timestamp, Tx};
use campfire_jobs::{JobQueue, QueueConfig, Registry, RunnerConfig};
use rusqlite::params;
use serde_json::json;
use std::sync::{Arc, Mutex};
fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../../vectors/github_notifier.json"
    ))
    .unwrap()
}
#[derive(Clone)]
struct Sink {
    queue: JobQueue,
    events: Arc<Mutex<Vec<Event>>>,
}
impl EventSink for Sink {
    fn emit(&self, event: Event) {
        self.events.lock().unwrap().push(event);
    }
    fn persist(&self, tx: &Tx<'_>, event: &Event) -> campfire_db::Result<()> {
        if let Event::Job(r) = event {
            self.queue.enqueue(tx, r)?;
        }
        Ok(())
    }
}
async fn database(case: &Value) -> (TestDb, Sink) {
    let sink = Sink {
        queue: JobQueue::new(
            &Registry::<()>::new(),
            &RunnerConfig::new(vec![
                QueueConfig::new("default", 1),
                QueueConfig::new("push", 1),
            ]),
        )
        .unwrap(),
        events: Arc::default(),
    };
    let now: jiff::Timestamp = "2026-01-01T12:00:00Z".parse().unwrap();
    let secret =
        serde_json::from_str::<Value>(include_str!("../../../../../../vectors/github.json"))
            .unwrap()["secret_key_base"]
            .as_str()
            .unwrap()
            .to_owned();
    let env = Env {
        clock: Arc::new(TestClock::frozen_at(Timestamp::from_jiff(now))),
        sink: Arc::new(sink.clone()),
        rich_text: Arc::new(crate::rich_text::AppRichText::new(
            Arc::new(rails_compat::Secrets::new(&secret)),
            Arc::new(campfire_kit::FrozenClock::new(now)),
        )),
        bcrypt_cost: 4,
        message_reference_syncs: vec![super::super::references::sync],
        user_deactivation_hooks: Vec::new(),
    };
    let directory =
        std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
    let fixture = tokio::task::spawn_blocking(move || TestDb::with_env(env, &directory))
        .await
        .unwrap();
    seed(&fixture.db, case).await;
    sink.events.lock().unwrap().clear();
    (fixture, sink)
}
async fn seed(db: &Database, case: &Value) {
    let case = case.clone();
    db.write(move|tx| {
        let now=tx.now();
        tx.conn().execute("INSERT INTO users (id,name,email_address,role,created_at,updated_at) VALUES (811,'Oracle','oracle@example.test',1,?,?)",params![now,now])?;
        tx.conn().execute("INSERT INTO users (id,name,email_address,role,status,github_login,inbox_preferences,created_at,updated_at) VALUES (812,'Kevin','reviewer@example.test',?,?,'kevin-gh',?,?,?)",params![if case["reviewer_bot"]==true {2}else{0},i64::from(case["inactive"]==true),case.get("preference").map(|v|json!({"github_review_requests":v})).unwrap_or(json!({})).to_string(),now,now])?;
        for (id,name,kind) in [(815,"Notifications","Rooms::Closed"),(825,"Verified","Rooms::Closed"),(835,"Unsubscribed open","Rooms::Open")] {
            tx.conn().execute("INSERT INTO rooms (id,name,type,creator_id,created_at,updated_at) VALUES (?,?,?,811,?,?)",params![id,name,kind,now,now])?;
            tx.conn().execute("INSERT INTO memberships (user_id,room_id,created_at,updated_at) VALUES (811,?,?,?)",params![id,now,now])?;
        }
        if case["member"]!=false {tx.conn().execute("INSERT INTO memberships (user_id,room_id,involvement,created_at,updated_at) VALUES (812,815,?,?,?)",params![case["involvement"].as_str().unwrap_or("everything"),now,now])?;}
        if case["deleted"]==true {tx.conn().execute("UPDATE rooms SET deleted_at=? WHERE id=815",[now])?;}
        if case["no_bot"]!=true {tx.conn().execute("INSERT INTO users (id,name,role,created_at,updated_at) VALUES (810,'GitHub',2,?,?)",params![now,now])?;}
        if case["no_subscriptions"]!=true {
            let all=json!(["opened","merged","closed","review_requested","review_submitted","checks_failed"]);
            tx.conn().execute("INSERT INTO github_repository_subscriptions (id,room_id,owner,repo,events,reader_verified,created_at,updated_at) VALUES (819,815,'rails','rails',?,?,?,?)",params![case.get("events").unwrap_or(&all).to_string(),case["verified"]==true,now,now])?;
            if case["no_bot"]!=true {tx.conn().execute("INSERT INTO memberships (user_id,room_id,created_at,updated_at) VALUES (810,815,?,?)",params![now,now])?;}
            if case["other"]==true {
                tx.conn().execute("INSERT INTO github_repository_subscriptions (id,room_id,owner,repo,events,reader_verified,created_at,updated_at) VALUES (829,825,'rails','rails',?,1,?,?)",params![all.to_string(),now,now])?;
                if case["no_bot"]!=true {tx.conn().execute("INSERT INTO memberships (user_id,room_id,created_at,updated_at) VALUES (810,825,?,?)",params![now,now])?;}
            }
        }
        if case["stored"]==true || case["thread"]==true {
            tx.conn().execute("INSERT INTO github_pull_requests (id,owner,repo,number,title,head_branch,html_url,created_at,updated_at) VALUES (816,'rails','rails',12,'Secret acquisition','shiny',?,?,?)",params![if case["no_url"]==true {""}else{"https://github.com/Rails/Rails/pull/12"},now,now])?;
        }
        if case["thread"]==true {
            tx.conn().execute("INSERT INTO messages (id,creator_id,room_id,client_message_id,created_at,updated_at) VALUES (818,811,815,'fixture-parent',?,?)",params![now,now])?;
            tx.conn().execute("INSERT INTO channel_threads (id,room_id,creator_id,name,parent_message_id,last_activity_at,locked_at,closed_at,created_at,updated_at) VALUES (817,815,811,'PR chat',818,?,?,?,?,?)",params![now.ago(jiff::SignedDuration::from_hours(48)),(case["locked"]==true).then_some(now),(case["closed"]==true).then_some(now),now,now])?;
            tx.conn().execute("INSERT INTO github_pull_request_threads (github_pull_request_id,room_id,channel_thread_id,created_at,updated_at) VALUES (816,815,817,?,?)",params![now,now])?;
            tx.conn().execute("INSERT INTO thread_memberships (thread_id,user_id,joined_at,created_at,updated_at) VALUES (817,811,?,?,?)",params![now,now,now])?;
        }
        Ok(())
    }).await.unwrap();
}
async fn snapshot(db: &Database, sink: &Sink) -> Value {
    let mut result=db.read(|conn| {
        let mut stmt=conn.prepare("SELECT id,room_id,thread_id,markdown_source,creator_id FROM messages WHERE room_id IN (815,825) AND client_message_id!='fixture-parent' ORDER BY id")?;
        let rows:Vec<(i64,i64,Option<i64>,String,i64)>=stmt.query_map([],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?,r.get(3)?,r.get(4)?)))?.collect::<rusqlite::Result<_>>()?;
        let mut messages=Vec::new();
        for (id,room,thread,source,creator) in &rows {
            let mut refs=conn.prepare("SELECT p.owner,p.repo,p.number FROM github_pull_requests p JOIN github_pull_request_references r ON r.github_pull_request_id=p.id WHERE r.message_id=? ORDER BY p.id")?;
            let refs:Vec<Value>=refs.query_map([id],|r|Ok(json!([r.get::<_,String>(0)?,r.get::<_,String>(1)?,r.get::<_,i64>(2)?])))?.collect::<rusqlite::Result<_>>()?;
            let bot:String=conn.query_row("SELECT name FROM users WHERE id=?",[creator],|r|r.get(0))?;
            messages.push(json!({"room_id":room,"thread":thread.is_some(),"source":source,"bot":bot,"refs":refs}));
        }
        let index=|id:Option<i64>|id.and_then(|id|rows.iter().position(|r|r.0==id));
        let mut stmt=conn.prepare("SELECT subscription_id,dedupe_key,message_id FROM github_notifications ORDER BY subscription_id,id")?;
        let notifications:Vec<Value>=stmt.query_map([],|r|Ok(json!({"subscription":r.get::<_,i64>(0)?,"key":r.get::<_,String>(1)?,"message":index(r.get(2)?)})))?.collect::<rusqlite::Result<_>>()?;
        let mut stmt=conn.prepare("SELECT user_id,source_id FROM activity_items WHERE event_type='pr_review_request' ORDER BY id")?;
        let activity:Vec<Value>=stmt.query_map([],|r|Ok(json!({"user_id":r.get::<_,i64>(0)?,"message":index(Some(r.get(1)?))})))?.collect::<rusqlite::Result<_>>()?;
        let bot_count:i64=conn.query_row("SELECT COUNT(*) FROM users WHERE name='GitHub' AND role=2 AND status=0",[],|r|r.get(0))?;
        use rusqlite::OptionalExtension;
        let thread:Option<Value>=conn.query_row("SELECT closed_at,locked_at,last_activity_at FROM channel_threads WHERE id=817",[],|r|Ok(json!({"closed":r.get::<_,Option<Timestamp>>(0)?.is_some(),"locked":r.get::<_,Option<Timestamp>>(1)?.is_some(),"fresh":r.get::<_,Timestamp>(2)?==Timestamp::from_jiff("2026-01-01T12:00:00Z".parse().unwrap())}))).optional()?;
        Ok(json!({"messages":messages,"notifications":notifications,"activity":activity,"bot_count":bot_count,"thread":thread}))
    }).await.unwrap();
    // The registered append description is emitted exactly once after each post commits.
    let broadcasts:Vec<Value>=sink.events.lock().unwrap().iter().filter_map(|event| {
        if let Event::Broadcast(request)=event && request.kind=="Github::Notifier#broadcast_create" {return Some(json!({"room_id":request.arguments["room_id"],"thread":request.arguments["thread_id"].is_number()}));}
        None
    }).collect();
    result["broadcasts"] = json!(broadcasts);
    result
}
async fn run_case(case: &Value) {
    let (fixture, sink) = database(case).await;
    for delivery in case["deliveries"].as_array().unwrap() {
        deliver(
            &fixture.db,
            delivery[0].as_str().unwrap().into(),
            delivery[1].clone(),
        )
        .await
        .unwrap();
    }
    assert_eq!(
        snapshot(&fixture.db, &sink).await,
        case["expected"],
        "{}",
        case["name"]
    );
    if case["name"] == "lazy_bot"
        && let Ok(path) = std::env::var("GITHUB_NOTIFIER_RUST_OUTPUT")
    {
        let _ = std::fs::remove_file(&path);
        let database = fixture
            .db
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT file FROM pragma_database_list WHERE name='main'",
                    [],
                    |row| row.get::<_, String>(0),
                )?)
            })
            .await
            .unwrap();
        tokio::task::spawn_blocking(move || {
            rusqlite::Connection::open(database)
                .unwrap()
                .execute("VACUUM INTO ?", [path])
                .unwrap();
        })
        .await
        .unwrap();
    }
}
#[tokio::test]
async fn github_notifier_security_redacts_per_subscription_and_neutralizes_mentions() {
    for case in vectors()["cases"].as_array().unwrap().iter().filter(|c| {
        [
            "private_unverified",
            "unknown_unverified",
            "private_verified",
            "private_checks",
            "per_subscription",
            "mention_security",
        ]
        .contains(&c["name"].as_str().unwrap())
    }) {
        run_case(case).await;
    }
}
#[tokio::test]
async fn github_notifier_posts_claims_references_inbox_and_thread_routes_match_rails() {
    for case in vectors()["cases"].as_array().unwrap() {
        run_case(case).await;
    }
}
#[tokio::test]
async fn github_notifier_concurrent_duplicates_claim_and_broadcast_once() {
    let case = &vectors()["cases"][0];
    let (fixture, sink) = database(case).await;
    let mut workers = Vec::new();
    for _ in 0..24 {
        let db = fixture.db.clone();
        let payload = case["deliveries"][0][1].clone();
        workers.push(tokio::spawn(async move {
            deliver(&db, "pull_request".into(), payload).await
        }));
    }
    for worker in workers {
        worker.await.unwrap().unwrap();
    }
    assert_eq!(snapshot(&fixture.db, &sink).await, case["expected"]);
}
#[tokio::test]
async fn github_notifier_queue_failure_rolls_back_post_and_keeps_claim_as_rails() {
    let case = vectors()["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == "lazy_bot")
        .unwrap()
        .clone();
    let (fixture, sink) = database(&case).await;
    fixture.db.write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_notifier_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'fixture queue down'); END;")?;Ok(())}).await.unwrap();
    assert!(
        deliver(
            &fixture.db,
            "pull_request".into(),
            case["deliveries"][0][1].clone()
        )
        .await
        .is_err()
    );
    let result = snapshot(&fixture.db, &sink).await;
    assert_eq!(result["messages"], json!([]));
    assert_eq!(result["broadcasts"], json!([]));
    assert_eq!(result["activity"], json!([]));
    assert_eq!(
        result["notifications"],
        json!([{"subscription":819,"key":"opened:rails/rails#12","message":null}])
    );
    assert_eq!(result["bot_count"], 1);
    let counts=fixture.db.read(|conn|Ok((conn.query_row("SELECT COUNT(*) FROM github_pull_requests",[],|r|r.get::<_,i64>(0))?,conn.query_row("SELECT COUNT(*) FROM memberships m JOIN users u ON u.id=m.user_id WHERE u.name='GitHub'",[],|r|r.get::<_,i64>(0))?))).await.unwrap();
    assert_eq!(counts, (0, 0)); // no orphan PR, no automatic grant to any open room
    fixture
        .db
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_notifier_job")?;
            Ok(())
        })
        .await
        .unwrap();
    deliver(
        &fixture.db,
        "pull_request".into(),
        case["deliveries"][0][1].clone(),
    )
    .await
    .unwrap();
    assert_eq!(snapshot(&fixture.db, &sink).await, result); // claimed failure is a no-op on redelivery
}

#[tokio::test]
async fn github_notifier_durable_handler_publishes_real_room_and_thread_frames() {
    use campfire_kit::Crypto;
    use futures_util::{SinkExt, StreamExt};
    use tokio::net::TcpStream;
    use tokio_tungstenite::{
        MaybeTlsStream, WebSocketStream,
        tungstenite::{Message as WsMessage, client::IntoClientRequest},
    };
    async fn frame(socket: &mut WebSocketStream<MaybeTlsStream<TcpStream>>) -> Value {
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let next = tokio::time::timeout_at(deadline, socket.next())
                .await
                .unwrap()
                .unwrap()
                .unwrap();
            if let WsMessage::Text(text) = next {
                let value: Value = serde_json::from_str(&text).unwrap();
                if value["type"] != "ping" {
                    return value;
                }
            }
        }
    }
    for name in ["opened", "thread_review"] {
        let case = vectors()["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap()
            .clone();
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
        let directory = tempfile::tempdir_in(scratch).unwrap();
        let secret: Value =
            serde_json::from_str(include_str!("../../../../../../vectors/github.json")).unwrap();
        let config = crate::config::Config::from_lookup(|key| match key {
            "SECRET_KEY_BASE" => Some(secret["secret_key_base"].as_str().unwrap().into()),
            "CAMPFIRE_STORAGE_PATH" => Some(directory.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        // No token: the queued fetch cannot make an external request.
        let booted = crate::app::boot_with_github_read(
            config,
            Arc::new(campfire_kit::FrozenClock::new(
                "2026-01-01T12:00:00Z".parse().unwrap(),
            )),
            super::super::client::ReadClient::new(None),
        )
        .await
        .unwrap();
        seed(&booted.app.db, &case).await;
        let session = booted
            .app
            .db
            .write(|tx| {
                campfire_db::Session::start_with(
                    tx,
                    811,
                    campfire_db::NewSession {
                        two_factor_verified: true,
                        ..Default::default()
                    },
                )
            })
            .await
            .unwrap();
        let cookie = campfire_kit::RailsCrypto::new(booted.app.secrets.clone())
            .sign_cookie("session_token", &session.token, None)
            .replace('+', "%2B")
            .replace('/', "%2F")
            .replace('=', "%3D");
        let mut listener = None;
        for port in 51500..=51549 {
            if let Ok(bound) =
                tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await
            {
                listener = Some(bound);
                break;
            }
        }
        let listener = listener.expect("ws15g port available");
        let address = listener.local_addr().unwrap();
        let router = booted.app.cable.router::<()>("/cable");
        let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let mut request = format!("ws://{address}/cable")
            .into_client_request()
            .unwrap();
        request
            .headers_mut()
            .insert("origin", format!("http://{address}").parse().unwrap());
        request
            .headers_mut()
            .insert("cookie", format!("session_token={cookie}").parse().unwrap());
        request.headers_mut().insert(
            "sec-websocket-protocol",
            "actioncable-v1-json".parse().unwrap(),
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        assert_eq!(frame(&mut socket).await, json!({"type":"welcome"}));
        let model = if name == "opened" {
            "Rooms::Closed"
        } else {
            "ChannelThread"
        };
        let id = if name == "opened" { 815 } else { 817 };
        let gid = rails_compat::global_id::GlobalId::new(model, id).to_param();
        let identifier=json!({"channel":"RoomMessagesChannel","signed_stream_name":rails_compat::turbo::signed_stream_name(&booted.app.secrets,&[&gid,"messages"])}).to_string();
        socket
            .send(WsMessage::Text(
                json!({"command":"subscribe","identifier":identifier})
                    .to_string()
                    .into(),
            ))
            .await
            .unwrap();
        assert_eq!(frame(&mut socket).await["type"], "confirm_subscription");
        let payload = case["deliveries"][0][1].clone();
        booted
            .app
            .db
            .write(move |tx| {
                tx.emit_after_commit(Event::job(
                    &super::super::jobs::DeliverSubscriptionEventJob {
                        event: "pull_request".into(),
                        payload,
                    },
                ));
                Ok(())
            })
            .await
            .unwrap();
        let received = frame(&mut socket).await;
        assert_eq!(received["identifier"], identifier);
        let html = received["message"].as_str().unwrap();
        assert!(html.contains("action=\"append\""), "{html}");
        assert!(
            html.contains(if name == "opened" {
                "target=\"messages_rooms_closed_815\""
            } else {
                "target=\"messages_channel_thread_817\""
            }),
            "{html}"
        );
        assert!(
            html.contains(if name == "opened" {
                "opened pull request"
            } else {
                "requested a review"
            }),
            "{html}"
        );
        assert!(campfire_cable::turbo::session_bound(html).is_none());
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let jobs = booted
                .app
                .db
                .read(campfire_jobs::inspect::all)
                .await
                .unwrap();
            if !jobs
                .iter()
                .any(|job| job.class == "Github::DeliverSubscriptionEventJob")
            {
                break;
            }
            assert!(tokio::time::Instant::now() < deadline, "{jobs:?}");
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        socket.close(None).await.unwrap();
        serving.abort();
        booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
    }
}
#[tokio::test]
async fn github_notifier_runtime_uses_default_retry_policy_and_fails_nontransient_posts_once() {
    use campfire_jobs::JobKind;
    let policy = super::super::jobs::DeliverSubscriptionEventJob::retry_policy();
    assert_eq!(policy.attempts, 5);
    assert_eq!(
        policy.retry_delay(1, None, 0.0),
        Some(std::time::Duration::from_secs(3))
    );
    assert_eq!(
        policy.retry_delay(4, None, 0.0),
        Some(std::time::Duration::from_secs(258))
    );
    assert_eq!(policy.retry_delay(5, None, 0.0), None);
    assert!((policy.retry_on)(
        &std::io::Error::new(std::io::ErrorKind::TimedOut, "fixture timeout").into()
    ));
    assert!(!(policy.retry_on)(&anyhow::anyhow!("invalid payload")));
    for mode in ["shape", "post"] {
        let case = &vectors()["cases"][0];
        let scratch =
            std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("../../../.scratch/ws15g");
        let directory = tempfile::tempdir_in(scratch).unwrap();
        let secret: Value =
            serde_json::from_str(include_str!("../../../../../../vectors/github.json")).unwrap();
        let config = crate::config::Config::from_lookup(|key| match key {
            "SECRET_KEY_BASE" => Some(secret["secret_key_base"].as_str().unwrap().into()),
            "CAMPFIRE_STORAGE_PATH" => Some(directory.path().to_string_lossy().into_owned()),
            "DISABLE_SSL" => Some("1".into()),
            _ => None,
        })
        .unwrap();
        let booted = crate::app::boot_with_github_read(
            config,
            Arc::new(campfire_kit::FrozenClock::new(
                "2026-01-01T12:00:00Z".parse().unwrap(),
            )),
            super::super::client::ReadClient::new(None),
        )
        .await
        .unwrap();
        seed(&booted.app.db, case).await;
        let payload = case["deliveries"][0][1].clone();
        // A wrong PR shape that reaches #dig is a permanent application error.
        let payload = if mode == "shape" {
            json!({"pull_request":{"number":12},"repository":[]})
        } else {
            payload
        };
        booted.app.db.write(move|tx|{
            if mode=="post" {tx.conn().execute_batch("CREATE TRIGGER reject_notifier_post BEFORE INSERT ON messages BEGIN SELECT RAISE(ABORT,'fixture post down'); END;")?;}
            tx.emit_after_commit(Event::job(&super::super::jobs::DeliverSubscriptionEventJob {event:"pull_request".into(),payload}));Ok(())
        }).await.unwrap();
        let deadline = tokio::time::Instant::now() + std::time::Duration::from_secs(5);
        loop {
            let jobs = booted
                .app
                .db
                .read(campfire_jobs::inspect::all)
                .await
                .unwrap();
            if let Some(job) = jobs.iter().find(|job| {
                job.class == "Github::DeliverSubscriptionEventJob"
                    && job.status == campfire_jobs::FAILED
            }) {
                assert_eq!(job.attempts, 1);
                assert_eq!(job.run_at, job.created_at);
                break;
            }
            assert!(tokio::time::Instant::now() < deadline, "{jobs:?}");
            tokio::time::sleep(std::time::Duration::from_millis(5)).await;
        }
        let counts = booted
            .app
            .db
            .read(|conn| {
                Ok((
                    conn.query_row(
                        "SELECT COUNT(*) FROM messages WHERE creator_id=810",
                        [],
                        |r| r.get::<_, i64>(0),
                    )?,
                    conn.query_row(
                        "SELECT COUNT(*) FROM github_notifications WHERE message_id IS NULL",
                        [],
                        |r| r.get::<_, i64>(0),
                    )?,
                ))
            })
            .await
            .unwrap();
        assert_eq!(counts, (0, if mode == "post" { 1 } else { 0 }));
        booted
            .jobs
            .shutdown(std::time::Duration::from_secs(1))
            .await;
    }
}
