//! Rails Accounts::Bots::GithubConnectionsControllerTest through the HTTP stack.
use super::*;
use crate::integrations::{
    github::accounts::{Account, AccountInput},
    net::{self, Network},
    test_support::{FakeResolver, FakeServer, MappingDialer, Route},
};
use std::sync::Arc;

async fn fixture(status: u16, body: &str) -> (Test, FakeServer) {
    let route = Route::new("GET", "api.github.com", "/user", status)
        .body(body.to_owned())
        .header("content-type", "application/json");
    let (server, roots) =
        FakeServer::start_named_tls_ws15e(vec![route], vec!["api.github.com".into()]).await;
    let resolver = Arc::new(FakeResolver::default());
    let ip: std::net::IpAddr = "93.184.216.34".parse().unwrap();
    resolver.set("api.github.com", vec![vec![ip]]);
    let dialer = Arc::new(MappingDialer {
        public: std::collections::HashSet::from([ip]),
        to: server.addr,
        dialed: std::sync::Mutex::new(vec![]),
    });
    let test = boot_seed_with_network(
        "default",
        seed_clock(),
        Network {
            resolver,
            dialer,
            tls: net::tls_config(roots),
        },
    )
    .await
    .expect("default seed");
    (test, server)
}
fn bot(test: &Test) -> i64 {
    test.label("users.bender").parse().unwrap()
}
fn path(test: &Test) -> String {
    format!("/account/bots/{}/github_connection", bot(test))
}
fn edit(test: &Test) -> String {
    format!("/account/bots/{}/edit", bot(test))
}
async fn admin(test: &Test) -> Browser<'_> {
    let mut browser = test.browser("198.51.100.175");
    browser
        .cookies
        .insert("session_token".into(), test.label("session_cookies.david"));
    browser.get(&edit(test)).await;
    browser.grant_sudo_access();
    browser
}
async fn link(test: &Test, token: &'static str) -> i64 {
    let user_id = bot(test);
    let crypto = test.booted.app.ar_encryption.clone();
    test.booted
        .app
        .db
        .write(move |tx| {
            Ok(Account::create(
                tx,
                &crypto,
                &AccountInput {
                    user_id,
                    github_login: "bender-machine",
                    access_token: token,
                    refresh_token: None,
                    token_expires_at: None,
                    token_source: "pat",
                },
            )?
            .id)
        })
        .await
        .unwrap()
}
async fn stored(
    test: &Test,
) -> Option<(
    i64,
    String,
    String,
    Option<String>,
    Option<String>,
    String,
    Option<String>,
    Option<String>,
)> {
    let user_id = bot(test);
    let crypto = test.booted.app.ar_encryption.clone();
    test.booted.app.db.read(move |conn| {
        use rusqlite::OptionalExtension;
        Ok(conn.query_row("SELECT id,github_login,access_token,disconnected_reason,last_error,token_source,refresh_token,token_expires_at FROM github_connected_accounts WHERE user_id=?", [user_id], |r| Ok((r.get(0)?,r.get(1)?,crypto.decrypt(&r.get::<_,String>(2)?).unwrap(),r.get(3)?,r.get(4)?,r.get(5)?,r.get(6)?,r.get(7)?))).optional()?)
    }).await.unwrap()
}
async fn audit_count(test: &Test, action: &'static str) -> i64 {
    test.booted
        .app
        .db
        .read(move |c| {
            Ok(c.query_row(
                "SELECT COUNT(*) FROM audit_logs WHERE action=?",
                [action],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn an_administrator_can_link_the_agents_account() {
    let (test, server) = fixture(200, r#"{"login":"bender-machine"}"#).await;
    let mut browser = admin(&test).await;
    assert_redirect(
        &browser
            .form(
                "post",
                &path(&test),
                &[("access_token", " \tagent-pat-pasted\n ")],
            )
            .await,
        &format!("http://{HOST}{}", edit(&test)),
    );
    assert!(
        browser
            .get(&edit(&test))
            .await
            .text()
            .contains("GitHub connected as bender-machine.")
    );
    let account = stored(&test).await.unwrap();
    assert_eq!(
        (
            &account.1,
            &account.2,
            account.3.as_deref(),
            account.5.as_str(),
            account.6.as_deref(),
            account.7.as_deref()
        ),
        (
            &"bender-machine".to_owned(),
            &"agent-pat-pasted".to_owned(),
            None,
            "pat",
            None,
            None
        )
    );
    assert!(
        test.booted
            .app
            .github_accounts
            .usable(account.0)
            .await
            .unwrap()
    );
    assert_eq!(audit_count(&test, "agent.github.connect").await, 1);
    let requests = server.received.lock().unwrap();
    assert_eq!(requests.len(), 1);
    assert_eq!(
        requests[0].header("authorization"),
        Some(format!("Bearer {}", "agent-pat-pasted").as_str())
    );
}
#[tokio::test]
async fn the_owner_without_admin_rights_can_neither_link_relink_nor_unlink() {
    let (test, server) = fixture(200, r#"{"login":"owner-machine"}"#).await;
    link(&test, "admin-linked").await;
    let id = bot(&test);
    let owner: i64 = test.label("users.kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE user_id=?",
                rusqlite::params![owner, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let before = stored(&test).await;
    let mut browser = test.browser("198.51.100.176");
    browser
        .cookies
        .insert("session_token".into(), test.label("session_cookies.kevin"));
    browser.get("/agents").await;
    for method in ["post", "delete"] {
        assert_eq!(
            browser
                .form(method, &path(&test), &[("access_token", "owner-pat")])
                .await
                .status,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(stored(&test).await, before);
    let page = browser.get(&edit(&test)).await;
    assert_eq!(page.status, StatusCode::OK);
    assert!(page.text().contains("Connected as bender-machine"));
    assert!(!page.text().contains(&format!("action=\"{}\"", path(&test))));
    assert!(server.received.lock().unwrap().is_empty());
}
#[tokio::test]
async fn another_member_gets_403_linking_and_unlinking() {
    let (test, server) = fixture(200, r#"{"login":"intruder"}"#).await;
    link(&test, "unchanged").await;
    let before = stored(&test).await;
    let mut browser = test.browser("198.51.100.177");
    browser
        .cookies
        .insert("session_token".into(), test.label("session_cookies.kevin"));
    browser.get("/agents").await;
    for id in [bot(&test), 0] {
        for method in ["post", "delete"] {
            assert_eq!(
                browser
                    .form(
                        method,
                        &format!("/account/bots/{id}/github_connection"),
                        &[("access_token", "intruder-pat")]
                    )
                    .await
                    .status,
                StatusCode::FORBIDDEN
            );
        }
    }
    assert_eq!(stored(&test).await, before);
    assert!(server.received.lock().unwrap().is_empty());
}
#[tokio::test]
async fn an_administrator_can_unlink_the_agents_account() {
    let (test, server) = fixture(200, "{}").await;
    link(&test, "unlink-token").await;
    let mut browser = admin(&test).await;
    for expected in [1, 1] {
        assert_redirect(
            &browser.form("delete", &path(&test), &[]).await,
            &format!("http://{HOST}{}", edit(&test)),
        );
        assert!(stored(&test).await.is_none());
        assert_eq!(
            audit_count(&test, "agent.github.disconnect").await,
            expected
        );
    }
    assert!(
        browser
            .get(&edit(&test))
            .await
            .text()
            .contains("GitHub disconnected.")
    );
    assert!(server.received.lock().unwrap().is_empty());
}
#[tokio::test]
async fn a_rejected_token_stores_nothing_and_shows_the_github_message() {
    let (test, server) = fixture(401, r#"{"message":"Bad credentials"}"#).await;
    let mut browser = admin(&test).await;
    assert_redirect(
        &browser
            .form("post", &path(&test), &[("access_token", "bogus")])
            .await,
        &format!("http://{HOST}{}", edit(&test)),
    );
    assert!(stored(&test).await.is_none());
    assert_eq!(audit_count(&test, "agent.github.connect").await, 0);
    assert!(
        browser
            .get(&edit(&test))
            .await
            .text()
            .contains("GitHub rejected that token. Check it and try again.")
    );
    assert_eq!(server.received.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn an_unreachable_github_shows_a_retry_message() {
    let (test, server) = fixture(503, "{}").await;
    let mut browser = admin(&test).await;
    assert_redirect(
        &browser
            .form("post", &path(&test), &[("access_token", "any")])
            .await,
        &format!("http://{HOST}{}", edit(&test)),
    );
    assert!(stored(&test).await.is_none());
    assert!(
        browser
            .get(&edit(&test))
            .await
            .text()
            .contains("Could not reach GitHub. Try again.")
    );
    assert_eq!(server.received.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn a_blank_token_is_rejected() {
    let (test, server) = fixture(200, "{}").await;
    let mut browser = admin(&test).await;
    assert_redirect(
        &browser
            .form("post", &path(&test), &[("access_token", " \t\n ")])
            .await,
        &format!("http://{HOST}{}", edit(&test)),
    );
    assert!(stored(&test).await.is_none());
    assert!(
        browser
            .get(&edit(&test))
            .await
            .text()
            .contains("Paste a token to connect GitHub.")
    );
    assert!(server.received.lock().unwrap().is_empty());
}
#[tokio::test]
async fn linking_again_after_a_disconnect_replaces_the_token_and_clears_the_reason() {
    let (test, server) = fixture(200, r#"{"login":"bender-machine"}"#).await;
    let id = link(&test, "old").await;
    test.booted.app.db.write(move |tx| { Account::mark_disconnected(tx,id,"GitHub rejected the linked token (401)")?; tx.conn().execute("UPDATE github_connected_accounts SET last_error='old error',token_source='app',refresh_token='old refresh',token_expires_at='2000-01-01 00:00:00' WHERE id=?",[id])?; Ok(()) }).await.unwrap();
    let mut browser = admin(&test).await;
    assert_redirect(
        &browser
            .form("post", &path(&test), &[("access_token", "fresh-token")])
            .await,
        &format!("http://{HOST}{}", edit(&test)),
    );
    let account = stored(&test).await.unwrap();
    assert_eq!(account.0, id);
    assert_eq!(account.2, "fresh-token");
    assert!(account.3.is_none());
    assert!(account.4.is_none());
    assert_eq!(account.5, "pat");
    assert!(account.6.is_none());
    assert!(account.7.is_none());
    assert_eq!(server.received.lock().unwrap().len(), 1);
}
#[tokio::test]
async fn the_bot_page_shows_the_login_without_ever_rendering_the_token() {
    let (test, _server) = fixture(200, "{}").await;
    let mut browser = admin(&test).await;
    let page = browser.get(&edit(&test)).await;
    page.assert_form(&path(&test));
    assert!(page.text().contains("name=\"access_token\""));
    link(&test, "super-secret-token").await;
    let page = browser.get(&edit(&test)).await;
    assert!(page.text().contains("Connected as bender-machine"));
    assert!(!page.text().contains("super-secret-token"));
    assert!(!page.text().contains("name=\"access_token\""));
}
#[tokio::test]
async fn deactivating_the_bot_disconnects_its_github_account_like_a_humans() {
    let (test, _server) = fixture(200, "{}").await;
    link(&test, "deactivate-token").await;
    let mut browser = admin(&test).await;
    assert_redirect(
        &browser
            .form("delete", &format!("/account/bots/{}", bot(&test)), &[])
            .await,
        &format!("http://{HOST}/account/bots"),
    );
    let account = stored(&test).await.unwrap();
    assert_eq!(account.3.as_deref(), Some("Account deactivated"));
    assert!(
        !test
            .booted
            .app
            .github_accounts
            .usable(account.0)
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn github_mutations_require_sudo_before_any_external_call_or_write() {
    let (test, server) = fixture(200, r#"{"login":"machine"}"#).await;
    link(&test, "unchanged").await;
    let before = stored(&test).await;
    let mut browser = test.browser("198.51.100.178");
    browser
        .cookies
        .insert("session_token".into(), test.label("session_cookies.david"));
    browser.get(&edit(&test)).await;
    for method in ["post", "delete"] {
        assert_redirect(
            &browser
                .form(method, &path(&test), &[("access_token", "pasted")])
                .await,
            &format!("http://{HOST}/sudo/new"),
        );
    }
    assert_eq!(stored(&test).await, before);
    assert!(server.received.lock().unwrap().is_empty());
}
#[tokio::test]
async fn github_audit_rejection_preserves_rails_committed_link_and_unlink() {
    let (test, _server) = fixture(200, r#"{"login":"machine"}"#).await;
    let mut browser = admin(&test).await;
    test.booted.app.db.write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_github_audit BEFORE INSERT ON audit_logs WHEN NEW.action LIKE 'agent.github.%' BEGIN SELECT RAISE(ABORT,'fixture audit rejection'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        browser
            .form("post", &path(&test), &[("access_token", "never-log-token")])
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        stored(&test).await.unwrap().2,
        "never-log-token",
        "Rails commits Account.save! before the audit insert"
    );
    assert_eq!(
        browser.form("delete", &path(&test), &[]).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert!(
        stored(&test).await.is_none(),
        "Rails commits Account.destroy! before the audit insert"
    );
    assert_eq!(audit_count(&test, "agent.github.connect").await, 0);
    assert_eq!(audit_count(&test, "agent.github.disconnect").await, 0);
}
