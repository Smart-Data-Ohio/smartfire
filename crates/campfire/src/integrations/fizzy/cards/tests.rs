use super::*;
use campfire_app::integrations::fizzy::accounts::Account;
use campfire_db::Message;
use jiff::SignedDuration;
use rails_compat::ar_encryption::ArEncryption;
use rusqlite::params;
use serde_json::Value;
use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        fizzy::{accounts, fetch},
        test_support::*,
    },
};
use campfire_db::{MessageChanges, NewMessage};
use serde_json::json;
use std::{sync::Arc, time::Duration};

async fn app() -> TestApp {
    let mut app = TestApp::boot()
        .await
        .expect("build both pinned parity seeds");
    app.booted.jobs.stop(Duration::from_secs(1)).await;
    app
}
async fn link(app: &TestApp, user: i64, token: &'static str) {
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            Account::relink(
                tx,
                &crypto,
                &accounts::Input {
                    user_id: user,
                    account_id: "897362094",
                    account_name: None,
                    fizzy_user_id: None,
                    fizzy_user_name: None,
                    token,
                },
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn ws15e_fizzy_ws8_refs_warm_only_author_reconcile_and_keep_import_quiet() {
    let app = app().await;
    link(&app, DAVID, "author-token").await;
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    app.db().write(move |tx| {
        let mut message = Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,markdown_source:Some("https://app.fizzy.do/897362094/cards/579 `https://app.fizzy.do/897362094/cards/888`".into()),..Default::default()})?;
        let cards=Card::for_message(tx.conn(),message.id)?;
        assert_eq!(cards.len(),1);
        assert_eq!(cards[0].number,579);
        let cache=Cache::find(tx.conn(),cards[0].id,DAVID)?.unwrap();
        assert!(Cache::find(tx.conn(),cards[0].id,JASON)?.is_none());
        assert!(!cache.request_fetch(tx)?);
        let reference: i64 = tx.conn().query_row("SELECT id FROM fizzy_card_references WHERE message_id=?",[message.id],|r|r.get(0))?;
        message.sync_external_references(tx,true)?;
        assert_eq!(reference,tx.conn().query_row("SELECT id FROM fizzy_card_references WHERE message_id=?",[message.id],|r|r.get::<_,i64>(0))?);
        message.edit(tx,MessageChanges {markdown_source:Some("no card now".into()),..Default::default()})?;
        assert!(Card::for_message(tx.conn(),message.id)?.is_empty());
        // Legacy rich text is eligible for Fizzy even though generic LinkEmbed ignores it.
        let legacy = Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:KEVIN,body:Some("<p>https://app.fizzy.do/a/cards/1</p><code>https://app.fizzy.do/a/cards/2</code>".into()),..Default::default()})?;
        assert_eq!(Card::for_message(tx.conn(),legacy.id)?.len(),1);
        tx.conn().execute("UPDATE action_text_rich_texts SET body='<p>https://app.fizzy.do/a/cards/3</p>' WHERE record_type='Message' AND record_id=?",[message.id])?;
        let imported = Message::find(tx.conn(),message.id)?;
        sync_message(tx,&imported,false,Some(&crypto))?;
        let imported_card = Card::for_message(tx.conn(),message.id)?[0].clone();
        assert_eq!(imported_card.number,3);
        assert!(Cache::find(tx.conn(),imported_card.id,DAVID)?.is_none());
        let streaming=Message::create(tx,NewMessage {room_id:ALL_TALK,creator_id:DAVID,streaming:true,markdown_source:Some("https://app.fizzy.do/a/cards/4".into()),..Default::default()})?;
        assert!(Card::for_message(tx.conn(),streaming.id)?.is_empty());
        Ok(())
    }).await.unwrap();
    let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
    assert_eq!(
        jobs.iter()
            .filter(|j| j.class == "Fizzy::FetchCardJob")
            .count(),
        1
    );
}

#[tokio::test]
async fn ws15e_fizzy_cache_boundary_claims_and_validation() {
    let app = app().await;
    app.db()
        .write(|tx| {
            assert!(Card::for_reference(tx, "", 579).is_err());
            assert!(Card::for_reference(tx, "acc", 0).is_err());
            let card = Card::for_reference(tx, "acc", 579)?;
            assert_eq!(card.web_url(), "https://app.fizzy.do/acc/cards/579");
            assert_eq!(card.id, Card::for_reference(tx, "acc", 579)?.id);
            assert_ne!(card.id, Card::for_reference(tx, "other", 579)?.id);
            let cache = Cache::for_viewer(tx, &card, DAVID)?;
            let vectors: Value = serde_json::from_str(include_str!(
                "../../../../../../vectors/ws15e_fizzy_cards.json"
            ))
            .unwrap();
            for case in vectors["serialized_payloads"].as_array().unwrap() {
                cache.save(tx, Some(&case["payload"]), None, None)?;
                let stored: Option<String> = tx.conn().query_row(
                    "SELECT payload FROM fizzy_card_caches WHERE id=?",
                    [cache.id],
                    |r| r.get(0),
                )?;
                assert_eq!(stored.as_deref(), case["stored"].as_str());
            }
            let now = tx.now();
            let mut snapshot = cache.clone();
            snapshot.fetched_at = Some(now.ago(STALE_AFTER));
            assert!(!snapshot.stale(now), "exact TTL equality is fresh");
            assert!(snapshot.stale(now.since(SignedDuration::from_micros(1))));
            assert!(cache.request_fetch(tx)?);
            assert!(
                !cache.request_fetch(tx)?,
                "old snapshot loses conditional claim"
            );
            cache.release_fetch_request(tx)?;
            assert!(cache.request_fetch(tx)?);
            tx.conn().execute(
                "UPDATE fizzy_card_caches SET fetch_requested_at=?1 WHERE id=?2",
                params![now.ago(STALE_AFTER), cache.id],
            )?;
            assert!(
                !cache.request_fetch_at(tx, now)?,
                "exact claim-window equality is recent"
            );
            assert!(cache.request_fetch_at(tx, now.since(SignedDuration::from_micros(1)))?);
            tx.conn().execute(
                "UPDATE fizzy_card_caches SET fetch_requested_at=?1 WHERE id=?2",
                params![tx.now().since(SignedDuration::from_secs(1)), cache.id],
            )?;
            assert!(!cache.request_fetch(tx)?);
            tx.conn().execute(
                "UPDATE fizzy_card_caches SET fetch_requested_at=?1 WHERE id=?2",
                params![tx.now().ago(SignedDuration::from_secs(301)), cache.id],
            )?;
            assert!(cache.request_fetch(tx)?);
            assert!(Cache::for_viewer(tx, &card, -1).is_err());
            Ok(())
        })
        .await
        .unwrap();
}

#[test]
fn ws15e_fizzy_web_urls_follow_configured_host() {
    if std::env::var_os("WS15E_FIZZY_URL_CHILD").is_some() {
        let card = Card {
            id: 3,
            account_id: "897362094".into(),
            number: 579,
        };
        assert_eq!(
            card.web_url(),
            "https://fizzy.example.com/897362094/cards/579"
        );
        return;
    }
    // Each Rails ENV override gets its own process; concurrent tests keep their default origin.
    for base in ["https://fizzy.example.com", "https://fizzy.example.com/"] {
        let output = std::process::Command::new(std::env::current_exe().unwrap())
            .args([
                "--exact",
                "integrations::fizzy::cards::tests::ws15e_fizzy_web_urls_follow_configured_host",
            ])
            .env("WS15E_FIZZY_URL_CHILD", "1")
            .env("FIZZY_API_BASE_URL", base)
            .output()
            .unwrap();
        assert!(
            output.status.success(),
            "{}",
            String::from_utf8_lossy(&output.stdout)
        );
        assert!(
            String::from_utf8_lossy(&output.stdout)
                .contains("test result: ok. 1 passed; 0 failed;")
        );
    }
}

#[tokio::test]
async fn ws15e_fizzy_fetch_results_are_private_and_match_failure_states() {
    let app = app().await;
    let card = app
        .db()
        .write(|tx| Card::for_reference(tx, "897362094", 579))
        .await
        .unwrap();
    let message = app
        .db()
        .write(|tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: KEVIN,
                    markdown_source: Some("https://app.fizzy.do/897362094/cards/579".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let path = format!(
        "/rooms/{ALL_TALK}/fizzy/cards/{}/card?message_id={}",
        card.id, message.id
    );
    for (index, (status, body)) in [
        (200, "{\"title\":\"Private David title\"}"),
        (404, "{}"),
        (403, "{}"),
        (500, "{}"),
        (401, "{}"),
    ]
    .iter()
    .enumerate()
    {
        let fixture_token = "david-secret";
        link(&app, DAVID, fixture_token).await;
        link(&app, JASON, "jason-secret").await;
        let id = card.id;
        app.db()
            .write(move |tx| {
                Cache::for_viewer(tx, &Card::find(tx.conn(), id)?, JASON)?.save(
                    tx,
                    Some(&json!({"title":"Jason only"})),
                    Some(tx.now()),
                    None,
                )?;
                if index == 3 {
                    Cache::for_viewer(tx, &Card::find(tx.conn(), id)?, DAVID)?.save(
                        tx,
                        Some(&json!({"title":"Retained David title"})),
                        None,
                        None,
                    )?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let server = FakeServer::start_ws15e(vec![
            Route::new("GET", "app.fizzy.do", "/897362094/cards/579.json", *status).body(*body),
        ])
        .await;
        let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: server.addr,
            dialed: Default::default(),
        });
        fetch::fetch(
            &app.booted.app,
            &network(resolver.clone(), dialer),
            "http://app.fizzy.do",
            id,
            DAVID,
        )
        .await
        .unwrap();
        assert_eq!(resolver.lookups(), ["app.fizzy.do"]);
        assert_eq!(server.received.lock().unwrap().len(), 1);
        assert_eq!(
            server.received.lock().unwrap()[0].header("authorization"),
            Some(format!("Bearer {fixture_token}").as_str())
        );
        let status = *status;
        app.db()
            .read(move |conn| {
                let cache = Cache::find(conn, id, DAVID)?.unwrap();
                assert_eq!(
                    Cache::find(conn, id, JASON)?.unwrap().payload.unwrap()["title"],
                    "Jason only"
                );
                match status {
                    200 => {
                        assert_eq!(cache.payload.unwrap()["title"], "Private David title");
                        assert!(cache.fetch_error.is_none());
                        assert!(cache.fetched_at.is_some());
                    }
                    403 | 404 => {
                        assert!(cache.payload.is_none());
                        assert_eq!(cache.fetch_error.as_deref(), Some(NOT_FOUND));
                        assert!(cache.fetched_at.is_some());
                    }
                    500 => {
                        assert_eq!(cache.payload.unwrap()["title"], "Retained David title");
                        assert_eq!(cache.fetch_error.as_deref(), Some("Fizzy returned 500"));
                    }
                    401 => {
                        assert!(cache.payload.is_none());
                        assert!(cache.fetch_error.is_none());
                        assert!(cache.fetched_at.is_none());
                        assert_eq!(
                            Account::for_user(conn, DAVID)?
                                .unwrap()
                                .disconnected_reason
                                .as_deref(),
                            Some(accounts::REJECTED_TOKEN_REASON)
                        );
                    }
                    _ => unreachable!(),
                }
                Ok(())
            })
            .await
            .unwrap();
        let response = app.david().get(&path).await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        let html = response.text();
        assert!(!html.contains("Jason only"));
        match status {
            200 => assert!(html.contains("Private David title")),
            403 | 404 => {
                assert!(html.contains("Fizzy card #579"));
                assert!(!html.contains("Connect Fizzy to preview"));
            }
            500 => assert!(html.contains("Retained David title")),
            401 => assert!(html.contains("Connect Fizzy to preview")),
            _ => unreachable!(),
        }
    }
    let resolver = Arc::new(FakeResolver::default());
    let net = crate::net::Network {
        resolver: resolver.clone(),
        ..crate::net::Network::system()
    };
    fetch::fetch(
        &app.booted.app,
        &net,
        "https://app.fizzy.do",
        card.id,
        KEVIN,
    )
    .await
    .unwrap();
    assert!(resolver.lookups().is_empty());
    let id = card.id;
    assert!(
        app.db()
            .read(move |conn| Cache::find(conn, id, KEVIN))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn ws15e_fizzy_transport_error_is_cached_and_job_has_one_attempt() {
    use campfire_jobs::JobKind;
    assert_eq!(fetch::FetchJob::retry_policy().attempts, 1);
    assert_eq!(
        fetch::FetchJob::retry_policy().retry_delay(1, None, 0.0),
        None
    );
    let app = app().await;
    link(&app, DAVID, "fixture-token").await;
    let card = app
        .db()
        .write(|tx| Card::for_reference(tx, "897362094", 579))
        .await
        .unwrap();
    let resolver = Arc::new(FakeResolver::default());
    let net = crate::net::Network {
        resolver: resolver.clone(),
        ..crate::net::Network::system()
    };
    fetch::fetch(
        &app.booted.app,
        &net,
        "https://app.fizzy.do",
        card.id,
        DAVID,
    )
    .await
    .unwrap();
    assert_eq!(resolver.lookups(), ["app.fizzy.do"]);
    let id = card.id;
    app.db()
        .read(move |conn| {
            let cache = Cache::find(conn, id, DAVID)?.unwrap();
            assert_eq!(
                cache.fetch_error.as_deref(),
                Some("Could not reach Fizzy (Socket Error)")
            );
            assert!(cache.fetched_at.is_some());
            Ok(())
        })
        .await
        .unwrap();
}
