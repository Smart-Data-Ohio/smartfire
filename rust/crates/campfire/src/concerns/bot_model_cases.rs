//! The three remaining User::BotTest reply-token model cases, without HTTP coercion.
use super::*;
use crate::controllers::presenters::test_support::{ALL_TALK, TestApp};
use serde_json::{Value, json};

const BOT: i64 = 1901500001;
fn oracle(key: &str) -> Value {
    let value: Value =
        serde_json::from_str(include_str!("../../../../vectors/agents_next_named.json")).unwrap();
    value["results"][key].clone()
}
async fn setup() -> TestApp {
    let app = TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE sqlite_sequence SET seq=? WHERE name='users'",
                [BOT - 1],
            )?;
            let bot = User::create_bot(tx, "Reply model bot", None)?;
            assert_eq!(bot.id, BOT);
            Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[bot.id])?;
            Ok(())
        })
        .await
        .unwrap();
    app
}
async fn authenticate(app: &TestApp, token: &str, room: i64, now: jiff::Timestamp) -> Option<i64> {
    let token = token.to_owned();
    let secrets = app.booted.app.secrets.clone();
    app.db()
        .read(move |conn| {
            Ok(
                authenticate_bot_reply_token(conn, &secrets, &token, &room.to_string(), now)?
                    .map(|bot| bot.id),
            )
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn ws11_next_bot_reply_room_binding_and_malformed_tokens() {
    let app = setup().await;
    let now = app.booted.app.clock.now();
    let token =
        rails_compat::verifiers::bot_reply::token_for(&app.booted.app.secrets, BOT, ALL_TALK, now);
    let lookup = token.clone();
    let key_auth = app
        .db()
        .read(move |conn| Ok(User::authenticate_bot(conn, &lookup)?.map(|bot| bot.id)))
        .await
        .unwrap();
    assert_eq!(
        json!({
            "token": token, "bot_id": BOT, "room_id": ALL_TALK,
            "valid": authenticate(&app, &token, ALL_TALK, now).await,
            "wrong_room": authenticate(&app, &token, 201306877, now).await,
            "tampered": authenticate(&app, &format!("{token}x"), ALL_TALK, now).await,
            "malformed": authenticate(&app, "bogus", ALL_TALK, now).await,
            "blank": authenticate(&app, "", ALL_TALK, now).await, "key_auth": key_auth
        }),
        oracle("reply_room")
    );
}
#[tokio::test]
async fn ws11_next_bot_reply_expiry() {
    let app = setup().await;
    let expected = oracle("reply_expiry");
    let token = expected["token"].as_str().unwrap();
    let now = app.booted.app.clock.now();
    assert_eq!(
        json!({"token": token,
            "before": authenticate(&app, token, ALL_TALK, now).await,
            "after": authenticate(&app, token, ALL_TALK, now+jiff::SignedDuration::from_mins(2)).await
        }),
        expected
    );
}
#[tokio::test]
async fn ws11_next_bot_reply_membership_and_deactivation() {
    let app = setup().await;
    let now = app.booted.app.clock.now();
    let token = oracle("reply_room")["token"].as_str().unwrap().to_owned();
    assert_eq!(authenticate(&app, &token, ALL_TALK, now).await, Some(BOT));
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                [BOT, ALL_TALK],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let absent = authenticate(&app, &token, ALL_TALK, now).await;
    app.db()
        .write(|tx| Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[BOT]))
        .await
        .unwrap();
    let restored = authenticate(&app, &token, ALL_TALK, now).await;
    app.db()
        .write(|tx| User::find(tx.conn(), BOT)?.deactivate(tx))
        .await
        .unwrap();
    assert_eq!(
        json!({"absent": absent, "restored": restored,
            "deactivated": authenticate(&app, &token, ALL_TALK, now).await
        }),
        oracle("reply_membership")
    );
}
