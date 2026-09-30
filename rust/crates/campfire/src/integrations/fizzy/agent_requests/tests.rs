use super::super::accounts::{Input, UNREADABLE_TOKEN_REASON};
use super::*;
use crate::controllers::presenters::test_support::*;
async fn fixture(name: &str) -> (TestApp, i64, i64) {
    let mut app = TestApp::boot_with_clock(std::sync::Arc::new(campfire_kit::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("pinned seed required");
    app.booted
        .jobs
        .stop(std::time::Duration::from_secs(1))
        .await;
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    let name = name.to_owned();
    let ids=app.db().write(move|tx| {
  let agent:i64=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
  tx.conn().execute("DELETE FROM activity_items WHERE (source_type='AgentApproval' AND source_id IN (SELECT id FROM agent_approvals WHERE agent_id=?)) OR (source_type='AgentBudgetNotice' AND source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id=?))",params![agent,agent])?;
  tx.conn().execute("DELETE FROM agent_approvals WHERE agent_id=?",[agent])?;
  tx.conn().execute("DELETE FROM agent_budget_notices WHERE agent_id=?",[agent])?;
  tx.conn().execute("UPDATE users SET role=0 WHERE role=1",[])?;
  tx.conn().execute("UPDATE users SET name='Owner',role=0 WHERE id=?",[DAVID])?;
  tx.conn().execute("UPDATE users SET name='Admin',role=1 WHERE id=?",[JASON])?;
  tx.conn().execute("UPDATE users SET inbox_preferences=? WHERE id=?",params![if name=="pref_optout" {"{\"agent_approvals\":false}"} else {"{}"},DAVID])?;
  tx.conn().execute("UPDATE agents SET owner_id=?,daily_external_action_cap=NULL,suspended_at=? WHERE id=?",params![if name=="missing_owner"{None}else{Some(DAVID)},if name=="suspended"{Some(tx.now())}else{None},agent])?;
  tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=? AND capability='external_action'",[agent])?;
  if name!="forbidden" {tx.conn().execute("INSERT INTO agent_grants (agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES (?,'external_action',?,?,?,?)",params![agent,if name=="room_grant"{Some(ALL_TALK)}else{None},JASON,tx.now(),tx.now()])?;}
  Account::disconnect(tx,DAVID)?;
  let account=if name=="no_account"{0}else {
   let account=Account::relink(tx,&crypto,&Input{user_id:DAVID,account_id:"897362094",account_name:None,fizzy_user_id:Some("03user1"),fizzy_user_name:Some("Owner"),token:"fixture-owner"})?;
   if name=="disconnected" {account.mark_disconnected(tx,"Disconnected")?;}
   account.id
  };
  if name=="budget" {
   tx.conn().execute("UPDATE agents SET daily_external_action_cap=1 WHERE id=?",[agent])?;
   tx.conn().execute("INSERT INTO agent_approvals (agent_id,action,summary,status,expires_at,created_at,updated_at) VALUES (?,'other','Other request','denied',?,?,?)",params![agent,tx.now().since(SignedDuration::from_secs(86400)),tx.now(),tx.now()])?;
   let approval=tx.conn().last_insert_rowid();
   fan_out(tx,DAVID,approval)?;
  }
  Ok((agent,account))
 }).await.unwrap();
    (app, ids.0, ids.1)
}
async fn call(app: &TestApp, agent: i64, fields: Value) -> ReadResult {
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| create(tx, &crypto, agent, fields, None, &TimeZone::UTC))
        .await
        .unwrap()
}
async fn snapshot(app: &TestApp, agent: i64, account: i64) -> Value {
    app.db().read(move|c| {
  let approvals:Vec<Value>=c.prepare("SELECT action,summary,payload,external_id,status,expires_at,created_at,room_id,fizzy_connected_account_id,fizzy_user_id,fizzy_user_name FROM agent_approvals WHERE agent_id=? ORDER BY id")?
   .query_map([agent],|r|{let expires:Timestamp=r.get(5)?;let created:Timestamp=r.get(6)?;Ok(json!({"action":r.get::<_,String>(0)?,"summary":r.get::<_,String>(1)?,"payload":r.get::<_,Option<String>>(2)?,"external_id":r.get::<_,Option<String>>(3)?,"status":r.get::<_,String>(4)?,"ttl":(expires.as_microsecond()-created.as_microsecond())/1_000_000,"room":r.get::<_,Option<i64>>(7)?,"identity_match":r.get::<_,Option<i64>>(8)?==Some(account)&&r.get::<_,Option<String>>(9)?.as_deref()==Some("03user1"),"identity_name":r.get::<_,Option<String>>(10)?}))})?.collect::<rusqlite::Result<_>>()?;
  let inbox:Vec<Value>=c.prepare("SELECT u.name,i.event_type,i.handled_at IS NOT NULL,i.read_at IS NOT NULL FROM activity_items i JOIN users u ON u.id=i.user_id WHERE (i.source_type='AgentApproval' AND i.source_id IN (SELECT id FROM agent_approvals WHERE agent_id=?)) OR (i.source_type='AgentBudgetNotice' AND i.source_id IN (SELECT id FROM agent_budget_notices WHERE agent_id=?)) ORDER BY i.event_type,u.name")?
   .query_map(params![agent,agent],|r|Ok(json!({"recipient":r.get::<_,String>(0)?,"type":r.get::<_,String>(1)?,"handled":r.get::<_,bool>(2)?,"read":r.get::<_,bool>(3)?})))?.collect::<rusqlite::Result<_>>()?;
  let notices:Vec<Value>=c.prepare("SELECT cap,day FROM agent_budget_notices WHERE agent_id=? ORDER BY id")?.query_map([agent],|r|Ok(json!({"cap":r.get::<_,String>(0)?,"day":r.get::<_,String>(1)?})))?.collect::<rusqlite::Result<_>>()?;
  let jobs:i64=c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Fizzy::PerformAgentActionJob'",[],|r|r.get(0))?;
  assert_eq!(jobs,0,"pending requests must never enqueue execution");
  Ok(json!({"approvals":approvals,"inbox":inbox,"notices":notices}))
 }).await.unwrap()
}
#[tokio::test]
async fn ws15e_fizzy_agent_requests_match_pinned_service_results() {
    let vectors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/ws15e_fizzy_agent_requests.json"
    )))
    .unwrap();
    for vector in vectors["requests"].as_array().unwrap() {
        let name = vector["name"].as_str().unwrap();
        let (app, agent, account) = fixture(name).await;
        if name.starts_with("replay") {
            call(&app,agent,json!({"kind":"comment","number":579,"body":" Nice work ","external_id":" replay "})).await;
            let name = name.to_owned();
            app.db()
                .write(move |tx| {
                    if name == "replay_cap" {
                        tx.conn().execute(
                            "UPDATE agents SET daily_external_action_cap=1 WHERE id=?",
                            [agent],
                        )?;
                    }
                    if name == "replay_expired" {
                        tx.conn().execute(
                            "UPDATE agent_approvals SET expires_at=? WHERE agent_id=?",
                            params![tx.now(), agent],
                        )?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
        }
        let result = call(&app, agent, vector["fields"].clone()).await;
        assert_eq!(json!(result.status), vector["status"], "{name}");
        assert_eq!(json!(result.error), vector["error"], "{name}");
        let mut body = result.body();
        if result.error.is_none() {
            let id = body["id"].as_i64().unwrap();
            let expiry = app
                .db()
                .read(move |c| {
                    c.query_row(
                        "SELECT expires_at FROM agent_approvals WHERE id=?",
                        [id],
                        |r| r.get::<_, Timestamp>(0),
                    )
                    .map_err(Into::into)
                })
                .await
                .unwrap();
            assert_eq!(body["expires_at"], iso(expiry));
            body.as_object_mut().unwrap().remove("id");
            body.as_object_mut().unwrap().remove("expires_at");
        }
        if name == "budget" {
            call(&app, agent, vector["fields"].clone()).await;
        }
        assert_eq!(body, vector["body"], "{name}");
        let actual = snapshot(&app, agent, account).await;
        for key in ["approvals", "inbox", "notices"] {
            assert_eq!(actual[key], vector[key], "{name} {key}");
        }
        println!("Fizzy agent request Rails service case {name}: 1 passed; 0 failed");
    }
}
#[test]
fn ws15e_fizzy_agent_budget_uses_zone_day_and_dst() {
    for (instant, zone, start, end, retry) in [
        (
            "2026-03-02T16:00:00Z",
            "UTC",
            "2026-03-02 00:00:00",
            "2026-03-03 00:00:00",
            28799,
        ),
        (
            "2026-03-08T06:30:00Z",
            "America/New_York",
            "2026-03-08 05:00:00",
            "2026-03-09 04:00:00",
            77399,
        ),
        (
            "2026-11-01T05:30:00Z",
            "America/New_York",
            "2026-11-01 04:00:00",
            "2026-11-02 05:00:00",
            84599,
        ),
        (
            "2026-03-03T00:00:00Z",
            "America/Los_Angeles",
            "2026-03-02 08:00:00",
            "2026-03-03 08:00:00",
            28799,
        ),
        (
            "2026-03-02T23:59:59.999999Z",
            "UTC",
            "2026-03-02 00:00:00",
            "2026-03-03 00:00:00",
            1,
        ),
    ] {
        let now = Timestamp::from_jiff(instant.parse().unwrap());
        let window = day_window(now, &TimeZone::get(zone).unwrap()).unwrap();
        assert_eq!(window.1.to_db(), start);
        assert_eq!(window.2.to_db(), end);
        assert_eq!(window.3, retry);
    }
}
#[tokio::test]
async fn ws15e_fizzy_agent_requests_are_atomic_and_concurrent_replays_do_not_burn_budget() {
    let (app, agent, _) = fixture("comment").await;
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db().write(move |tx| {tx.conn().execute("UPDATE agents SET daily_external_action_cap=1 WHERE id=?",[agent])?;Ok(())}).await.unwrap();
    let tasks=(0..12).map(|_| {let db=app.db().clone();let crypto=ArEncryption::new(&app.booted.app.secrets);tokio::spawn(async move {db.write(move|tx|create(tx,&crypto,agent,json!({"kind":"comment","number":579,"body":"Nice work","external_id":" same "}),None,&TimeZone::UTC)).await.unwrap().body()["id"].as_i64().unwrap()})}).collect::<Vec<_>>();
    let mut ids = std::collections::BTreeSet::new();
    for task in tasks {
        ids.insert(task.await.unwrap());
    }
    assert_eq!(ids.len(), 1);
    let count=app.db().read(move|c|c.query_row("SELECT COUNT(*) FROM activity_items WHERE source_type='AgentApproval' AND source_id IN (SELECT id FROM agent_approvals WHERE agent_id=?)",[agent],|r|r.get::<_,i64>(0)).map_err(Into::into)).await.unwrap();
    assert_eq!(count, 2);
    app.db().write(move|tx|{tx.conn().execute("UPDATE agents SET daily_external_action_cap=NULL WHERE id=?",[agent])?;tx.conn().execute_batch("CREATE TRIGGER ws15e_inbox_failure BEFORE INSERT ON activity_items WHEN NEW.source_type='AgentApproval' BEGIN SELECT RAISE(ABORT,'test inbox failure'); END")?;Ok(())}).await.unwrap();
    let failed = app
        .db()
        .write(move |tx| {
            create(
                tx,
                &crypto,
                agent,
                json!({"kind":"close","number":579,"external_id":" second "}),
                None,
                &TimeZone::UTC,
            )
        })
        .await;
    assert!(failed.is_err());
    let count = app
        .db()
        .read(move |c| {
            c.query_row(
                "SELECT COUNT(*) FROM agent_approvals WHERE agent_id=?",
                [agent],
                |r| r.get::<_, i64>(0),
            )
            .map_err(Into::into)
        })
        .await
        .unwrap();
    assert_eq!(count, 1);
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER ws15e_inbox_failure")?;
            Ok(())
        })
        .await
        .unwrap();
    // Failed ciphertext is cached before any approval is constructed.
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE fizzy_connected_accounts SET access_token='corrupt' WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        call(&app, agent, json!({"kind":"close","number":579}))
            .await
            .status,
        422
    );
    assert_eq!(
        app.db()
            .read(|c| Account::for_user(c, DAVID))
            .await
            .unwrap()
            .unwrap()
            .disconnected_reason
            .as_deref(),
        Some(UNREADABLE_TOKEN_REASON)
    );
}
