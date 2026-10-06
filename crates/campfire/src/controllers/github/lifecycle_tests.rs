use super::card_tests::Fresh;
use crate::integrations::github::{
    accounts::{Account, AccountInput},
    tests::crypto,
};
use serde_json::json;
#[tokio::test]
async fn github_user_and_bot_deactivation_disconnects_inside_real_user_transaction() {
    for bot in [false, true] {
        let fresh = Fresh::new(&json!({})).await;
        let id=fresh.app.db.write(move|tx|{let user=if bot{campfire_db::User::create_bot(tx,"Deactivation test",None)?}else{campfire_db::User::find(tx.conn(),811)?};
 Account::create(tx,&crypto(),&AccountInput{user_id:user.id,github_login:"oracle",access_token:"fixture-retained",refresh_token:Some("fixture-refresh-retained"),token_source:"app",token_expires_at:None})?;
 tx.conn().execute("UPDATE github_connected_accounts SET last_error='old error',updated_at='2025-01-01 12:00:00' WHERE user_id=?",[user.id])?;Ok(user.id)}).await.unwrap();
        fresh
            .app
            .db
            .write(move |tx| campfire_db::User::find(tx.conn(), id)?.deactivate(tx))
            .await
            .unwrap();
        fresh.app.db.read(move|conn|{let account=Account::for_user(conn,id)?.unwrap();assert_eq!(account.disconnected_reason.as_deref(),Some("Account deactivated"));assert!(!account.connected());assert_eq!(account.last_error.as_deref(),Some("old error"));assert_eq!(account.updated_at,campfire_db::Timestamp::from_jiff("2026-01-01T12:00:00Z".parse().unwrap()));let(a,r)=conn.query_row("SELECT access_token,refresh_token FROM github_connected_accounts WHERE user_id=?",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?;assert_eq!(crypto().decrypt(&a).unwrap(),"fixture-retained");assert_eq!(crypto().decrypt(&r).unwrap(),"fixture-refresh-retained");Ok(())}).await.unwrap();
        assert!(fresh.server.received().is_empty());
    }
}
#[tokio::test]
async fn github_deactivation_preserves_credentials_noop_stamps_and_rails_validation_and_rollback() {
    let cases: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/github_lifecycle.json")).unwrap();
    for case in cases.as_array().unwrap() {
        let fresh = Fresh::new(&json!({})).await;
        let input = case.clone();
        let id=fresh.app.db.write(move|tx|{let user=if input["bot"]==true{campfire_db::User::create_bot(tx,"Lifecycle bot",None)?}else{campfire_db::User::find(tx.conn(),811)?};
 let a=Account::create(tx,&crypto(),&AccountInput{user_id:user.id,github_login:"oracle",access_token:"fixture-retained",refresh_token:Some("fixture-refresh-retained"),token_source:"app",token_expires_at:None})?;
 tx.conn().execute("UPDATE github_connected_accounts SET last_error='old error',disconnected_reason=?,updated_at='2025-01-01 12:00:00',token_source=? WHERE id=?",rusqlite::params![input["initial_reason"].as_str(),if input["invalid"]==true{"invalid"}else{"app"},a.id])?;
 if input["reject_user"]==true{tx.conn().execute_batch(&format!("CREATE TRIGGER reject_lifecycle_user BEFORE UPDATE ON users WHEN NEW.id={} AND NEW.status=1 BEGIN SELECT RAISE(ABORT,'user rejected'); END;",user.id))?;}
 Ok(user.id)}).await.unwrap();
        let result = fresh
            .app
            .db
            .write(move |tx| campfire_db::User::find(tx.conn(), id)?.deactivate(tx))
            .await;
        assert_eq!(
            result.is_err(),
            !case["error"].is_null(),
            "{}",
            case["name"]
        );
        let actual=fresh.app.db.read(move|conn|{let a=Account::for_user(conn,id)?.unwrap();let status=conn.query_row("SELECT status FROM users WHERE id=?",[id],|r|r.get::<_,i64>(0))?;let(a_raw,r_raw)=conn.query_row("SELECT access_token,refresh_token FROM github_connected_accounts WHERE user_id=?",[id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,String>(1)?)))?;
 Ok(json!({"status":if status==1{"deactivated"}else{"active"},"reason":a.disconnected_reason,"last_error":a.last_error,"updated_at":a.updated_at.jiff().to_string(),"access":crypto().decrypt(&a_raw).unwrap(),"refresh":crypto().decrypt(&r_raw).unwrap()}))}).await.unwrap();
        for key in [
            "status",
            "reason",
            "last_error",
            "updated_at",
            "access",
            "refresh",
        ] {
            assert_eq!(actual[key], case[key], "{} {key}", case["name"]);
        }
        assert!(fresh.server.received().is_empty());
    }
}
#[tokio::test]
async fn github_blank_account_disconnect_reason_keeps_after_save_verified_login_callback() {
    let fresh = Fresh::new(&json!({"linked":true})).await;
    fresh
        .app
        .db
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET github_login=NULL WHERE id=811", [])?;
            let a = Account::for_user(tx.conn(), 811)?.unwrap();
            Account::mark_disconnected(tx, a.id, " ")?;
            assert_eq!(
                tx.conn()
                    .query_row("SELECT github_login FROM users WHERE id=811", [], |r| {
                        r.get::<_, String>(0)
                    })?,
                "oracle"
            );
            tx.conn()
                .execute("UPDATE users SET github_login=NULL WHERE id=811", [])?;
            Account::mark_disconnected(tx, a.id, " ")?;
            assert_eq!(
                tx.conn()
                    .query_row("SELECT github_login FROM users WHERE id=811", [], |r| {
                        r.get::<_, String>(0)
                    })?,
                "oracle"
            );
            Ok(())
        })
        .await
        .unwrap();
    assert!(fresh.server.received().is_empty());
}
