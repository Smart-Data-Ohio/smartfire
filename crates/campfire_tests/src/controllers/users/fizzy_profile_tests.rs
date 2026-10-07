use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use serde_json::Value;

async fn replay(name: &str) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_fizzy_profile.json"
    ))
    .unwrap();
    let row = vectors["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap()
        .clone();
    let setup = row.clone();
    let crypto = rails_compat::ar_encryption::ArEncryption::new(&app.booted.app.secrets);
    app.db()
        .write(move |tx| {
            use crate::integrations::fizzy::accounts::{Account, Input};
            Account::disconnect(tx, DAVID)?;
            if let Some(token) = setup["token"].as_str() {
                let account = Account::create(
                    tx,
                    &crypto,
                    &Input {
                        user_id: DAVID,
                        account_id: "fixture-account",
                        account_name: Some("Team <& Co"),
                        fizzy_user_id: None,
                        fizzy_user_name: Some("Person <& Co"),
                        token,
                    },
                )?;
                if setup["unreadable"] == true {
                    tx.conn().execute(
                        "UPDATE fizzy_connected_accounts SET access_token=? WHERE id=?",
                        rusqlite::params![token, account.id],
                    )?;
                }
                if let Some(reason) = setup["reason"].as_str() {
                    account.mark_disconnected(tx, reason)?;
                }
            }
            Ok(())
        })
        .await
        .unwrap();
    let panel = presenters::fizzy_profile::connection(&app.booted.app, DAVID)
        .await
        .unwrap();
    let html = super::people_tests::render_with(
        &app,
        |_| {},
        |ctx| {
            campfire_views::users::FizzyConnection { ctx, panel: &panel }
                .render()
                .unwrap()
        },
    );
    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/fizzy-{name}.actual"), &html).unwrap();
        std::fs::write(
            format!("{dir}/fizzy-{name}.expected"),
            row["html"].as_str().unwrap(),
        )
        .unwrap();
    }
    assert!(
        html == row["html"].as_str().unwrap(),
        "{name}: complete Fizzy fragment differs"
    );
    assert!(!html.contains("ws8br2-fizzy-fixture-token"));
    let reason = app
        .db()
        .read(|c| {
            Ok(
                crate::integrations::fizzy::accounts::Account::for_user(c, DAVID)?
                    .and_then(|a| a.disconnected_reason),
            )
        })
        .await
        .unwrap();
    assert_eq!(serde_json::json!(reason), row["reason_after"]);
    let response = app.david().get("/users/me/profile").await;
    assert_eq!(response.status, axum::http::StatusCode::OK);
    assert_eq!(
        response.text().contains("Connected as Person &lt;&amp; Co"),
        panel.connected()
    );
    assert!(!response.text().contains("ws8br2-fizzy-fixture-token"));
}
macro_rules! scenario {
    ($name:ident) => {
        #[tokio::test]
        async fn $name() {
            replay(stringify!($name)).await;
        }
    };
}
scenario!(missing);
scenario!(usable);
scenario!(blank);
scenario!(blank_reason);
scenario!(unreadable);
scenario!(rejected);
