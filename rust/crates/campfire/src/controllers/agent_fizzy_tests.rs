//! Fizzy REST/MCP share WS15e's services, exercised through the full router and actual HTTP.
use super::agent_http_tests::{AGENT, SECRET, initialize};
use super::presenters::test_support::{Req, TestApp};
use crate::integrations::fizzy::{
    State,
    accounts::{Account, Input},
};
use crate::integrations::test_support::{FakeResolver, FakeServer, MappingDialer, Route, network};
use campfire_kit::Method;
use serde_json::{Value, json};
use std::sync::Arc;

async fn check(case: &Value) {
    let config = case["setup"].clone();
    let status = config["remote_status"].as_u64().unwrap_or(200) as u16;
    let body = if status >= 400 {
        "{\"message\":\"Denied\"}"
    } else {
        "[{\"id\":\"board\",\"name\":\"Engineering α & β < >\",\"extra\":{\"first\":1,\"second\":2}}]"
    };
    let mut routes = vec![];
    for account in ["897362094", "other"] {
        routes.push(Route::new("GET", "www.example.com", &format!("/{account}/boards.json"), status).body(body));
    }
    routes.extend([
        Route::new("GET", "www.example.com", "/897362094/boards/board.json", 200).body("{\"id\":\"board\",\"name\":\"Engineering α & β < >\"}"),
        Route::new("GET", "www.example.com", "/897362094/boards/board/columns.json", 200).body("[{\"id\":\"col\",\"name\":\"Working\"}]"),
        Route::new("GET", "www.example.com", "/897362094/cards/579.json", 200).body("{\"number\":579,\"title\":\"α & β < >\"}"),
    ]);
    // The oracle records CGI.escape's exact target, including Ruby array/hash coercions.
    for call in case["calls"].as_array().unwrap() {
        let path = call["path"].as_str().unwrap();
        if path.starts_with("/897362094/search.json?") {
            routes.push(Route::new("GET", "www.example.com", path, 200).body("{\"number\":579,\"title\":\"α & β < >\"}"));
        }
    }
    let server = FakeServer::start_tls(routes).await;
    let resolver = Arc::new(FakeResolver::new([("www.example.com", vec!["93.184.216.34"])]));
    let dialer = Arc::new(MappingDialer {
        public: ["93.184.216.34".parse().unwrap()].into(),
        to: server.addr,
        dialed: Default::default(),
    });
    let clock = Arc::new(campfire_kit::FrozenClock::new("2026-03-02T16:00:00Z".parse().unwrap()));
    let app = TestApp::boot_with_fizzy(
        clock,
        State {
            network: network(resolver.clone(), dialer),
            base: "https://www.example.com".into(),
        },
    )
    .await
    .unwrap();
    initialize(&app).await;
    let crypto = app.booted.app.ar_encryption.clone();
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE id=?",
                rusqlite::params![if config["no_owner"] == true { None } else { Some(127326141) }, AGENT],
            )?;
            if config["no_grant"] != true {
                tx.conn().execute(
                    "INSERT INTO agent_grants(agent_id,capability,room_id,granted_by_id,created_at,updated_at) VALUES(?,'fizzy',?,127326141,?,?)",
                    rusqlite::params![AGENT, if config["room_grant"] == true { Some(486777696) } else { None }, tx.now(), tx.now()],
                )?;
            }
            Account::disconnect(tx, 127326141)?;
            if config["no_account"] != true {
                let account = Account::create(
                    tx,
                    &crypto,
                    &Input {
                        user_id: 127326141,
                        account_id: "897362094",
                        account_name: None,
                        fizzy_user_id: Some("owner-id"),
                        fizzy_user_name: None,
                        token: "fixture-owner",
                    },
                )?;
                if config["disconnected"] == true {
                    account.mark_disconnected(tx, "Disconnected")?;
                }
                if config["corrupt"] == true {
                    tx.conn()
                        .execute("UPDATE fizzy_connected_accounts SET access_token='not encrypted' WHERE id=?", [account.id])?;
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    let mut req = Req::new(
        Method::from_bytes(case["method"].as_str().unwrap().to_uppercase().as_bytes()).unwrap(),
        case["path"].as_str().unwrap(),
    )
    .header("accept", "application/json")
    .header("content-type", "application/json")
    .header("authorization", &["Bearer", SECRET].join(" "));
    if let Some(body) = case["body"].as_str() {
        req = req.body(body);
    }
    let reply = app.anonymous().send(req).await;
    let name = case["name"].as_str().unwrap();
    assert_eq!(reply.status.as_u16(), case["status"].as_u64().unwrap() as u16, "{name}: {}", reply.text());
    assert_eq!(reply.text(), case["response_body"].as_str().unwrap(), "{name}: raw response bytes");
    for header in ["Content-Type", "Cache-Control", "Pragma", "Retry-After", "Location"] {
        assert_eq!(
            reply.header(header),
            case["response_headers"][header.to_ascii_lowercase()].as_str(),
            "{name}: {header}"
        );
    }
    let received = server.received();
    assert_eq!(
        json!(received.iter().map(|r| json!({"method":r.method,"path":r.target})).collect::<Vec<_>>()),
        case["calls"],
        "{name}: outbound requests"
    );
    for r in &received {
        assert_eq!(r.header("authorization"), Some("Bearer fixture-owner"));
        assert_eq!(r.header("accept"), Some("application/json"));
        assert_eq!(r.header("user-agent"), Some("Smartfire-Fizzy"));
    }
    assert_eq!(resolver.lookups().len(), received.len(), "{name}: rejected input makes no DNS lookup");
    let reason = app
        .db()
        .read(|c| Ok(Account::for_user(c, 127326141)?.and_then(|a| a.disconnected_reason)))
        .await
        .unwrap();
    assert_eq!(json!(reason), case["reason"], "{name}: linked-account state");
    println!("WS11-api Fizzy wire case {name}: 1 passed; 0 failed");
}
#[tokio::test]
async fn fizzy_read_wire_successes() {
    let vectors: Value = serde_json::from_str(include_str!("../../../../vectors/agent_fizzy_http.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().take(8) {
        check(case).await;
    }
}
#[tokio::test]
async fn fizzy_read_wire_errors_and_coercions() {
    let vectors: Value = serde_json::from_str(include_str!("../../../../vectors/agent_fizzy_http.json")).unwrap();
    for case in vectors["cases"].as_array().unwrap().iter().skip(8) {
        check(case).await;
    }
}
