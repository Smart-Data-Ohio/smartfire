//! Drive controller security and viewer/recipient policy through the full router.
use super::google_api_tests::{self as support, Recorded};
use crate::controllers::presenters::test_support::{
    ALL_TALK, BENDER, BENDER_KEY, DAVID, DIRECT_KEVIN_BENDER, JASON, KEVIN, Req, TestApp,
};
use campfire_db::Timestamp;
use hyper::{Method, StatusCode};
use serde_json::{Value, json};
use std::sync::Arc;
async fn app() -> (TestApp, Arc<Recorded>) {
    let a = TestApp::boot().await.expect("default seed required");
    let r = Recorded::new(vec![]);
    support::install(&a, r.clone()).await;
    a.booted.app.google.drive().install_picker(true);
    a.db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM google_accounts", [])?;
            Ok(())
        })
        .await
        .unwrap();
    (a, r)
}
async fn grant(a: &TestApp, id: i64) {
    support::grant(
        a,
        id,
        Timestamp::from_jiff(a.booted.app.clock.now()).since(jiff::SignedDuration::from_hours(1)),
        true,
    )
    .await;
}
const FILE: &str = "1AbcDefGhIjKlMnOpQrSt";
fn json_req(method: Method, path: &str, body: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&body).unwrap())
}
#[tokio::test]
async fn google_drive_viewer_inaccessible_file_is_empty_404_and_never_reuses_other_viewer_cache() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    grant(&a, JASON).await;
    let path = format!("/google/drive/files/{FILE}");
    let mut owner = a.sign_in(DAVID).await;
    let mut viewer = a.sign_in(JASON).await;
    r.answer(200, support::vectors()["drive_file"].clone());
    let reply = owner.get(&path).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["name"], "Q3 Planning");
    assert_eq!(reply.json()["kind"], "document");
    assert_eq!(owner.get(&path).await.status, StatusCode::OK);
    assert_eq!(r.calls.lock().unwrap().len(), 1);
    r.answer(
        403,
        json!({"error":{"message":"private document metadata"}}),
    );
    let denied = viewer.get(&path).await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
    assert!(denied.body.is_empty());
    assert_eq!(r.calls.lock().unwrap().len(), 2);
}
#[tokio::test]
async fn google_drive_requires_viewer_grant_and_valid_id_and_has_distinct_anonymous_behavior() {
    let (a, r) = app().await;
    let mut anon = a.anonymous();
    assert_eq!(
        anon.get("/google/drive/files").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        anon.get(&format!("/google/drive/files/{FILE}"))
            .await
            .location(),
        Some("http://campfire.test/session/new")
    );
    let mut b = a.sign_in(DAVID).await;
    assert_eq!(
        b.get("/google/drive/files").await.status,
        StatusCode::NOT_FOUND
    );
    grant(&a, DAVID).await;
    for id in ["short", "not%20a%20file%20id!!"] {
        let reply = b.get(&format!("/google/drive/files/{id}")).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        assert!(reply.body.is_empty());
    }
    a.db().write(|tx|{tx.conn().execute("UPDATE google_accounts SET scopes='https://www.googleapis.com/auth/drive.metadata.readonly' WHERE user_id=?",[DAVID])?;Ok(())}).await.unwrap();
    assert_eq!(
        b.get("/google/drive/files").await.status,
        StatusCode::NOT_FOUND
    );
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_drive_lists_never_cache_trim_and_cap_terms_and_return_502_on_failure() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    let v = support::vectors();
    for _ in 0..2 {
        r.answer(200, v["drive_list"].clone());
        let reply = b
            .get("/google/drive/files?q=%20%20bob%27s%5Cdraft%20%20")
            .await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.json()["files"].as_array().unwrap().len(), 2);
    }
    let calls = r.calls.lock().unwrap().clone();
    assert_eq!(calls.len(), 2);
    assert!(
        calls[0]["path"]
            .as_str()
            .unwrap()
            .contains("bob%5C%27s%5C%5Cdraft")
    );
    let term = "x".repeat(105);
    r.answer(200, json!({"files":[]}));
    b.get(&format!("/google/drive/files?q={term}")).await;
    let path = r.calls.lock().unwrap()[2]["path"]
        .as_str()
        .unwrap()
        .to_string();
    let u = url::Url::parse(&format!("https://www.googleapis.com{path}")).unwrap();
    let q = u
        .query_pairs()
        .find(|(k, _)| k == "q")
        .unwrap()
        .1
        .into_owned();
    assert_eq!(
        q,
        format!("name contains '{}' and trashed=false", "x".repeat(100))
    );
    r.answer(500, json!({}));
    let reply = b.get("/google/drive/files").await;
    assert_eq!(reply.status, StatusCode::BAD_GATEWAY);
    assert_eq!(reply.json(), json!({"error":"drive_unavailable"}));
}
#[tokio::test]
async fn google_drive_show_and_list_throttle_before_new_http_with_independent_budgets() {
    let (a, r) = app().await;
    grant(&a, DAVID).await;
    let mut b = a.sign_in(DAVID).await;
    r.answer(200, support::vectors()["drive_file"].clone());
    let path = format!("/google/drive/files/{FILE}");
    for _ in 0..60 {
        assert_eq!(b.get(&path).await.status, StatusCode::OK);
    }
    assert_eq!(b.get(&path).await.status, StatusCode::TOO_MANY_REQUESTS);
    for _ in 0..30 {
        r.answer(200, json!({"files":[]}));
        assert_eq!(b.get("/google/drive/files").await.status, StatusCode::OK);
    }
    let denied = b.get("/google/drive/files").await;
    assert_eq!(denied.status, StatusCode::TOO_MANY_REQUESTS);
    assert_eq!(denied.json(), json!({"error":"rate_limited"}));
    assert_eq!(r.calls.lock().unwrap().len(), 31);
}
#[tokio::test]
async fn google_drive_recipients_require_human_membership_but_no_google_grant() {
    let (a, r) = app().await;
    let path = format!("/rooms/{ALL_TALK}/drive_recipients");
    let mut b = a.sign_in(DAVID).await;
    let reply = b
        .send(Req::new(Method::GET, &path).header("accept", "application/json"))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.headers["cache-control"], "no-store");
    assert_eq!(
        reply.json()["recipients"],
        json!([{"id":JASON,"name":"Jason","email":"jason@37signals.com"}])
    );
    let reply = b
        .send(
            Req::new(
                Method::GET,
                &format!("/rooms/{DIRECT_KEVIN_BENDER}/drive_recipients"),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    assert!(reply.body.is_empty());
    let mut anon = a.anonymous();
    assert_eq!(
        anon.send(Req::new(Method::GET, &path).header("accept", "application/json"))
            .await
            .status,
        StatusCode::UNAUTHORIZED
    );
    let mut bot = a.sign_in(BENDER).await;
    assert_eq!(bot.get(&path).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        anon.get(&format!("{path}?bot_key={BENDER_KEY}"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(r.calls.lock().unwrap().is_empty());
}
#[tokio::test]
async fn google_drive_recipient_validation_rechecks_stale_membership_and_preserves_order() {
    let (a, _) = app().await;
    let path = format!("/rooms/{ALL_TALK}/drive_recipients/validate");
    let mut b = a.sign_in(DAVID).await;
    b.get("/users/me/profile").await;
    let valid = b
        .write(json_req(
            Method::POST,
            &path,
            json!({"user_ids":[format!(" {JASON} "),JASON]}),
        ))
        .await;
    assert_eq!(valid.status, StatusCode::OK);
    assert_eq!(valid.json()["recipients"].as_array().unwrap().len(), 1);
    for value in [
        json!("mail@external.test"),
        json!(["mail@external.test"]),
        Value::Null,
    ] {
        let reply = b
            .write(json_req(Method::POST, &path, json!({"user_ids":value})))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(reply.json(), json!({"error":"invalid_recipients"}));
    }
    let reply = b
        .write(json_req(
            Method::POST,
            &path,
            json!({"user_ids":[KEVIN,DAVID,BENDER]}),
        ))
        .await;
    assert_eq!(
        reply.json(),
        json!({"error":"invalid_recipients","invalid_ids":[KEVIN,DAVID,BENDER]})
    );
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                [ALL_TALK, JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = b
        .write(json_req(Method::POST, &path, json!({"user_ids":[JASON]})))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(reply.json()["invalid_ids"], json!([JASON]));
    let empty = b
        .write(json_req(Method::POST, &path, json!({"user_ids":[]})))
        .await;
    assert_eq!(empty.status, StatusCode::OK);
    assert_eq!(empty.json(), json!({"recipients":[]}));
}
#[tokio::test]
async fn google_drive_recipients_exclude_agents_inactive_and_bad_email_without_domain_filter() {
    let (a, _) = app().await;
    let path = format!("/rooms/{ALL_TALK}/drive_recipients");
    let mut b = a.sign_in(DAVID).await;
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='external@contractor.test' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        b.get(&path).await.json()["recipients"][0]["email"],
        "external@contractor.test"
    );
    for status in [1, 2] {
        a.db()
            .write(move |tx| {
                tx.conn()
                    .execute("UPDATE users SET status=? WHERE id=?", [status, JASON])?;
                Ok(())
            })
            .await
            .unwrap();
        assert_eq!(b.get(&path).await.json()["recipients"], json!([]));
    }
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET status=0,email_address='not-an-email' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(b.get(&path).await.json()["recipients"], json!([]));
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET email_address='external@contractor.test' WHERE id=?",
                [JASON],
            )?;
            tx.conn().execute(
                "INSERT INTO agents(user_id,owner_id,created_at,updated_at) VALUES(?,?,?,?)",
                rusqlite::params![JASON, DAVID, tx.now(), tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(b.get(&path).await.json()["recipients"], json!([]));
    let mut agent = a.sign_in(JASON).await;
    assert_eq!(agent.get(&path).await.status, StatusCode::FORBIDDEN);
}
