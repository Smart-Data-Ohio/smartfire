//! The SPA's product tour stamp over the seeded app with `SPA_ENABLED`: the SPA calls classic's
//! `PATCH /users/me/tour` (users/tours#update) as the SPA session, with the CSRF token
//! `/api/v1/boot` gave it, and `/api/v1/me` then reports the tour completed.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::Value;

use super::api_tests::{app, get};
use crate::controllers::presenters::test_support::{Browser, DAVID, Req, TestApp};

async fn tour_completed(b: &mut Browser<'_>) -> bool {
    let reply = b.send(get("/api/v1/me")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let me: api::Me = serde_json::from_slice(&reply.body).unwrap();
    me.preferences.tour_completed
}

/// The token the SPA's client holds: `/api/v1/boot`'s `csrfToken`.
async fn spa_token(b: &mut Browser<'_>) -> String {
    let reply = b.send(get("/api/v1/boot")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    let boot: Value = serde_json::from_slice(&reply.body).unwrap();
    boot["csrfToken"].as_str().unwrap().to_string()
}

/// The request the SPA's client sends: no body, JSON accepted, the token in `X-CSRF-Token`.
fn stamp(token: &str) -> Req {
    Req::new(Method::PATCH, "/users/me/tour")
        .header("accept", "application/json")
        .header(campfire_kit::csrf::HEADER, token)
}

async fn newcomer(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET tour_completed_at=NULL WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn the_spa_session_stamps_the_tour_through_classics_endpoint() {
    let Some(a) = app(true).await else { return };
    newcomer(&a).await;
    let mut b = a.sign_in(DAVID).await;
    assert!(!tour_completed(&mut b).await);

    let token = spa_token(&mut b).await;
    let reply = b.send(stamp(&token)).await;
    assert_eq!(reply.status, StatusCode::NO_CONTENT, "{}", reply.text());
    assert!(reply.body.is_empty());

    assert!(tour_completed(&mut b).await);
}

#[tokio::test]
async fn a_stale_token_is_refused_without_stamping() {
    let Some(a) = app(true).await else { return };
    newcomer(&a).await;
    let mut b = a.sign_in(DAVID).await;

    let reply = b.send(stamp("not-a-token")).await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert!(!tour_completed(&mut b).await);
}
