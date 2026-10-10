use super::*;
use campfire_db::Timestamp;
use crate::integrations::{
    github::{
        accounts::{Account, AccountInput},
        pull_requests::PullRequest,
        tests::crypto,
    },
    test_support::TestDb,
};
use campfire_db::{Clock, TestClock};
use rusqlite::params;
use serde_json::{Value, json};
use std::sync::Arc;
#[tokio::test]
async fn github_health_counts_limits_boundaries_configuration_and_section_match_rails() {
    let clock = TestClock::frozen_at(Timestamp::from_jiff(
        "2026-01-01T12:00:00Z".parse().unwrap(),
    ));
    let now = clock.now();
    let scratch =
        std::path::PathBuf::from(std::env::var_os("TMPDIR").unwrap()).join("ws15g");
    let fixture =
        tokio::task::spawn_blocking(move || TestDb::in_dir(Arc::new(clock), &scratch))
            .await
            .unwrap();
    let encryption = crypto();
    fixture.db.write(move|tx| {
   for i in 0..13 {let user=900+i;tx.conn().execute("INSERT INTO users (id,name,email_address,created_at,updated_at) VALUES (?,?,?, ?,?)",params![user,format!("User {i}"),format!("user{i}@example.test"),now,now])?;
let a=Account::create(tx,&encryption,&AccountInput {user_id:user,github_login:&format!("login{i}"),access_token:&format!("fixture-secret-{i}"),refresh_token:None,token_expires_at:None,token_source:if i%2==0{"app"}else{"pat"}})?;
tx.conn().execute("UPDATE github_connected_accounts SET disconnected_reason=?,last_error=?,updated_at=? WHERE id=?",params![if i==0{None}else if i==1{Some(String::new())}else{Some(format!("reason <{i}>"))},if i==0{None}else if i==1{Some(String::new())}else{Some(format!("error &{i}"))},now.since(jiff::SignedDuration::from_secs(i)),a.id])?;
let pr=PullRequest::for_reference(tx,&format!("owner{i}"),"repo",i+1)?;tx.conn().execute("UPDATE github_pull_requests SET fetch_error=?,updated_at=? WHERE id=?",params![if i==0{None}else if i==1{Some(String::new())}else{Some(format!("fetch <{i}>"))},now.since(jiff::SignedDuration::from_secs(i)),pr.id])?;
   }
   for (i,age) in [-86_400_000_001i64,-86_400_000_000,0].into_iter().enumerate() {tx.conn().execute("INSERT INTO github_webhook_deliveries (delivery_guid,event,created_at,updated_at) VALUES (?,'ping',?,?)",params![format!("health-{i}"),now.since(jiff::SignedDuration::from_micros(age)),now])?;}Ok(())
  }).await.unwrap();
    let cases: Value =
        serde_json::from_str(include_str!("../../../../../../vectors/github_health.json"))
            .unwrap();
    for case in cases.as_array().unwrap() {
        let case = case.clone();
        fixture
            .db
            .read(move |conn| {
                let snapshot = snapshot(conn, now, |key| {
                    case["config"][key].as_str().map(str::to_owned)
                })?;
                let value = serde_json::to_value(snapshot).unwrap();
                assert_eq!(value, case["snapshot"]);
                assert_eq!(value["connected"], json!(1));
                assert_eq!(value["deliveries_24h"], json!(2));
                Ok(())
            })
            .await
            .unwrap();
    }
}
