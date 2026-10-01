//! Request-level ports of test/controllers/users/stars_controller_test.rb.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use askama::Template;

fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_stars.json")).unwrap()
}

fn path(id: i64) -> String {
    campfire_routes::user_star(id)
}

async fn stars(app: &TestApp, viewer: i64, target: i64) -> i64 {
    app.db().read(move |conn| Ok(conn.query_row(
        "SELECT COUNT(*) FROM user_stars WHERE user_id=? AND starred_user_id=?",
        rusqlite::params![viewer, target], |row| row.get(0),
    )?)).await.unwrap()
}

fn json_request(method: Method, id: i64) -> Req {
    Req::new(method, &path(id)).header("accept", "application/json")
}

#[tokio::test]
async fn ws12_stars_json_mutations_are_idempotent_and_private() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    let mut david = app.david();
    let mut jason = app.sign_in(JASON).await;
    for _ in 0..2 {
        let reply = david.write(json_request(Method::POST, KEVIN)).await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(reply.text(), "{\"starred\":true}");
    }
    assert_eq!(stars(&app, DAVID, KEVIN).await, 1);
    assert_eq!(jason.write(json_request(Method::POST, KEVIN)).await.status, StatusCode::OK);
    for _ in 0..2 {
        let reply = david.write(json_request(Method::DELETE, KEVIN)).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert_eq!(reply.text(), "{\"starred\":false}");
    }
    assert_eq!(stars(&app, DAVID, KEVIN).await, 0);
    assert_eq!(stars(&app, JASON, KEVIN).await, 1);
    assert_eq!(david.write(json_request(Method::POST, BENDER)).await.status, StatusCode::OK);
    assert_eq!(stars(&app, DAVID, BENDER).await, 1);
}

#[tokio::test]
async fn ws12_stars_authorization_and_csrf_are_enforced() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    let mut david = app.david();
    let mut bot = app.sign_in(BENDER).await;
    let mut anonymous = app.anonymous();
    for method in [Method::POST, Method::DELETE] {
        assert_eq!(david.write(json_request(method.clone(), DAVID)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(david.write(json_request(method.clone(), -1)).await.status, StatusCode::NOT_FOUND);
        assert_eq!(bot.write(json_request(method.clone(), KEVIN)).await.status, StatusCode::FORBIDDEN);
        assert_eq!(anonymous.write(json_request(method.clone(), KEVIN)).await.status, StatusCode::UNAUTHORIZED);
        let request = json_request(method.clone(), KEVIN).form(&[("bot_key", BENDER_KEY)]);
        assert_eq!(anonymous.write(request).await.status, StatusCode::FORBIDDEN);
        assert_eq!(david.send(json_request(method, KEVIN)).await.status, StatusCode::UNPROCESSABLE_ENTITY);
    }
    assert_eq!(stars(&app, DAVID, DAVID).await, 0);
    assert_eq!(stars(&app, BENDER, KEVIN).await, 0);
    assert_eq!(stars(&app, DAVID, KEVIN).await, 0);
}

#[tokio::test]
async fn ws12_stars_concurrent_requests_keep_one_row() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    let mut one = app.david();
    let mut two = app.david();
    one.authenticity_token().await;
    two.authenticity_token().await;
    let (one, two) = tokio::join!(
        one.write(json_request(Method::POST, KEVIN)),
        two.write(json_request(Method::POST, KEVIN)),
    );
    assert_eq!(one.status, StatusCode::OK);
    assert_eq!(two.status, StatusCode::OK);
    assert_eq!(stars(&app, DAVID, KEVIN).await, 1);
}

#[tokio::test]
async fn ws12_stars_match_rails_json_bytes_statuses_headers_and_rows() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    let mut david = app.david();
    let mut bot = app.sign_in(BENDER).await;
    let mut anonymous = app.anonymous();
    for case in vectors()["http"].as_array().unwrap() {
        let viewer = case["viewer_id"].as_i64();
        let browser = match viewer {Some(DAVID)=>&mut david,Some(BENDER)=>&mut bot,None=>&mut anonymous,_=>panic!("unexpected viewer")};
        let target = case["target"].as_i64().unwrap();
        let method = case["method"].as_str().unwrap().to_uppercase().parse().unwrap();
        let response = browser.write(json_request(method,target)).await;
        assert_eq!(u64::from(response.status.as_u16()),case["status"].as_u64().unwrap(),"{case}");
        assert_eq!(response.text(),case["body"].as_str().unwrap(),"{case}");
        assert_eq!(response.content_type(),case["content_type"].as_str(),"{case}");
        assert_eq!(stars(&app,viewer.unwrap_or(-1),target).await,case["count"].as_i64().unwrap());
    }
}

#[tokio::test]
async fn ws12_stars_toggle_and_stream_match_every_rails_byte() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    for case in vectors()["fragments"].as_array().unwrap() {
        let starred = case["starred"].as_bool().unwrap();
        let html = super::people_tests::render(&app, |_| campfire_views::users::StarToggle {user_id:KEVIN,starred}.render().unwrap());
        assert_eq!(html,case["html"].as_str().unwrap());
        let stream = super::people_tests::render(&app, |_| campfire_views::users::star_stream(KEVIN,starred).unwrap());
        assert_eq!(stream,case["stream"].as_str().unwrap());
    }
}

#[tokio::test]
async fn ws12_stars_turbo_response_changes_the_card_and_carries_session_csrf() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    let mut david = app.david();
    for (method,starred,label) in [(Method::POST,true,"★ Unstar"),(Method::DELETE,false,"☆ Star")] {
        let response = david.write(Req::new(method,&path(KEVIN)).header("accept","text/vnd.turbo-stream.html")).await;
        assert_eq!(response.status,StatusCode::OK);
        assert_eq!(response.content_type(),Some("text/vnd.turbo-stream.html; charset=utf-8"));
        let html = response.text();
        assert!(html.starts_with(&format!("<turbo-stream action=\"replace\" target=\"star_user_{KEVIN}\"><template>")));
        assert!(html.contains(label));
        assert!(html.contains("name=\"authenticity_token\""));
        assert_eq!(stars(&app,DAVID,KEVIN).await,i64::from(starred));
        let card = david.get(&campfire_routes::user_card(KEVIN)).await;
        assert!(card.text().contains(label));
    }
}

#[tokio::test]
async fn ws12_stars_html_redirects_and_inactive_targets_match_rails() {
    let app = TestApp::boot_frozen().await.expect("WS12 requires default seed");
    let mut david = app.david();
    let response = david.write(Req::new(Method::POST,&path(KEVIN))).await;
    assert_eq!(response.location(),Some(format!("http://campfire.test/users/{KEVIN}").as_str()));
    let response = david.write(Req::new(Method::DELETE,&path(KEVIN)).header("referer","http://campfire.test/users")).await;
    assert_eq!(response.location(),Some("http://campfire.test/users"));
    app.db().write(|tx| {tx.conn().execute("UPDATE users SET status=1 WHERE id=?",[KEVIN])?;Ok(())}).await.unwrap();
    assert_eq!(david.write(json_request(Method::POST,KEVIN)).await.status,StatusCode::OK);
    assert_eq!(stars(&app,DAVID,KEVIN).await,1);
    assert!(!david.get(&campfire_routes::user_card(KEVIN)).await.text().contains(&format!("id=\"star_user_{KEVIN}\"")));
    assert_eq!(app.anonymous().write(Req::new(Method::POST,&path(KEVIN))).await.location(),Some("http://campfire.test/session/new"));
}
