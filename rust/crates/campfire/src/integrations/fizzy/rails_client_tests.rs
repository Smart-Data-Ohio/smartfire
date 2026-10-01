//! One native case per pinned test/models/fizzy/client_test.rb behavior.
use super::client::*;
use crate::integrations::{net, test_support::*};
use serde_json::{Value, json};
use std::sync::Arc;
const TOKEN: &str = "fixture-client-token";
const ACCOUNT: &str = "897362094";
async fn setup(routes: Vec<Route>) -> (FakeServer, Client, Arc<FakeResolver>) {
    let (server, roots) =
        FakeServer::start_named_tls_ws15e(routes, vec!["app.fizzy.do".into()]).await;
    let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let network = net::Network {
        resolver: resolver.clone(),
        dialer,
        tls: net::tls_config(roots),
    };
    (
        server,
        Client::new(network, TOKEN.into(), DEFAULT_BASE),
        resolver,
    )
}
fn response(method: &str, path: &str, status: u16, body: Value) -> Route {
    Route::new(method, "app.fizzy.do", path, status)
        .header("Content-Type", "application/json")
        .body(body.to_string())
}
fn assert_request(server: &FakeServer, method: &str, path: &str, body: Option<&str>) {
    let received = server.received();
    assert_eq!(received.len(), 1);
    assert_eq!(received[0].method, method);
    assert_eq!(received[0].target, path);
    assert_eq!(
        received[0].header("Authorization"),
        Some(format!("Bearer {TOKEN}").as_str())
    );
    if let Some(body) = body {
        assert_eq!(received[0].body, body.as_bytes());
    }
}
#[tokio::test]
async fn ws15e_rails_fizzy_identity_accounts() {
    let payload = json!({"accounts":[{"id":ACCOUNT,"name":"Smart Data","user":{"name":"David"}}]});
    let (s, c, _) = setup(vec![response("GET", "/my/identity.json", 200, payload)]).await;
    let value = c.identity().await.unwrap();
    assert_eq!(value["accounts"][0]["name"], "Smart Data");
    assert_eq!(value["accounts"][0]["user"]["name"], "David");
    assert_request(&s, "GET", "/my/identity.json", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_identity_unauthorized() {
    let (s, c, _) = setup(vec![response("GET", "/my/identity.json", 401, json!({}))]).await;
    assert_eq!(
        c.identity().await.unwrap_err().kind,
        ErrorKind::Unauthorized
    );
    assert_request(&s, "GET", "/my/identity.json", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_boards() {
    let (s, c, _) = setup(vec![response(
        "GET",
        "/897362094/boards.json",
        200,
        json!([{"name":"Engineering"}]),
    )])
    .await;
    assert_eq!(c.boards(ACCOUNT).await.unwrap()[0]["name"], "Engineering");
    assert_request(&s, "GET", "/897362094/boards.json", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_card_steps() {
    let (s, c, _) = setup(vec![response(
        "GET",
        "/897362094/cards/579.json",
        200,
        json!({"title":"Fix the billing bug","steps":[{"completed":true},{"completed":false}]}),
    )])
    .await;
    let v = c.card(ACCOUNT, "579").await.unwrap();
    assert_eq!(v["title"], "Fix the billing bug");
    assert_eq!(v["steps"].as_array().unwrap().len(), 2);
    assert_request(&s, "GET", "/897362094/cards/579.json", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_search() {
    let (s, c, _) = setup(vec![response(
        "GET",
        "/897362094/search.json?q=billing+bug",
        200,
        json!([{"number":579}]),
    )])
    .await;
    assert_eq!(
        c.search(ACCOUNT, "billing bug").await.unwrap()[0]["number"],
        579
    );
    assert_request(&s, "GET", "/897362094/search.json?q=billing+bug", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_create_card() {
    let (s, c, _) = setup(vec![response(
        "POST",
        "/897362094/boards/03board1/cards.json",
        201,
        json!({"number":580}),
    )])
    .await;
    assert_eq!(
        c.create_card(
            ACCOUNT,
            "03board1",
            json!("New card"),
            Some(json!("From chat"))
        )
        .await
        .unwrap()["number"],
        580
    );
    assert_request(
        &s,
        "POST",
        "/897362094/boards/03board1/cards.json",
        Some(r#"{"card":{"title":"New card","description":"From chat"}}"#),
    );
}
#[tokio::test]
async fn ws15e_rails_fizzy_create_comment() {
    let (s, c, _) = setup(vec![response(
        "POST",
        "/897362094/cards/579/comments.json",
        201,
        json!({"url":"https://app.fizzy.do/897362094/cards/579/comments/03comment1"}),
    )])
    .await;
    assert_eq!(
        c.create_comment(ACCOUNT, "579", json!("Nice work"))
            .await
            .unwrap()["url"],
        "https://app.fizzy.do/897362094/cards/579/comments/03comment1"
    );
    assert_request(
        &s,
        "POST",
        "/897362094/cards/579/comments.json",
        Some(r#"{"comment":{"body":"Nice work"}}"#),
    );
}
#[tokio::test]
async fn ws15e_rails_fizzy_move() {
    let (s, c, _) = setup(vec![response(
        "POST",
        "/897362094/cards/579/triage.json",
        204,
        Value::Null,
    )])
    .await;
    assert!(
        c.move_to_column(ACCOUNT, "579", json!("03column1"))
            .await
            .unwrap()
    );
    assert_request(
        &s,
        "POST",
        "/897362094/cards/579/triage.json",
        Some(r#"{"column_id":"03column1"}"#),
    );
}
#[tokio::test]
async fn ws15e_rails_fizzy_close_and_reopen() {
    let (s, c, _) = setup(vec![
        response(
            "POST",
            "/897362094/cards/579/closure.json",
            204,
            Value::Null,
        ),
        response(
            "DELETE",
            "/897362094/cards/579/closure.json",
            204,
            Value::Null,
        ),
    ])
    .await;
    assert!(c.close_card(ACCOUNT, "579").await.unwrap());
    assert!(c.reopen_card(ACCOUNT, "579").await.unwrap());
    assert_eq!(
        s.received()
            .iter()
            .map(|r| r.method.as_str())
            .collect::<Vec<_>>(),
        ["POST", "DELETE"]
    );
}
#[tokio::test]
async fn ws15e_rails_fizzy_not_found() {
    let (s, c, _) = setup(vec![response(
        "GET",
        "/897362094/cards/404.json",
        404,
        json!({}),
    )])
    .await;
    assert_eq!(
        c.card(ACCOUNT, "404").await.unwrap_err().kind,
        ErrorKind::NotFound
    );
    assert_request(&s, "GET", "/897362094/cards/404.json", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_forbidden_message() {
    let (s, c, _) = setup(vec![response(
        "GET",
        "/897362094/cards/579.json",
        403,
        json!({"error":"no access"}),
    )])
    .await;
    let e = c.card(ACCOUNT, "579").await.unwrap_err();
    assert_eq!(e.kind, ErrorKind::Forbidden);
    assert_eq!(e.message, "Fizzy refused: no access");
    assert_request(&s, "GET", "/897362094/cards/579.json", None);
}
#[tokio::test]
async fn ws15e_rails_fizzy_refused_message() {
    let (_server, c, _) = setup(vec![response(
        "POST",
        "/897362094/boards/03board1/cards.json",
        422,
        json!({"message":"Title can't be blank"}),
    )])
    .await;
    let e = c
        .create_card(ACCOUNT, "03board1", json!(""), None)
        .await
        .unwrap_err();
    assert_eq!(e.kind, ErrorKind::Refused);
    assert_eq!(e.message, "Fizzy refused: Title can't be blank");
}
#[tokio::test]
async fn ws15e_rails_fizzy_server_error() {
    let (_server, c, _) = setup(vec![response(
        "GET",
        "/897362094/boards.json",
        500,
        json!("boom"),
    )])
    .await;
    let e = c.boards(ACCOUNT).await.unwrap_err();
    assert_eq!(e.kind, ErrorKind::Other);
    assert_eq!(e.message, "Fizzy returned 500");
}
#[tokio::test]
async fn ws15e_rails_fizzy_transport_error() {
    let (s, c, resolver) = setup(vec![]).await;
    resolver.set("app.fizzy.do", vec![vec![]]);
    assert_eq!(c.boards(ACCOUNT).await.unwrap_err().kind, ErrorKind::Other);
    assert!(s.received().is_empty());
}
#[tokio::test]
async fn ws15e_rails_fizzy_traversal_before_dns() {
    let (s, c, resolver) = setup(vec![]).await;
    assert!(c.card("../my", "579").await.is_err());
    assert!(c.card(ACCOUNT, "579/x").await.is_err());
    assert!(
        c.create_card(ACCOUNT, "../../x", json!("t"), None)
            .await
            .is_err()
    );
    assert!(resolver.lookups().is_empty());
    assert!(s.received().is_empty());
}
