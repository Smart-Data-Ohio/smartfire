//! Installed dependency hooks, checked against actual pinned User#destroy! results.
use crate::controllers::presenters::test_support::{SEED_NOW, TestApp};
use campfire_db::{NewUser, PasswordDigest, User};
use rusqlite::params;
use serde_json::{Value, json};
use std::sync::Arc;
#[tokio::test]
async fn ws11_peer_account_removal_matches_rails_commit_and_failure_rollback() {
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../vectors/agents_account_removal_contract.json"
    ))
    .unwrap();
    for reject in [false, true] {
        let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
            SEED_NOW.parse().unwrap(),
        ));
        let (app, _dir) = TestApp::boot_with_clock(clock)
            .await
            .expect("default seed")
            .stop_jobs()
            .await;
        let crypto = app.ar_encryption.clone();
        let uid=app.db.write(move|tx| {
   let u=User::create(tx,NewUser {name:"WS11 removal".into(),email_address:Some(format!("ws11-removal-{reject}@example.test")),password_digest:Some(PasswordDigest::create("ws11 public fixture password",4)?),..Default::default()})?;
   tx.conn().execute("INSERT INTO github_connected_accounts(user_id,github_login,access_token,created_at,updated_at) VALUES(?,'ws11-removal',?,?,?)",params![u.id,crypto.encrypt("ws11 public fixture"),tx.now(),tx.now()])?;
   tx.conn().execute("INSERT INTO fizzy_connected_accounts(user_id,fizzy_account_id,access_token,created_at,updated_at) VALUES(?,'ws11-public',?,?,?)",params![u.id,crypto.encrypt("ws11 public fixture"),tx.now(),tx.now()])?;
   if reject {tx.conn().execute_batch("CREATE TRIGGER ws11_reject_fizzy_removal BEFORE DELETE ON fizzy_connected_accounts BEGIN SELECT RAISE(ABORT,'WS11 rejected peer deletion'); END")?;}
   Ok(u.id)
  }).await.unwrap();
        let result = app
            .db
            .write(move |tx| User::find(tx.conn(), uid)?.destroy(tx))
            .await;
        let actual=app.db.read(move|c|Ok(json!({"error":result.err().map(|_|"peer deletion failed"),"user":User::find_by_id(c,uid)?.is_some(),"github":c.query_row("SELECT EXISTS(SELECT 1 FROM github_connected_accounts WHERE user_id=?)",[uid],|r|r.get::<_,bool>(0))?,"fizzy":c.query_row("SELECT EXISTS(SELECT 1 FROM fizzy_connected_accounts WHERE user_id=?)",[uid],|r|r.get::<_,bool>(0))?}))).await.unwrap();
        assert_eq!(
            actual,
            oracle["cases"][if reject { "failure" } else { "success" }]
        );
    }
}
