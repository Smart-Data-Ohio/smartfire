use super::google_api_tests::{self as support, Recorded};
use crate::{
    controllers::presenters::test_support::{ALL_TALK, DAVID, Req, TestApp},
    integrations::google::{api, calendar_sync},
};
use campfire_db::{CalendarEvent, Timestamp};
use campfire_kit::FrozenClock;
use hyper::Method;
use rusqlite::OptionalExtension;
use serde_json::{Value, json};
use std::{sync::Arc, time::Duration};

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/google_calendar_consumers.json"
    ))
    .unwrap()
}
async fn fixture(case: &Value) -> (TestApp, Arc<Recorded>) {
    let at: jiff::Timestamp = "2026-03-02T16:00:00Z".parse().unwrap();
    let mut app = TestApp::boot_with_clock_and_env(
        Arc::new(FrozenClock::new(at)),
        &[("APP_URL", "http://campfire.test")],
    )
    .await
    .unwrap();
    app.booted.jobs.stop(Duration::from_secs(5)).await;
    let recorded = Recorded::new(vec![]);
    app.booted.app.google.install_api(api::Api::new(
        api::Config {
            client_secret: "FAKE-calendar-consumer-secret".into(),
            ..support::config()
        },
        recorded.clone(),
    ));
    app.db().write(|tx| {tx.conn().execute_batch("DELETE FROM event_calendar_entries;DELETE FROM google_accounts;DELETE FROM calendar_push_channels;DELETE FROM background_jobs;")?;Ok(())}).await.unwrap();
    let spec = case["spec"].clone();
    if spec["account"] != false {
        support::grant(
            &app,
            DAVID,
            Timestamp::from_jiff(at).since(jiff::SignedDuration::from_hours(1)),
            false,
        )
        .await;
    }
    let seed_case = case.clone();
    app.db().write(move |tx| {
        let case=seed_case;
        for row in case["events"].as_array().unwrap() {
            let id=row["id"].as_i64().unwrap();
            let head=case["events"][0]["id"].as_i64().unwrap();
            let series=spec["series"]==true;
            tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,ends_at,time_zone,series_id,recurrence_rule,recurrence_until,meet_link_requested,meet_link,cancelled_at,created_at,updated_at) VALUES(?,?,?,'Calendar consumer',?,?,'UTC',?,?,?,?,?,?,?,?)",rusqlite::params![id,ALL_TALK,DAVID,row["starts_at"].as_str().unwrap().parse::<jiff::Timestamp>().map(Timestamp::from_jiff).unwrap(),row["ends_at"].as_str().map(|s|s.parse::<jiff::Timestamp>().map(Timestamp::from_jiff).unwrap()),series.then_some(head),series.then_some("weekly"),series.then_some("2026-03-16"),row["requested"].as_bool().unwrap(),row["link"].as_str(),(row["cancelled"]==true).then_some(tx.now()),tx.now(),tx.now()])?;
            if let Some(response)=row["response"].as_str() {
                tx.conn().execute("INSERT INTO event_attendances(event_id,user_id,response,created_at,updated_at) VALUES(?,?,?,?,?)",rusqlite::params![id,DAVID,response,tx.now(),tx.now().ago(jiff::SignedDuration::from_secs(60))])?;
            }
        }
        let index=spec["index"].as_u64().unwrap_or(0) as usize;
        let id=case["events"][index]["id"].as_i64().unwrap();
        if spec["invalid"]==true {tx.conn().execute("UPDATE events SET title='' WHERE id=?",[id])?;}
        if spec["inactive"]==true {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[DAVID])?;}
        if spec["nonmember"]==true {tx.conn().execute("DELETE FROM memberships WHERE room_id=? AND user_id=?",[ALL_TALK,DAVID])?;}
        if spec["disconnected"]==true {tx.conn().execute("UPDATE google_accounts SET disconnected_reason='revoked' WHERE user_id=?",[DAVID])?;}
        if let Some(scope)=spec["scopes"].as_str() {tx.conn().execute("UPDATE google_accounts SET scopes=? WHERE user_id=?",rusqlite::params![scope,DAVID])?;}
        tx.conn().execute("INSERT INTO calendar_push_channels(user_id,channel_id,token_digest,created_at,updated_at) VALUES(?,'fixture-consumer-channel','fixture',?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;
        if let Some(entry_id)=case["entry_id"].as_i64() {
            tx.conn().execute("INSERT INTO event_calendar_entries(id,event_id,user_id,google_event_id,synced_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",rusqlite::params![entry_id,id,DAVID,campfire_db::models::google_entry::google_id(id,DAVID),tx.now(),tx.now(),tx.now()])?;
            if spec["extra"]==true {
                let extra=case["events"].as_array().unwrap().last().unwrap()["id"].as_i64().unwrap();
                tx.conn().execute("INSERT INTO event_calendar_entries(id,event_id,user_id,google_event_id,synced_at,created_at,updated_at) VALUES(?,?,?,?,?,?,?)",rusqlite::params![entry_id+1,extra,DAVID,campfire_db::models::google_entry::google_id(extra,DAVID),tx.now(),tx.now(),tx.now()])?;
            }
        } else if let Some(call)=case["calls"].as_array().unwrap().iter().find(|c|c["method"]=="PATCH") {
            let request=call["body"]["conferenceData"]["createRequest"]["requestId"].as_str().unwrap();
            let entry_id=request.rsplit('-').next().unwrap().parse::<i64>().unwrap();
            tx.conn().execute("INSERT INTO sqlite_sequence(name,seq) SELECT 'event_calendar_entries',? WHERE NOT EXISTS(SELECT 1 FROM sqlite_sequence WHERE name='event_calendar_entries')",[entry_id-1])?;
            tx.conn().execute("UPDATE sqlite_sequence SET seq=? WHERE name='event_calendar_entries'",[entry_id-1])?;
        }
        Ok(())
    }).await.unwrap();
    let spec = &case["spec"];
    // Only record calls Rails actually makes, including errors and retries; unknown calls panic.
    for call in case["calls"].as_array().unwrap() {
        let method: Method = call["method"].as_str().unwrap().parse().unwrap();
        let response = if call["path"] == "/token" {
            spec["refresh"].clone()
        } else if matches!(method, Method::POST | Method::PUT) {
            spec.get("insert").cloned().unwrap_or(json!([200, {}]))
        } else if method == Method::GET && call != &case["calls"][0] {
            json!([200,{"status":"confirmed"}])
        } else {
            spec.get("remote")
                .cloned()
                .unwrap_or(if spec["kind"] == "meet" {
                    json!([200,{"hangoutLink":"https://meet.google.com/abc-defg-hij"}])
                } else {
                    json!([200,{"status":"confirmed"}])
                })
        };
        let path = call["path"].as_str().unwrap();
        if response[0] == "timeout" {
            recorded.fail_for(method, path)
        } else {
            recorded.answer_for(
                method,
                path,
                response[0].as_u64().unwrap() as u16,
                response[1].clone(),
            )
        }
    }
    (app, recorded)
}

#[tokio::test]
async fn google_consumers_match_every_recorded_rails_state_and_request() {
    let oracle = vectors();
    for case in oracle["cases"].as_array().unwrap() {
        let name = case["spec"]["name"].as_str().unwrap();
        let (a, r) = fixture(case).await;
        let index = case["spec"]["index"].as_u64().unwrap_or(0) as usize;
        let event_id = case["events"][index]["id"].as_i64().unwrap();
        let result = if case["spec"]["kind"] == "meet" {
            calendar_sync::meet(&a.booted.app, event_id).await
        } else {
            calendar_sync::inbound(&a.booted.app, DAVID).await
        };
        let error = result.as_ref().err().map(|e| match e {
            api::Error::Storage(campfire_db::Error::RecordInvalid(_)) => {
                "ActiveRecord::RecordInvalid"
            }
            other => other.class(),
        });
        assert_eq!(
            json!(error),
            case["error"],
            "{name}: failure classification {result:?}"
        );
        let actual = r
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(|call| {
                let body = call["body"].as_str().unwrap();
                let body = if body.is_empty() {
                    Value::Null
                } else if call["path"] == "/token" {
                    serde_json::to_value(
                        url::form_urlencoded::parse(body.as_bytes())
                            .into_owned()
                            .collect::<std::collections::BTreeMap<_, _>>(),
                    )
                    .unwrap()
                } else {
                    serde_json::from_str(body).unwrap()
                };
                json!({"method":call["method"],"path":call["path"],"body":body})
            })
            .collect::<Vec<_>>();
        assert_eq!(
            json!(actual),
            case["calls"],
            "{name}: exact Google exchanges"
        );
        let ids = case["events"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| r["id"].as_i64().unwrap())
            .collect::<Vec<_>>();
        let snapshot=a.db().read(move |c| {
            let responses=ids.iter().map(|id|CalendarEvent::find(c,*id)?.response_for(c,Some(DAVID))).collect::<campfire_db::Result<Vec<_>>>()?;
            let event=CalendarEvent::find(c,event_id)?;
            let link=event.meet_link;
            let requested=event.meet_link_requested;
            let attendance_updated_at=ids.iter().map(|id| c.query_row("SELECT updated_at FROM event_attendances WHERE event_id=? AND user_id=?",rusqlite::params![id,DAVID],|row| row.get::<_,Timestamp>(0)).optional().map(|stamp| stamp.map(|stamp|stamp.jiff().to_string()))).collect::<rusqlite::Result<Vec<_>>>()?;
            let jobs=c.prepare("SELECT job_class,arguments FROM background_jobs WHERE job_class LIKE 'Calendar::%' ORDER BY id")?.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?.into_iter().map(|(class,args)|{let args:Value=serde_json::from_str(&args).unwrap();json!({"class":class,"args":[args["event_id"],args["user_id"]]})}).collect::<Vec<_>>();
            let channel_error=c.query_row("SELECT last_error FROM calendar_push_channels WHERE user_id=?",[DAVID],|r|r.get::<_,Option<String>>(0))?;
            let connected=campfire_db::models::google_account::GoogleAccount::for_user(c,DAVID)?.map(|a|a.connected());
            Ok(json!({"responses":responses,"link":link,"requested":requested,"attendance_updated_at":attendance_updated_at,"jobs":jobs,"channel_error":channel_error,"connected":connected}))
        }).await.unwrap();
        for key in [
            "responses",
            "link",
            "requested",
            "attendance_updated_at",
            "jobs",
            "channel_error",
            "connected",
        ] {
            assert_eq!(snapshot[key], case[key], "{name}: {key}");
        }
        println!("Pinned Rails Calendar consumer {name}: passed");
    }
    println!("Pinned Rails Calendar consumers: 38 exercised; 0 skipped");
}

#[tokio::test]
async fn google_consumer_decline_and_jobs_rollback_together_on_late_enqueue_failure() {
    let v = vectors();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["spec"]["name"] == "head_declines_followers")
        .unwrap();
    let (a, _) = fixture(case).await;
    let follower = case["events"][1]["id"].as_i64().unwrap();
    a.db().write(move |tx| {tx.conn().execute_batch(&format!("CREATE TRIGGER reject_consumer_follower BEFORE INSERT ON background_jobs WHEN NEW.job_class='Calendar::SyncEntryJob' AND json_extract(NEW.arguments,'$.event_id')={follower} BEGIN SELECT RAISE(ABORT,'late queue rejection'); END"))?;Ok(())}).await.unwrap();
    assert!(calendar_sync::inbound(&a.booted.app, DAVID).await.is_err());
    let ids = case["events"]
        .as_array()
        .unwrap()
        .iter()
        .map(|r| r["id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    a.db()
        .read(move |c| {
            for id in ids {
                assert_eq!(
                    CalendarEvent::find(c, id)?.response_for(c, Some(DAVID))?,
                    Some("going".into())
                );
            }
            assert_eq!(
                c.query_row("SELECT count(*) FROM background_jobs", [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn google_consumer_meet_save_failure_keeps_committed_entry_and_rolls_back_link() {
    let v = vectors();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["spec"]["name"] == "provisions")
        .unwrap();
    let (a, _) = fixture(case).await;
    let id = case["events"][0]["id"].as_i64().unwrap();
    a.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_consumer_link BEFORE UPDATE OF meet_link ON events BEGIN SELECT RAISE(ABORT,'link rejection'); END")?;Ok(())}).await.unwrap();
    assert!(calendar_sync::meet(&a.booted.app, id).await.is_err());
    a.db()
        .read(move |c| {
            assert!(CalendarEvent::find(c, id)?.meet_link.is_none());
            assert!(
                campfire_db::models::google_entry::find(c, id, DAVID)?
                    .unwrap()
                    .synced_at
                    .is_some()
            );
            assert_eq!(
                c.query_row("SELECT count(*) FROM background_jobs", [], |r| r
                    .get::<_, i64>(0))?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn google_consumer_meet_publishes_rails_card_over_real_socket_after_commit() {
    use futures_util::{SinkExt, StreamExt};
    use tokio_tungstenite::tungstenite::{Message, client::IntoClientRequest};
    let v = vectors();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["spec"]["name"] == "provisions")
        .unwrap();
    let (a, _) = fixture(case).await;
    let id = case["events"][0]["id"].as_i64().unwrap();
    let message = case["announcement_id"].as_i64().unwrap();
    a.db().write(move |tx| {
        tx.conn().execute("INSERT INTO messages(id,room_id,creator_id,client_message_id,created_at,updated_at) VALUES(?,?,?,'consumer-provisions',?,?)",rusqlite::params![message,ALL_TALK,DAVID,tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO event_references(event_id,message_id,created_at,updated_at) VALUES(?,?,?,?)",rusqlite::params![id,message,tx.now(),tx.now()])?;Ok(())
    }).await.unwrap();
    let mut listener = None;
    for port in super::google_test_support::socket_ports() {
        match tokio::net::TcpListener::bind((std::net::Ipv4Addr::LOCALHOST, port)).await {
            Ok(l) => {
                listener = Some(l);
                break;
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => (),
            Err(e) => panic!("bind consumer socket: {e}"),
        }
    }
    let listener = listener.expect("free WS14g test port");
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let browser = a.sign_in(DAVID).await;
    let mut req = format!("ws://{addr}/cable").into_client_request().unwrap();
    req.headers_mut()
        .insert("cookie", browser.cookie_header().parse().unwrap());
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
    let room = a
        .db()
        .read(|c| campfire_db::Room::find(c, ALL_TALK))
        .await
        .unwrap();
    let signed = rails_compat::turbo::signed_stream_name(
        &a.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    let identifier =
        json!({"channel":"RoomMessagesChannel","signed_stream_name":signed}).to_string();
    socket
        .send(Message::Text(
            json!({"command":"subscribe","identifier":identifier})
                .to_string()
                .into(),
        ))
        .await
        .unwrap();
    assert_eq!(next(&mut socket).await["type"], "confirm_subscription");
    calendar_sync::meet(&a.booted.app, id).await.unwrap();
    let delivered = next(&mut socket).await["message"].clone();
    assert_eq!(delivered, case["frames"][0]["payload"]);
    let observer = rusqlite::Connection::open(&a.booted.app.config.storage.database).unwrap();
    assert_eq!(
        observer
            .query_row("SELECT meet_link FROM events WHERE id=?", [id], |r| r
                .get::<_, String>(0))
            .unwrap(),
        case["link"].as_str().unwrap()
    );
    socket.close(None).await.unwrap();
    server.abort();
    let _ = server.await;
}

#[tokio::test]
async fn google_compound_attendance_attributes_match_rails_bytes() {
    let mut a = TestApp::boot().await.unwrap();
    a.booted.jobs.stop(Duration::from_secs(5)).await;
    let v: Value = serde_json::from_str(include_str!(
        "../../../../vectors/google_attendance_params.json"
    ))
    .unwrap();
    let id = v["event_id"].as_i64().unwrap();
    a.db().write(move |tx| {
        tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(?,?,?,'Compound response',?,'UTC',?,?)",rusqlite::params![id,ALL_TALK,DAVID,Timestamp::parse_db("2026-03-03 09:00:00").unwrap(),tx.now(),tx.now()])?;
        tx.conn().execute("INSERT INTO event_attendances(event_id,user_id,response,created_at,updated_at) VALUES(?,?,'going',?,?)",rusqlite::params![id,DAVID,tx.now(),tx.now()])?;
        Ok(())
    }).await.unwrap();
    let mut browser = a.david();
    for case in v["cases"].as_array().unwrap() {
        let get = case["location"] == "show";
        let req = Req::new(
            if get { Method::GET } else { Method::PATCH },
            &format!("/rooms/{ALL_TALK}/events/{id}/attendance"),
        )
        .header("turbo-frame", "fixture-frame")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&case["params"]).unwrap());
        let response = if get {
            browser.send(req).await
        } else {
            browser.write(req).await
        };
        let label = format!("{} {}", case["name"], case["location"]);
        assert_eq!(
            response.status.as_u16() as u64,
            case["status"].as_u64().unwrap(),
            "{label}"
        );
        let html = response.text();
        for (key, pattern) in [
            ("frame_tag", r"<turbo-frame\b[^>]*>"),
            ("input_tag", r#"<input\b[^>]*\bname="message_id"[^>]*>"#),
        ] {
            let actual = regex::Regex::new(pattern)
                .unwrap()
                .find(&html)
                .map(|m| m.as_str());
            assert_eq!(json!(actual), case[key], "{label}: exact {key}");
        }
    }
}

#[tokio::test]
async fn google_pending_meet_conference_uses_the_real_durable_retry_handler() {
    let v = vectors();
    let case = v["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["spec"]["name"] == "pending")
        .unwrap();
    let (a, _) = fixture(case).await;
    let id = case["events"][0]["id"].as_i64().unwrap();
    a.db()
        .write(move |tx| {
            tx.emit_after_commit(campfire_db::Event::job(
                &campfire_db::models::google_calendar::MeetLinkJob { event_id: id },
            ));
            Ok(())
        })
        .await
        .unwrap();
    let runner = campfire_jobs::start(
        a.db().clone(),
        a.booted.app.jobs.queue.clone(),
        crate::jobs::registry(),
        a.booted.app.clone(),
        crate::jobs::runner_config(&a.booted.app.config),
    );
    tokio::time::timeout(Duration::from_secs(5),async {loop {
        let ready=a.db().read(|c|Ok(c.query_row("SELECT EXISTS(SELECT 1 FROM background_jobs WHERE job_class='Calendar::MeetLinkJob' AND status='ready' AND attempts=1)",[],|r|r.get::<_,bool>(0))?)).await.unwrap();
        if ready {break}tokio::time::sleep(Duration::from_millis(10)).await;
    }}).await.unwrap();
    runner.shutdown(Duration::from_secs(5)).await;
    a.db()
        .read(move |c| {
            let job = campfire_jobs::inspect::all(c)?
                .into_iter()
                .find(|j| j.class == "Calendar::MeetLinkJob")
                .unwrap();
            assert_eq!(job.arguments, json!({"event_id":id}));
            let seconds = (job.run_at.as_microsecond() - job.updated_at.as_microsecond()) as f64
                / 1_000_000.0;
            assert!((3.0..3.15).contains(&seconds));
            assert!(job.last_error.unwrap().contains("conference still pending"));
            assert!(CalendarEvent::find(c, id)?.meet_link.is_none());
            let raw: String = c.query_row(
                "SELECT arguments FROM background_jobs WHERE job_class='Calendar::MeetLinkJob'",
                [],
                |r| r.get(0),
            )?;
            let raw: Value = serde_json::from_str(&raw).unwrap();
            assert_eq!(
                raw["_campfire_retry_metadata_v1"]["counts"]["[Google::Client::Unavailable]"],
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn google_inbound_preloads_events_once_for_both_rails_batch_sizes() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/google_inbound_preload.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let at: jiff::Timestamp = "2026-03-02T16:00:00Z".parse().unwrap();
        let mut a = TestApp::boot_with_clock(Arc::new(FrozenClock::new(at)))
            .await
            .unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let r = Recorded::new(vec![]);
        a.booted
            .app
            .google
            .install_api(api::Api::new(support::config(), r.clone()));
        support::grant(
            &a,
            DAVID,
            Timestamp::from_jiff(at).since(jiff::SignedDuration::from_hours(1)),
            false,
        )
        .await;
        let size = row["size"].as_i64().unwrap();
        a.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM event_calendar_entries;DELETE FROM background_jobs;")?;
            tx.conn().execute("UPDATE google_accounts SET scopes=? WHERE user_id=?",[campfire_db::models::google_account::CALENDAR_SCOPE,&DAVID.to_string()])?;
            for i in 0..size {
                let id=9_500_000_000+i;
                tx.conn().execute("INSERT INTO events(id,room_id,organizer_id,title,starts_at,time_zone,created_at,updated_at) VALUES(?,?,?,'Preload',?,'UTC',?,?)",rusqlite::params![id,ALL_TALK,DAVID,tx.now().since(jiff::SignedDuration::from_hours(1)),tx.now(),tx.now()])?;
                CalendarEvent::respond(tx,id,DAVID,"going",false)?;
                let entry=campfire_db::models::google_entry::reserve(tx,id,DAVID)?;
                campfire_db::models::google_entry::success(tx,&entry)?;
            }
            tx.conn().execute_batch("DELETE FROM background_jobs;")?;
            Ok(())
        }).await.unwrap();
        // Owner callbacks enqueue after commit; clear fixture work only after it was published.
        a.db()
            .write(|tx| {
                tx.conn().execute_batch("DELETE FROM background_jobs;")?;
                Ok(())
            })
            .await
            .unwrap();
        for id in row["calls"].as_array().unwrap() {
            r.answer_for(
                Method::GET,
                &format!("{}/{}", api::EVENTS, id.as_str().unwrap()),
                200,
                json!({"status":"confirmed"}),
            );
        }
        let probe = crate::controllers::rooms::query_probe::SqlProbe::start(
            a.db(),
            a.booted.app.config.db_readers,
        )
        .await;
        calendar_sync::inbound(&a.booted.app, DAVID).await.unwrap();
        let queries = probe.finish().await;
        let event_queries = queries
            .iter()
            .filter(|q| {
                let sql = q.sql.to_ascii_uppercase();
                sql.trim_start().starts_with("SELECT")
                    && (sql.contains("FROM EVENTS") || sql.contains("JOIN EVENTS"))
            })
            .collect::<Vec<_>>();
        assert_eq!(
            event_queries.len() as u64,
            row["event_reads"].as_u64().unwrap(),
            "{size}: {event_queries:?}"
        );
        let calls = r
            .calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| {
                c["path"]
                    .as_str()
                    .unwrap()
                    .rsplit('/')
                    .next()
                    .unwrap()
                    .to_owned()
            })
            .collect::<Vec<_>>();
        assert_eq!(json!(calls), row["calls"]);
        let state = a
            .db()
            .read(move |c| {
                let responses = (0..size)
                    .map(|i| {
                        CalendarEvent::find(c, 9_500_000_000 + i)?.response_for(c, Some(DAVID))
                    })
                    .collect::<campfire_db::Result<Vec<_>>>()?;
                let jobs = c
                    .prepare("SELECT job_class FROM background_jobs ORDER BY id")?
                    .query_map([], |r| r.get::<_, String>(0))?
                    .collect::<rusqlite::Result<Vec<_>>>()?;
                Ok((responses, jobs))
            })
            .await
            .unwrap();
        assert_eq!(json!(state.0), row["responses"]);
        assert_eq!(json!(state.1), row["jobs"]);
        println!(
            "Inbound preload: {size} entries; {} event SELECT; {} recorded Google GETs",
            event_queries.len(),
            calls.len()
        );
    }
}
