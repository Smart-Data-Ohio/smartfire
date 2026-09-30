//! Rails accounts/bots_controller_test.rb mutation cases through the seeded HTTP stack.
use super::*;

#[tokio::test]
async fn create_requires_administrator_and_sudo() {
    let test = boot_seed("default").await.expect("default seed");
    let mut member = test.browser("198.51.100.201");
    member.sign_in(&test.label("emails.kevin")).await;
    assert_eq!(
        member
            .form("post", "/account/bots", &[("user[name]", "Denied UI bot")])
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let mut admin = test.browser("198.51.100.202");
    admin.sign_in(&test.label("emails.david")).await;
    assert_redirect(
        &admin
            .form("post", "/account/bots", &[("user[name]", "Denied UI bot")])
            .await,
        "http://campfire.test/sudo/new",
    );
    assert_eq!(
        test.booted
            .app
            .db
            .read(|conn| Ok(conn.query_row(
                "SELECT COUNT(*) FROM users WHERE name='Denied UI bot'",
                [],
                |r| r.get::<_, i64>(0)
            )?))
            .await
            .unwrap(),
        0
    );
}

#[tokio::test]
async fn create_reveals_key_once_creates_workspace_agent_and_audits() {
    let test = boot_seed("default").await.expect("default seed");
    let mut admin = test.browser("198.51.100.203");
    admin.sign_in(&test.label("emails.david")).await;
    admin.grant_sudo_access();
    let response = admin
        .form("post", "/account/bots", &[("user[name]", "Reveal UI bot")])
        .await;
    assert_eq!(response.status, StatusCode::CREATED);
    assert_eq!(response.header("cache-control"), Some("no-store"));
    assert_eq!(response.header("pragma"), Some("no-cache"));
    let html = response.text();
    let key = html
        .split("aria-label=\"Bot key\"")
        .next()
        .unwrap()
        .rsplit("value=\"")
        .next()
        .unwrap()
        .split('"')
        .next()
        .unwrap()
        .to_string();
    let key_copy = key.clone();
    test.booted
        .app
        .db
        .read(move |conn| {
            let bot = campfire_db::User::authenticate_bot(conn, &key_copy)?
                .expect("new key authenticates");
            let token = key_copy.split_once('-').unwrap().1;
            assert_eq!(token.len(), 12);
            assert!(token.chars().all(|c| c.is_ascii_alphanumeric()));
            assert_eq!(
                bot.bot_token_digest.as_deref(),
                Some(campfire_db::models::user::digest_bot_token(token).as_str())
            );
            let agent = campfire_db::Agent::for_user(conn, bot.id)?.expect("workspace Agent");
            assert_eq!(agent.kind, campfire_db::AgentKind::Workspace);
            assert_eq!(agent.owner_id, Some(127326141));
            assert_eq!(
                conn.query_row("SELECT bot_token FROM users WHERE id=?", [bot.id], |r| {
                    r.get::<_, Option<String>>(0)
                })?,
                None
            );
            let audit: String = conn.query_row(
                "SELECT details FROM audit_logs WHERE action='agent.create' AND target_id=?",
                [agent.id],
                |r| r.get(0),
            )?;
            assert!(audit.contains("Reveal UI bot"));
            assert!(!audit.contains(&key_copy));
            Ok(())
        })
        .await
        .unwrap();
    assert!(!admin.get("/account/bots").await.text().contains(&key));
}

#[tokio::test]
async fn owner_updates_profile_without_sudo_and_keeps_webhook() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE user_id=?",
                [owner_id, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let original = test
        .booted
        .app
        .db
        .read(move |conn| campfire_db::User::find(conn, id)?.webhook_url(conn))
        .await
        .unwrap();
    let mut owner = test.browser("198.51.100.204");
    owner.sign_in(&test.label("emails.kevin")).await;
    let path = format!("/account/bots/{id}");
    let response = owner
        .form(
            "patch",
            &path,
            &[
                ("user[name]", "Owner edit"),
                ("agent[provider]", "Anthropic"),
                ("agent[runtime]", "Claude Code"),
                ("agent[description]", "Helps out"),
                ("agent[daily_message_cap]", "12"),
            ],
        )
        .await;
    assert_redirect(&response, "http://campfire.test/account/bots");
    test.booted
        .app
        .db
        .read(move |conn| {
            let bot = campfire_db::User::find(conn, id)?;
            assert_eq!(bot.name, "Owner edit");
            assert_eq!(bot.webhook_url(conn)?, original);
            let agent = campfire_db::Agent::for_user(conn, id)?.unwrap();
            assert_eq!(agent.provider.as_deref(), Some("Anthropic"));
            assert_eq!(agent.runtime.as_deref(), Some("Claude Code"));
            assert_eq!(agent.description.as_deref(), Some("Helps out"));
            assert_eq!(agent.daily_message_cap, Some(12));
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.update' AND target_id=?",
                    [agent.id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn owner_cannot_change_webhook_but_can_submit_unchanged() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE user_id=?",
                [owner_id, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let original = test
        .booted
        .app
        .db
        .read(move |conn| campfire_db::User::find(conn, id)?.webhook_url(conn))
        .await
        .unwrap()
        .unwrap();
    let mut owner = test.browser("198.51.100.205");
    owner.sign_in(&test.label("emails.kevin")).await;
    let path = format!("/account/bots/{id}");
    for url in ["https://attacker.example/hook", ""] {
        assert_eq!(
            owner
                .form(
                    "patch",
                    &path,
                    &[("user[name]", "Unchanged"), ("user[webhook_url]", url)]
                )
                .await
                .status,
            StatusCode::FORBIDDEN
        );
    }
    assert_redirect(
        &owner
            .form(
                "patch",
                &path,
                &[
                    ("user[name]", "Unchanged"),
                    ("user[webhook_url]", &original),
                ],
            )
            .await,
        "http://campfire.test/account/bots",
    );
    assert_eq!(
        test.booted
            .app
            .db
            .read(move |conn| campfire_db::User::find(conn, id)?.webhook_url(conn))
            .await
            .unwrap(),
        Some(original)
    );
}

#[tokio::test]
async fn only_changed_webhook_requires_sudo_and_audits_origin() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    let original = test
        .booted
        .app
        .db
        .read(move |conn| campfire_db::User::find(conn, id)?.webhook_url(conn))
        .await
        .unwrap()
        .unwrap();
    let mut admin = test.browser("198.51.100.206");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{id}");
    assert_redirect(
        &admin
            .form(
                "patch",
                &path,
                &[("user[name]", "Renamed"), ("user[webhook_url]", &original)],
            )
            .await,
        "http://campfire.test/account/bots",
    );
    assert_redirect(
        &admin
            .form(
                "patch",
                &path,
                &[(
                    "user[webhook_url]",
                    "https://example.com/new-hook?secret=fixture",
                )],
            )
            .await,
        "http://campfire.test/sudo/new",
    );
    assert_eq!(
        test.booted
            .app
            .db
            .read(move |conn| campfire_db::User::find(conn, id)?.webhook_url(conn))
            .await
            .unwrap(),
        Some(original)
    );
    admin.grant_sudo_access();
    assert_redirect(
        &admin
            .form(
                "patch",
                &path,
                &[(
                    "user[webhook_url]",
                    "https://example.com/new-hook?secret=fixture",
                )],
            )
            .await,
        "http://campfire.test/account/bots",
    );
    test.booted.app.db.read(move |conn| {
        let details: String = conn.query_row("SELECT details FROM audit_logs WHERE action='agent.webhook_url.change' ORDER BY id DESC LIMIT 1", [], |r| r.get(0))?;
        assert!(!details.contains("secret=fixture"));
        assert!(details.contains("example.com"));
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn invalid_agent_form_renders_422_without_saving_bot() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    let mut admin = test.browser("198.51.100.207");
    admin.sign_in(&test.label("emails.david")).await;
    let description = "x".repeat(501);
    let response = admin
        .form(
            "patch",
            &format!("/account/bots/{id}"),
            &[
                ("user[name]", "Should roll back"),
                ("agent[description]", &description),
            ],
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        response
            .text()
            .contains("Description is too long (maximum is 500 characters)")
    );
    assert!(response.text().contains("field_with_errors"));
    test.booted
        .app
        .db
        .read(move |conn| {
            assert_eq!(campfire_db::User::find(conn, id)?.name, "Bender Bot");
            assert_eq!(
                campfire_db::Agent::for_user(conn, id)?.unwrap().description,
                None
            );
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn legacy_edit_does_not_create_agent_or_remove_omitted_webhook() {
    let test = boot_seed("default").await.expect("default seed");
    let id = test
        .booted
        .app
        .db
        .write(|tx| {
            Ok(campfire_db::User::create_bot(
                tx,
                "Legacy mutation",
                Some("https://example.com/legacy"),
            )?
            .id)
        })
        .await
        .unwrap();
    let mut admin = test.browser("198.51.100.208");
    admin.sign_in(&test.label("emails.david")).await;
    let path = format!("/account/bots/{id}");
    assert_redirect(
        &admin
            .form(
                "patch",
                &path,
                &[
                    ("user[name]", "Legacy renamed"),
                    ("agent[provider]", "ignored"),
                ],
            )
            .await,
        "http://campfire.test/account/bots",
    );
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(campfire_db::Agent::for_user(conn, id)?.is_none());
            assert_eq!(
                campfire_db::User::find(conn, id)?
                    .webhook_url(conn)?
                    .as_deref(),
                Some("https://example.com/legacy")
            );
            Ok(())
        })
        .await
        .unwrap();
    admin.grant_sudo_access();
    assert_redirect(
        &admin
            .form("patch", &path, &[("user[webhook_url]", "")])
            .await,
        "http://campfire.test/account/bots",
    );
    assert!(
        test.booted
            .app
            .db
            .read(move |conn| campfire_db::User::find(conn, id)?.webhook_url(conn))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn delete_requires_admin_and_records_suspension() {
    let test = boot_seed("default").await.expect("default seed");
    let id: i64 = test.label("users.bender").parse().unwrap();
    let owner_id: i64 = test.label("users.kevin").parse().unwrap();
    test.booted
        .app
        .db
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE agents SET owner_id=? WHERE user_id=?",
                [owner_id, id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut owner = test.browser("198.51.100.209");
    owner.sign_in(&test.label("emails.kevin")).await;
    let path = format!("/account/bots/{id}");
    assert_eq!(
        owner.form("delete", &path, &[]).await.status,
        StatusCode::FORBIDDEN
    );
    let mut admin = test.browser("198.51.100.210");
    admin.sign_in(&test.label("emails.david")).await;
    assert_redirect(
        &admin.form("delete", &path, &[]).await,
        "http://campfire.test/account/bots",
    );
    test.booted
        .app
        .db
        .read(move |conn| {
            assert!(!campfire_db::User::find(conn, id)?.is_active());
            let agent = campfire_db::Agent::for_user(conn, id)?.unwrap();
            assert_eq!(
                conn.query_row(
                    "SELECT COUNT(*) FROM audit_logs WHERE action='agent.suspend' AND target_id=?",
                    [agent.id],
                    |r| r.get::<_, i64>(0)
                )?,
                1
            );
            Ok(())
        })
        .await
        .unwrap();
}
