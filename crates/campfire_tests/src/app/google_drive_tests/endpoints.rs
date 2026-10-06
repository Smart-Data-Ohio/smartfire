//! Full pinned Rails responses and recorded requests, including exact cache/budget boundaries.
use super::*;
use campfire_kit::FrozenClock;
use std::time::Duration;
fn observe(reply: &crate::controllers::presenters::test_support::Reply, r: &Recorded) -> Value {
    json!({"status":reply.status.as_u16(),"body":if reply.body.is_empty(){json!("")}else{reply.json()},"cache_control":reply.headers.get("cache-control").map(|h|h.to_str().unwrap()),"calls":r.calls.lock().unwrap().len()})
}
#[tokio::test]
async fn google_drive_endpoint_requests_match_pinned_rails_cache_expiry_and_user_budgets() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/google_endpoint_cases.json"
    ))
    .unwrap();
    for row in oracle["rows"].as_array().unwrap() {
        let now = jiff::Timestamp::from_second(oracle["now"].as_i64().unwrap()).unwrap();
        let clock = Arc::new(FrozenClock::new(now));
        let mut a = TestApp::boot_with_clock(clock.clone()).await.unwrap();
        a.booted.jobs.stop(Duration::from_secs(5)).await;
        let r = Recorded::new(vec![]);
        support::install(&a, r.clone()).await;
        a.db()
            .write(|tx| {
                tx.conn().execute("DELETE FROM google_accounts", [])?;
                Ok(())
            })
            .await
            .unwrap();
        let name = row["spec"]["name"].as_str().unwrap();
        if name != "show_no_account" {
            grant(&a, DAVID).await;
        }
        if name.ends_with("budget") {
            grant(&a, JASON).await;
        }
        let setup = name.to_owned();
        a.db().write(move |tx| {
            match setup.as_str() {
                "show_retired"=>{tx.conn().execute("UPDATE google_accounts SET scopes='https://www.googleapis.com/auth/drive.metadata.readonly' WHERE user_id=?",[DAVID])?;}
                "show_calendar_only"=>{tx.conn().execute("UPDATE google_accounts SET scopes=? WHERE user_id=?",rusqlite::params![campfire_db::models::google_account::CALENDAR_SCOPE,DAVID])?;}
                "show_disconnected"=>{tx.conn().execute("UPDATE google_accounts SET disconnected_reason='Disconnected' WHERE user_id=?",[DAVID])?;}
                _=>{}
            }
            Ok(())
        }).await.unwrap();
        let mut b = a.sign_in(DAVID).await;
        let path = row["path"].as_str().unwrap();
        let mut actual = Vec::new();
        for offset in row["offsets"].as_array().unwrap() {
            clock.set(now + jiff::SignedDuration::from_secs(offset.as_i64().unwrap()));
            if row["fault"] == "transport" {
                r.fail_next();
            } else {
                r.answer(
                    row["fault"].as_u64().unwrap_or(200) as u16,
                    oracle[if name.starts_with("show") {
                        "file"
                    } else {
                        "list"
                    }]
                    .clone(),
                );
            }
            let reply = b
                .send(Req::new(Method::GET, path).header("accept", "application/json"))
                .await;
            actual.push(observe(&reply, &r));
            // A cached or throttled response leaves the queued fixture unused.
            r.answers.lock().unwrap().clear();
        }
        assert_eq!(json!(actual), row["observations"], "{name}: replies");
        if !row["other"].is_null() {
            let mut other = a.sign_in(JASON).await;
            r.answer(
                200,
                oracle[if name.starts_with("show") {
                    "file"
                } else {
                    "list"
                }]
                .clone(),
            );
            assert_eq!(
                observe(&other.get(path).await, &r),
                row["other"],
                "{name}: independent user"
            );
        }
        assert_eq!(
            json!(*r.calls.lock().unwrap()),
            row["requests"],
            "{name}: recorded requests"
        );
    }
    println!(
        "Pinned Rails Drive endpoint cases: {} exercised; 0 skipped; recorded HTTP only",
        oracle["rows"].as_array().unwrap().len()
    );
}
