//! Legacy webhook secret callers; Agent secret rotation is still a missing WS11 seam.
use super::*;
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
            .get(&format!("/account/bots/{id}/edit"))
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
    assert!(
        admin
            .get(&format!("/account/bots/{id}/edit"))
            .await
            .text()
            .contains("Set a webhook URL before generating a signing secret.")
    );
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
async fn audit_failure_rolls_back_legacy_secret_rotation() {
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
    assert!(secret(&test, id).await.is_none());
}
