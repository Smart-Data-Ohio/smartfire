//! `/api/v1/admin/bots` (S7) over the seeded app with `SPA_ENABLED`. As in the admin tests, each
//! write runs on two apps frozen at the same instant, once through the classic bot pages and once
//! through the API, and the whole database and every publication must come out the same.

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Value, json};

use super::admin_tests::{
    app, assert_parity, audits, classic, dump, error, get, nothing, parse, spa, write,
};
use crate::controllers::presenters::test_support::{Browser, DAVID, KEVIN, Req, TestApp};

/// Bender: a workspace agent David owns, with a webhook.
const BENDER: i64 = 394959859;
/// Deploy Bot: a legacy bot, no agent, no webhook.
const DEPLOY: i64 = 773523956;
/// Old Bot: deactivated.
const OLD: i64 = 773523957;

/// A classic write that answers a page (a new bot's key, a new credential's secret).
async fn classic_page(b: &mut Browser<'_>, method: Method, path: &str, fields: &[(&str, &str)]) {
    let reply = b.write(Req::new(method, path).form(fields)).await;
    assert!(
        reply.status.is_success(),
        "{path}: {}: {}",
        reply.status,
        reply.text()
    );
}

/// A classic page read that must succeed (the credential and grant pages give a legacy bot its
/// agent).
async fn classic_read(b: &mut Browser<'_>, path: &str) {
    let reply = b.send(Req::new(Method::GET, path)).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
}

/// Gives Deploy Bot a workspace agent Kevin (a member) owns.
async fn kevin_owns_deploy(a: &TestApp) {
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "INSERT INTO agents(user_id,owner_id,kind,created_at,updated_at) VALUES(?,?,'workspace',?,?)",
                rusqlite::params![DEPLOY, KEVIN, tx.now(), tx.now()],
            )?;
            Ok(())
        })
        .await
        .unwrap();
}

/// A room Bender is in, from its grants page (Bender has an agent, so reading it writes nothing).
async fn benders_room(_: &TestApp, b: &mut Browser<'_>) -> Value {
    let list: api::GrantList = parse(
        &b.send(get(&format!("/api/v1/admin/bots/{BENDER}/grants")))
            .await,
    );
    json!(list.rooms.first().expect("Bender is in a room").id)
}

async fn bender_has_a_credential(_: &TestApp, b: &mut Browser<'_>) -> Value {
    let created: api::CredentialCreated = spa(
        b,
        Method::POST,
        &format!("/api/v1/admin/bots/{BENDER}/credentials"),
        json!({"name": "ci", "expiresAt": null}),
    )
    .await;
    let newest = created
        .credentials
        .credentials
        .iter()
        .map(|each| each.id)
        .max();
    json!(newest.unwrap())
}

async fn bender_has_a_grant(_: &TestApp, b: &mut Browser<'_>) -> Value {
    let list: api::GrantList = spa(
        b,
        Method::POST,
        &format!("/api/v1/admin/bots/{BENDER}/grants"),
        json!({"capability": "react", "roomId": null}),
    )
    .await;
    json!(list.grants[0].id)
}

// --- Reads and gates ---------------------------------------------------------------------------

#[tokio::test]
async fn the_list_and_a_bot_read_as_the_classic_pages() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let list: api::BotList = parse(&david.send(get("/api/v1/admin/bots")).await);
    let names: Vec<_> = list.bots.iter().map(|bot| bot.name.as_str()).collect();
    assert_eq!(names, ["Bender Bot", "Deploy Bot"], "active bots, by name");
    let bender = &list.bots[0];
    assert_eq!(bender.ownership, "Workspace agent · Owned by David");
    assert!(!bender.rooms.is_empty());
    let room = &bender.rooms[0];
    assert_eq!(
        room.message_command,
        format!(
            "curl -d 'Hello!' http://campfire.test/rooms/{}/BOT_KEY/messages",
            room.id
        )
    );
    assert_eq!(list.bots[1].ownership, "no owner recorded");

    let bot: api::Bot = parse(
        &david
            .send(get(&format!("/api/v1/admin/bots/{BENDER}")))
            .await,
    );
    assert!(bot.can_administer);
    assert_eq!(
        bot.webhook_url.as_deref(),
        Some("http://example.com/bender")
    );
    let agent = bot.agent.expect("Bender has an agent");
    assert_eq!(agent.ledger_url, format!("/agents/{}/events", agent.id));
    assert!(!agent.suspended);

    let legacy: api::Bot = parse(
        &david
            .send(get(&format!("/api/v1/admin/bots/{DEPLOY}")))
            .await,
    );
    assert!(
        legacy.agent.is_none(),
        "reading a legacy bot gives it no agent"
    );
    let missing = david.send(get(&format!("/api/v1/admin/bots/{OLD}"))).await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn members_see_only_the_bots_they_own() {
    let Some(a) = app().await else { return };
    kevin_owns_deploy(&a).await;
    let mut kevin = a.sign_in(KEVIN).await;
    kevin.grant_sudo().await;
    let before = dump(&a).await;
    for (method, path, body) in [
        (Method::GET, "/api/v1/admin/bots".to_string(), Value::Null),
        (
            Method::POST,
            "/api/v1/admin/bots".to_string(),
            json!({"name": "Nope"}),
        ),
        (
            Method::GET,
            format!("/api/v1/admin/bots/{BENDER}"),
            Value::Null,
        ),
        (
            Method::PATCH,
            format!("/api/v1/admin/bots/{BENDER}"),
            json!({"name": "Mine"}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/bots/{DEPLOY}"),
            Value::Null,
        ),
        (
            Method::PUT,
            format!("/api/v1/admin/bots/{DEPLOY}/key"),
            Value::Null,
        ),
        (
            Method::PUT,
            format!("/api/v1/admin/bots/{DEPLOY}/github_connection"),
            json!({"accessToken": "x"}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{DEPLOY}/credentials"),
            json!({"name": "ci", "expiresAt": null}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{DEPLOY}/grants"),
            json!({"capability": "react", "roomId": null}),
        ),
        (
            Method::PATCH,
            format!("/api/v1/admin/bots/{DEPLOY}"),
            json!({"webhookUrl": "https://example.com/mine"}),
        ),
    ] {
        let reply = write(&mut kevin, method.clone(), &path, body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
        assert!(!reply.body.is_empty(), "{method} {path}: an empty 403");
        assert_eq!(error(&reply)["_tag"], "Forbidden", "{method} {path}");
    }
    assert_eq!(dump(&a).await, before, "nothing changed");

    // The owner opens and edits their own bot, and reads its credentials and grants.
    let bot: api::Bot = parse(
        &kevin
            .send(get(&format!("/api/v1/admin/bots/{DEPLOY}")))
            .await,
    );
    assert!(!bot.can_administer);
    let change: api::BotChange = spa(
        &mut kevin,
        Method::PATCH,
        &format!("/api/v1/admin/bots/{DEPLOY}"),
        json!({"name": "Kevin's Deploy Bot"}),
    )
    .await;
    assert_eq!(change.bot.name, "Kevin's Deploy Bot");
    let credentials: api::CredentialList = parse(
        &kevin
            .send(get(&format!("/api/v1/admin/bots/{DEPLOY}/credentials")))
            .await,
    );
    assert!(!credentials.can_issue);
    let grants: api::GrantList = parse(
        &kevin
            .send(get(&format!("/api/v1/admin/bots/{DEPLOY}/grants")))
            .await,
    );
    assert!(!grants.can_grant);
}

#[tokio::test]
async fn guarded_writes_change_nothing_without_the_password() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    for (method, path, body) in [
        (
            Method::POST,
            "/api/v1/admin/bots".to_string(),
            json!({"name": "Robo"}),
        ),
        (
            Method::PUT,
            format!("/api/v1/admin/bots/{BENDER}/key"),
            Value::Null,
        ),
        (
            Method::PATCH,
            format!("/api/v1/admin/bots/{BENDER}"),
            json!({"webhookUrl": "https://example.com/elsewhere"}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{BENDER}/webhook_secret"),
            Value::Null,
        ),
        (
            Method::PUT,
            format!("/api/v1/admin/bots/{BENDER}/github_connection"),
            json!({"accessToken": "x"}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/bots/{BENDER}/github_connection"),
            Value::Null,
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{BENDER}/credentials"),
            json!({"name": "ci", "expiresAt": null}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/bots/{BENDER}/grants"),
            json!({"capability": "react", "roomId": null}),
        ),
    ] {
        let reply = write(&mut david, method.clone(), &path, body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(error(&reply)["_tag"], "SudoRequired", "{method} {path}");
    }
    assert_eq!(dump(&a).await, before, "nothing changed");

    // The same webhook URL isn't a change, so it needs no password.
    let change: api::BotChange = spa(
        &mut david,
        Method::PATCH,
        &format!("/api/v1/admin/bots/{BENDER}"),
        json!({"name": "Bender", "webhookUrl": "http://example.com/bender"}),
    )
    .await;
    assert_eq!(change.bot.name, "Bender");
}

// --- Writes, against the classic pages ---------------------------------------------------------

#[tokio::test]
async fn a_new_bot_matches_the_classic_form() {
    let outcome = assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic_page(
                b,
                Method::POST,
                "/account/bots",
                &[
                    ("user[name]", "Robo"),
                    ("user[icon_name]", "robot"),
                    ("user[webhook_url]", "https://example.com/robo"),
                ],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let key: api::BotKey = spa(
                b,
                Method::POST,
                "/api/v1/admin/bots",
                json!({
                    "name": "Robo",
                    "iconName": "robot",
                    "webhookUrl": "https://example.com/robo",
                    "avatar": null
                }),
            )
            .await;
            assert_eq!(key.name, "Robo");
            assert!(key.key.starts_with(&format!("{}-", key.id)), "{}", key.key);
            assert!(key.example_command.contains("/rooms/ROOM_ID/"));
        },
    )
    .await;
    let Some(outcome) = outcome else { return };
    assert!(audits(&outcome).contains("agent.create"));
}

#[tokio::test]
async fn a_new_bot_without_a_name_does_what_the_classic_form_does() {
    let statuses = std::sync::Mutex::new(Vec::new());
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            let reply = b
                .write(Req::new(Method::POST, "/account/bots").form(&[("user[name]", "")]))
                .await;
            statuses.lock().unwrap().push(reply.status.is_success());
        },
        async |b: &mut Browser<'_>, _| {
            let reply = write(
                b,
                Method::POST,
                "/api/v1/admin/bots",
                json!({"name": "", "iconName": null, "webhookUrl": null, "avatar": null}),
            )
            .await;
            statuses
                .lock()
                .unwrap()
                .push(reply.status == StatusCode::OK);
        },
    )
    .await;
    let statuses = statuses.into_inner().unwrap();
    if let [classic, spa] = statuses[..] {
        assert_eq!(classic, spa, "both save, or neither does");
    }
}

#[tokio::test]
async fn an_edit_matches_the_classic_form() {
    let outcome = assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic(
                b,
                Method::PATCH,
                &format!("/account/bots/{BENDER}"),
                &[
                    ("user[name]", "Bender Bending Rodriguez"),
                    ("user[icon_name]", "robot"),
                    ("user[webhook_url]", "https://example.com/bender2"),
                    ("agent[provider]", "anthropic"),
                    ("agent[runtime]", "claude"),
                    ("agent[description]", "Bends things"),
                    ("agent[daily_message_cap]", "50"),
                    ("agent[daily_board_post_cap]", ""),
                    ("agent[daily_external_action_cap]", "5"),
                ],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let change: api::BotChange = spa(
                b,
                Method::PATCH,
                &format!("/api/v1/admin/bots/{BENDER}"),
                json!({
                    "name": "Bender Bending Rodriguez",
                    "iconName": "robot",
                    "webhookUrl": "https://example.com/bender2",
                    "avatar": null,
                    "agent": {
                        "provider": "anthropic",
                        "runtime": "claude",
                        "description": "Bends things",
                        "dailyMessageCap": "50",
                        "dailyBoardPostCap": "",
                        "dailyExternalActionCap": "5"
                    }
                }),
            )
            .await;
            let agent = change.bot.agent.unwrap();
            assert_eq!(agent.daily_message_cap, Some(50));
            assert_eq!(agent.daily_board_post_cap, None);
        },
    )
    .await;
    let Some(outcome) = outcome else { return };
    let audits = audits(&outcome);
    assert!(audits.contains("agent.webhook_url.change"), "{audits}");
    assert!(audits.contains("agent.update"), "{audits}");
}

#[tokio::test]
async fn a_bad_budget_is_refused_as_the_classic_form_refuses_it() {
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            let reply = b
                .write(
                    Req::new(Method::PATCH, &format!("/account/bots/{BENDER}")).form(&[
                        ("user[name]", "Bender Bot"),
                        ("agent[daily_message_cap]", "lots"),
                    ]),
                )
                .await;
            assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        },
        async |b: &mut Browser<'_>, _| {
            let reply = write(
                b,
                Method::PATCH,
                &format!("/api/v1/admin/bots/{BENDER}"),
                json!({"name": "Bender Bot", "agent": {"dailyMessageCap": "lots"}}),
            )
            .await;
            assert_eq!(
                reply.status,
                StatusCode::UNPROCESSABLE_ENTITY,
                "{}",
                reply.text()
            );
            let fields = &error(&reply)["fields"];
            assert!(fields["dailyMessageCap"].is_array(), "{fields}");
        },
    )
    .await;
}

#[tokio::test]
async fn removing_a_bot_matches_the_classic_page() {
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic(b, Method::DELETE, &format!("/account/bots/{BENDER}"), &[]).await;
        },
        async |b: &mut Browser<'_>, _| {
            let removed: api::BotRemoved = spa(
                b,
                Method::DELETE,
                &format!("/api/v1/admin/bots/{BENDER}"),
                Value::Null,
            )
            .await;
            assert_eq!(removed.id, BENDER);
        },
    )
    .await;
}

#[tokio::test]
async fn the_kill_switch_matches_the_classic_page() {
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic(
                b,
                Method::POST,
                &format!("/account/bots/{BENDER}/kill_switch"),
                &[],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let change: api::BotChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/bots/{BENDER}/kill_switch"),
                Value::Null,
            )
            .await;
            assert!(change.bot.agent.unwrap().suspended);
            let notice = change.notice.unwrap();
            assert!(notice.starts_with("Agent suspended; "), "{notice}");
        },
    )
    .await;

    // A legacy bot has no agent to suspend.
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    let reply = write(
        &mut david,
        Method::POST,
        &format!("/api/v1/admin/bots/{DEPLOY}/kill_switch"),
        Value::Null,
    )
    .await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a_new_key_matches_the_classic_page() {
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic_page(b, Method::PUT, &format!("/account/bots/{BENDER}/key"), &[]).await;
        },
        async |b: &mut Browser<'_>, _| {
            let key: api::BotKey = spa(
                b,
                Method::PUT,
                &format!("/api/v1/admin/bots/{BENDER}/key"),
                Value::Null,
            )
            .await;
            assert_eq!(key.id, BENDER);
        },
    )
    .await;
}

#[tokio::test]
async fn the_signing_secret_matches_the_classic_page() {
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic(
                b,
                Method::POST,
                &format!("/account/bots/{BENDER}/webhook_secret"),
                &[],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let change: api::BotChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/bots/{BENDER}/webhook_secret"),
                Value::Null,
            )
            .await;
            assert!(change.bot.signing_secret.is_some());
        },
    )
    .await;

    // A legacy bot with no webhook has nothing to sign: the classic alert, nothing written.
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    let before = dump(&a).await;
    let reply = write(
        &mut david,
        Method::POST,
        &format!("/api/v1/admin/bots/{DEPLOY}/webhook_secret"),
        Value::Null,
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&reply)["message"],
        "Set a webhook URL before generating a signing secret."
    );
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn a_blank_github_token_is_refused_and_a_disconnect_matches() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    let before = dump(&a).await;
    let reply = write(
        &mut david,
        Method::PUT,
        &format!("/api/v1/admin/bots/{BENDER}/github_connection"),
        json!({"accessToken": "  "}),
    )
    .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        error(&reply)["fields"]["accessToken"][0],
        "Paste a token to connect GitHub."
    );
    assert_eq!(dump(&a).await, before);

    // Nothing linked: both disconnect without a write.
    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic(
                b,
                Method::DELETE,
                &format!("/account/bots/{BENDER}/github_connection"),
                &[],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let change: api::BotChange = spa(
                b,
                Method::DELETE,
                &format!("/api/v1/admin/bots/{BENDER}/github_connection"),
                Value::Null,
            )
            .await;
            assert_eq!(change.notice.as_deref(), Some("GitHub disconnected."));
        },
    )
    .await;
}

// --- Credentials and grants --------------------------------------------------------------------

#[tokio::test]
async fn opening_a_legacy_bots_credentials_gives_it_an_agent_as_classic_does() {
    let outcome = assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic_read(b, &format!("/account/bots/{DEPLOY}/credentials")).await;
        },
        async |b: &mut Browser<'_>, _| {
            let list: api::CredentialList = parse(
                &b.send(get(&format!("/api/v1/admin/bots/{DEPLOY}/credentials")))
                    .await,
            );
            assert!(list.credentials.is_empty() && list.can_issue);
        },
    )
    .await;
    let Some(outcome) = outcome else { return };
    assert!(audits(&outcome).contains("agent.create"));

    assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic_read(b, &format!("/account/bots/{DEPLOY}/grants")).await;
        },
        async |b: &mut Browser<'_>, _| {
            let list: api::GrantList = parse(
                &b.send(get(&format!("/api/v1/admin/bots/{DEPLOY}/grants")))
                    .await,
            );
            assert!(list.legacy);
            assert_eq!(list.capabilities.len(), 7);
        },
    )
    .await;
}

#[tokio::test]
async fn issuing_and_revoking_credentials_match_the_classic_pages() {
    let outcome = assert_parity(
        nothing,
        async |b: &mut Browser<'_>, _| {
            classic_page(
                b,
                Method::POST,
                &format!("/account/bots/{BENDER}/credentials"),
                &[
                    ("agent_credential[name]", "ci"),
                    ("agent_credential[expires_at]", "2030-01-02T03:04"),
                ],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let created: api::CredentialCreated = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/bots/{BENDER}/credentials"),
                json!({"name": "ci", "expiresAt": "2030-01-02T03:04"}),
            )
            .await;
            assert!(!created.secret.is_empty());
            let credential = &created.credentials.credentials[0];
            assert_eq!(credential.state, api::CredentialState::Active);
            assert_eq!(credential.last_four.len(), 4);
        },
    )
    .await;
    let Some(outcome) = outcome else { return };
    assert!(audits(&outcome).contains("agent.credential.create"));

    assert_parity(
        bender_has_a_credential,
        async |b: &mut Browser<'_>, id: Value| {
            classic(
                b,
                Method::DELETE,
                &format!("/account/bots/{BENDER}/credentials/{id}"),
                &[],
            )
            .await;
        },
        async |b: &mut Browser<'_>, id: Value| {
            let list: api::CredentialList = spa(
                b,
                Method::DELETE,
                &format!("/api/v1/admin/bots/{BENDER}/credentials/{id}"),
                Value::Null,
            )
            .await;
            let revoked = list.credentials.iter().find(|each| json!(each.id) == id);
            assert_eq!(revoked.unwrap().state, api::CredentialState::Revoked);
        },
    )
    .await;
}

#[tokio::test]
async fn a_credential_without_a_name_is_refused() {
    let Some(a) = app().await else { return };
    let mut david = a.sign_in(DAVID).await;
    david.grant_sudo().await;
    // The page's first visit gives nothing here (Bender has an agent), so nothing may change.
    let before = dump(&a).await;
    let reply = write(
        &mut david,
        Method::POST,
        &format!("/api/v1/admin/bots/{BENDER}/credentials"),
        json!({"name": "", "expiresAt": null}),
    )
    .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    assert!(error(&reply)["fields"]["name"].is_array());
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn granting_and_revoking_match_the_classic_pages() {
    assert_parity(
        benders_room,
        async |b: &mut Browser<'_>, room: Value| {
            classic(
                b,
                Method::POST,
                &format!("/account/bots/{BENDER}/grants"),
                &[
                    ("agent_grant[capability]", "post_messages"),
                    ("agent_grant[room_id]", &room.to_string()),
                ],
            )
            .await;
        },
        async |b: &mut Browser<'_>, room: Value| {
            let list: api::GrantList = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/bots/{BENDER}/grants"),
                json!({"capability": "post_messages", "roomId": room}),
            )
            .await;
            assert_eq!(list.grants.len(), 1);
            assert!(!list.legacy);
        },
    )
    .await;

    // Granting what's already granted changes nothing.
    assert_parity(
        bender_has_a_grant,
        async |b: &mut Browser<'_>, _| {
            classic(
                b,
                Method::POST,
                &format!("/account/bots/{BENDER}/grants"),
                &[
                    ("agent_grant[capability]", "react"),
                    ("agent_grant[room_id]", ""),
                ],
            )
            .await;
        },
        async |b: &mut Browser<'_>, _| {
            let list: api::GrantList = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/bots/{BENDER}/grants"),
                json!({"capability": "react", "roomId": null}),
            )
            .await;
            assert_eq!(list.grants.len(), 1);
        },
    )
    .await;

    assert_parity(
        bender_has_a_grant,
        async |b: &mut Browser<'_>, id: Value| {
            classic(
                b,
                Method::DELETE,
                &format!("/account/bots/{BENDER}/grants/{id}"),
                &[],
            )
            .await;
        },
        async |b: &mut Browser<'_>, id: Value| {
            let list: api::GrantList = spa(
                b,
                Method::DELETE,
                &format!("/api/v1/admin/bots/{BENDER}/grants/{id}"),
                Value::Null,
            )
            .await;
            assert!(list.grants[0].revoked);
        },
    )
    .await;
}
