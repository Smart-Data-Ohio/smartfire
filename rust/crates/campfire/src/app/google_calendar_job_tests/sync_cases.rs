//! Original EntrySync consumer cases through recorded HTTP and registered durable jobs.
use super::*;
use crate::integrations::google::entry_sync;
use campfire_db::models::google_entry;
use rusqlite::OptionalExtension;
fn time(v: &Value) -> Option<Timestamp> {
    v.as_str().and_then(Timestamp::parse_db)
}
fn stamp(t: Timestamp) -> String {
    format!("{} UTC", t.to_db())
}
#[tokio::test]
async fn google_calendar_sync_consumers_match_pinned_rails_rows_requests_and_retries() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_sync_entry_cases.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let spec = &row["spec"];
        let name = spec["name"].as_str().unwrap();
        let at: jiff::Timestamp = oracle["now"].as_str().unwrap().parse().unwrap();
        let a = TestApp::boot_with_clock_and_env(
            Arc::new(campfire_kit::FrozenClock::new(at)),
            &[("APP_URL", "http://campfire.test")],
        )
        .await
        .unwrap()
        .without_job_runner()
        .await;
        let r = Recorded::new(vec![]);
        let mut config = support::config();
        config.client_secret = "FAKE-calendar-client-secret".into();
        a.booted.app.google.install_api(Api::new(config, r.clone()));
        let event = row["event"].clone();
        let venue = row["venue"].clone();
        let id = event["id"].as_i64().unwrap();
        a.db().write(move |tx| {
            tx.conn().execute_batch("DELETE FROM google_accounts;DELETE FROM event_calendar_entries;DELETE FROM background_jobs")?;
            if let Some(vid)=venue["id"].as_i64() {
                tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(?,'Rooms::Voice',?,?,?,?)",rusqlite::params![vid,venue["name"].as_str().unwrap(),DAVID,tx.now(),tx.now()])?;
            }
            tx.conn().execute("UPDATE events SET starts_at=?,ends_at=?,venue_room_id=? WHERE id=?",rusqlite::params![time(&event["starts_at"]).unwrap(),time(&event["ends_at"]),event["venue_room_id"].as_i64(),id])?;
            Ok(())
        }).await.unwrap();
        if spec["account"] != false {
            support::grant(
                &a,
                DAVID,
                Timestamp::from_jiff(at).since(jiff::SignedDuration::from_secs(
                    if spec["expired"] == true { -1 } else { 3600 },
                )),
                false,
            )
            .await;
        }
        let setup = spec.clone();
        a.db().write(move |tx| {
            if setup["disconnected"]==true {tx.conn().execute("UPDATE google_accounts SET disconnected_reason='revoked' WHERE user_id=?",[DAVID])?;}
            if let Some(scope)=setup["scope"].as_str() {tx.conn().execute("UPDATE google_accounts SET scopes=? WHERE user_id=?",rusqlite::params![scope,DAVID])?;}
            if setup["unreadable"]==true {tx.conn().execute("UPDATE google_accounts SET access_token='broken-AR-ciphertext' WHERE user_id=?",[DAVID])?;}
            if setup["existing"]==true {
                google_entry::reserve(tx,id,DAVID)?;
                tx.conn().execute("UPDATE event_calendar_entries SET synced_at=?,last_error='old error' WHERE event_id=? AND user_id=?",rusqlite::params![(setup["synced"]==true).then(||tx.now().ago(jiff::SignedDuration::from_hours(24))),id,DAVID])?;
            }
            Ok(())
        }).await.unwrap();
        let steps = spec["steps"]
            .as_array()
            .cloned()
            .unwrap_or_else(|| vec![spec.clone()]);
        let mut observations = Vec::new();
        for (i, step) in steps.iter().enumerate() {
            let change = step.clone();
            let response = step["response"]
                .as_str()
                .or(spec["response"].as_str())
                .map(str::to_owned);
            a.db().write(move |tx| {
                if let Some(response)=response {tx.conn().execute("UPDATE event_attendances SET response=? WHERE event_id=? AND user_id=?",rusqlite::params![response,id,DAVID])?;}
                if change["time_change"]==true {let event=campfire_db::CalendarEvent::find(tx.conn(),id)?;tx.conn().execute("UPDATE events SET starts_at=? WHERE id=?",rusqlite::params![event.starts_at.since(jiff::SignedDuration::from_secs(1800)),id])?;}
                if change["cancel"]==true {tx.conn().execute("UPDATE events SET cancelled_at=? WHERE id=?",rusqlite::params![tx.now(),id])?;}
                if change["leave"]==true {tx.conn().execute("DELETE FROM memberships WHERE room_id=(SELECT room_id FROM events WHERE id=?) AND user_id=?",rusqlite::params![id,DAVID])?;}
                tx.conn().execute("DELETE FROM background_jobs",[])?;
                Ok(())
            }).await.unwrap();
            r.calls.lock().unwrap().clear();
            let recorded = &row["observations"][i]["calls"];
            let defaults = json!([[200, {}]]);
            let answers = step
                .get("answers")
                .or(spec.get("answers"))
                .unwrap_or(&defaults)
                .as_array()
                .unwrap();
            for (answer, call) in answers.iter().zip(recorded.as_array().unwrap()) {
                let method = call["method"]
                    .as_str()
                    .unwrap()
                    .parse::<hyper::Method>()
                    .unwrap();
                let path = call["path"].as_str().unwrap();
                if answer[0] == "timeout" {
                    r.fail_for(method, path);
                } else if answer[0] == "transport" {
                    use crate::integrations::net::http::HttpError;
                    let error = match answer[1].as_str().unwrap() {
                        "Net::ReadTimeout" => HttpError::ReadTimeout,
                        "Net::WriteTimeout" => HttpError::WriteTimeout,
                        "SocketError" => {
                            HttpError::Unresolvable("recorded private transport detail".into())
                        }
                        "OpenSSL::SSL::SSLError" => {
                            HttpError::Tls("recorded private transport detail".into())
                        }
                        "Errno::ECONNREFUSED" => {
                            HttpError::Io(std::io::Error::from_raw_os_error(libc::ECONNREFUSED))
                        }
                        "Errno::ECONNRESET" => {
                            HttpError::Io(std::io::Error::from_raw_os_error(libc::ECONNRESET))
                        }
                        "EOFError" => HttpError::ConnectionClosed,
                        "Net::HTTPBadResponse" => {
                            HttpError::Http("recorded private transport detail".into())
                        }
                        other => panic!("unrecorded transport class {other}"),
                    };
                    r.fail_for_error(method, path, error.into());
                } else {
                    r.answer_for(
                        method,
                        path,
                        answer[0].as_u64().unwrap() as u16,
                        answer[1].clone(),
                    );
                }
            }
            let target_id = spec["event_id"].as_i64().unwrap_or(id);
            let target_user = spec["user_id"].as_i64().unwrap_or(DAVID);
            let error = if spec["job"] == true {
                let mut drain = super::super::google_test_support::QueueDrain::install(&a).await;
                a.db()
                    .write(move |tx| {
                        tx.emit_after_commit(campfire_db::Event::job(
                            &campfire_db::models::google_calendar::SyncEntryJob {
                                event_id: target_id,
                                user_id: target_user,
                            },
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
                let scheduled = tokio::time::timeout(
                    std::time::Duration::from_secs(5),
                    drain.job_attempt(&a, "Calendar::SyncEntryJob", 1),
                )
                .await
                .unwrap();
                runner.shutdown(std::time::Duration::from_secs(5)).await;
                if let Some(scheduled) = scheduled {
                    let delta = (scheduled.as_microsecond()
                        - Timestamp::from_jiff(at).as_microsecond())
                        as f64
                        / 1_000_000.;
                    assert!((3.0..3.15).contains(&delta));
                }
                None
            } else {
                entry_sync::sync(&a.booted.app, target_id, target_user)
                    .await
                    .err()
                    .map(|e| e.class().to_owned())
            };
            let state=a.db().read(move |conn| {
                let entry=conn.query_row("SELECT event_id,user_id,google_event_id,synced_at,last_error FROM event_calendar_entries WHERE event_id=? AND user_id=?",rusqlite::params![id,DAVID],|r| Ok(json!({"event_id":r.get::<_,i64>(0)?,"user_id":r.get::<_,i64>(1)?,"google_event_id":r.get::<_,String>(2)?,"synced_at":r.get::<_,Option<Timestamp>>(3)?.map(stamp),"last_error":r.get::<_,Option<String>>(4)?}))).optional()?;
                let disconnected=campfire_db::models::google_account::GoogleAccount::for_user(conn,DAVID)?.and_then(|a|a.disconnected_reason);
                let jobs=conn.prepare("SELECT job_class,arguments,status,attempts FROM background_jobs WHERE job_class LIKE 'Calendar::%'")?.query_map([],|r| {let raw:Value=r.get(1)?;let raw=raw.get("_campfire_retry_metadata_v1").map(|m|&m["arguments"]).unwrap_or(&raw);assert_eq!(r.get::<_,String>(2)?,"ready");assert_eq!(r.get::<_,i64>(3)?,1);Ok(json!({"class":r.get::<_,String>(0)?,"args":[raw["event_id"],raw["user_id"]]}))})?.collect::<rusqlite::Result<Vec<_>>>()?;
                Ok(json!({"entry":entry,"disconnected":disconnected,"jobs":jobs}))
            }).await.unwrap();
            let calls=r.calls.lock().unwrap().iter().map(|c|json!({"method":c["method"],"path":c["path"],"body":if c["body"]==""{Value::Null}else if c["path"]=="/token"{json!(url::form_urlencoded::parse(c["body"].as_str().unwrap().as_bytes()).into_owned().collect::<std::collections::BTreeMap<_,_>>())}else{serde_json::from_str::<Value>(c["body"].as_str().unwrap()).unwrap()},"access_token":c["access_token"]})).collect::<Vec<_>>();
            observations.push(json!({"calls":calls,"error":error,"entry":state["entry"],"disconnected":state["disconnected"],"jobs":state["jobs"]}));
        }
        assert_eq!(
            json!(observations),
            row["observations"],
            "pinned Rails EntrySync {name}"
        );
    }
    println!(
        "Pinned Rails EntrySync consumers: 39 scenarios; registered durable retries; 0 skipped"
    );
}
