use crate::{
    controllers::presenters::test_support::*,
    integrations::fizzy::{
        accounts::{Account, Input},
        cards::{Cache, Card},
    },
};
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::json;

async fn app() -> TestApp {
    let mut app = TestApp::boot()
        .await
        .expect("build both pinned parity seeds");
    app.booted
        .jobs
        .stop(std::time::Duration::from_secs(1))
        .await;
    app
}
async fn message(app: &TestApp) -> (Message, Card) {
    app.db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: KEVIN,
                    client_message_id: Some("ws15e-fizzy-cards".into()),
                    markdown_source: Some("https://app.fizzy.do/897362094/cards/579".into()),
                    ..Default::default()
                },
            )?;
            let card = Card::for_message(tx.conn(), message.id)?[0].clone();
            Ok((message, card))
        })
        .await
        .unwrap()
}
fn path(message: &Message, card: &Card) -> String {
    format!(
        "/rooms/{}/fizzy/cards/{}/card?message_id={}",
        message.room_id, card.id, message.id
    )
}

#[tokio::test]
async fn ws15e_fizzy_frames_authorize_room_message_reference_before_cache_or_enqueue() {
    let app = app().await;
    let (message, card) = message(&app).await;
    let url = path(&message, &card);
    assert_eq!(app.anonymous().get(&url).await.status, StatusCode::FOUND);
    let wrong = format!(
        "/rooms/{DIRECT_KEVIN_BENDER}/fizzy/cards/{}/card?message_id={}",
        card.id, message.id
    );
    assert_eq!(app.david().get(&wrong).await.status, StatusCode::NOT_FOUND);
    let wrong = format!(
        "/rooms/{QUIET_CORNER}/fizzy/cards/{}/card?message_id={}",
        card.id, message.id
    );
    assert_eq!(
        app.david().get(&wrong).await.status,
        StatusCode::NOT_FOUND,
        "membership alone is insufficient"
    );
    let other = app
        .db()
        .write(|tx| Card::for_reference(tx, "897362094", 2))
        .await
        .unwrap();
    assert_eq!(
        app.david().get(&path(&message, &other)).await.status,
        StatusCode::NOT_FOUND
    );
    let response = app.sign_in(JASON).await.get(&url).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Connect Fizzy to preview"));
    let id = card.id;
    assert!(
        app.db()
            .read(move |conn| Cache::find(conn, id, JASON))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn ws15e_fizzy_frames_use_viewer_cache_and_claim_stale_refresh_once() {
    let app = app().await;
    let (message, card) = message(&app).await;
    let id = card.id;
    app.db().write(move|tx| {
        let card=Card::find(tx.conn(),id)?;
        Cache::for_viewer(tx,&card,DAVID)?.save(tx,Some(&json!({"title":"Private David","url":"javascript:bad","assignees":[{"name":"X","avatar_url":"http://bad.test/image"}]})),Some(tx.now()),None)?;
        Cache::for_viewer(tx,&card,JASON)?.save(tx,Some(&json!({"title":"Private Jason"})),Some(tx.now().ago(jiff::SignedDuration::from_mins(6))),None)?;
        Ok(())
    }).await.unwrap();
    let url = path(&message, &card);
    let response = app.david().get(&url).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Private David"));
    assert!(!response.text().contains("Private Jason"));
    assert!(!response.text().contains("javascript:"));
    assert!(!response.text().contains("http://bad.test"));
    // The seed has no Jason connection; establish it without touching his cache.
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            Account::relink(
                tx,
                &crypto,
                &Input {
                    user_id: JASON,
                    account_id: "897362094",
                    account_name: None,
                    fizzy_user_id: None,
                    fizzy_user_name: None,
                    token: "fixture-jason",
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for _ in 0..2 {
        let response = app.sign_in(JASON).await.get(&url).await;
        assert!(response.text().contains("Private Jason"));
        assert!(!response.text().contains("Private David"));
    }
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    let jobs: Vec<_> = jobs
        .iter()
        .filter(|j| j.class == "Fizzy::FetchCardJob")
        .collect();
    assert_eq!(jobs.len(), 1);
    let html = app
        .db()
        .read(move |conn| crate::controllers::presenters::fizzy_cards::container(conn, &message))
        .await
        .unwrap();
    assert!(html.contains("ws15e-fizzy-cards"));
    assert!(html.contains("loading=\"lazy\""));
    assert!(!html.contains("Private"));
}

#[tokio::test]
async fn ws15e_fizzy_create_and_frame_claim_roll_back_when_durable_insert_is_rejected() {
    let app = app().await;
    let (message, card) = message(&app).await;
    app.db().write(|tx| Ok(tx.conn().execute_batch("CREATE TRIGGER ws15e_reject_fizzy_jobs BEFORE INSERT ON background_jobs WHEN NEW.job_class='Fizzy::FetchCardJob' BEGIN SELECT RAISE(ABORT,'Fizzy job rejected'); END")?)).await.unwrap();
    let response = app.david().get(&path(&message, &card)).await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    let id = card.id;
    assert!(
        app.db()
            .read(move |conn| Cache::find(conn, id, DAVID))
            .await
            .unwrap()
            .is_none()
    );
    let response = app
        .david()
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[
                    (
                        "message[markdown_source]",
                        "https://app.fizzy.do/897362094/cards/777",
                    ),
                    ("message[client_message_id]", "ws15e-fizzy-rollback"),
                ]),
        )
        .await;
    assert_eq!(
        response.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        response.text()
    );
    app.db()
        .read(|conn| {
            for sql in [
                "SELECT COUNT(*) FROM messages WHERE client_message_id='ws15e-fizzy-rollback'",
                "SELECT COUNT(*) FROM fizzy_cards WHERE number=777",
                "SELECT COUNT(*) FROM background_jobs WHERE job_class='Fizzy::FetchCardJob'",
            ] {
                assert_eq!(conn.query_row(sql, [], |r| r.get::<_, i64>(0))?, 0);
            }
            Ok(())
        })
        .await
        .unwrap();
}
