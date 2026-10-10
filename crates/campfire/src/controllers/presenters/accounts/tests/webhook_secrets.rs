//! Rails agent and legacy webhook secret controllers through the seeded HTTP stack.
use super::*;

#[tokio::test]
async fn bot_edit_reads_ws11_profile_and_secret_without_creating_or_rotating_secrets() {
    // Rails Accounts::BotsController#edit uses Agent.secret || Webhook.secret:
    // an encrypted empty string hides the display and does not use the fallback.
    for (agent_value, webhook_value, displayed) in [
        (None, None, None),
        (
            None,
            Some("legacy display fixture"),
            Some("legacy display fixture"),
        ),
        (Some(""), Some("legacy display fixture"), None),
        (
            Some("agent display fixture"),
            Some("legacy display fixture"),
            Some("agent display fixture"),
        ),
    ] {
        let test = boot_seed("default").await.expect("default seed");
        let bot: i64 = test.label("users.bender").parse().unwrap();
        let crypto = test.booted.app.ar_encryption.clone();
        let before = test
            .booted
            .app
            .db
            .write(move |tx| {
                let agent = campfire_db::Agent::for_user(tx.conn(), bot)?.unwrap();
                let agent_cipher = agent_value.map(|value| crypto.encrypt(value));
                let webhook_cipher = webhook_value.map(|value| crypto.encrypt(value));
                tx.conn().execute(
                    "UPDATE agents SET webhook_signing_secret=? WHERE id=?",
                    rusqlite::params![agent_cipher, agent.id],
                )?;
                tx.conn().execute(
                    "UPDATE webhooks SET signing_secret=? WHERE user_id=?",
                    rusqlite::params![webhook_cipher, bot],
                )?;
                Ok((agent.id, agent_cipher, agent.updated_at))
            })
            .await
            .unwrap();
        let mut browser = test.browser("198.51.100.168");
        browser
            .cookies
            .insert("session_token".into(), test.label("session_cookies.david"));
        for _ in 0..2 {
            let page = browser.get(&format!("/api/v1/admin/bots/{bot}")).await;
            assert_eq!(page.status, StatusCode::OK);
            let html = page.text();

            if let Some(value) = displayed {
                assert!(html.contains(value));
            }
            if displayed != Some("legacy display fixture") {
                assert!(!html.contains("legacy display fixture"));
            }
        }
        let after = test
            .booted
            .app
            .db
            .read(move |conn| {
                let agent = campfire_db::Agent::for_user(conn, bot)?.unwrap();
                let cipher: Option<String> = conn.query_row(
                    "SELECT webhook_signing_secret FROM agents WHERE id=?",
                    [agent.id],
                    |r| r.get(0),
                )?;
                Ok((agent.id, cipher, agent.updated_at))
            })
            .await
            .unwrap();
        assert_eq!(
            after, before,
            "edit GET must not ensure or reset an agent secret"
        );
        assert_eq!(secret(&test, bot).await.as_deref(), webhook_value);
    }
}
async fn legacy(test: &Test, url: Option<&str>) -> i64 {
    let url = url.map(str::to_owned);
    test.booted
        .app
        .db
        .write(move |tx| {
            Ok(campfire_db::User::create_bot(tx, "Legacy signing", url.as_deref())?.id)
        })
        .await
        .unwrap()
}
async fn secret(test: &Test, id: i64) -> Option<String> {
    let crypto = test.booted.app.ar_encryption.clone();
    test.booted
        .app
        .db
        .read(move |conn| {
            campfire_db::Webhook::find_by_user(conn, id)?
                .map(|w| w.signing_secret(&crypto))
                .transpose()
                .map(Option::flatten)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn legacy_secret_rotation_requires_sudo_and_audits_without_plaintext() {
    let test = boot_seed("default").await.expect("default seed");
    let id = legacy(&test, Some("https://example.test/receiver")).await;
    let mut admin = test.browser("198.51.100.161");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{id}/webhook_secret");
    assert_redirect(
        &admin.form("post", &path, &[]).await,
        "http://campfire.test/sudo/new",
    );
    assert!(secret(&test, id).await.is_none());
    admin.grant_sudo_access();
    assert_redirect(
        &admin.form("post", &path, &[]).await,
        &format!("http://campfire.test/account/bots/{id}/edit"),
    );
    let first = secret(&test, id).await.unwrap();
    assert_eq!(first.len(), 64);
    assert_redirect(
        &admin.form("post", &path, &[]).await,
        &format!("http://campfire.test/account/bots/{id}/edit"),
    );
    let second = secret(&test, id).await.unwrap();
    assert_ne!(first, second);
    assert!(
        admin
            .get(&format!("/api/v1/admin/bots/{id}"))
            .await
            .text()
            .contains(&second)
    );
    test.booted.app.db.read(move |conn|{
        assert!(campfire_db::Agent::for_user(conn,id)?.is_none());
        let mut statement=conn.prepare("SELECT target_type,details FROM audit_logs WHERE action='agent.webhook_secret.reset' AND target_id=?")?;
        let records=statement.query_map([id],|r|Ok((r.get::<_,String>(0)?,r.get::<_,Option<String>>(1)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(records.len(),2);for (kind,details) in records {assert_eq!(kind,"User");let details=details.unwrap_or_default();assert!(!details.contains(&first));assert!(!details.contains(&second));}Ok(())
    }).await.unwrap();
}
#[tokio::test]
async fn legacy_missing_url_redirects_with_exact_alert_and_never_creates_agent() {
    let test = boot_seed("default").await.expect("default seed");
    let id = legacy(&test, None).await;
    let mut admin = test.browser("198.51.100.162");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    assert_redirect(
        &admin
            .form("post", &format!("/account/bots/{id}/webhook_secret"), &[])
            .await,
        &format!("http://campfire.test/account/bots/{id}/edit"),
    );
    let shell = admin.get("/app/").await;
    assert_eq!(shell.status, StatusCode::OK);
    assert!(shell.text().contains("Set a webhook URL before generating a signing secret."));
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(campfire_db::Agent::for_user(conn, id)?.is_none());
            assert!(campfire_db::Webhook::find_by_user(conn, id)?.is_none());
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn secret_rotation_finds_bot_then_rejects_nonowner_before_sudo() {
    let test = boot_seed("default").await.expect("default seed");
    let id = legacy(&test, Some("https://example.test/receiver")).await;
    let mut member = test.browser("198.51.100.163");
    member.sign_in(&test.label("emails.kevin")).await;
    for bot in [id, test.label("users.bender").parse().unwrap()] {
        assert_eq!(
            member
                .form("post", &format!("/account/bots/{bot}/webhook_secret"), &[])
                .await
                .status,
            StatusCode::FORBIDDEN
        );
    }
    assert_eq!(
        member
            .form("post", "/account/bots/0/webhook_secret", &[])
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert!(secret(&test, id).await.is_none());
}
#[tokio::test]
async fn audit_failure_preserves_legacy_secret_rotation() {
    let test = boot_seed("default").await.expect("default seed");
    let id = legacy(&test, Some("https://example.test/receiver")).await;
    test.booted.app.db.write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_secret_audit BEFORE INSERT ON audit_logs WHEN NEW.action='agent.webhook_secret.reset' BEGIN SELECT RAISE(ABORT,'test audit rejection'); END;")?;Ok(())}).await.unwrap();
    let mut admin = test.browser("198.51.100.164");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    assert_eq!(
        admin
            .form("post", &format!("/account/bots/{id}/webhook_secret"), &[])
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(secret(&test, id).await.unwrap().len(), 64);
}

async fn agent_secret(test: &Test, bot_id: i64) -> (i64, String, String, campfire_db::Timestamp) {
    let crypto = test.booted.app.ar_encryption.clone();
    test.booted
        .app
        .db
        .read(move |conn| {
            let agent = campfire_db::Agent::for_user(conn, bot_id)?.unwrap();
            let encrypted: String = conn.query_row(
                "SELECT webhook_signing_secret FROM agents WHERE id=?",
                [agent.id],
                |r| r.get(0),
            )?;
            let plaintext = crypto
                .decrypt(&encrypted)
                .map_err(|e| campfire_db::Error::Other(e.to_string()))?;
            Ok((agent.id, plaintext, encrypted, agent.updated_at))
        })
        .await
        .unwrap()
}

async fn initialize_agent_secret(test: &Test, bot_id: i64) {
    let crypto = test.booted.app.ar_encryption.clone();
    test.booted
        .app
        .db
        .write(move |tx| {
            let agent = campfire_db::Agent::for_user(tx.conn(), bot_id)?.unwrap();
            campfire_db::models::agent_access::ensure_webhook_signing_secret(
                tx, &crypto, agent.id,
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn agent_secret_rotation_calls_ws11_for_admin_and_owner_without_rotating_legacy_secret() {
    let test = boot_seed("default").await.expect("default seed");
    let bot_id: i64 = test.label("users.bender").parse().unwrap();
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    initialize_agent_secret(&test, bot_id).await;
    let crypto = test.booted.app.ar_encryption.clone();
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE user_id=?",
                [owner_id, bot_id],
            )?;
            campfire_db::Webhook::find_by_user(tx.conn(), bot_id)?
                .unwrap()
                .reset_signing_secret(tx, &crypto)?;
            Ok(())
        })
        .await
        .unwrap();
    let legacy_secret = secret(&test, bot_id).await.unwrap();
    let path = format!("/account/bots/{bot_id}/webhook_secret");
    let mut retired = Vec::new();
    for (viewer, ip) in [
        ("emails.david", "198.51.100.165"),
        ("emails.kevin", "198.51.100.166"),
    ] {
        let before = agent_secret(&test, bot_id).await;
        let mut browser = test.browser(ip);
        browser.sign_in(&test.label(viewer)).await;
        assert_redirect(
            &browser.form("post", &path, &[]).await,
            "http://campfire.test/sudo/new",
        );
        assert_eq!(agent_secret(&test, bot_id).await, before);
        browser.grant_sudo_access();
        assert_redirect(
            &browser.form("post", &path, &[]).await,
            &format!("http://campfire.test/account/bots/{bot_id}/edit"),
        );
        let after = agent_secret(&test, bot_id).await;
        assert_ne!(before.1, after.1);
        assert_ne!(before.2, after.2);
        assert_eq!(after.1.len(), 64);
        assert!(!after.2.contains(&after.1));
        retired.push(before.1);
        assert_eq!(
            secret(&test, bot_id).await.as_deref(),
            Some(legacy_secret.as_str())
        );
        let edit = browser.get(&format!("/api/v1/admin/bots/{bot_id}")).await;
        assert_eq!(edit.status, StatusCode::OK);
        assert!(!edit.text().contains(&legacy_secret));
    }
    let current = agent_secret(&test, bot_id).await;
    test.booted.app.db.read(move |conn| {
        let mut q=conn.prepare("SELECT target_type,target_id,details FROM audit_logs WHERE action='agent.webhook_secret.reset'")?;
        let logs=q.query_map([],|r|Ok((r.get::<_,String>(0)?,r.get::<_,i64>(1)?,r.get::<_,Option<String>>(2)?)))?.collect::<rusqlite::Result<Vec<_>>>()?;
        assert_eq!(logs.len(),2);
        for (kind,id,details) in logs {
            assert_eq!((kind.as_str(),id),("Agent",current.0));
            let details=details.unwrap_or_default();
            for value in retired.iter().chain([&current.1,&legacy_secret]) { assert!(!details.contains(value)); }
        }
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn agent_secret_rotation_rejects_nonowner_with_sudo_and_preserves_ciphertext() {
    let test = boot_seed("default").await.expect("default seed");
    let bot_id: i64 = test.label("users.bender").parse().unwrap();
    initialize_agent_secret(&test, bot_id).await;
    let before = agent_secret(&test, bot_id).await;
    let mut member = test.browser("198.51.100.167");
    member.sign_in(&test.label("emails.kevin")).await;
    member.grant_sudo_access();
    assert_eq!(
        member
            .form(
                "post",
                &format!("/account/bots/{bot_id}/webhook_secret"),
                &[]
            )
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(agent_secret(&test, bot_id).await, before);
    test.booted
        .app
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.webhook_secret.reset'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn agent_secret_write_failure_rolls_back_ciphertext_timestamp_and_audit() {
    let test = boot_seed("default").await.expect("default seed");
    let bot_id: i64 = test.label("users.bender").parse().unwrap();
    initialize_agent_secret(&test, bot_id).await;
    let before = agent_secret(&test, bot_id).await;
    test.booted.app.db.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_agent_secret_write BEFORE UPDATE OF webhook_signing_secret ON agents BEGIN SELECT RAISE(ABORT,'test secret write rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    let mut admin = test.browser("198.51.100.168");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    assert_eq!(
        admin
            .form(
                "post",
                &format!("/account/bots/{bot_id}/webhook_secret"),
                &[]
            )
            .await
            .status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(agent_secret(&test, bot_id).await, before);
    test.booted
        .app
        .db
        .read(|conn| {
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.webhook_secret.reset'",
                    [],
                    |r| r.get::<_, i64>(0)
                )?,
                0
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn bot_edit_calls_github_owner_usability_and_marks_unreadable_tokens_disconnected() {
    use crate::integrations::github::accounts::{Account, AccountInput, UNREADABLE_TOKEN_REASON};
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    let crypto = test.booted.app.ar_encryption.clone();
    let id = test.booted.app.db.write(move |tx| {
        let account = Account::create(tx, &crypto, &AccountInput {
            user_id: bot, github_login: "machine-read-fixture", access_token: "never-render-pat-fixture",
            refresh_token: None, token_expires_at: None, token_source: "pat",
        })?;
        tx.conn().execute("UPDATE github_connected_accounts SET access_token='unreadable-cipher-fixture',updated_at='2026-03-01 00:00:00.000000' WHERE id=?", [account.id])?;
        Ok(account.id)
    }).await.unwrap();
    let mut admin = test.browser("198.51.100.245");
    admin.sign_in(&test.label("emails.david")).await;
    let response = admin.get(&format!("/api/v1/admin/bots/{bot}")).await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains(UNREADABLE_TOKEN_REASON));
    for secret in ["never-render-pat-fixture", "unreadable-cipher-fixture"] {
        assert!(!response.text().contains(secret));
    }
    let stamp = test
        .booted
        .app
        .db
        .read(move |conn| {
            let account = Account::find(conn, id)?.unwrap();
            assert_eq!(
                account.disconnected_reason.as_deref(),
                Some(UNREADABLE_TOKEN_REASON)
            );
            assert!(account.updated_at.to_db().as_str() > "2026-03-01 00:00:00.000000");
            Ok(account.updated_at)
        })
        .await
        .unwrap();
    assert_eq!(
        admin.get(&format!("/api/v1/admin/bots/{bot}")).await.status,
        StatusCode::OK
    );
    test.booted
        .app
        .db
        .read(move |conn| {
            assert_eq!(Account::find(conn, id)?.unwrap().updated_at, stamp);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn audit_failure_preserves_key_and_agent_secret_rotations() {
    let test = boot_seed("default").await.expect("default seed");
    let bot: i64 = test.label("users.bender").parse().unwrap();
    initialize_agent_secret(&test, bot).await;
    let before = agent_secret(&test, bot).await;
    let old_key = test.label("bot_keys.bender");
    test.booted.app.db.write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_rotation_audit BEFORE INSERT ON audit_logs WHEN NEW.action IN ('agent.credential.reset','agent.webhook_secret.reset') BEGIN SELECT RAISE(ABORT,'audit unavailable'); END;")?;
        Ok(())
    }).await.unwrap();
    let mut admin = test.browser("198.51.100.170");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    for (method, action) in [("put", "key"), ("post", "webhook_secret")] {
        assert_eq!(
            admin
                .form(method, &format!("/account/bots/{bot}/{action}"), &[])
                .await
                .status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    test.booted.app.db.read(move |conn| {
        assert!(campfire_db::User::authenticate_bot(conn, &old_key)?.is_none());
        let (plain, digest): (Option<String>, Option<String>) = conn.query_row("SELECT bot_token,bot_token_digest FROM users WHERE id=?", [bot], |r| Ok((r.get(0)?, r.get(1)?)))?;
        assert!(plain.is_none());
        assert!(digest.is_some());
        assert_eq!(conn.query_row("SELECT COUNT(*) FROM audit_logs WHERE action IN ('agent.credential.reset','agent.webhook_secret.reset')", [], |r| r.get::<_, i64>(0))?, 0);
        Ok(())
    }).await.unwrap();
    let after = agent_secret(&test, bot).await;
    assert_eq!(after.0, before.0);
    assert_ne!(after.1, before.1);
    assert_ne!(after.2, before.2);
}
