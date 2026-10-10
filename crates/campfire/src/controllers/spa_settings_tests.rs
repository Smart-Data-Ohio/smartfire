//! `/api/v1/settings` (S7) over the seeded app with `SPA_ENABLED`. Each write is run twice, on
//! two apps frozen at the same instant: once through the classic form and once through the API.
//! The rows, audit entries and classic frames must come out the same.

use std::net::SocketAddr;
use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use futures_util::{SinkExt as _, StreamExt as _};
use serde_json::{Map, Value, json};
use tokio_tungstenite::tungstenite::Message as WsMessage;
use tokio_tungstenite::tungstenite::client::IntoClientRequest as _;

use crate::controllers::presenters::test_support::{
    Browser, DAVID, JASON, KEVIN, Reply, Req, SEED_NOW, TestApp,
};

/// David's password in the seed.
const PASSWORD: &str = "secret123456";

async fn app() -> Option<TestApp> {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    TestApp::boot_seed_with_env("default", clock, &[("SPA_ENABLED", "1")]).await
}

fn get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

fn json_body(method: Method, path: &str, body: &Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body.to_string())
}

fn parse<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("{error}: {}", reply.text()))
}

fn error(reply: &Reply) -> Value {
    let envelope: api::ApiErrorResponse = parse(reply);
    serde_json::to_value(&envelope.error).unwrap()
}

async fn read(b: &mut Browser<'_>) -> api::Settings {
    let reply = b.send(get("/api/v1/settings")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(&reply)
}

async fn write(b: &mut Browser<'_>, method: Method, path: &str, body: Value) -> Reply {
    b.write(json_body(method, path, &body)).await
}

/// Every key of a write body, `null` unless given.
fn update(keys: &[&str], given: Value) -> Value {
    let mut body: Map<String, Value> = keys
        .iter()
        .map(|key| (key.to_string(), Value::Null))
        .collect();
    for (key, value) in given.as_object().unwrap() {
        assert!(body.contains_key(key), "{key} isn't a key of this body");
        body.insert(key.clone(), value.clone());
    }
    Value::Object(body)
}

const PROFILE: &[&str] = &[
    "name",
    "emailAddress",
    "currentPassword",
    "password",
    "bio",
    "githubLogin",
];
const APPEARANCE: &[&str] = &["theme", "textSize", "timeZone"];
const CALLS: &[&str] = &["voiceMode", "pushToTalkKey"];
const NOTIFICATIONS: &[&str] = &[
    "dndEnabled",
    "quietHoursEnabled",
    "quietHoursStart",
    "quietHoursEnd",
    "meetingDndEnabled",
    "oooNotifyEnabled",
    "keywordAlerts",
    "inbox",
];
const STATUS: &[&str] = &[
    "presenceSetting",
    "customStatusEmoji",
    "customStatusText",
    "customStatusExpiresIn",
    "clearCustomStatus",
    "meetingStatusEnabled",
    "oooCalendarEnabled",
    "oooPreset",
    "oooUntilCustom",
    "oooNote",
    "clearOoo",
];

/// What a settings write can leave behind for David: his row (but the password digest, salted
/// afresh each time), his keyword alerts, DND exceptions, sessions and the audit log.
async fn snapshot(a: &TestApp) -> Value {
    a.db()
        .read(|conn| {
            let mut statement = conn.prepare("SELECT * FROM users WHERE id = ?")?;
            let names: Vec<String> = statement.column_names().into_iter().map(str::to_string).collect();
            let user = statement.query_row([DAVID], |row| {
                let mut columns = Map::new();
                for (index, name) in names.iter().enumerate() {
                    if name == "password_digest" {
                        continue;
                    }
                    let value: rusqlite::types::Value = row.get(index)?;
                    columns.insert(
                        name.clone(),
                        match value {
                            rusqlite::types::Value::Null => Value::Null,
                            rusqlite::types::Value::Integer(number) => json!(number),
                            rusqlite::types::Value::Real(number) => json!(number),
                            rusqlite::types::Value::Text(text) => json!(text),
                            rusqlite::types::Value::Blob(bytes) => json!(bytes),
                        },
                    );
                }
                Ok(Value::Object(columns))
            })?;
            let strings = |sql: &str| -> rusqlite::Result<Vec<String>> {
                conn.prepare(sql)?
                    .query_map([DAVID], |row| row.get::<_, String>(0))?
                    .collect()
            };
            Ok(json!({
                "user": user,
                "keywords": strings("SELECT phrase FROM keyword_alerts WHERE user_id = ? ORDER BY id")?,
                "allowed": strings("SELECT CAST(allowed_user_id AS TEXT) FROM dnd_allowed_users WHERE user_id = ? ORDER BY id")?,
                "sessions": strings("SELECT CAST(id AS TEXT) FROM sessions WHERE user_id = ? ORDER BY id")?,
                "audits": strings("SELECT json_array(action, details, ip_address, user_agent, actor_id, created_at) FROM audit_logs WHERE target_id = ? ORDER BY id")?,
            }))
        })
        .await
        .unwrap()
}

/// What one write left in David's persisted rows.
struct Outcome {
    rows: Value,
}

/// Runs `exercise` as David on a fresh app, recording what it leaves behind.
async fn outcome<F>(exercise: F) -> Option<Outcome>
where
    F: AsyncFnOnce(&mut Browser<'_>),
{
    let a = app().await?;
    let mut b = a.sign_in(DAVID).await;
    b.authenticity_token().await;
    a.publications().take();
    exercise(&mut b).await;
    Some(Outcome {
        rows: snapshot(&a).await,
    })
}

/// The classic form and the API write leave the same rows and publish the same frames.
async fn assert_parity<C, S>(classic: C, spa: S) -> Option<(Outcome, Outcome)>
where
    C: AsyncFnOnce(&mut Browser<'_>),
    S: AsyncFnOnce(&mut Browser<'_>),
{
    let classic = outcome(classic).await?;
    let spa = outcome(spa).await?;
    assert_eq!(spa.rows, classic.rows, "rows");
    Some((classic, spa))
}

fn form(path: &str, method: Method, fields: &[(&str, &str)]) -> Req {
    Req::new(method, path).form(fields)
}

async fn classic(b: &mut Browser<'_>, req: Req) {
    let reply = b.write(req).await;
    assert!(
        reply.status.is_redirection(),
        "{}: {}",
        reply.status,
        reply.text()
    );
}

async fn spa(b: &mut Browser<'_>, method: Method, path: &str, body: Value) -> api::Settings {
    let reply = write(b, method, path, body).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    parse(&reply)
}

#[tokio::test]
async fn settings_exist_without_flags() {
    let clock = crate::controllers::presenters::test_support::seed_clock();
    let Some(a) = TestApp::boot_seed_with_env("default", clock, &[]).await else {
        return;
    };
    let mut b = a.sign_in(DAVID).await;

    for path in [
        "/api/v1/settings",
        "/api/v1/settings/sessions",
        "/api/v1/settings/push_subscriptions",
    ] {
        assert_eq!(b.send(get(path)).await.status, StatusCode::OK, "{path}");
    }
}

#[tokio::test]
async fn the_settings_read_as_the_classic_profile_page_shows_them() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let reply = b.send(get("/api/v1/settings")).await;
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let settings: api::Settings = parse(&reply);

    assert_eq!(settings.profile.user_id, DAVID);
    assert!(settings.profile.has_password);
    assert!(!settings.profile.bot);
    let me: api::Me = parse(&b.send(get("/api/v1/me")).await);
    assert_eq!(settings.profile.avatar_url, me.user.avatar_url);
    let zones = &settings.appearance.time_zones;
    assert_eq!(
        zones.first().map(|zone| zone.value.as_str()),
        Some("Etc/GMT+12")
    );
    assert!(zones.iter().any(|zone| zone.value == "Europe/London"));
    assert_eq!(
        settings
            .notifications
            .inbox
            .iter()
            .map(|switch| switch.key.as_str())
            .collect::<Vec<_>>(),
        campfire_db::models::user::profile_settings::INBOX_KEYS
    );
    assert_eq!(settings.calls.push_to_talk_key.as_deref(), Some("`"));
    assert_eq!(settings.integrations.manage_path, "/users/me/profile");
    assert_eq!(settings.integrations.slack_import_path, "/slack/imports");


}

#[tokio::test]
async fn a_profile_change_saves_as_the_classic_form_does() {
    let email = "david-parity@example.test";
    let Some((_, spa_side)) = assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/profile",
                    Method::PATCH,
                    &[
                        ("user[name]", "David Parity"),
                        ("user[bio]", "Ships things"),
                        ("user[email_address]", email),
                        ("user[current_password]", PASSWORD),
                        ("user[password]", "a-new-password-1"),
                        ("user[github_login]", " DHH "),
                    ],
                ),
            )
            .await
        },
        async |b| {
            let settings = spa(
                b,
                Method::PATCH,
                "/api/v1/settings/profile",
                update(
                    PROFILE,
                    json!({
                        "name": "David Parity",
                        "bio": "Ships things",
                        "emailAddress": email,
                        "currentPassword": PASSWORD,
                        "password": "a-new-password-1",
                        "githubLogin": " DHH ",
                    }),
                ),
            )
            .await;
            assert_eq!(settings.profile.name, "David Parity");
            assert_eq!(settings.profile.email_address.as_deref(), Some(email));
            assert_eq!(settings.profile.github_login.as_deref(), Some("dhh"));
        },
    )
    .await
    else {
        return;
    };
    let audits = spa_side.rows["audits"].to_string();
    assert!(audits.contains("user.email.change"), "{audits}");
    assert!(audits.contains("user.password.change"), "{audits}");
}

#[tokio::test]
async fn an_email_change_needs_the_current_password() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let before = snapshot(&a).await;
    for (password, message) in [
        (Value::Null, "is required to change your email address"),
        (json!("wrong"), "is incorrect"),
    ] {
        let reply = write(
            &mut b,
            Method::PATCH,
            "/api/v1/settings/profile",
            update(
                PROFILE,
                json!({"emailAddress": "elsewhere@example.test", "currentPassword": password}),
            ),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        let error = error(&reply);
        assert_eq!(error["_tag"], "Validation");
        assert_eq!(error["fields"]["currentPassword"], json!([message]));
    }
    // The same address in another case isn't a change.
    let same = read(&mut b)
        .await
        .profile
        .email_address
        .unwrap()
        .to_uppercase();
    let reply = write(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/profile",
        update(PROFILE, json!({"emailAddress": same})),
    )
    .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(snapshot(&a).await["audits"], before["audits"]);
}

#[tokio::test]
async fn appearance_and_calls_save_as_the_classic_form_does() {
    assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/profile",
                    Method::PATCH,
                    &[
                        ("user[theme]", "dark"),
                        ("user[text_size]", "large"),
                        ("user[time_zone]", "Europe/London"),
                    ],
                ),
            )
            .await
        },
        async |b| {
            let settings = spa(
                b,
                Method::PATCH,
                "/api/v1/settings/appearance",
                update(
                    APPEARANCE,
                    json!({"theme": "dark", "textSize": "large", "timeZone": "Europe/London"}),
                ),
            )
            .await;
            assert_eq!(settings.appearance.theme, api::Theme::Dark);
            assert_eq!(settings.appearance.text_size, api::TextSize::Large);
            assert_eq!(
                settings.appearance.time_zone.as_deref(),
                Some("Europe/London")
            );
            // `GET /api/v1/me` carries them too.
            let me: api::Me = parse(&b.send(get("/api/v1/me")).await);
            assert_eq!(me.preferences.theme, api::Theme::Dark);
            assert!(me.preferences.time_zone_explicit);
        },
    )
    .await;
    assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/profile",
                    Method::PATCH,
                    &[
                        ("user[voice_mode]", "push_to_talk"),
                        ("user[push_to_talk_key]", " Space "),
                    ],
                ),
            )
            .await
        },
        async |b| {
            let settings = spa(
                b,
                Method::PATCH,
                "/api/v1/settings/calls",
                update(
                    CALLS,
                    json!({"voiceMode": "push_to_talk", "pushToTalkKey": " Space "}),
                ),
            )
            .await;
            assert_eq!(settings.calls.voice_mode, api::VoiceMode::PushToTalk);
            assert_eq!(settings.calls.push_to_talk_key.as_deref(), Some("Space"));
        },
    )
    .await;
    // Clearing the zone is "Not set (use system)".
    assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/profile",
                    Method::PATCH,
                    &[("user[time_zone]", "")],
                ),
            )
            .await
        },
        async |b| {
            let settings = spa(
                b,
                Method::PATCH,
                "/api/v1/settings/appearance",
                update(APPEARANCE, json!({"timeZone": ""})),
            )
            .await;
            assert_eq!(settings.appearance.time_zone, None);
        },
    )
    .await;
}

#[tokio::test]
async fn notifications_save_as_the_classic_forms_do() {
    assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/notification_settings",
                    Method::PATCH,
                    &[
                        ("user[dnd_enabled]", "1"),
                        ("user[quiet_hours_enabled]", "1"),
                        ("user[quiet_hours_start]", "22:00"),
                        ("user[quiet_hours_end]", "07:30"),
                        ("user[ooo_notify_enabled]", "0"),
                        ("user[keyword_alerts]", "deploy freeze\nLaunch day\nlaunch DAY"),
                    ],
                ),
            )
            .await;
            classic(
                b,
                form(
                    "/users/me/profile",
                    Method::PATCH,
                    &[
                        ("user[inbox_preferences][agent_work]", "0"),
                        ("user[inbox_preferences][event_reminders]", "1"),
                    ],
                ),
            )
            .await;
        },
        async |b| {
            let settings = spa(
                b,
                Method::PATCH,
                "/api/v1/settings/notifications",
                update(
                    NOTIFICATIONS,
                    json!({
                        "dndEnabled": true,
                        "quietHoursEnabled": true,
                        "quietHoursStart": "22:00",
                        "quietHoursEnd": "07:30",
                        "oooNotifyEnabled": false,
                        "keywordAlerts": ["deploy freeze", "Launch day", "launch DAY"],
                        "inbox": {"agent_work": false, "event_reminders": true, "no_such_switch": false},
                    }),
                ),
            )
            .await;
            let notifications = &settings.notifications;
            assert!(notifications.dnd_enabled && notifications.quiet_hours_enabled);
            assert_eq!(notifications.quiet_hours_start.as_deref(), Some("22:00"));
            assert_eq!(notifications.keyword_alerts, ["Launch day", "deploy freeze"]);
            let off: Vec<_> = notifications
                .inbox
                .iter()
                .filter(|switch| !switch.enabled)
                .map(|switch| switch.key.as_str())
                .collect();
            assert!(
                off.contains(&"agent_work") && !off.contains(&"event_reminders"),
                "{off:?}"
            );
        },
    )
    .await;
    // Turning DND off clears its timer, as the classic switch does.
    assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/notification_settings",
                    Method::PATCH,
                    &[("user[dnd_enabled]", "0"), ("user[keyword_alerts]", "")],
                ),
            )
            .await
        },
        async |b| {
            spa(
                b,
                Method::PATCH,
                "/api/v1/settings/notifications",
                update(
                    NOTIFICATIONS,
                    json!({"dndEnabled": false, "keywordAlerts": []}),
                ),
            )
            .await;
        },
    )
    .await;
}

#[tokio::test]
async fn a_status_change_saves_and_broadcasts_as_the_classic_form_does() {
    let Some((_classic_side, _)) = assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/status",
                    Method::PATCH,
                    &[
                        ("user[presence_setting]", "dnd"),
                        ("user[custom_status_emoji]", "🌴"),
                        ("user[custom_status_text]", "On a beach"),
                        ("user[custom_status_expires_in]", "hour_1"),
                        ("user[ooo_preset]", "tomorrow"),
                        ("user[ooo_note]", "Back Friday"),
                    ],
                ),
            )
            .await
        },
        async |b| {
            let settings = spa(
                b,
                Method::PATCH,
                "/api/v1/settings/status",
                update(
                    STATUS,
                    json!({
                        "presenceSetting": "dnd",
                        "customStatusEmoji": "🌴",
                        "customStatusText": "On a beach",
                        "customStatusExpiresIn": "hour_1",
                        "oooPreset": "tomorrow",
                        "oooNote": "Back Friday",
                    }),
                ),
            )
            .await;
            let status = &settings.status;
            assert_eq!(status.presence_setting, api::PresenceSetting::Dnd);
            assert_eq!(status.custom_status_text.as_deref(), Some("On a beach"));
            assert!(status.custom_status_expires_at.is_some());
            assert!(status.ooo_manual && status.ooo_until.is_some());
            assert_eq!(status.ooo_note.as_deref(), Some("Back Friday"));
        },
    )
    .await
    else {
        return;
    };
    // Clearing both, and a status-free change that announces nothing.
    assert_parity(
        async |b| {
            classic(
                b,
                form(
                    "/users/me/status",
                    Method::PATCH,
                    &[("user[clear_custom_status]", "1"), ("user[clear_ooo]", "1")],
                ),
            )
            .await;
            classic(
                b,
                form(
                    "/users/me/status",
                    Method::PATCH,
                    &[("user[meeting_status_enabled]", "0")],
                ),
            )
            .await;
        },
        async |b| {
            spa(
                b,
                Method::PATCH,
                "/api/v1/settings/status",
                update(STATUS, json!({"clearCustomStatus": true, "clearOoo": true})),
            )
            .await;
            spa(
                b,
                Method::PATCH,
                "/api/v1/settings/status",
                update(STATUS, json!({"meetingStatusEnabled": false})),
            )
            .await;
        },
    )
    .await;
}

#[tokio::test]
async fn an_out_of_office_end_must_be_ahead() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let before = snapshot(&a).await;
    for custom in [json!("2001-01-01T09:00"), json!("not a time"), Value::Null] {
        let reply = write(
            &mut b,
            Method::PATCH,
            "/api/v1/settings/status",
            update(
                STATUS,
                json!({"oooPreset": "custom", "oooUntilCustom": custom, "customStatusText": "kept?"}),
            ),
        )
        .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert_eq!(
            error(&reply)["fields"]["oooUntil"],
            json!(["needs a future date and time"])
        );
    }
    assert_eq!(snapshot(&a).await, before, "nothing saved");

    let long = write(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/status",
        update(STATUS, json!({"customStatusText": "x".repeat(101)})),
    )
    .await;
    assert_eq!(
        long.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        long.text()
    );
    assert!(
        error(&long)["fields"]["customStatusText"].is_array(),
        "{}",
        long.text()
    );
}

#[tokio::test]
async fn a_status_change_reaches_the_sync_socket() {
    let Some(a) = app().await else { return };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut jason = a.sign_in(JASON).await;
    let mut sync = Sync::connect(addr, &jason.cookie_header()).await;
    let _ = jason.authenticity_token().await;
    let mut b = a.sign_in(DAVID).await;
    spa(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/status",
        update(STATUS, json!({"customStatusText": "Heads down"})),
    )
    .await;
    let presence = sync
        .until(|payload| matches!(payload, api::SyncPayload::Presence(presence) if presence.user_id == DAVID))
        .await;
    let api::SyncPayload::Presence(presence) = presence else {
        unreachable!()
    };
    assert!(
        presence
            .status_text
            .as_deref()
            .is_some_and(|text| text.contains("Heads down")),
        "{presence:?}"
    );
    server.abort();
}

#[tokio::test]
async fn dnd_exceptions_are_active_people_other_than_you() {
    let Some((_, spa_side)) = assert_parity(
        async |b| {
            classic(
                b,
                Req::new(Method::POST, &format!("/users/{KEVIN}/dnd_allowance")),
            )
            .await
        },
        async |b| {
            let settings = spa(
                b,
                Method::POST,
                &format!("/api/v1/settings/dnd_allowances/{KEVIN}"),
                Value::Null,
            )
            .await;
            assert!(
                settings
                    .notifications
                    .allowed_people
                    .iter()
                    .any(|person| person.user_id == KEVIN)
            );
            // Twice is still once.
            spa(
                b,
                Method::POST,
                &format!("/api/v1/settings/dnd_allowances/{KEVIN}"),
                Value::Null,
            )
            .await;
        },
    )
    .await
    else {
        return;
    };
    assert_eq!(spa_side.rows["allowed"], json!([KEVIN.to_string()]));

    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let me = b
        .write(
            Req::new(
                Method::POST,
                &format!("/api/v1/settings/dnd_allowances/{DAVID}"),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(me.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", me.text());
    let nobody = b
        .write(
            Req::new(Method::POST, "/api/v1/settings/dnd_allowances/1")
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        (nobody.status, error(&nobody)["_tag"].clone()),
        (StatusCode::NOT_FOUND, json!("NotFound"))
    );
    spa(
        &mut b,
        Method::POST,
        &format!("/api/v1/settings/dnd_allowances/{KEVIN}"),
        Value::Null,
    )
    .await;
    let removed = spa(
        &mut b,
        Method::DELETE,
        &format!("/api/v1/settings/dnd_allowances/{KEVIN}"),
        Value::Null,
    )
    .await;
    assert!(
        removed
            .notifications
            .allowed_people
            .iter()
            .all(|person| person.user_id != KEVIN)
    );
}

#[tokio::test]
async fn sessions_list_and_revoke_as_the_classic_page_does() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let other = a.sign_in(DAVID).await;
    let third = a.sign_in(DAVID).await;
    drop((other, third));

    let reply = b.send(get("/api/v1/settings/sessions")).await;
    assert_eq!(reply.header("pragma"), Some("no-cache"));
    let list: api::SessionList = parse(&reply);
    assert_eq!(list.notice, None);
    assert_eq!(
        list.sessions
            .iter()
            .filter(|session| session.current)
            .count(),
        1
    );
    let others: Vec<i64> = list
        .sessions
        .iter()
        .filter(|session| !session.current)
        .map(|session| session.id)
        .collect();
    assert!(others.len() >= 2, "{list:?}");

    let revoked = b
        .write(
            Req::new(
                Method::DELETE,
                &format!("/api/v1/settings/sessions/{}", others[0]),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(revoked.status, StatusCode::OK, "{}", revoked.text());
    let list: api::SessionList = parse(&revoked);
    assert_eq!(list.notice.as_deref(), Some("Signed out that session."));
    assert!(list.sessions.iter().all(|session| session.id != others[0]));

    let missing = b
        .write(
            Req::new(
                Method::DELETE,
                &format!("/api/v1/settings/sessions/{}", others[0]),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(missing.status, StatusCode::NOT_FOUND);

    let rest = b
        .write(
            Req::new(Method::POST, "/api/v1/settings/sessions/revoke_others")
                .header("accept", "application/json"),
        )
        .await;
    let list: api::SessionList = parse(&rest);
    assert_eq!(list.sessions.len(), 1, "{list:?}");
    assert!(
        list.notice
            .as_deref()
            .is_some_and(|notice| notice.starts_with("Signed out ")),
        "{list:?}"
    );
    let none = b
        .write(
            Req::new(Method::POST, "/api/v1/settings/sessions/revoke_others")
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        parse::<api::SessionList>(&none).notice.as_deref(),
        Some("No other sessions to sign out.")
    );
    let audits = snapshot(&a).await["audits"].to_string();
    assert!(
        audits.contains("session.revoke\\\"") || audits.contains("session.revoke"),
        "{audits}"
    );
    assert!(audits.contains("session.revoke_others"), "{audits}");

    // Revoking this browser's own session signs it out.
    let current = list.sessions[0].id;
    let out = b
        .write(
            Req::new(
                Method::DELETE,
                &format!("/api/v1/settings/sessions/{current}"),
            )
            .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        (out.status, error(&out)["_tag"].clone()),
        (StatusCode::UNAUTHORIZED, json!("Unauthorized"))
    );
    assert_eq!(
        b.send(get("/api/v1/settings")).await.status,
        StatusCode::UNAUTHORIZED
    );
}

#[tokio::test]
async fn revoking_the_others_matches_the_classic_page() {
    assert_parity(
        async |b| {
            classic(
                b,
                Req::new(Method::DELETE, "/users/me/sessions/revoke_others"),
            )
            .await;
        },
        async |b| {
            let reply = b
                .write(
                    Req::new(Method::POST, "/api/v1/settings/sessions/revoke_others")
                        .header("accept", "application/json"),
                )
                .await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        },
    )
    .await;
}

#[tokio::test]
async fn push_subscriptions_are_your_own() {
    let Some(a) = app().await else { return };
    let ids = a
        .db()
        .write(|tx| {
            let mut ids = Vec::new();
            for (user, endpoint) in [(DAVID, "https://push.example/david"), (KEVIN, "https://push.example/kevin")] {
                tx.conn().execute(
                    "INSERT INTO push_subscriptions (user_id, endpoint, p256dh_key, auth_key, user_agent, created_at, updated_at) VALUES (?, ?, 'p256', 'auth', 'Mozilla/5.0 (X11; Linux x86_64; rv:141.0) Gecko/20100101 Firefox/141.0', ?, ?)",
                    rusqlite::params![user, endpoint, tx.now(), tx.now()],
                )?;
                ids.push(tx.conn().last_insert_rowid());
            }
            Ok(ids)
        })
        .await
        .unwrap();
    let mut b = a.sign_in(DAVID).await;
    let list: api::PushSubscriptionList =
        parse(&b.send(get("/api/v1/settings/push_subscriptions")).await);
    let mine = list
        .push_subscriptions
        .iter()
        .find(|subscription| subscription.id == ids[0])
        .expect("David's subscription");
    assert_eq!(mine.browser, "Firefox");
    assert!(
        list.push_subscriptions
            .iter()
            .all(|subscription| subscription.id != ids[1])
    );

    for id in [ids[1], ids[0]] {
        let reply = b
            .write(
                Req::new(
                    Method::DELETE,
                    &format!("/api/v1/settings/push_subscriptions/{id}"),
                )
                .header("accept", "application/json"),
            )
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    }
    let left: i64 = a
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM push_subscriptions WHERE id IN (?, ?)",
                [ids[0], ids[1]],
                |row| row.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(left, 1, "only Kevin's is left");
}

#[tokio::test]
async fn an_avatar_comes_from_a_direct_upload() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let forged = write(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": "forged"}),
    )
    .await;
    assert_eq!(
        forged.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        forged.text()
    );
    assert!(error(&forged)["fields"]["signedId"].is_array());

    let png = include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png");
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            png,
            campfire_storage::Filename::new("me.png"),
            Some("image/png"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    let signed_id =
        campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None);
    let attached = spa(
        &mut b,
        Method::PUT,
        "/api/v1/settings/avatar",
        json!({"signedId": signed_id}),
    )
    .await;
    assert!(attached.profile.avatar_attached);
    let removed = spa(
        &mut b,
        Method::DELETE,
        "/api/v1/settings/avatar",
        Value::Null,
    )
    .await;
    assert!(!removed.profile.avatar_attached);
}

#[tokio::test]
async fn writes_need_the_csrf_token_and_a_valid_body() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let forged = b
        .send(json_body(
            Method::PATCH,
            "/api/v1/settings/appearance",
            &update(APPEARANCE, json!({"theme": "dark"})),
        ))
        .await;
    assert_eq!(error(&forged)["_tag"], "InvalidAuthenticityToken");
    let wrong = write(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/appearance",
        json!({"theme": "sepia"}),
    )
    .await;
    assert_eq!(
        (wrong.status, error(&wrong)["_tag"].clone()),
        (StatusCode::UNPROCESSABLE_ENTITY, json!("Validation"))
    );
    let signed_out = a.anonymous().send(get("/api/v1/settings")).await;
    assert_eq!(signed_out.status, StatusCode::UNAUTHORIZED);
}

// --- The sync socket ---------------------------------------------------------------------------

struct Sync {
    socket: tokio_tungstenite::WebSocketStream<
        tokio_tungstenite::MaybeTlsStream<tokio::net::TcpStream>,
    >,
}

impl Sync {
    async fn connect(addr: SocketAddr, cookie: &str) -> Self {
        let mut request = format!("ws://{addr}/api/v1/sync")
            .into_client_request()
            .unwrap();
        let headers = request.headers_mut();
        headers.insert("cookie", cookie.parse().unwrap());
        headers.insert("host", "campfire.test".parse().unwrap());
        headers.insert("origin", "http://campfire.test".parse().unwrap());
        let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
        let mut sync = Self { socket };
        let hello = json!({"t": "hello", "v": 1, "resume": null, "topics": []});
        sync.socket
            .send(WsMessage::Text(hello.to_string().into()))
            .await
            .unwrap();
        assert!(matches!(
            sync.next().await,
            api::ServerFrame::Welcome { .. }
        ));
        sync
    }

    async fn next(&mut self) -> api::ServerFrame {
        loop {
            let message = tokio::time::timeout(Duration::from_secs(10), self.socket.next())
                .await
                .expect("a sync frame in time")
                .expect("an open socket")
                .unwrap();
            if let WsMessage::Text(text) = message {
                return serde_json::from_str(&text)
                    .unwrap_or_else(|error| panic!("{error}: {text}"));
            }
        }
    }

    async fn until(&mut self, wanted: impl Fn(&api::SyncPayload) -> bool) -> api::SyncPayload {
        loop {
            if let api::ServerFrame::Batch { events } = self.next().await
                && let Some(event) = events.into_iter().find(|event| wanted(&event.payload))
            {
                return event.payload;
            }
        }
    }
}
