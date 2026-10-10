use super::google_api_tests::{self as support, Recorded};
use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use crate::integrations::google::{
    api::{self, Api, ApiRequest},
    client::{Client, Unavailable},
};
use crate::net::BoxFuture;
use campfire_db::{Timestamp, models::google_account::GoogleAccount};
use hyper::{Method, StatusCode};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::{Value, json};
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
};

struct Race {
    arrivals: tokio::sync::Barrier,
    completed_first: tokio::sync::Notify,
    token_calls: AtomicUsize,
    responses: Vec<Value>,
}
impl Client for Race {
    fn request<'a>(
        &'a self,
        host: &'a str,
        _method: Method,
        target: &'a str,
        _headers: Vec<(String, String)>,
        _body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>> {
        Box::pin(async move {
            assert!(matches!(
                host,
                "oauth2.googleapis.com" | "www.googleapis.com"
            ));
            if target != "/token" {
                return Ok((200, b"{}".to_vec()));
            }
            let ordinal = self.token_calls.fetch_add(1, Ordering::SeqCst);
            self.arrivals.wait().await;
            if ordinal == 1 {
                self.completed_first.notified().await;
            }
            Ok((200, serde_json::to_vec(&self.responses[ordinal]).unwrap()))
        })
    }
}
#[tokio::test]
async fn review_refresh_race_preserves_rails_partial_update() {
    use campfire_db::models::google_account::ConnectionGrant;
    let v: Value = serde_json::from_str(include_str!(
        "../../../../vectors/google_refresh_races.json"
    ))
    .unwrap();
    let now = Timestamp::from_second(v["now"].as_i64().unwrap());
    let a = TestApp::boot_without_periodic_with_clock(Arc::new(campfire_kit::FrozenClock::new(
        now.jiff(),
    )))
    .await
    .unwrap();
    for case in v["cases"].as_array().unwrap() {
        let enc = ArEncryption::new(&a.booted.app.secrets);
        let original = case["original"].as_str().map(str::to_owned);
        a.db()
            .write(move |tx| {
                GoogleAccount::save_connection(
                    tx,
                    &enc,
                    ConnectionGrant {
                        user_id: DAVID,
                        email: "fixture@example.test".into(),
                        access_token: original,
                        refresh_token: Some("refresh-token".into()),
                        access_token_expires_at: Some(now.ago(jiff::SignedDuration::from_hours(1))),
                        scopes: None,
                    },
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let transport = Arc::new(Race {
            arrivals: tokio::sync::Barrier::new(2),
            completed_first: tokio::sync::Notify::new(),
            token_calls: AtomicUsize::new(0),
            responses: case["responses"].as_array().unwrap().clone(),
        });
        let api = Api::new(support::config(), transport.clone());
        let mut first = api
            .credentials(a.db(), &a.booted.app.secrets, DAVID)
            .await
            .unwrap();
        let mut second = api
            .credentials(a.db(), &a.booted.app.secrets, DAVID)
            .await
            .unwrap();
        let first = async {
            let result = api
                .request_with(
                    &mut first,
                    a.db(),
                    &a.booted.app.secrets,
                    ApiRequest::calendar(Method::GET, api::EVENTS, None),
                    now,
                )
                .await;
            transport.completed_first.notify_one();
            result
        };
        let second = api.request_with(
            &mut second,
            a.db(),
            &a.booted.app.secrets,
            ApiRequest::calendar(Method::GET, api::EVENTS, None),
            now,
        );
        let (one, two) = tokio::join!(first, second);
        assert!(one.is_ok() && two.is_ok());
        assert_eq!(transport.token_calls.load(Ordering::SeqCst), 2);
        let account = a
            .db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap();
        let enc = ArEncryption::new(&a.booted.app.secrets);
        assert_eq!(
            account.access_token(&enc).unwrap().as_deref(),
            case["final_token"].as_str(),
            "{case}"
        );
        assert_eq!(
            account.access_token_expires_at.unwrap().jiff(),
            case["final_expiry"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .unwrap(),
            "{case}"
        );
        assert_eq!(
            account.refresh_token(&enc).unwrap().as_deref(),
            case["refresh_token"].as_str()
        );
    }
}
#[tokio::test]
async fn review_drive_null_response_keeps_rails_production_500() {
    let a = TestApp::boot_without_periodic().await.unwrap();
    let now = Timestamp::from_jiff(a.booted.app.clock.now());
    support::grant(
        &a,
        DAVID,
        now.since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
    let v: Value =
        serde_json::from_str(include_str!("../../../../vectors/google_drive_null.json")).unwrap();
    for case in v["cases"].as_array().unwrap() {
        let r = Recorded::new(vec![]);
        r.answer(200, Value::Null);
        support::install(&a, r.clone()).await;
        let mut browser = a.david();
        let response = browser
            .send(
                Req::new(Method::GET, case["path"].as_str().unwrap()).header("accept", "text/html"),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{case}"
        );
        assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert!(case["body_is_production_500"].as_bool().unwrap());
        assert_eq!(
            response.body.as_slice(),
            campfire_static_assets::serve(&campfire_static_assets::StaticRequest {
                method: "GET",
                path: "/500.html",
                ..Default::default()
            })
            .unwrap()
            .body
            .as_ref()
        );
        assert_eq!(
            r.calls.lock().unwrap().len(),
            case["google_calls"].as_u64().unwrap() as usize
        );
    }
}

fn boundaries() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/google_time_boundaries.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn google_access_expiry_boundaries_match_rails_for_account_and_snapshot() {
    use crate::integrations::google::api::Credentials;
    use rails_compat::calendar_credentials::Snapshot;
    let v = boundaries();
    let now = Timestamp::from_second(v["now"].as_i64().unwrap());
    let a = TestApp::boot_without_periodic().await.unwrap();
    for case in v["access"].as_array().unwrap() {
        let expiry = now.since(jiff::SignedDuration::from_micros(
            case["microseconds"].as_i64().unwrap(),
        ));
        support::grant(&a, DAVID, expiry, true).await;
        let account = a
            .db()
            .read(|c| GoogleAccount::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap();
        assert_eq!(
            account
                .access_token_expired(&ArEncryption::new(&a.booted.app.secrets), now)
                .unwrap(),
            case["expired"].as_bool().unwrap(),
            "{case}"
        );
        for snapshot in [false, true] {
            let r = Recorded::new(
                case["responses"]
                    .as_array()
                    .unwrap()
                    .iter()
                    .map(|p| {
                        (
                            p[0].as_u64().unwrap() as u16,
                            p[1].as_str().unwrap().as_bytes().to_vec(),
                        )
                    })
                    .collect(),
            );
            let api = Api::new(support::config(), r.clone());
            let mut credentials = if snapshot {
                Credentials::snapshot(Snapshot {
                    access_token: Some("access-token".into()),
                    refresh_token: Some("refresh-token".into()),
                    access_token_expires_at: Some(expiry.jiff()),
                })
            } else {
                api.credentials(a.db(), &a.booted.app.secrets, DAVID)
                    .await
                    .unwrap()
            };
            api.request_with(
                &mut credentials,
                a.db(),
                &a.booted.app.secrets,
                ApiRequest::calendar(
                    Method::GET,
                    "/calendar/v3/calendars/primary/events/boundary",
                    None,
                ),
                now,
            )
            .await
            .unwrap();
            let calls: Vec<Value> = r
                .calls
                .lock()
                .unwrap()
                .iter()
                .map(|c| json!({"method":c["method"],"path":c["path"]}))
                .collect();
            assert_eq!(json!(calls), case["requests"], "snapshot={snapshot} {case}");
            assert!(r.answers.lock().unwrap().is_empty());
        }
    }
}
#[test]
fn google_flow_expiry_boundaries_match_rails() {
    use crate::integrations::google::sign_in;
    let v = boundaries();
    let secrets = rails_compat::Secrets::new("FAKE-boundary-secret");
    let now = jiff::Timestamp::from_second(v["now"].as_i64().unwrap()).unwrap();
    let signed = sign_in::state_verifier(&secrets).generate(&json!("fixture-state"), None, None);
    for c in v["flow"].as_array().unwrap() {
        let flow = json!({"state":"fixture-state","nonce":"fixture-nonce","verifier":"fixture-verifier","exp":now.as_second()+c["offset"].as_i64().unwrap()});
        assert_eq!(
            sign_in::valid_flow(&flow, &signed, &secrets, now),
            c["valid"].as_bool().unwrap()
        );
    }
}
#[tokio::test]
async fn google_meeting_throttle_and_followup_boundaries_match_rails() {
    use crate::integrations::google::meeting_refresh;
    use campfire_db::models::google_meeting_cache as cache;
    let v = boundaries();
    let now = Timestamp::from_second(v["now"].as_i64().unwrap());
    let a = TestApp::boot_without_periodic().await.unwrap();
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("DELETE FROM google_accounts WHERE user_id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    support::grant(
        &a,
        DAVID,
        now.since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET meeting_status_enabled=1 WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for c in v["cache"].as_array().unwrap() {
        let fetched =
            now.ago(jiff::SignedDuration::from_secs(60))
                .since(jiff::SignedDuration::from_micros(
                    c["microseconds"].as_i64().unwrap(),
                ));
        a.db()
            .write(move |tx| {
                cache::complete(tx, DAVID, Some(json!([])), Some(json!([])), None, fetched)?;
                tx.conn().execute(
                    "UPDATE calendar_meeting_caches SET refresh_pending_at=? WHERE user_id=?",
                    rusqlite::params![now, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let r = Recorded::new(vec![]);
        r.answer(200, json!({"items":[]}));
        support::install(&a, r.clone()).await;
        let result = meeting_refresh::refresh(&a.booted.app, DAVID, now)
            .await
            .unwrap();
        assert_eq!(format!("{result:?}").to_lowercase(), c["result"]);
        assert_eq!(
            r.calls.lock().unwrap().len(),
            c["requests"].as_array().unwrap().len()
        );
    }
    for c in v["followup"].as_array().unwrap() {
        let pending =
            now.ago(jiff::SignedDuration::from_secs(60))
                .since(jiff::SignedDuration::from_micros(
                    c["microseconds"].as_i64().unwrap(),
                ));
        a.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE calendar_meeting_caches SET refresh_pending_at=? WHERE user_id=?",
                    rusqlite::params![pending, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let claimed = a
            .db()
            .write(move |tx| cache::follow_up(tx, DAVID, now))
            .await
            .unwrap();
        assert_eq!(claimed, c["claimed"].as_bool().unwrap());
    }
}
#[tokio::test]
async fn google_channel_renewal_boundary_matches_rails() {
    use crate::integrations::google::calendar;
    use campfire_db::models::google_calendar::replace_watch;
    let v = boundaries();
    let now = Timestamp::from_second(v["now"].as_i64().unwrap());
    let clock = Arc::new(campfire_kit::FrozenClock::new(now.jiff()));
    let a = TestApp::boot_without_periodic_with_clock(clock)
        .await
        .unwrap();
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            tx.conn()
                .execute("DELETE FROM calendar_push_channels", [])?;
            Ok(())
        })
        .await
        .unwrap();
    support::grant(
        &a,
        DAVID,
        now.since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
    for c in v["renewal"].as_array().unwrap() {
        let expiry = now.since(jiff::SignedDuration::from_hours(24)).since(
            jiff::SignedDuration::from_micros(c["microseconds"].as_i64().unwrap()),
        );
        a.db()
            .write(move |tx| {
                replace_watch(
                    tx,
                    DAVID,
                    "fixture-channel",
                    "fixture-digest",
                    Some("fixture-resource"),
                    Some(expiry),
                )
            })
            .await
            .unwrap();
        let r = Recorded::new(vec![]);
        r.answer(
            200,
            json!({"resourceId":"new-resource","expiration":(now.as_second()+172800)*1000}),
        );
        r.answer(204, Value::Null);
        let mut config = support::config();
        config.webhook_url = Some("https://campfire.test/google/calendar/notifications".into());
        a.booted.app.google.install_api(Api::new(config, r.clone()));
        calendar::renew(&a.booted.app).await.unwrap();
        assert_eq!(
            r.calls.lock().unwrap().len(),
            if c["renewed"].as_bool().unwrap() {
                2
            } else {
                0
            },
            "{c}"
        );
    }
}

#[tokio::test]
async fn review_emitted_calendar_jobs_have_working_consumers() {
    use campfire_db::{
        Event,
        models::google_calendar::{InboundSyncJob, MeetLinkJob},
    };
    let a = TestApp::boot_without_periodic().await.unwrap();
    support::install(&a, Recorded::new(vec![])).await;
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            Ok(())
        })
        .await
        .unwrap();
    a.db()
        .write(|tx| {
            tx.emit_after_commit(Event::job(&InboundSyncJob((DAVID,))));
            tx.emit_after_commit(Event::job(&MeetLinkJob { event_id: -1 }));
            Ok(())
        })
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        loop {
            let running = a.db().read(|c| Ok(c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class IN ('Calendar::InboundSyncJob','Calendar::MeetLinkJob') AND status != 'failed'",[],|r|r.get::<_,i64>(0))?)).await.unwrap();
            if running == 0 { break; }
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    }).await.unwrap();
    let errors = a.db().read(|c| Ok(c.prepare("SELECT last_error FROM background_jobs WHERE job_class IN ('Calendar::InboundSyncJob','Calendar::MeetLinkJob') AND status='failed'")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?)).await.unwrap();
    assert!(
        errors.is_empty(),
        "Calendar work permanently failed: {errors:?}"
    );
}

struct InboundGate {
    recorded: Arc<Recorded>,
    entered: tokio::sync::Notify,
    release: tokio::sync::Notify,
    path: String,
}
impl Client for InboundGate {
    fn request<'a>(
        &'a self,
        host: &'a str,
        method: Method,
        target: &'a str,
        headers: Vec<(String, String)>,
        body: Vec<u8>,
    ) -> BoxFuture<'a, Result<(u16, Vec<u8>), Unavailable>> {
        Box::pin(async move {
            if method == Method::GET && target == self.path {
                self.entered.notify_one();
                self.release.notified().await;
            }
            self.recorded
                .request(host, method, target, headers, body)
                .await
        })
    }
}
#[tokio::test]
async fn review_calendar_runner_declines_and_provisions_like_rails() {
    use campfire_db::{
        Event,
        models::google_calendar::{InboundSyncJob, MeetLinkJob},
    };
    use hyper::Method;
    let at: jiff::Timestamp = "2026-03-02T16:00:00Z".parse().unwrap();
    let a = TestApp::boot_without_periodic_with_clock(Arc::new(campfire_kit::FrozenClock::new(at)))
        .await
        .unwrap();
    let mut drain = super::google_test_support::QueueDrain::install(&a).await;
    let r = Recorded::new(vec![]);
    a.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DELETE FROM event_calendar_entries;DELETE FROM google_accounts;")?;
            Ok(())
        })
        .await
        .unwrap();
    support::grant(
        &a,
        DAVID,
        Timestamp::from_jiff(a.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(1)),
        false,
    )
    .await;
    let decline = 9_500_000_001i64;
    let meet = 9_500_000_002i64;
    let remote = campfire_db::models::google_entry::google_id(decline, DAVID);
    let gate = Arc::new(InboundGate {
        recorded: r.clone(),
        entered: tokio::sync::Notify::new(),
        release: tokio::sync::Notify::new(),
        path: format!("/calendar/v3/calendars/primary/events/{remote}"),
    });
    a.booted
        .app
        .google
        .install_api(Api::new(support::config(), gate.clone()));
    r.answer_for(
        Method::GET,
        &format!("/calendar/v3/calendars/primary/events/{remote}"),
        200,
        json!({"status":"cancelled"}),
    );
    r.answer_for(
        Method::DELETE,
        &format!("/calendar/v3/calendars/primary/events/{remote}"),
        200,
        json!({}),
    );
    r.answer_for(
        Method::POST,
        "/calendar/v3/calendars/primary/events",
        200,
        json!({}),
    );
    let conference = campfire_db::models::google_entry::google_id(meet, DAVID);
    r.answer_for(
        Method::PATCH,
        &format!("/calendar/v3/calendars/primary/events/{conference}?conferenceDataVersion=1"),
        200,
        json!({"hangoutLink":"https://meet.google.com/abc-defg-hij"}),
    );
    a.db().write(move |tx| {
        for id in [decline,meet] {
            tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,meet_link_requested,created_at,updated_at) VALUES(?,?,?,'Runner consumer',?,'UTC',?,?,?)",rusqlite::params![id,crate::controllers::presenters::test_support::ALL_TALK,DAVID,tx.now().since(jiff::SignedDuration::from_hours(1)),id==meet,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO event_attendances(event_id,user_id,response,created_at,updated_at) VALUES(?,?,'going',?,?)",rusqlite::params![id,DAVID,tx.now(),tx.now()])?;
        }
        let entry=campfire_db::models::google_entry::reserve(tx,decline,DAVID)?;
        campfire_db::models::google_entry::success(tx,&entry)?;
        Ok(())
    }).await.unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(5), async {
        a.db()
            .write(|tx| {
                tx.emit_after_commit(Event::job(&InboundSyncJob((DAVID,))));
                Ok(())
            })
            .await
            .unwrap();
        // Pin the inbound scope before Meet reserves its entry, while leaving
        // both consumers free to run concurrently after the HTTP gate opens.
        gate.entered.notified().await;
        a.db()
            .write(move |tx| {
                tx.emit_after_commit(Event::job(&MeetLinkJob { event_id: meet }));
                Ok(())
            })
            .await
            .unwrap();
        gate.release.notify_one();
        drain.calendar(&a).await;
    })
    .await
    .expect("Calendar consumers did not drain within five seconds");
    let (errors,response,link)=a.db().read(move |c| {
        let errors=c.prepare("SELECT last_error FROM background_jobs WHERE job_class IN ('Calendar::InboundSyncJob','Calendar::MeetLinkJob','Calendar::SyncEntryJob') AND status='failed'")?.query_map([],|r|r.get::<_,String>(0))?.collect::<rusqlite::Result<Vec<_>>>()?;
        Ok((errors,c.query_row("SELECT response FROM event_attendances WHERE event_id=? AND user_id=?",[decline,DAVID],|r|r.get::<_,String>(0))?,c.query_row("SELECT meet_link FROM events WHERE id=?",[meet],|r|r.get::<_,Option<String>>(0))?))
    }).await.unwrap();
    assert!(errors.is_empty(), "Calendar consumers failed: {errors:?}");
    assert_eq!(response, "declined");
    assert_eq!(
        link.as_deref(),
        Some("https://meet.google.com/abc-defg-hij")
    );
    assert_eq!(r.calls.lock().unwrap().len(), 4);
}
