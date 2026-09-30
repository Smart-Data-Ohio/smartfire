use super::*;
use std::sync::{Arc, Mutex};
use tokio::io::{AsyncBufReadExt, AsyncReadExt, AsyncWriteExt, BufReader};
use tokio::net::TcpListener;
use tokio::task::JoinHandle;

const KEY: &str = "ws13-fixture-api-key";
const SECRET: &str = "ws13-fixture-api-secret";
const NOW: i64 = 1_767_268_800;

fn vectors() -> Value {
    serde_json::from_str(include_str!("protocol_vectors.json")).unwrap()
}
fn config(internal: &str) -> Config {
    Config::from_lookup(|name| {
        Some(
            match name {
                "LIVEKIT_URL" => "wss://huddle.example.test",
                "LIVEKIT_INTERNAL_URL" => internal,
                "LIVEKIT_API_KEY" => KEY,
                "LIVEKIT_API_SECRET" => SECRET,
                _ => "ws13-fixture-gateway-secret",
            }
            .into(),
        )
    })
}

#[test]
fn join_admin_and_opaque_names_match_pinned_rails() {
    let v = vectors();
    assert_eq!(
        livekit::room_name(SECRET, v["room_id"].as_i64().unwrap()),
        v["room_name"]
    );
    for token in v["tokens"].as_array().unwrap() {
        let can_publish = livekit::can_publish(
            token["muted"].as_bool().unwrap(),
            token["stage"].as_bool().unwrap(),
            token["role"].as_str(),
        );
        let p = livekit::Participant {
            name: "Fixture \"User\" ☃",
            identity: v["identity"].as_str().unwrap(),
            room_name: v["room_name"].as_str().unwrap(),
            can_publish,
        };
        assert_eq!(
            livekit::participant_token_with_jti(KEY, SECRET, &p, NOW, v["jti"].as_str().unwrap()),
            token["token"],
            "{}",
            token["name"]
        );
        assert_eq!(
            livekit::verify(token["token"].as_str().unwrap(), KEY, SECRET, NOW)
                .unwrap()
                .identity,
            p.identity
        );
    }
    for (key, grant) in [
        (
            "admin_remove",
            serde_json::json!({"roomAdmin": true, "room": v["room_name"]}),
        ),
        ("admin_delete", serde_json::json!({"roomCreate": true})),
    ] {
        assert_eq!(
            livekit::admin_token_with_jti(
                KEY,
                SECRET,
                grant.as_object().unwrap(),
                NOW,
                v["jti"].as_str().unwrap()
            ),
            v[key]
        );
    }
}

#[test]
fn strict_token_shapes_match_pinned_rails() {
    for case in vectors()["shapes"].as_array().unwrap() {
        let actual = livekit::verify(case["token"].as_str().unwrap(), KEY, SECRET, NOW)
            .ok()
            .map(|c| serde_json::json!({ "identity": c.identity, "room_name": c.room_name }));
        assert_eq!(
            actual.unwrap_or(Value::Null),
            case["coordinates"],
            "{}",
            case["name"]
        );
    }
}

#[test]
fn listener_and_server_muted_members_never_receive_publish_grants() {
    let v = vectors();
    for name in [
        "listener",
        "missing_stage_role",
        "server_muted",
        "muted_host",
    ] {
        let case = v["tokens"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap();
        let publish = livekit::can_publish(
            case["muted"].as_bool().unwrap(),
            case["stage"].as_bool().unwrap(),
            case["role"].as_str(),
        );
        assert!(!publish, "{name}");
        let grant = livekit::participant_video_grant("room", publish);
        assert_eq!(grant["canPublish"], false);
        assert_eq!(grant["canPublishSources"], serde_json::json!([]));
    }
}

#[test]
fn configured_endpoints_and_twirp_prefixes_match_rails() {
    for case in vectors()["urls"].as_array().unwrap() {
        let mut c = config(case["internal_url"].as_str().unwrap());
        c.public_url = case["public_url"]
            .as_str()
            .filter(|s| !s.is_empty())
            .map(str::to_string);
        assert_eq!(
            c.configured(),
            case["configured"].as_bool().unwrap(),
            "{case}"
        );
        let endpoint = endpoint_uri(c.internal_url.as_deref().unwrap(), "RemoveParticipant")
            .ok()
            .map(|u| Value::String(u.to_s()));
        assert_eq!(endpoint.unwrap_or(Value::Null), case["endpoint"], "{case}");
    }
    for missing in [
        "LIVEKIT_URL",
        "LIVEKIT_INTERNAL_URL",
        "LIVEKIT_API_KEY",
        "LIVEKIT_API_SECRET",
        "LIVEKIT_GATEWAY_SECRET",
    ] {
        let c = Config::from_lookup(|name| {
            if name == missing {
                Some(" \t\n".into())
            } else {
                Some(
                    match name {
                        "LIVEKIT_URL" => "wss://public.test",
                        "LIVEKIT_INTERNAL_URL" => "ws://internal.test",
                        _ => "fixture",
                    }
                    .into(),
                )
            }
        });
        assert!(!c.configured(), "{missing}");
    }
}

#[derive(Clone, Debug)]
struct Received {
    head: String,
    body: Vec<u8>,
}
struct Server {
    url: String,
    received: Arc<Mutex<Vec<Received>>>,
    task: JoinHandle<()>,
}
impl Drop for Server {
    fn drop(&mut self) {
        self.task.abort();
    }
}

impl Server {
    async fn start(status: u16, delay: Duration) -> Self {
        let listener = if std::env::var_os("CABLE_TEST_PORT_RANGE").is_some() {
            crate::channels::tests::support::bind_listener().await
        } else {
            let mut listener = None;
            for port in 52300..=52399 {
                if let Ok(bound) = TcpListener::bind(("127.0.0.1", port)).await {
                    listener = Some(bound);
                    break;
                }
            }
            listener.expect("no free WS13 test port")
        };
        let url = format!("http://{}", listener.local_addr().unwrap());
        let received = Arc::new(Mutex::new(Vec::new()));
        let log = received.clone();
        let task = tokio::spawn(async move {
            while let Ok((stream, _)) = listener.accept().await {
                let mut reader = BufReader::new(stream);
                let mut head = String::new();
                loop {
                    let mut line = String::new();
                    if reader.read_line(&mut line).await.unwrap_or(0) == 0 {
                        break;
                    }
                    head.push_str(&line);
                    if line == "\r\n" {
                        break;
                    }
                }
                let length = head
                    .lines()
                    .find_map(|line| {
                        let (name, value) = line.split_once(':')?;
                        name.eq_ignore_ascii_case("content-length")
                            .then(|| value.trim().parse::<usize>().unwrap())
                    })
                    .unwrap_or(0);
                let mut body = vec![0; length];
                if reader.read_exact(&mut body).await.is_err() {
                    continue;
                }
                log.lock().unwrap().push(Received { head, body });
                tokio::time::sleep(delay).await;
                // A Location is always present so a mistaken redirect implementation is observable.
                let reply = format!(
                    "HTTP/1.1 {status} Fixture\r\nLocation: /redirected\r\nContent-Length: 23\r\nConnection: close\r\n\r\nsensitive upstream body"
                );
                let _ = reader.get_mut().write_all(reply.as_bytes()).await;
            }
        });
        Self {
            url,
            received,
            task,
        }
    }
}

#[tokio::test]
async fn twirp_uses_only_the_requested_room_and_scoped_admin_grants() {
    let server = Server::start(200, Duration::ZERO).await;
    let service = RoomService::new(config(&format!("{}/prefix/?ignore#fragment", server.url)));
    service
        .remove_participant("opaque-room", "opaque-participant", NOW)
        .await
        .unwrap();
    service.delete_room("opaque-room", NOW).await.unwrap();
    let requests = server.received.lock().unwrap().clone();
    assert_eq!(requests.len(), 2);
    for (index, request) in requests.iter().enumerate() {
        let action = if index == 0 {
            "RemoveParticipant"
        } else {
            "DeleteRoom"
        };
        assert!(
            request.head.starts_with(&format!(
                "POST /prefix/twirp/livekit.RoomService/{action} HTTP/1.1\r\n"
            )),
            "{}",
            request.head
        );
        let authorization = request
            .head
            .lines()
            .find_map(|line| line.strip_prefix("Authorization: "))
            .unwrap();
        let jwt = authorization.strip_prefix("Bearer ").unwrap();
        let claims = rails_compat::jwt::decode(
            jwt,
            rails_compat::jwt::Key::Hs256(SECRET.as_bytes()),
            &rails_compat::jwt::Validation::default(),
            NOW,
        )
        .unwrap()
        .payload;
        assert_eq!(claims["iss"], KEY);
        assert_eq!(claims["exp"], NOW + 60);
        assert_eq!(claims["nbf"], NOW - 5);
        let grant = if index == 0 {
            serde_json::json!({"roomAdmin": true, "room": "opaque-room"})
        } else {
            serde_json::json!({"roomCreate": true})
        };
        assert_eq!(claims["video"], grant);
        assert!(claims.get("sub").is_none());
        let body = if index == 0 {
            serde_json::json!({"room": "opaque-room", "identity": "opaque-participant"})
        } else {
            serde_json::json!({"room": "opaque-room"})
        };
        assert_eq!(
            serde_json::from_slice::<Value>(&request.body).unwrap(),
            body
        );
    }
}

#[tokio::test]
async fn twirp_missing_room_and_participant_are_successes() {
    let server = Server::start(404, Duration::ZERO).await;
    let service = RoomService::new(config(&server.url));
    service
        .remove_participant("absent-room", "absent-participant", NOW)
        .await
        .unwrap();
    service.delete_room("absent-room", NOW).await.unwrap();
    assert_eq!(server.received.lock().unwrap().len(), 2);
}

#[tokio::test]
async fn twirp_failures_do_not_follow_redirects_or_expose_bodies() {
    for status in [302, 401, 403, 429, 500, 503] {
        let server = Server::start(status, Duration::ZERO).await;
        let error = RoomService::new(config(&server.url))
            .remove_participant("room", "participant", NOW)
            .await
            .unwrap_err();
        assert_eq!(error.status, Some(status));
        assert_eq!(error.code, None);
        assert!(!error.to_string().contains("sensitive"));
        assert_eq!(server.received.lock().unwrap().len(), 1);
    }
}

#[tokio::test]
async fn twirp_invalid_urls_and_missing_configuration_fail_safely() {
    for url in [
        "ftp://internal.test",
        "relative",
        "http://bad host",
        "http://internal.test/ü",
    ] {
        let error = RoomService::new(config(url))
            .delete_room("room", NOW)
            .await
            .unwrap_err();
        assert_eq!(error.code, Some("URI::InvalidURIError"));
        assert!(!error.to_string().contains(url));
    }
    assert!(
        RoomService::new(Config::default())
            .delete_room("room", NOW)
            .await
            .is_err()
    );
}

#[tokio::test]
async fn twirp_read_timeout_is_reported_safely() {
    assert_eq!(OPEN_TIMEOUT, Duration::from_secs(3));
    assert_eq!(READ_TIMEOUT, Duration::from_secs(5));
    let server = Server::start(200, Duration::from_secs(10)).await;
    let service = RoomService::new(config(&server.url));
    let timeouts = Timeouts {
        open: OPEN_TIMEOUT,
        read: Duration::from_millis(40),
    };
    let error = service
        .post(
            "DeleteRoom",
            serde_json::json!({"room": "room"}),
            serde_json::json!({"roomCreate": true}).as_object().unwrap(),
            NOW,
            &timeouts,
        )
        .await
        .unwrap_err();
    assert_eq!(error.code, Some("Net::ReadTimeout"));
    assert_eq!(server.received.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn cleanup_failed_network_retains_retry_and_absent_participant_completes() {
    use campfire_db::models::huddle_cleanup::{HuddleCleanup, Operation};
    let app = crate::controllers::presenters::test_support::TestApp::boot()
        .await
        .expect("WS13 needs the parity seed");
    let id = app
        .db()
        .write(|tx| {
            Ok(HuddleCleanup::create(
                tx,
                Operation::RemoveParticipant,
                "opaque-room",
                Some("opaque-participant"),
                None,
                false,
            )?
            .id)
        })
        .await
        .unwrap();
    let failing = Server::start(503, Duration::ZERO).await;
    assert!(
        !crate::jobs::huddle::perform(app.db(), RoomService::new(config(&failing.url)), id, false)
            .await
            .unwrap()
    );
    let row = app
        .db()
        .read(move |conn| Ok(HuddleCleanup::find_by_id(conn, id)?.unwrap()))
        .await
        .unwrap();
    assert_eq!(row.attempts, 1);
    assert!(row.completed_at.is_none());
    assert_eq!(
        row.next_attempt_at.unwrap(),
        row.last_attempted_at
            .unwrap()
            .since(jiff::SignedDuration::from_secs(15))
    );
    assert!(
        !crate::jobs::huddle::perform(app.db(), RoomService::new(config(&failing.url)), id, false)
            .await
            .unwrap()
    );
    assert_eq!(failing.received.lock().unwrap().len(), 1);
    // Move the persisted deadline to the app's clock; the DB differential covers time travel.
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE huddle_cleanups SET next_attempt_at=? WHERE id=?",
                rusqlite::params![tx.now(), id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let absent = Server::start(404, Duration::ZERO).await;
    assert!(
        crate::jobs::huddle::perform(app.db(), RoomService::new(config(&absent.url)), id, false)
            .await
            .unwrap()
    );
    assert!(
        !crate::jobs::huddle::perform(app.db(), RoomService::new(config(&absent.url)), id, false)
            .await
            .unwrap()
    );
    let row = app
        .db()
        .read(move |conn| Ok(HuddleCleanup::find_by_id(conn, id)?.unwrap()))
        .await
        .unwrap();
    assert_eq!(row.attempts, 2);
    assert!(row.completed_at.is_some());
    assert_eq!(row.next_attempt_at, None);
    assert_eq!(absent.received.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn huddle_reconciler_ends_a_quiet_presenter_without_admin_configuration() {
    use crate::controllers::presenters::test_support::{TestApp,DAVID};
    let app=TestApp::boot().await.expect("WS13 needs the parity seed");
    let db=app.booted.app.db.clone();
    app.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let stream=db.write(|tx| {
        let room=campfire_db::Room::create_for(tx,campfire_db::RoomType::Stage,Some("WS13 stale presenter"),DAVID,&[DAVID])?;
        let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),room.id,DAVID)?.unwrap();
        let session=campfire_db::Session::start(tx,DAVID,None,None)?;
        let grant=campfire_db::models::huddle_grant::HuddleGrant::issue(tx,session.id,member.id,room.id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false})?;
        tx.conn().execute("UPDATE huddle_grants SET last_seen_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(31)),grant.id])?;
        Ok(tx.conn().query_row("INSERT INTO streams(room_id,membership_id,user_id,quality,started_at,created_at,updated_at) VALUES(?,?,?,'1080p15',?,?,?) RETURNING id",rusqlite::params![room.id,member.id,DAVID,tx.now(),tx.now(),tx.now()],|r|r.get::<_,i64>(0))?)
    }).await.unwrap();
    assert_eq!(crate::jobs::huddle::reconcile(&db,RoomService::new(Config::default())).await.unwrap(),0);
    assert!(db.read(move |conn|Ok(conn.query_row("SELECT ended_at IS NOT NULL FROM streams WHERE id=?",[stream],|r|r.get::<_,bool>(0))?)).await.unwrap(),"the process reconciler left a quiet presenter live");
}

#[tokio::test]
async fn resolver_failure_preserves_prior_items_and_does_not_stop_cleanup() {
    let (logs, _guard) = Ws13bLogs::capture();
    use campfire_db::models::{huddle_cleanup::{HuddleCleanup,Operation},huddle_grant::HuddleGrant};
    use crate::controllers::presenters::test_support::{TestApp,DAVID,JASON,KEVIN,DIRECT_DAVID_JASON};
    let app=TestApp::boot().await.expect("WS13 needs the parity seed");
    let db=app.booted.app.db.clone();
    app.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let (first,second,cleanup,stream)=db.write(|tx| {
        let session=campfire_db::Session::start(tx,DAVID,None,None)?;
        let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
        let grant=HuddleGrant::issue(tx,session.id,member.id,member.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13-fixture-value".into()),admin_configured:false})?;
        let first=campfire_db::ActivityItem::find_by_user_and_source(tx.conn(),JASON,"HuddleGrant",grant.id)?.unwrap().id;
        let second=campfire_db::ActivityItem::refresh_unread(tx,KEVIN,"HuddleGrant",grant.id,"huddle_started")?.id;
        tx.conn().execute("UPDATE activity_items SET created_at=? WHERE id IN (?,?)",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(120)),first,second])?;
        tx.conn().execute_batch(&format!("CREATE TRIGGER ws13_reject_resolution BEFORE UPDATE ON activity_items WHEN NEW.id={second} BEGIN SELECT RAISE(ABORT,'ws13 reject resolution'); END"))?;
        let stream=stage_stream_state_fixture(tx,"1080p15")?;
        let cleanup=HuddleCleanup::create(tx,Operation::DeleteRoom,"ws13-resolver-failure",None,None,false)?.id;
        tx.conn().execute_batch(&format!("CREATE TRIGGER ws13_require_resolution_order BEFORE UPDATE ON huddle_cleanups WHEN NEW.id={cleanup} AND ((SELECT event_type FROM activity_items WHERE id={first})!='huddle_missed' OR (SELECT ended_at FROM streams WHERE id={stream}) IS NULL) BEGIN SELECT RAISE(ABORT,'ws13 cleanup ran first'); END"))?;
        Ok((first,second,cleanup,stream))
    }).await.unwrap();
    let server=Server::start(404,Duration::ZERO).await;
    assert_eq!(crate::jobs::huddle::reconcile(&db,RoomService::new(config(&server.url))).await.unwrap(),1);
    db.read(move |conn| {
        assert_eq!(campfire_db::ActivityItem::find(conn,first)?.event_type,"huddle_missed");
        assert_eq!(campfire_db::ActivityItem::find(conn,second)?.event_type,"huddle_started");
        assert!(HuddleCleanup::find_by_id(conn,cleanup)?.unwrap().completed_at.is_some());
        assert!(conn.query_row("SELECT ended_at IS NOT NULL FROM streams WHERE id=?",[stream],|r|r.get::<_,bool>(0))?);
        Ok(())
    }).await.unwrap();
    assert_eq!(server.received.lock().unwrap().len(),1);
    logs.assert_oracle("resolver_sql_failure");
}

fn stage_stream_state_fixture(tx:&mut campfire_db::Tx<'_>,quality:&str)->campfire_db::Result<i64> {
    use crate::controllers::presenters::test_support::DAVID;
    let room=campfire_db::Room::create_for(tx,campfire_db::RoomType::Stage,Some("WS13 phase failure stage"),DAVID,&[DAVID])?;
    let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),room.id,DAVID)?.unwrap();
    Ok(tx.conn().query_row("INSERT INTO streams(room_id,membership_id,user_id,quality,started_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?) RETURNING id",rusqlite::params![room.id,member.id,DAVID,quality,tx.now(),tx.now(),tx.now()],|r|r.get(0))?)
}

#[tokio::test]
async fn huddle_stale_stream_failure_preserves_prior_ends_and_does_not_stop_cleanup() {
    let (logs, _guard) = Ws13bLogs::capture();
    use campfire_db::models::huddle_cleanup::{HuddleCleanup,Operation};
    let app=crate::controllers::presenters::test_support::TestApp::boot().await.expect("WS13 needs the parity seed");
    let db=app.booted.app.db.clone();
    app.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let (first,second,cleanup)=db.write(|tx| {
        let first=stage_stream_state_fixture(tx,"1080p15")?;
        let second=stage_stream_state_fixture(tx,"4k60")?;
        let cleanup=HuddleCleanup::create(tx,Operation::DeleteRoom,"ws13-stream-failure",None,None,false)?.id;
        Ok((first,second,cleanup))
    }).await.unwrap();
    let server=Server::start(404,Duration::ZERO).await;
    assert_eq!(crate::jobs::huddle::reconcile(&db,RoomService::new(config(&server.url))).await.unwrap(),1);
    db.read(move |conn| {
        assert!(conn.query_row("SELECT ended_at IS NOT NULL FROM streams WHERE id=?",[first],|r|r.get::<_,bool>(0))?);
        assert!(conn.query_row("SELECT ended_at IS NULL FROM streams WHERE id=?",[second],|r|r.get::<_,bool>(0))?);
        assert!(HuddleCleanup::find_by_id(conn,cleanup)?.unwrap().completed_at.is_some());
        Ok(())
    }).await.unwrap();
    assert_eq!(server.received.lock().unwrap().len(),1);
    logs.assert_oracle("stream_validation_failure");
}

// A thread-local subscriber captures the actual process reconciler's messages.
// The SQLite triggers above remain real failures; no phase is mocked in Rust.
struct Ws13bLogs(Arc<Mutex<Vec<u8>>>);
struct Ws13bLogWriter(Arc<Mutex<Vec<u8>>>);
impl std::io::Write for Ws13bLogWriter {
    fn write(&mut self, bytes: &[u8]) -> std::io::Result<usize> {
        self.0.lock().unwrap().extend_from_slice(bytes);
        Ok(bytes.len())
    }
    fn flush(&mut self) -> std::io::Result<()> {
        Ok(())
    }
}
impl Ws13bLogs {
    fn capture() -> (Self, tracing::subscriber::DefaultGuard) {
        let bytes = Arc::new(Mutex::new(Vec::new()));
        let writer = bytes.clone();
        let subscriber = tracing_subscriber::fmt()
            .with_ansi(false)
            .with_target(false)
            .with_level(false)
            .without_time()
            .with_writer(move || Ws13bLogWriter(writer.clone()))
            .finish();
        (Self(bytes), tracing::subscriber::set_default(subscriber))
    }
    fn assert_oracle(&self, name: &str) {
        let vectors: Value =
            serde_json::from_str(include_str!("huddle_job_contract_vectors.json")).unwrap();
        let row = vectors["reconciler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap();
        let bytes = self.0.lock().unwrap();
        let text = String::from_utf8_lossy(&bytes);
        let messages: Vec<_> = text
            .lines()
            .filter(|line| {
                line.starts_with("Huddle invitation resolution failed:")
                    || line.starts_with("Huddle stream reconciliation failed:")
            })
            .collect();
        assert_eq!(serde_json::json!(messages), row["logs"], "{text}");
    }
}

#[test]
fn reconciler_standard_error_messages_match_the_rails_oracle() {
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_job_contract_vectors.json")).unwrap();
    for (name, phase) in [
        ("resolver_failure", "invitation resolution"),
        ("stream_failure", "stream reconciliation"),
    ] {
        let row = vectors["reconciler"]
            .as_array()
            .unwrap()
            .iter()
            .find(|c| c["name"] == name)
            .unwrap();
        let message = crate::jobs::huddle::reconciliation_failure_message(
            phase,
            &campfire_db::Error::Other("boom".into()),
        );
        assert_eq!(serde_json::json!([message]), row["logs"]);
    }
}

#[tokio::test]
async fn one_huddle_pass_runs_all_three_phases_in_rails_order() {
    use crate::controllers::presenters::test_support::{DAVID, DIRECT_DAVID_JASON, JASON, TestApp};
    use campfire_db::models::{
        huddle_cleanup::{HuddleCleanup, Operation},
        huddle_grant::HuddleGrant,
    };
    let app = TestApp::boot()
        .await
        .expect("WS13b requires the parity seed");
    let db = app.booted.app.db.clone();
    app.booted.jobs.shutdown(Duration::from_secs(2)).await;
    let cleanup=db.write(|tx| {
        let session=campfire_db::Session::start(tx,DAVID,None,None)?;
        let member=campfire_db::Membership::find_by_room_and_user(tx.conn(),DIRECT_DAVID_JASON,DAVID)?.unwrap();
        let grant=HuddleGrant::issue(tx,session.id,member.id,member.room_id,&campfire_db::models::room_delete::HuddleConfig {api_secret:Some("ws13b-fixture-value".into()),admin_configured:false})?;
        let item=campfire_db::ActivityItem::find_by_user_and_source(tx.conn(),JASON,"HuddleGrant",grant.id)?.unwrap().id;
        tx.conn().execute("UPDATE activity_items SET created_at=? WHERE id=?",rusqlite::params![tx.now().ago(jiff::SignedDuration::from_secs(120)),item])?;
        let stream=stage_stream_state_fixture(tx,"1080p15")?;
        let cleanup=HuddleCleanup::create(tx,Operation::DeleteRoom,"ws13b-all-phases",None,None,false)?.id;
        tx.conn().execute_batch(&format!("CREATE TABLE ws13b_phase_order(seq INTEGER PRIMARY KEY AUTOINCREMENT,name TEXT NOT NULL);
            CREATE TRIGGER ws13b_invitation_phase AFTER UPDATE ON activity_items WHEN NEW.id={item} AND OLD.event_type='huddle_started' AND NEW.event_type='huddle_missed' BEGIN INSERT INTO ws13b_phase_order(name) VALUES('invitations'); END;
            CREATE TRIGGER ws13b_stream_order BEFORE UPDATE ON streams WHEN NEW.id={stream} AND NEW.ended_at IS NOT NULL AND (SELECT event_type FROM activity_items WHERE id={item})!='huddle_missed' BEGIN SELECT RAISE(ABORT,'ws13b stream ran first'); END;
            CREATE TRIGGER ws13b_stream_phase AFTER UPDATE ON streams WHEN NEW.id={stream} AND OLD.ended_at IS NULL AND NEW.ended_at IS NOT NULL BEGIN INSERT INTO ws13b_phase_order(name) VALUES('streams'); END;
            CREATE TRIGGER ws13b_cleanup_order BEFORE UPDATE ON huddle_cleanups WHEN NEW.id={cleanup} AND NEW.attempts>OLD.attempts AND (SELECT ended_at FROM streams WHERE id={stream}) IS NULL BEGIN SELECT RAISE(ABORT,'ws13b cleanup ran first'); END;
            CREATE TRIGGER ws13b_cleanup_phase AFTER UPDATE ON huddle_cleanups WHEN NEW.id={cleanup} AND NEW.attempts>OLD.attempts BEGIN INSERT INTO ws13b_phase_order(name) VALUES('cleanup'); END;"))?;
        Ok(cleanup)
    }).await.unwrap();
    let server = Server::start(404, Duration::ZERO).await;
    assert_eq!(
        crate::jobs::huddle::reconcile(&db, RoomService::new(config(&server.url)))
            .await
            .unwrap(),
        1
    );
    let phases = db
        .read(|conn| {
            let mut statement = conn.prepare("SELECT name FROM ws13b_phase_order ORDER BY seq")?;
            Ok(statement
                .query_map([], |r| r.get::<_, String>(0))?
                .collect::<std::result::Result<Vec<_>, _>>()?)
        })
        .await
        .unwrap();
    let vectors: Value =
        serde_json::from_str(include_str!("huddle_job_contract_vectors.json")).unwrap();
    assert_eq!(serde_json::json!(phases), vectors["reconciler"][0]["calls"]);
    assert!(
        db.read(move |conn| Ok(HuddleCleanup::find_by_id(conn, cleanup)?
            .unwrap()
            .completed_at
            .is_some()))
            .await
            .unwrap()
    );
    assert_eq!(server.received.lock().unwrap().len(), 1);
    assert_eq!(
        crate::jobs::huddle::reconcile(&db, RoomService::new(config(&server.url)))
            .await
            .unwrap(),
        0
    );
    assert_eq!(server.received.lock().unwrap().len(), 1);
}

#[tokio::test]
async fn cleanup_reconciles_at_most_one_hundred_due_rows_without_admin_configuration_idles() {
    use campfire_db::models::huddle_cleanup::{HuddleCleanup, Operation};
    let app = crate::controllers::presenters::test_support::TestApp::boot()
        .await
        .expect("WS13 needs the parity seed");
    app.db()
        .write(|tx| {
            for index in 0..102 {
                HuddleCleanup::create(
                    tx,
                    Operation::DeleteRoom,
                    &format!("opaque-room-{index}"),
                    None,
                    None,
                    false,
                )?;
            }
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        crate::jobs::huddle::reconcile(app.db(), RoomService::new(Config::default()))
            .await
            .unwrap(),
        0
    );
    let server = Server::start(404, Duration::ZERO).await;
    assert_eq!(
        crate::jobs::huddle::reconcile(app.db(), RoomService::new(config(&server.url)))
            .await
            .unwrap(),
        100
    );
    assert_eq!(server.received.lock().unwrap().len(), 100);
    let pending: i64 = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM huddle_cleanups WHERE completed_at IS NULL AND attempts=0",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(pending, 2);
}

#[tokio::test]
async fn cleanup_background_queue_and_http_enqueue_rollback() {
    // The child boots the real runner with fixture LiveKit environment variables, without
    // changing this shared test process's environment or replacing a production handler.
    if std::env::var("WS13_CLEANUP_CHILD").is_err() {
        let server = Server::start(200, Duration::ZERO).await;
        let output = tokio::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "huddle::tests::cleanup_background_queue_and_http_enqueue_rollback",
                "--nocapture",
            ])
            .env("WS13_CLEANUP_CHILD", "1")
            .env("LIVEKIT_INTERNAL_URL", &server.url)
            .env("LIVEKIT_URL", "wss://huddle.example.test")
            .env("LIVEKIT_API_KEY", KEY)
            .env("LIVEKIT_API_SECRET", SECRET)
            .env("LIVEKIT_GATEWAY_SECRET", "ws13-fixture-gateway-secret")
            .kill_on_drop(true)
            .output()
            .await
            .unwrap();
        assert!(
            output.status.success(),
            "{}\n{}",
            String::from_utf8_lossy(&output.stdout),
            String::from_utf8_lossy(&output.stderr)
        );
        let requests = server.received.lock().unwrap();
        assert_eq!(requests.len(), 1);
        assert_eq!(
            serde_json::from_slice::<Value>(&requests[0].body).unwrap(),
            serde_json::json!({"room": "opaque-queued-room"})
        );
        return;
    }
    use crate::controllers::presenters::test_support::{ALL_TALK, Req, TestApp};
    use campfire_db::models::huddle_cleanup::HuddleCleanup;
    let app = TestApp::boot().await.expect("WS13 needs the parity seed");
    let cleanup_id = app
        .db()
        .write(|tx| Ok(HuddleCleanup::create_room_deletion(tx, "opaque-queued-room", true)?.id))
        .await
        .unwrap();
    let deadline = tokio::time::Instant::now() + Duration::from_secs(10);
    loop {
        let row = app
            .db()
            .read(move |conn| Ok(HuddleCleanup::find_by_id(conn, cleanup_id)?.unwrap()))
            .await
            .unwrap();
        if row.completed_at.is_some() {
            assert_eq!(row.attempts, 1);
            break;
        }
        assert!(
            tokio::time::Instant::now() < deadline,
            "durable cleanup runner did not complete the row"
        );
        tokio::time::sleep(Duration::from_millis(10)).await;
    }
    let loop_config = crate::jobs::periodic::huddle_reconciler(Duration::from_secs(5));
    assert_eq!(
        loop_config
            .tasks()
            .map(|t| (t.name(), t.interval()))
            .collect::<Vec<_>>(),
        vec![("huddle reconciliation", Duration::from_secs(5))]
    );
    let before = app.db().read(|conn| {
        Ok(conn.query_row("SELECT (SELECT COUNT(*) FROM memberships WHERE room_id=?1), (SELECT COUNT(*) FROM messages WHERE room_id=?1), (SELECT COUNT(*) FROM huddle_cleanups)", [ALL_TALK], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?)
    }).await.unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws13_reject_cleanup_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Huddle::CleanupJob' BEGIN SELECT RAISE(ABORT,'ws13 job rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    let reply = app
        .david()
        .write(Req::new(
            axum::http::Method::DELETE,
            &format!("/rooms/{ALL_TALK}"),
        ))
        .await;
    assert_eq!(reply.status, axum::http::StatusCode::INTERNAL_SERVER_ERROR);
    let after = app.db().read(|conn| {
        assert!(campfire_db::Room::find_by_id(conn, ALL_TALK)?.is_some());
        Ok(conn.query_row("SELECT (SELECT COUNT(*) FROM memberships WHERE room_id=?1), (SELECT COUNT(*) FROM messages WHERE room_id=?1), (SELECT COUNT(*) FROM huddle_cleanups)", [ALL_TALK], |r| Ok((r.get::<_, i64>(0)?, r.get::<_, i64>(1)?, r.get::<_, i64>(2)?)))?)
    }).await.unwrap();
    assert_eq!(
        after, before,
        "room deletion must roll back with its rejected cleanup enqueue"
    );
    app.booted.jobs.shutdown(Duration::from_secs(1)).await;
}
