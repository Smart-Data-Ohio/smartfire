//! Watch lifecycle, renewal, authentication and preload properties observed in pinned Rails.
use super::google_api_tests::{self as support, Recorded};
use crate::{
    controllers::presenters::test_support::{DAVID, TestApp},
    integrations::google::{api, calendar},
};
use campfire_db::{Timestamp, models::google_calendar::PushChannel};
use campfire_kit::FrozenClock;
use serde_json::{Value, json};
use std::sync::Arc;

fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../vectors/google_push_channels.json"
    ))
    .unwrap()
}
fn stamp(v: &Value) -> Option<Timestamp> {
    v.as_str().map(|s| Timestamp::from_jiff(s.parse().unwrap()))
}
fn time(v: Option<Timestamp>) -> Value {
    json!(v.map(|s| format!("{:.6}", s.jiff())))
}
async fn fixture(row: &Value, now: Timestamp) -> (TestApp, Arc<Recorded>) {
    let a = TestApp::boot_with_clock(Arc::new(FrozenClock::new(now.jiff())))
        .await
        .unwrap()
        .without_job_runner()
        .await;
    let r = Recorded::new(vec![]);
    let spec = row["spec"].clone();
    let mut config = support::config();
    config.webhook_url = (spec["callback"] != false)
        .then(|| "https://app.test/google/calendar/notifications".into());
    if spec["secret"] == false {
        config.client_secret.clear();
    }
    a.booted
        .app
        .google
        .install_api(api::Api::new(config, r.clone()));
    let ids = row["user_ids"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect::<Vec<_>>();
    let setup_ids = ids.clone();
    let initial = row["initial"].clone();
    let setup = spec.clone();
    a.db().write(move |tx| {
        tx.conn().execute_batch("DELETE FROM calendar_push_channels;DELETE FROM google_accounts;DELETE FROM background_jobs;")?;
        for (i,id) in setup_ids.iter().enumerate() {
            if *id != DAVID {
                tx.conn().execute("INSERT INTO users(id,name,email_address,status,created_at,updated_at) VALUES(?,?,?,0,?,?)",rusqlite::params![id,format!("Push fixture {}",i-1),format!("push-fixture-{}@example.test",i-1),tx.now(),tx.now()])?;
            }
        }
        for c in initial.as_array().unwrap() {
            tx.conn().execute("INSERT INTO calendar_push_channels(id,user_id,channel_id,token_digest,resource_id,expires_at,last_message_number,last_notification_at,last_error,created_at,updated_at) VALUES(?,?,?,?,?,?,?,?,?,?,?)",rusqlite::params![c["id"].as_i64().unwrap(),c["user_id"].as_i64().unwrap(),c["channel_id"].as_str().unwrap(),c["token_digest"].as_str().unwrap(),c["resource_id"].as_str(),stamp(&c["expires_at"]),c["last_message_number"].as_i64().unwrap(),stamp(&c["last_notification_at"]),c["last_error"].as_str(),stamp(&c["created_at"]).unwrap(),stamp(&c["updated_at"]).unwrap()])?;
        }
        Ok(())
    }).await.unwrap();
    if spec["account"] != false {
        for id in ids {
            support::grant(
                &a,
                id,
                now.since(jiff::SignedDuration::from_hours(1)),
                false,
            )
            .await;
            let setup = setup.clone();
            a.db().write(move |tx| {
                if setup["disconnected"] == true {
                    tx.conn().execute("UPDATE google_accounts SET disconnected_reason='revoked' WHERE user_id=?",[id])?;
                }
                if let Some(scope) = setup["scopes"].as_str() {
                    tx.conn().execute("UPDATE google_accounts SET scopes=? WHERE user_id=?",rusqlite::params![scope,id])?;
                }
                if setup["unreadable"] == true {
                    tx.conn().execute("UPDATE google_accounts SET refresh_token='broken-AR-ciphertext' WHERE user_id=?",[id])?;
                }
                if setup["unreadable_access"] == true {
                    tx.conn().execute("UPDATE google_accounts SET access_token='broken-AR-ciphertext' WHERE user_id=?",[id])?;
                }
                Ok(())
            }).await.unwrap();
        }
    }
    let response = spec
        .get("response")
        .cloned()
        .unwrap_or(json!([200,{"resourceId":"new-resource"}]));
    for call in row["calls"].as_array().unwrap() {
        let path = call["path"].as_str().unwrap();
        if path.ends_with("/events/watch") {
            if response[0] == "timeout" {
                r.fail_for(hyper::Method::POST, path);
            } else {
                r.answer_for(
                    hyper::Method::POST,
                    path,
                    response[0].as_u64().unwrap() as u16,
                    response[1].clone(),
                );
            }
        } else if spec["stop_status"] == "timeout" {
            r.fail_for(hyper::Method::POST, path);
        } else {
            r.answer_for(
                hyper::Method::POST,
                path,
                spec["stop_status"].as_u64().unwrap_or(200) as u16,
                json!({}),
            );
        }
    }
    let offsets = row["initial_offsets_us"].as_object().unwrap().clone();
    a.db()
        .read(move |c| {
            for (id, expected) in offsets {
                let at = c.query_row(
                    "SELECT expires_at FROM calendar_push_channels WHERE user_id=?",
                    [id.parse::<i64>().unwrap()],
                    |r| r.get::<_, Option<Timestamp>>(0),
                )?;
                assert_eq!(
                    json!(at.map(|t| t.as_microsecond() - now.as_microsecond())),
                    expected,
                    "persisted expiry offset must match exact oracle microseconds"
                );
            }
            Ok(())
        })
        .await
        .unwrap();
    (a, r)
}
fn observed_calls(r: &Recorded) -> Value {
    json!(
        r.calls
            .lock()
            .unwrap()
            .iter()
            .map(|c| {
                let mut body: Value = serde_json::from_str(c["body"].as_str().unwrap()).unwrap();
                let mut call = json!({"method":c["method"],"path":c["path"],"body":body,"content_type":c["content_type"],"access_token":c["access_token"]});
                if c["path"].as_str().unwrap().ends_with("/events/watch") {
                    let id = body.as_object_mut().unwrap().remove("id").unwrap();
                    let token = body.as_object_mut().unwrap().remove("token").unwrap();
                    let parsed = uuid::Uuid::parse_str(id.as_str().unwrap()).unwrap();
                    call["uuid_v4"] = json!(
                        parsed.get_version_num() == 4
                            && parsed.get_variant() == uuid::Variant::RFC4122
                            && parsed.hyphenated().to_string() == id
                    );
                    call["token_hex_64"] = json!(token.as_str().is_some_and(|s| {
                        s.len() == 64
                            && s.bytes()
                                .all(|b| b.is_ascii_digit() || (b'a'..=b'f').contains(&b))
                    }));
                    call["body"] = body;
                }
                call
            })
            .collect::<Vec<_>>()
    )
}
async fn states(a: &TestApp, r: &Recorded, row: &Value) -> Value {
    let watches = r
        .calls
        .lock()
        .unwrap()
        .iter()
        .filter(|c| c["path"].as_str().unwrap().ends_with("/events/watch"))
        .map(|c| serde_json::from_str::<Value>(c["body"].as_str().unwrap()).unwrap())
        .collect::<Vec<_>>();
    let row = row.clone();
    a.db().read(move |conn| {
        use rusqlite::OptionalExtension;
        let mut channels = Vec::new();
        let mut disconnected = Vec::new();
        for user in row["user_ids"].as_array().unwrap() {
            let id=user.as_i64().unwrap();
            let channel=conn.query_row("SELECT id,channel_id,token_digest,resource_id,expires_at,last_message_number,last_notification_at,last_error,created_at,updated_at FROM calendar_push_channels WHERE user_id=?",[id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?,r.get::<_,String>(2)?,json!({"user_id":id,"resource_id":r.get::<_,Option<String>>(3)?,"expires_at":time(r.get(4)?),"last_message_number":r.get::<_,i64>(5)?,"last_notification_at":time(r.get(6)?),"last_error":r.get::<_,Option<String>>(7)?,"created_at":time(r.get(8)?),"updated_at":time(r.get(9)?)})))).optional()?;
            channels.push(if let Some((channel_id, identity, digest, mut state))=channel {
                let old=row["initial"].as_array().unwrap().iter().find(|c|c["user_id"]==*user);
                let watch=watches.iter().find(|w|w["id"]==identity);
                state["same_row"]=json!(old.map(|c|c["id"]==channel_id));
                state["old_identity"]=json!(identity==format!("old-{id}"));
                let token=watch.and_then(|w|w["token"].as_str()).map(str::to_string).unwrap_or_else(||format!("old-token-{id}"));
                state["digest_matches"]=json!(digest==PushChannel::digest(&token));
                state
            } else { Value::Null });
            disconnected.push(conn.query_row("SELECT disconnected_reason FROM google_accounts WHERE user_id=?",[id],|r|r.get::<_,Option<String>>(0)).optional()?.flatten());
        }
        Ok(json!({"channels":channels,"disconnected":disconnected}))
    }).await.unwrap()
}
#[tokio::test]
async fn google_push_channel_watch_renewal_and_preload_match_pinned_rails() {
    let v = vectors();
    let now = stamp(&v["now"]).unwrap();
    for row in v["rows"].as_array().unwrap() {
        let name = row["spec"]["name"].as_str().unwrap();
        let (a, r) = fixture(row, now).await;
        let probe = crate::controllers::rooms::query_probe::SqlProbe::start(
            a.db(),
            a.booted.app.config.db_readers,
        )
        .await;
        let result = if row["spec"]["renew"] == true {
            calendar::renew(&a.booted.app).await
        } else {
            calendar::watch(&a.booted.app, DAVID).await
        };
        let queries = probe.finish().await;
        assert_eq!(
            json!(result.as_ref().err().map(api::Error::class)),
            row["error"],
            "{name}: failure classification"
        );
        assert_eq!(
            observed_calls(&r),
            row["calls"],
            "{name}: complete Google requests and random-value properties"
        );
        let state = states(&a, &r, row).await;
        assert_eq!(
            state["channels"], row["channels"],
            "{name}: stored channel identity, digest and state"
        );
        assert_eq!(
            state["disconnected"], row["disconnected"],
            "{name}: account state"
        );
        if name.starts_with("preload_") {
            let reads = queries
                .iter()
                .filter(|q| {
                    let sql = q.sql.to_ascii_uppercase().replace('"', "");
                    sql.trim_start().starts_with("SELECT")
                        && (sql.contains("FROM USERS") || sql.contains("JOIN USERS"))
                })
                .count();
            // Rust needs no User model for a fresh channel; Rails needs one batch preload.
            assert!(
                reads <= row["user_reads"].as_u64().unwrap() as usize,
                "{name}: per-channel user lookup regressed: {reads}; {queries:?}"
            );
            let account_reads = queries
                .iter()
                .filter(|q| {
                    let sql = q.sql.to_ascii_uppercase().replace('"', "");
                    sql.trim_start().starts_with("SELECT")
                        && (sql.contains("FROM GOOGLE_ACCOUNTS")
                            || sql.contains("JOIN GOOGLE_ACCOUNTS"))
                })
                .count();
            assert_eq!(
                account_reads,
                row["account_reads"].as_u64().unwrap() as usize,
                "{name}: per-channel account lookup regressed; {queries:?}"
            );
            println!(
                "Push account preload: {name}; {account_reads} GoogleAccount SELECTs; Rails {}",
                row["account_reads"]
            );
            println!(
                "Push preload: {} channels; {reads} User SELECTs; Rails {} User SELECT",
                row["user_ids"].as_array().unwrap().len(),
                row["user_reads"]
            );
        }
    }
    println!(
        "Pinned Rails PushChannel: {} watch/renew scenarios; 0 skipped",
        v["rows"].as_array().unwrap().len()
    );
}

#[tokio::test]
async fn google_push_channel_unreadable_access_records_watch_error() {
    let v = vectors();
    let now = stamp(&v["now"]).unwrap();
    let mut actual = Vec::new();
    let mut expected = Vec::new();
    for (mode, case_name) in [("watch", "rewatch"), ("renew", "renew_soon")] {
        let row = v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["spec"]["name"] == case_name)
            .unwrap();
        let (a, recorded) = fixture(row, now).await;
        a.db()
            .write(|tx| {
                // Keep the refresh token usable; only credential decoding fails.
                tx.conn().execute(
                    "UPDATE google_accounts SET access_token='broken-AR-ciphertext' WHERE user_id=?",
                    [DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let outcome = if mode == "watch" {
            calendar::watch(&a.booted.app, DAVID).await
        } else {
            calendar::renew(&a.booted.app).await
        };
        let (last_error, disconnected) = a
            .db()
            .read(|conn| {
                let last_error = conn.query_row(
                    "SELECT last_error FROM calendar_push_channels WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, Option<String>>(0),
                )?;
                let disconnected = conn.query_row(
                    "SELECT disconnected_reason FROM google_accounts WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, Option<String>>(0),
                )?;
                Ok((last_error, disconnected))
            })
            .await
            .unwrap();
        let observed = json!({"mode":mode,"error":outcome.as_ref().err().map(api::Error::class),"last_error":last_error,"disconnected":disconnected,"calls":recorded.calls.lock().unwrap().len()});
        println!("Unreadable access watch: {observed}");
        actual.push(observed);
        expected.push(json!({"mode":mode,"error":null,"last_error":"Unauthorized: Google token could not be read","disconnected":"The stored token could not be read; reconnect","calls":0}));
    }
    assert_eq!(
        actual, expected,
        "unreadable access must retain Rails watch error handling"
    );
}
#[tokio::test]
async fn google_push_channel_tokens_and_notification_claims_match_pinned_rails() {
    let v = vectors();
    let channel = PushChannel {
        id: 0,
        user_id: DAVID,
        channel_id: "fixture".into(),
        token_digest: PushChannel::digest("secret-token"),
    };
    for row in v["tokens"].as_array().unwrap() {
        assert_eq!(
            json!(channel.token_matches(row["token"].as_str().unwrap_or_default())),
            row["matches"],
            "{row}"
        );
    }
    let now = stamp(&v["now"]).unwrap();
    let row = &v["rows"][0];
    let (a, _) = fixture(row, now).await;
    a.db()
        .write(|tx| {
            PushChannel::create(
                tx,
                DAVID,
                "claim-fixture",
                &PushChannel::digest("claim-token"),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for expected in v["claims"].as_array().unwrap() {
        let number = expected["number"].as_str().unwrap().to_string();
        let actual=a.db().write(move |tx| {
            let channel=PushChannel::for_channel(tx.conn(),"claim-fixture")?.unwrap();
            let won=channel.claim(tx,&number)?;
            let (last,at)=tx.conn().query_row("SELECT last_message_number,last_notification_at FROM calendar_push_channels WHERE id=?",[channel.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,Option<Timestamp>>(1)?)))?;
            Ok(json!({"number":number,"won":won,"last_message_number":last,"last_notification_at":time(at)}))
        }).await.unwrap();
        assert_eq!(actual, *expected);
    }
    println!(
        "Pinned Rails PushChannel authentication: 6 token cases; 5 notification claims; 0 skipped"
    );
}

#[tokio::test]
async fn google_push_channel_renewal_boundary_offsets_are_exact() {
    let v = vectors();
    let now = stamp(&v["now"]).unwrap();
    for (suffix, expected) in [
        ("before", 86_399_999_999),
        ("at", 86_400_000_000),
        ("after", 86_400_000_001),
    ] {
        let name = format!("renew_{suffix}_boundary");
        let row = v["rows"]
            .as_array()
            .unwrap()
            .iter()
            .find(|r| r["spec"]["name"] == name)
            .unwrap();
        let (a, _) = fixture(row, now).await;
        let at = a
            .db()
            .read(|c| {
                Ok(c.query_row(
                    "SELECT expires_at FROM calendar_push_channels WHERE user_id=?",
                    [DAVID],
                    |r| r.get::<_, Timestamp>(0),
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            at.as_microsecond() - now.as_microsecond(),
            expected,
            "{name}: persisted expiry offset must be exact microseconds"
        );
    }
}
async fn assert_account_preload_budget(name: &str) {
    let v = vectors();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["spec"]["name"] == name)
        .unwrap();
    let (a, _) = fixture(row, stamp(&v["now"]).unwrap()).await;
    let probe = crate::controllers::rooms::query_probe::SqlProbe::start(
        a.db(),
        a.booted.app.config.db_readers,
    )
    .await;
    calendar::renew(&a.booted.app).await.unwrap();
    let queries = probe.finish().await;
    let reads = queries
        .iter()
        .filter(|q| {
            let sql = q.sql.to_ascii_uppercase().replace('"', "");
            sql.trim_start().starts_with("SELECT")
                && (sql.contains("FROM GOOGLE_ACCOUNTS") || sql.contains("JOIN GOOGLE_ACCOUNTS"))
        })
        .count();
    assert_eq!(row["account_reads"], json!(2), "Rails batch observation");
    assert_eq!(
        reads,
        row["account_reads"].as_u64().unwrap() as usize,
        "{name}: GoogleAccount reads must match Rails' two batch loads; {queries:?}"
    );
}
#[tokio::test]
async fn google_push_channel_two_accounts_use_two_batch_reads() {
    assert_account_preload_budget("preload_2").await;
}
#[tokio::test]
async fn google_push_channel_twelve_accounts_use_two_batch_reads() {
    assert_account_preload_budget("preload_12").await;
}
