use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        fizzy::{
            accounts::{Account, Input},
            cards::{Cache, Card},
        },
        test_support::{FakeServer, Route, ws15e_http_case, ws15e_http_case_listener},
    },
};
use axum::http::{Method, StatusCode};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::json;

#[tokio::test]
async fn ws15e_fizzy_connection_http_matrix() {
    if let Ok(case) = std::env::var("WS15E_FIZZY_CONNECTION_CASE") {
        return run(&case).await;
    }
    for case in [
        "link",
        "blank",
        "rejected",
        "empty",
        "transport",
        "relink",
        "disconnect",
        "caches",
        "sudo",
        "csrf",
    ] {
        let output = ws15e_http_case("WS15E_FIZZY_CONNECTION_CASE", case,
            "controllers::fizzy_connections::tests::ws15e_fizzy_connection_http_matrix").await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{case}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("1 passed; 0 failed"),
            "child must execute: {stdout}"
        );
        println!("Fizzy connections Rails case {case}: 1 passed; 0 failed");
    }
}
async fn run(case: &str) {
    let status = match case {
        "rejected" => 401,
        "transport" => 500,
        _ => 200,
    };
    let identity = if case == "empty" {
        json!({"accounts":[]})
    } else {
        json!({"accounts":[{"slug":"/897362094","name":"Smart Data","user":{"id":"03user1","name":"David"}}]})
    };
    let listener = ws15e_http_case_listener();
    let server = FakeServer::on_listener(
        vec![
            Route::new("GET", "127.0.0.1", "/my/identity.json", status).body(identity.to_string()),
        ],
        None,
        listener,
    )
    .await;
    let mut app = TestApp::boot().await.expect("pinned seeds required");
    app.booted
        .jobs
        .stop(std::time::Duration::from_secs(1))
        .await;
    app.db()
        .write(|tx| Account::disconnect(tx, DAVID))
        .await
        .unwrap();
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    if matches!(case, "relink" | "disconnect" | "caches") {
        let cipher = ArEncryption::new(&app.booted.app.secrets);
        app.db()
            .write(move |tx| {
                let a = Account::relink(
                    tx,
                    &cipher,
                    &Input {
                        user_id: DAVID,
                        account_id: "897362094",
                        account_name: Some("Old"),
                        fizzy_user_id: Some("old"),
                        fizzy_user_name: Some("Old"),
                        token: "old-token",
                    },
                )?;
                a.mark_disconnected(tx, "old disconnect")?;
                let card = Card::for_reference(tx, "897362094", 579)?;
                Cache::for_viewer(tx, &card, DAVID)?;
                Cache::for_viewer(tx, &card, JASON)?;
                Ok(())
            })
            .await
            .unwrap();
    }
    let mut browser = app.david();
    if case != "sudo" {
        browser.grant_sudo().await;
    }
    let token = if case == "blank" {
        "  "
    } else {
        "  new-token  "
    };
    let method = if matches!(case, "disconnect" | "caches") {
        Method::DELETE
    } else {
        Method::POST
    };
    let request = Req::new(method, "/fizzy/connection").form(&[("access_token", token)]);
    let response = if case == "csrf" {
        browser.send(request).await
    } else {
        browser.write(request).await
    };
    assert_eq!(
        response.status,
        if case == "csrf" {
            StatusCode::UNPROCESSABLE_ENTITY
        } else {
            StatusCode::FOUND
        }
    );
    if case == "sudo" {
        assert!(response.location().unwrap().ends_with("/sudo/new"));
    } else if case != "csrf" {
        assert!(response.location().unwrap().ends_with("/users/me/profile"));
    }
    let account = app
        .db()
        .read(|c| Account::for_user(c, DAVID))
        .await
        .unwrap();
    if matches!(case, "link" | "relink") {
        let account = account.unwrap();
        assert_eq!(account.account_id, "897362094");
        assert_eq!(account.fizzy_user_id.as_deref(), Some("03user1"));
        assert_eq!(account.account_name.as_deref(), Some("Smart Data"));
        assert!(account.connected());
        app.db()
            .write(move |tx| {
                assert_eq!(account.usable_token(tx, &crypto)?, Some("new-token".into()));
                Ok(())
            })
            .await
            .unwrap();
        let (details,cipher): (String,String) = app.db().read(|c| Ok((c.query_row("SELECT details FROM audit_logs WHERE action='fizzy.account.connect' ORDER BY id DESC LIMIT 1",[],|r|r.get(0))?, c.query_row("SELECT access_token FROM fizzy_connected_accounts WHERE user_id=?",[DAVID],|r|r.get(0))?))).await.unwrap();
        assert!(!details.contains("new-token"));
        assert!(!cipher.contains("new-token"));
        let flash = browser.flash().to_string();
        assert!(flash.contains("Fizzy connected as David (Smart Data)."));
    } else {
        assert!(account.is_none());
    }
    if matches!(case, "blank" | "sudo" | "csrf" | "disconnect" | "caches") {
        assert!(server.received.lock().unwrap().is_empty());
    }
    if case == "caches" {
        app.db()
            .read(|c| {
                assert_eq!(
                    c.query_row(
                        "SELECT COUNT(*) FROM fizzy_card_caches WHERE user_id=?",
                        [DAVID],
                        |r| r.get::<_, i64>(0)
                    )?,
                    0
                );
                assert_eq!(
                    c.query_row(
                        "SELECT COUNT(*) FROM fizzy_card_caches WHERE user_id=?",
                        [JASON],
                        |r| r.get::<_, i64>(0)
                    )?,
                    1
                );
                Ok(())
            })
            .await
            .unwrap();
    }
    if let Some(expected) = match case {
        "blank" => Some("Paste a token to connect Fizzy."),
        "rejected" => Some("Fizzy rejected that token. Check it and try again."),
        "empty" => Some("That token has no Fizzy account to use."),
        "transport" => Some("Could not reach Fizzy. Try again."),
        "disconnect" | "caches" => Some("Fizzy disconnected."),
        _ => None,
    } {
        assert!(
            browser.flash().to_string().contains(expected)
        );
    }
    if case == "link" {
        assert_eq!(
            server.received.lock().unwrap()[0]
                .headers
                .iter()
                .find(|(k, _)| k.eq_ignore_ascii_case("authorization"))
                .unwrap()
                .1,
            format!("Bearer {}", token.trim())
        );
    }
}
