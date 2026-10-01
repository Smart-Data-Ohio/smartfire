//! Bots::ClearPlaintextTokensTest, exercised through the production periodic service.
use crate::controllers::presenters::test_support::TestApp;
use campfire_db::{Connection, User};
use rusqlite::params;
use serde_json::{Value, json};
fn gold(key: &str) -> Value {
    let all: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/agents_bot_plaintext_named_cases.json"
    ))
    .unwrap();
    all["results"][key].clone()
}
fn row(conn: &Connection, id: i64, current: &str, old: Option<&str>) -> campfire_db::Result<Value> {
    let u = User::find(conn, id)?;
    let plaintext = conn.query_row("SELECT bot_token FROM users WHERE id=?", [id], |r| {
        r.get::<_, Option<String>>(0)
    })?;
    Ok(
        json!({"plaintext":plaintext,"digest":u.bot_token_digest,"current_auth":User::authenticate_bot(conn,current)?.map(|u|u.id)==Some(id),"old_auth":old.map(|key|User::authenticate_bot(conn,key).map(|u|u.map(|u|u.id)==Some(id))).transpose()?}),
    )
}
#[tokio::test]
async fn ws11_plaintext_case_rekeys_stale_and_digestless_leaves_clean() {
    let (app, _dir) = TestApp::boot().await.expect("seed").stop_jobs().await;
    crate::jobs::periodic::clear_plaintext_bot_tokens(&app.db)
        .await
        .unwrap();
    let (stale, digestless, clean, key) = app
        .db
        .write(|tx| {
            let stale = User::create_bot(tx, "Stale Digest", None)?;
            tx.conn().execute(
                "UPDATE users SET bot_token='CurrentKey12',bot_token_digest=? WHERE id=?",
                params![
                    campfire_db::user::digest_bot_token("OldKey123456"),
                    stale.id
                ],
            )?;
            let digestless = User::create_bot(tx, "Digestless", None)?;
            tx.conn().execute(
                "UPDATE users SET bot_token='NoDigestKey1',bot_token_digest=NULL WHERE id=?",
                [digestless.id],
            )?;
            let clean = User::create_bot(tx, "Clean", None)?;
            Ok((
                stale.id,
                digestless.id,
                clean.id,
                clean.plain_bot_key().unwrap(),
            ))
        })
        .await
        .unwrap();
    let count = crate::jobs::periodic::clear_plaintext_bot_tokens(&app.db)
        .await
        .unwrap();
    app.db.read(move|conn| {
        let clean_plaintext=conn.query_row("SELECT bot_token FROM users WHERE id=?",[clean],|r|r.get::<_,Option<String>>(0))?;
        assert_eq!(json!({"count":count,"stale":row(conn,stale,&format!("{stale}-CurrentKey12"),Some(&format!("{stale}-OldKey123456")))?,"digestless":row(conn,digestless,&format!("{digestless}-NoDigestKey1"),None)?,"clean_plaintext":clean_plaintext,"clean_auth":User::authenticate_bot(conn,&key)?.map(|u|u.id)==Some(clean)}),gold("heal"));Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws11_plaintext_case_key_reset_beats_snapshot() {
    let (app, _dir) = TestApp::boot().await.expect("seed").stop_jobs().await;
    crate::jobs::periodic::clear_plaintext_bot_tokens(&app.db)
        .await
        .unwrap();
    let (id, key) = app
        .db
        .write(|tx| {
            let mut bot = User::create_bot(tx, "Racing Reset", None)?;
            tx.conn().execute(
                "UPDATE users SET bot_token='LeakedKey123',bot_token_digest=NULL WHERE id=?",
                [bot.id],
            )?;
            let key = bot.reset_bot_key(tx)?;
            Ok((bot.id, key))
        })
        .await
        .unwrap();
    let healed =
        crate::jobs::periodic::heal_plaintext_bot_token(&app.db, id, "LeakedKey123".into())
            .await
            .unwrap();
    app.db.read(move|conn| {
        assert_eq!(json!({"healed":healed,"new_auth":User::authenticate_bot(conn,&key)?.map(|u|u.id)==Some(id),"old_auth":User::authenticate_bot(conn,&format!("{id}-LeakedKey123"))?.map(|u|u.id)==Some(id)}),gold("race"));Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn ws11_plaintext_case_repeat_noop() {
    let (app, _dir) = TestApp::boot().await.expect("seed").stop_jobs().await;
    crate::jobs::periodic::clear_plaintext_bot_tokens(&app.db)
        .await
        .unwrap();
    assert_eq!(
        json!(
            crate::jobs::periodic::clear_plaintext_bot_tokens(&app.db)
                .await
                .unwrap()
        ),
        gold("repeat")
    );
}
