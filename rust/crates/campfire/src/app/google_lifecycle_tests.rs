//! Real lifecycle routes, encrypted credentials and queue, with pinned recorded Google HTTP.
use super::{google_api_tests as support, google_connection_tests};
use crate::{
    controllers::presenters::test_support::{DAVID, JASON, Req, TestApp},
    integrations::{
        google::{
            api,
            client::{Client, Unavailable},
        },
        net::BoxFuture,
    },
};
use campfire_db::{Database, Timestamp};
use campfire_kit::FrozenClock;
use hyper::{Method, StatusCode};
use serde_json::{Value, json};
use std::{
    sync::{Arc, Mutex},
    time::Duration,
};

struct RecordedStop {
    db: Database,
    calls: Mutex<Vec<Value>>,
}
impl Client for RecordedStop {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: Method,
        target: &'a str,
        _headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>> {
        Box::pin(async move {
            assert_eq!(host, "www.googleapis.com");
            assert_eq!(method, Method::POST);
            assert_eq!(target, "/calendar/v3/channels/stop");
            let snapshot = self
                .db
                .read(|c| {
                    Ok((
                        campfire_db::User::find(c, DAVID)?.status.name(),
                        c.query_row(
                            "SELECT EXISTS(SELECT 1 FROM calendar_meeting_caches WHERE user_id=?)",
                            [DAVID],
                            |r| r.get::<_, bool>(0),
                        )?,
                    ))
                })
                .await
                .unwrap();
            self.calls.lock().unwrap().push(json!({"path":target,"body":serde_json::from_slice::<Value>(&body).unwrap(),"status":snapshot.0,"cache":snapshot.1}));
            Ok((200, b"{}".to_vec()))
        })
    }
}
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../vectors/google_lifecycle.json")).unwrap()
}
async fn fixture(case: &Value) -> (TestApp, Arc<RecordedStop>) {
    let at = "2026-03-02T16:00:00Z".parse().unwrap();
    let mut a = TestApp::boot_with_clock_and_env(
        Arc::new(FrozenClock::new(at)),
        &[("APP_URL", "http://campfire.test")],
    )
    .await
    .unwrap();
    a.booted.jobs.stop(Duration::from_secs(5)).await;
    let client = Arc::new(RecordedStop {
        db: a.db().clone(),
        calls: Mutex::new(vec![]),
    });
    a.booted.app.google.install_api(api::Api::new(
        api::Config {
            client_secret: "FAKE-lifecycle-secret".into(),
            ..support::config()
        },
        client.clone(),
    ));
    a.db().write(|tx| {tx.conn().execute_batch("DELETE FROM google_accounts;DELETE FROM event_calendar_entries;DELETE FROM calendar_push_channels;DELETE FROM calendar_meeting_caches;DELETE FROM background_jobs;")?;Ok(())}).await.unwrap();
    let id = case["account_id"].as_i64().unwrap_or(9_800_000_000);
    if case["spec"]["account"] != false {
        support::grant(
            &a,
            DAVID,
            Timestamp::from_jiff(at).since(jiff::SignedDuration::from_hours(1)),
            false,
        )
        .await;
    }
    let input = case.clone();
    a.db().write(move |tx| {
  let spec=&input["spec"];
  tx.conn().execute("UPDATE users SET meeting_status_enabled=1,ooo_calendar_enabled=1,ooo_until=?,ooo_note=?,ooo_broadcast=1 WHERE id=?",rusqlite::params![(spec["manual"]==true).then_some(tx.now().since(jiff::SignedDuration::from_hours(24))),(spec["manual"]==true).then_some("Own note"),DAVID])?;
  tx.conn().execute("UPDATE google_accounts SET id=?,email=?,scopes=NULL,disconnected_reason=? WHERE user_id=?",rusqlite::params![id,if spec["invalid"]==true {""}else{"fixture@example.test"},spec["reason"].as_str(),DAVID])?;
  if spec["unreadable"]==true {tx.conn().execute("UPDATE google_accounts SET refresh_token='unreadable' WHERE user_id=?",[DAVID])?;}
  tx.conn().execute("INSERT INTO calendar_push_channels(user_id,channel_id,token_digest,resource_id,created_at,updated_at) VALUES(?,'lifecycle-channel','fixture','lifecycle-resource',?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;
  if spec["cache"]!=false {
   let busy=json!([["2026-03-02T15:59:00Z","2026-03-02T17:00:00Z"]]).to_string();
   tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,fetched_at,in_meeting_broadcast,created_at,updated_at) VALUES(?,?,?,?,1,?,?)",rusqlite::params![DAVID,busy,busy,tx.now(),tx.now(),tx.now()])?;
  }
  for event in input["entry_event_ids"].as_array().unwrap() {let event=event.as_i64().unwrap();tx.conn().execute("INSERT INTO event_calendar_entries(event_id,user_id,google_event_id,synced_at,created_at,updated_at) VALUES(?,?,?,?,?,?)",rusqlite::params![event,DAVID,format!("lifecycle-{event}"),tx.now(),tx.now(),tx.now()])?;}
  Ok(())
 }).await.unwrap();
    (a, client)
}
async fn exercise(
    a: &TestApp,
    case: &Value,
) -> crate::controllers::presenters::test_support::Reply {
    let deactivation = case["spec"]["kind"] == "deactivate";
    let mut browser = a.sign_in(if deactivation { JASON } else { DAVID }).await;
    browser.get("/users/me/profile").await;
    google_connection_tests::sudo(a, &mut browser).await;
    browser
        .write(Req::new(
            Method::DELETE,
            &if deactivation {
                format!("/account/users/{DAVID}")
            } else {
                "/google/connection".into()
            },
        ))
        .await
}
#[tokio::test]
async fn google_lifecycle_matches_pinned_rails_state_jobs_and_remote_stop_order() {
    let v = vectors();
    for case in v["cases"].as_array().unwrap() {
        let (a, client) = fixture(case).await;
        let name = case["spec"]["name"].as_str().unwrap();
        let response = exercise(&a, case).await;
        let expected = if case["error"].is_null() {
            StatusCode::FOUND
        } else {
            StatusCode::INTERNAL_SERVER_ERROR
        };
        assert_eq!(response.status, expected, "{name}: {}", response.text());
        if expected == StatusCode::INTERNAL_SERVER_ERROR {
            assert_eq!(
                response.text(),
                include_str!("../../../../../public/500.html")
            );
        }
        let state=a.db().read(|c| {
   let account=campfire_db::models::google_account::GoogleAccount::for_user(c,DAVID)?.map(|a|json!({"id":a.id,"email":a.email,"disconnected_reason":a.disconnected_reason}));
   let row=c.query_row("SELECT ooo_until,ooo_broadcast FROM users WHERE id=?",[DAVID],|r|Ok((r.get::<_,Option<Timestamp>>(0)?,r.get::<_,Option<bool>>(1)?)))?;
   let entries=c.prepare("SELECT event_id FROM event_calendar_entries WHERE user_id=? ORDER BY id")?.query_map([DAVID],|r|r.get::<_,i64>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
   let jobs=c.prepare("SELECT job_class,arguments FROM background_jobs WHERE job_class LIKE 'Calendar::%' ORDER BY id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
   Ok((json!({"account":account,"user_status":campfire_db::User::find(c,DAVID)?.status.name(),"cache":c.query_row("SELECT EXISTS(SELECT 1 FROM calendar_meeting_caches WHERE user_id=?)",[DAVID],|r|r.get::<_,bool>(0))?,"channel":c.query_row("SELECT EXISTS(SELECT 1 FROM calendar_push_channels WHERE user_id=?)",[DAVID],|r|r.get::<_,bool>(0))?,"ooo_until":row.0.map(|t|t.jiff().to_string()),"ooo_broadcast":row.1,"entry_ids":entries}),jobs))
  }).await.unwrap();
        for key in [
            "account",
            "user_status",
            "cache",
            "channel",
            "ooo_until",
            "ooo_broadcast",
            "entry_ids",
        ] {
            assert_eq!(state.0[key], case[key], "{name}: {key}");
        }
        let jobs=state.1.into_iter().map(|(class,args)| {
   let mut args:Value=serde_json::from_str(&args).unwrap();
   if class=="Calendar::DisconnectCleanupJob" {
    let credentials=rails_compat::calendar_credentials::decrypt_snapshot(&a.booted.app.secrets,args[1].as_str().unwrap(),a.booted.app.clock.now()).unwrap();
    args[1]=json!({"access_token":credentials.access_token,"refresh_token":credentials.refresh_token,"access_token_expires_at":credentials.access_token_expires_at.map(|t|t.to_string())});
   } else {args=json!([args["event_id"],args["user_id"]]);}
   json!({"class":class,"args":args})
  }).collect::<Vec<_>>();
        let mut expected_jobs = case["jobs"].clone();
        for job in expected_jobs.as_array_mut().unwrap() {
            if job["class"] == "Calendar::DisconnectCleanupJob"
                && let Some(s) = job["args"][1]["access_token_expires_at"].as_str()
            {
                job["args"][1]["access_token_expires_at"] =
                    json!(s.parse::<jiff::Timestamp>().unwrap().to_string());
            }
        }
        assert_eq!(
            json!(jobs),
            expected_jobs,
            "{name}: durable jobs and decrypted snapshot"
        );
        assert_eq!(
            json!(*client.calls.lock().unwrap()),
            case["calls"],
            "{name}: observed committed state at Google HTTP"
        );
    }
    println!("Pinned Rails Google lifecycle: 8 exercised; 0 skipped");
}

#[tokio::test]
async fn google_disconnect_publishes_complete_rails_badges_and_ooo_notices_after_commit() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
    let v = vectors();
    for case in v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|c| c["spec"]["kind"] == "disconnect" && c["spec"]["cache"] != false)
    {
        let (a, _) = fixture(case).await;
        let mut listener = None;
        for port in 53100..=53199 {
            match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
                Ok(l) => {
                    listener = Some(l);
                    break;
                }
                Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => (),
                Err(e) => panic!("bind lifecycle socket: {e}"),
            }
        }
        let listener = listener.expect("free WS14g test port");
        let addr = listener.local_addr().unwrap();
        let router = a.booted.router.clone();
        let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
        let observer = a.sign_in(JASON).await;
        let mut req = format!("ws://{addr}/cable").into_client_request().unwrap();
        req.headers_mut()
            .insert("cookie", observer.cookie_header().parse().unwrap());
        req.headers_mut()
            .insert("origin", format!("http://{addr}").parse().unwrap());
        req.headers_mut().insert(
            "sec-websocket-protocol",
            "actioncable-v1-json".parse().unwrap(),
        );
        let (mut socket, _) = tokio_tungstenite::connect_async(req).await.unwrap();
        async fn next(
            socket: &mut tokio_tungstenite::WebSocketStream<
                tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
            >,
        ) -> Value {
            tokio::time::timeout(Duration::from_secs(5), async {
                loop {
                    if let Message::Text(s) = socket.next().await.unwrap().unwrap() {
                        let v: Value = serde_json::from_str(&s).unwrap();
                        if v["type"] != "ping" {
                            return v;
                        }
                    }
                }
            })
            .await
            .unwrap()
        }
        assert_eq!(next(&mut socket).await["type"], "welcome");
        let mut expected = std::collections::BTreeMap::new();
        for frame in case["frames"].as_array().unwrap() {
            let parts = frame["stream"]
                .as_str()
                .unwrap()
                .split(':')
                .collect::<Vec<_>>();
            let signed = rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &parts);
            let identifier =
                json!({"channel":"Turbo::StreamsChannel","signed_stream_name":signed}).to_string();
            socket
                .send(Message::Text(
                    json!({"command":"subscribe","identifier":identifier})
                        .to_string()
                        .into(),
                ))
                .await
                .unwrap();
            assert_eq!(next(&mut socket).await["type"], "confirm_subscription");
            expected.insert(signed, frame["payload"].clone());
        }
        assert_eq!(exercise(&a, case).await.status, StatusCode::FOUND);
        for _ in 0..expected.len() {
            let actual = next(&mut socket).await;
            let channel: Value =
                serde_json::from_str(actual["identifier"].as_str().unwrap()).unwrap();
            assert_eq!(
                actual["message"],
                expected
                    .remove(channel["signed_stream_name"].as_str().unwrap())
                    .unwrap(),
                "complete Rails stream bytes"
            );
            let sql = rusqlite::Connection::open(&a.booted.app.config.storage.database).unwrap();
            assert_eq!(
                sql.query_row(
                    "SELECT count(*) FROM calendar_meeting_caches WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, i64>(0)
                )
                .unwrap(),
                0
            );
        }
        assert!(expected.is_empty());
        socket.close(None).await.unwrap();
        server.abort();
        let _ = server.await;
    }
}

#[tokio::test]
async fn google_user_deactivation_and_its_cleanup_jobs_rollback_on_late_enqueue_failure() {
    let v = vectors();
    let case = &v["cases"][0];
    let (a, client) = fixture(case).await;
    let event = case["entry_event_ids"][1].as_i64().unwrap();
    a.db().write(move |tx| {tx.conn().execute_batch(&format!("CREATE TRIGGER reject_lifecycle_sync BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::SyncEntryJob' AND json_extract(NEW.arguments,'$.event_id')={event} BEGIN SELECT RAISE(ABORT,'late lifecycle queue failure'); END"))?;Ok(())}).await.unwrap();
    assert_eq!(
        exercise(&a, case).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    a.db()
        .read(|c| {
            assert_eq!(
                campfire_db::User::find(c, DAVID)?.status,
                campfire_db::Status::Active
            );
            assert!(
                campfire_db::models::google_account::GoogleAccount::for_user(c, DAVID)?
                    .unwrap()
                    .connected()
            );
            for table in ["calendar_push_channels", "calendar_meeting_caches"] {
                assert_eq!(
                    c.query_row(
                        &format!("SELECT count(*) FROM {table} WHERE user_id=?"),
                        [DAVID],
                        |r| r.get::<_, i64>(0)
                    )?,
                    1
                );
            }
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM background_jobs WHERE job_class LIKE 'Calendar::%'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            assert_eq!(
                c.query_row(
                    "SELECT count(*) FROM event_calendar_entries WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, i64>(0)
                )?,
                2
            );
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        json!(*client.calls.lock().unwrap()),
        case["calls"],
        "remote stop precedes the rolled-back writer"
    );
}
