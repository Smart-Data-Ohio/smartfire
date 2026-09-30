use super::*;
use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        fizzy::accounts::{Input, UNREADABLE_TOKEN_REASON},
        test_support::{FakeResolver, FakeServer, MappingDialer, Route, network},
    },
};
use rusqlite::params;
use std::sync::Arc;
const OWNER_TOKEN: &str = "fixture-owner";
async fn fixture(case: &str) -> (TestApp, i64) {
    let mut app = TestApp::boot().await.expect("pinned seeds required");
    app.shutdown_jobs().await;
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    let case = case.to_owned();
    let agent=app.db().write(move|tx| {
        let agent:i64=tx.conn().query_row("SELECT id FROM agents WHERE user_id=?",[BENDER],|r|r.get(0))?;
        tx.conn().execute("UPDATE agents SET owner_id=?,suspended_at=NULL WHERE id=?",params![if case=="missing_owner" {None} else {Some(DAVID)},agent])?;
        tx.conn().execute("DELETE FROM agent_grants WHERE agent_id=? AND capability='fizzy'",[agent])?;
        if case!="forbidden" {tx.conn().execute("INSERT INTO agent_grants (agent_id,capability,granted_by_id,created_at,updated_at) VALUES (?,'fizzy',?,?,?)",params![agent,DAVID,tx.now(),tx.now()])?;}
        Account::disconnect(tx,DAVID)?;
        if case!="no_account" {
            let account=Account::relink(tx,&crypto,&Input{user_id:DAVID,account_id:"897362094",account_name:None,fizzy_user_id:Some("owner-id"),fizzy_user_name:None,token:OWNER_TOKEN})?;
            if case=="disconnected" {account.mark_disconnected(tx,"Disconnected")?;}
        }
        Ok(agent)
    }).await.unwrap();
    (app, agent)
}
#[tokio::test]
async fn ws15e_fizzy_agent_reads_match_pinned_service_results() {
    let vectors: Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/ws15e_fizzy_agent_reads.json"
    )))
    .unwrap();
    for case in vectors["reads"].as_array().unwrap() {
        let (app, agent) = fixture(case["name"].as_str().unwrap()).await;
        let name = case["name"].as_str().unwrap();
        let status = if let Some(status) = name.strip_prefix("status_") {
            status.parse().unwrap()
        } else {
            200
        };
        let server = FakeServer::start(vec![
            Route::new("GET", "app.fizzy.do", "/897362094/boards.json", status).body(
                if status >= 400 {
                    "{\"message\":\"Denied\"}"
                } else {
                    "[{\"id\":\"board\",\"name\":\"Engineering\"}]"
                },
            ),
            Route::new("GET", "app.fizzy.do", "/other/boards.json", 200)
                .body("[{\"id\":\"board\",\"name\":\"Engineering\"}]"),
            Route::new("GET", "app.fizzy.do", "/897362094/boards/board.json", 200)
                .body("{\"id\":\"board\",\"name\":\"Engineering\"}"),
            Route::new(
                "GET",
                "app.fizzy.do",
                "/897362094/boards/board/columns.json",
                200,
            )
            .body("[{\"id\":\"col\",\"name\":\"Working\"}]"),
            Route::new(
                "GET",
                "app.fizzy.do",
                "/897362094/search.json?q=+hi+~+%26+%E4%B8%AD%E6%96%87",
                200,
            )
            .body("{\"number\":579}"),
            Route::new("GET", "app.fizzy.do", "/897362094/cards/579.json", 200)
                .body("{\"number\":579}"),
        ])
        .await;
        let resolver = Arc::new(FakeResolver::new([("app.fizzy.do", vec!["93.184.216.34"])]));
        let dialer = Arc::new(MappingDialer {
            public: ["93.184.216.34".parse().unwrap()].into(),
            to: server.addr,
            dialed: Default::default(),
        });
        let fields = &case["fields"];
        let operation = match case["operation"].as_str().unwrap() {
            "boards" => Read::Boards {
                account: fields["account_id"].clone(),
            },
            "board" => Read::Board {
                account: fields["account_id"].clone(),
                board: fields["board_id"].clone(),
            },
            "search_cards" => Read::Search {
                account: fields["account_id"].clone(),
                query: fields["query"].clone(),
            },
            "card" => Read::Card {
                account: fields["account_id"].clone(),
                number: fields["number"].clone(),
            },
            _ => unreachable!(),
        };
        let result = read(
            &app.booted.app,
            &network(resolver.clone(), dialer),
            "http://app.fizzy.do",
            agent,
            operation,
        )
        .await
        .unwrap();
        assert_eq!(json!(result.status), case["status"], "{name}");
        assert_eq!(json!(result.payload), case["payload"], "{name}");
        assert_eq!(json!(result.error), case["error"], "{name}");
        assert_eq!(result.body(), case["body"], "{name}");
        {
            let received = server.received.lock().unwrap();
            let calls: Vec<_> = received
                .iter()
                .map(|r| json!({"method":r.method,"path":r.target}))
                .collect();
            assert_eq!(json!(calls), case["calls"], "{name}");
            for call in received.iter() {
                assert!(
                    call.headers
                        .iter()
                        .any(|(k, v)| k.eq_ignore_ascii_case("authorization")
                            && v == &format!("Bearer {OWNER_TOKEN}"))
                );
            }
            assert_eq!(resolver.lookups().len(), received.len());
        }
        let account = app
            .db()
            .read(|c| Account::for_user(c, DAVID))
            .await
            .unwrap();
        let reason = account.and_then(|a| a.disconnected_reason);
        assert_eq!(json!(reason), case["reason"], "{name}");
        println!("Fizzy agent read Rails service case {name}: 1 passed; 0 failed");
    }
}
#[tokio::test]
async fn ws15e_fizzy_agent_reads_reject_room_grants_suspension_and_corrupt_owner_token() {
    for case in ["room_grant", "suspended", "corrupt"] {
        let (app, agent) = fixture(case).await;
        app.db().write(move|tx| {match case {"room_grant"=>{tx.conn().execute("UPDATE agent_grants SET room_id=? WHERE agent_id=? AND capability='fizzy'",params![ALL_TALK,agent])?;},"suspended"=>{tx.conn().execute("UPDATE agents SET suspended_at=? WHERE id=?",params![tx.now(),agent])?;},"corrupt"=>{tx.conn().execute("UPDATE fizzy_connected_accounts SET access_token='not encrypted' WHERE user_id=?",[DAVID])?;},_=>unreachable!()}Ok(())}).await.unwrap();
        let resolver = Arc::new(FakeResolver::new([]));
        let dialer = Arc::new(MappingDialer {
            public: Default::default(),
            to: "127.0.0.1:51594".parse().unwrap(),
            dialed: Default::default(),
        });
        let result = read(
            &app.booted.app,
            &network(resolver.clone(), dialer),
            "http://app.fizzy.do",
            agent,
            Read::Boards {
                account: Value::Null,
            },
        )
        .await
        .unwrap();
        assert_eq!(result.status, if case == "corrupt" { 422 } else { 403 });
        assert!(resolver.lookups().is_empty());
        if case == "corrupt" {
            assert_eq!(
                app.db()
                    .read(|c| Account::for_user(c, DAVID))
                    .await
                    .unwrap()
                    .unwrap()
                    .disconnected_reason
                    .as_deref(),
                Some(UNREADABLE_TOKEN_REASON)
            );
        }
    }
}

#[tokio::test]
async fn ws15e_review_legacy_inactive_owner_cannot_read_fizzy() {
    for status in [1,2] {
        let (app,agent) = fixture("linked").await;
        app.db().write(move |tx| {tx.conn().execute("UPDATE users SET status=? WHERE id=?",params![status,DAVID])?;Ok(())}).await.unwrap();
        let server=FakeServer::start(vec![Route::new("GET","app.fizzy.do","/897362094/boards.json",200).body("[]")]).await;
        let resolver=Arc::new(FakeResolver::new([("app.fizzy.do",vec!["93.184.216.34"])]));
        let dialer=Arc::new(MappingDialer {public:["93.184.216.34".parse().unwrap()].into(),to:server.addr,dialed:Default::default()});
        let result=read(&app.booted.app,&network(resolver.clone(),dialer),"http://app.fizzy.do",agent,Read::Boards {account:json!(null)}).await.unwrap();
        assert_eq!(result.status,403,"unsafe legacy owner cannot authenticate agent reads");
        assert!(server.received().is_empty());
        assert!(resolver.lookups().is_empty());
    }
}
