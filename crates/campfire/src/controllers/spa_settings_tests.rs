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
    ALL_TALK, Browser, DAVID, JASON, KEVIN, Reply, Req, SEED_NOW, TestApp,
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
                    // SPA ordering metadata has its own revision tests below.
                    if name == "password_digest" || name == "activity_revision" {
                        continue;
                    }
                    let value: rusqlite::types::Value = row.get(index)?;
                    columns.insert(
                        name.clone(),
                        match value {
                            rusqlite::types::Value::Null => Value::Null,
                            rusqlite::types::Value::Integer(number) => json!(number),
                            rusqlite::types::Value::Real(number) => json!(number),
                            rusqlite::types::Value::Text(text) if name == "inbox_preferences" => {
                                let mut preferences: Value = serde_json::from_str(&text).unwrap();
                                if let Some(values) = preferences.as_object_mut() {
                                    values.remove("settings_revision");
                                }
                                json!(preferences.to_string())
                            }
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

/// A classic page of David's following his status badge and out-of-office notice streams, so
/// their publications are recorded.
async fn follow_status(
    a: &TestApp,
) -> (
    crate::channels::tests::support::Client,
    tokio::task::JoinHandle<()>,
) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = listener.local_addr().unwrap();
    let router = a.booted.app.cable.router::<()>("/cable");
    let serving = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    let headers = request.headers_mut();
    headers.insert("origin", format!("http://{address}").parse().unwrap());
    headers.insert(
        "sec-websocket-protocol",
        "actioncable-v1-json".parse().unwrap(),
    );
    headers.insert(
        "cookie",
        crate::controllers::presenters::test_support::david_cookie()
            .parse()
            .unwrap(),
    );
    let mut client = crate::channels::tests::support::Client {
        socket: tokio_tungstenite::connect_async(request).await.unwrap().0,
    };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let gid = campfire_app::cable::user_gid(DAVID).to_param();
    for stream in ["status", "ooo_notice"] {
        let signed =
            rails_compat::turbo::signed_stream_name(&a.booted.app.secrets, &[&gid, stream]);
        let identifier = crate::channels::tests::support::identifier(
            json!({"channel": "Turbo::StreamsChannel", "signed_stream_name": signed}),
        );
        client.confirm(&identifier).await;
    }
    (client, serving)
}

/// The frames published until they stop coming (some go out after the response).
async fn settle(a: &TestApp) -> Vec<(String, String)> {
    let capture = a.publications();
    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 10 {
        tokio::time::sleep(Duration::from_millis(50)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    frames
}

/// What one write did: David's rows afterwards and the classic frames it published.
struct Outcome {
    rows: Value,
    frames: Vec<(String, String)>,
}

/// Runs `exercise` as David on a fresh app, recording what it leaves behind.
async fn outcome<F>(exercise: F) -> Option<Outcome>
where
    F: AsyncFnOnce(&mut Browser<'_>),
{
    let a = app().await?;
    let (_client, cable) = follow_status(&a).await;
    let mut b = a.sign_in(DAVID).await;
    b.authenticity_token().await;
    a.publications().take();
    exercise(&mut b).await;
    let frames = settle(&a).await;
    cable.abort();
    Some(Outcome {
        rows: snapshot(&a).await,
        frames,
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
    assert_eq!(spa.frames, classic.frames, "frames");
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
async fn settings_exist_only_with_the_spa() {
    let clock = crate::controllers::presenters::test_support::seed_clock();
    let Some(a) = TestApp::boot_seed_with_env("default", clock, &[]).await else {
        return;
    };
    let mut b = a.sign_in(DAVID).await;
    let unknown = b.send(get("/no-such-page")).await.status;
    for path in [
        "/api/v1/settings",
        "/api/v1/settings/sessions",
        "/api/v1/settings/push_subscriptions",
    ] {
        assert_eq!(b.send(get(path)).await.status, unknown, "{path}");
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

    // The classic page agrees on what it can show.
    let page = b.classic_page("/users/me/profile").await.text();
    assert!(page.contains(&settings.profile.name), "the name");
    for switch in &settings.notifications.inbox {
        assert!(page.contains(&switch.label), "{}", switch.label);
    }
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
    let Some((classic_side, _)) = assert_parity(
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
    assert!(
        classic_side
            .frames
            .iter()
            .any(|(stream, frame)| stream.ends_with(":status") && frame.contains("Back Friday")),
        "the status badge: {:?}",
        classic_side.frames
    );
    assert!(
        classic_side
            .frames
            .iter()
            .any(|(stream, _)| stream.ends_with(":ooo_notice")),
        "the out-of-office notice: {:?}",
        classic_side.frames
    );

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
        self.until_event(wanted).await.payload
    }

    async fn until_event(&mut self, wanted: impl Fn(&api::SyncPayload) -> bool) -> api::SyncEvent {
        loop {
            if let api::ServerFrame::Batch { events } = self.next().await
                && let Some(event) = events.into_iter().find(|event| wanted(&event.payload))
            {
                return event;
            }
        }
    }
}


#[tokio::test]
async fn a9_notification_writes_reach_all_of_the_users_sync_sessions() {
    let Some(a) = app().await else {
        panic!("restored default seed required")
    };
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let router = a.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut phone = a.sign_in(DAVID).await;
    let desktop = a.sign_in(DAVID).await;
    let other_user = a.sign_in(JASON).await;
    let mut phone_sync = Sync::connect(addr, &phone.cookie_header()).await;
    let mut desktop_sync = Sync::connect(addr, &desktop.cookie_header()).await;
    let mut other_sync = Sync::connect(addr, &other_user.cookie_header()).await;
    let room_id = a
        .db()
        .read(|conn| Ok(campfire_db::Membership::for_user(conn, DAVID)?[0].room_id))
        .await
        .unwrap();
    let mut previous = read(&mut phone).await.revision;

    for body in [
        json!({"defaultNotificationLevel":"mentions"}),
        json!({"roomNotification":{"roomId":room_id,"level":null}}),
        json!({"roomMute":{"roomId":room_id,"duration":"minutes15"}}),
        json!({"roomMute":{"roomId":room_id,"duration":"off"}}),
    ] {
        let response = write(
            &mut phone,
            Method::PATCH,
            "/api/v1/settings/notifications",
            body,
        )
        .await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let saved: api::Settings = parse(&response);
        assert!(saved.revision > previous);
        previous = saved.revision;
        for sync in [&mut phone_sync, &mut desktop_sync] {
            let event = sync
                .until_event(|payload| matches!(payload, api::SyncPayload::SettingsUpdated(_)))
                .await;
            assert_eq!(event.topic, "user");
            let api::SyncPayload::SettingsUpdated(snapshot) = event.payload else {
                unreachable!()
            };
            assert_eq!(*snapshot, saved);
        }
    }
    let response = write(
        &mut phone,
        Method::PUT,
        &format!("/api/v1/rooms/{room_id}/involvement"),
        json!({"involvement":"everything"}),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let saved: api::InvolvementChange = parse(&response);
    assert!(saved.settings.revision > previous);
    assert!(
        !saved
            .settings
            .notifications
            .room_notification_levels
            .contains_key(&room_id.to_string())
    );
    for sync in [&mut phone_sync, &mut desktop_sync] {
        let event = sync
            .until_event(|payload| matches!(payload, api::SyncPayload::SettingsUpdated(_)))
            .await;
        assert_eq!(event.topic, "user");
        let api::SyncPayload::SettingsUpdated(snapshot) = event.payload else {
            unreachable!()
        };
        assert_eq!(*snapshot, saved.settings);
    }

    // A public presence update fences all the writes above on the other user's socket.
    let response = write(
        &mut phone,
        Method::PATCH,
        "/api/v1/settings/status",
        json!({"customStatusText":"Notification sync complete"}),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    loop {
        if let api::ServerFrame::Batch { events } = other_sync.next().await {
            assert!(
                events
                    .iter()
                    .all(|event| !matches!(event.payload, api::SyncPayload::SettingsUpdated(_)))
            );
            if events.iter().any(|event| matches!(&event.payload,
                api::SyncPayload::Presence(presence) if presence.user_id == DAVID
                    && presence.status_text.as_deref().is_some_and(|text| text.contains("Notification sync complete"))
            )) {
                break;
            }
        }
    }
    server.abort();
}

#[tokio::test]
async fn a9_notification_default_mute_and_membership_scope() {
    let Some(app) = app().await else {
        panic!("restored default seed required")
    };
    let mut b = app.sign_in(DAVID).await;
    let rooms = app
        .booted
        .app
        .db
        .read(|conn| campfire_db::Membership::for_user(conn, DAVID))
        .await
        .unwrap();
    let room_id = rooms[0].room_id;
    let saved = write(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/notifications",
        json!({
            "defaultNotificationLevel":"mentions",
            "roomNotification":{"roomId":room_id,"level":null},
            "roomMute":{"roomId":room_id,"duration":"minutes15"}
        }),
    )
    .await;
    assert_eq!(saved.status, StatusCode::OK, "{}", saved.text());
    let saved: api::Settings = parse(&saved);
    assert_eq!(
        saved.notifications.default_notification_level,
        api::NotificationLevel::Mentions
    );
    assert_eq!(
        saved
            .notifications
            .room_notification_levels
            .get(&room_id.to_string()),
        Some(&None)
    );
    let until: jiff::Timestamp = saved.notifications.room_mute_until[&room_id.to_string()]
        .as_ref()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        until,
        SEED_NOW.parse::<jiff::Timestamp>().unwrap() + jiff::SignedDuration::from_secs(15 * 60)
    );
    let sidebar = b.send(get("/api/v1/sidebar")).await;
    assert_eq!(sidebar.status, StatusCode::OK, "{}", sidebar.text());
    let sidebar: api::Sidebar = parse(&sidebar);
    let row = sidebar
        .rows
        .iter()
        .find(|row| row.room.id == room_id)
        .expect("muted room remains in sidebar");
    assert_eq!(row.notification_count, 0);
    assert_eq!(row.thread_notification_count, 0);
    let badge = b.send(get("/api/v1/activity/unread_count")).await;
    assert_eq!(badge.status, StatusCode::OK, "{}", badge.text());
    let unmuted = write(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/notifications",
        json!({"roomMute":{"roomId":room_id,"duration":"off"}}),
    )
    .await;
    assert_eq!(unmuted.status, StatusCode::OK);
    assert!(
        parse::<api::Settings>(&unmuted)
            .notifications
            .room_mute_until
            .is_empty()
    );
    let denied = write(
        &mut b,
        Method::PATCH,
        "/api/v1/settings/notifications",
        json!({"roomMute":{"roomId":i64::MAX,"duration":"forever"}}),
    )
    .await;
    assert_eq!(denied.status, StatusCode::NOT_FOUND);
}

#[tokio::test]
async fn a9_involvement_returns_the_settings_revision_and_cleared_override() {
    let Some(app) = app().await else {
        panic!("restored default seed required")
    };
    let mut b = app.sign_in(DAVID).await;
    let room_id = app
        .booted
        .app
        .db
        .read(|conn| Ok(campfire_db::Membership::for_user(conn, DAVID)?[0].room_id))
        .await
        .unwrap();
    for involvement in ["everything", "mentions", "nothing", "muted", "invisible"] {
        let inherited = write(
            &mut b,
            Method::PATCH,
            "/api/v1/settings/notifications",
            json!({
                "defaultNotificationLevel": "nothing",
                "roomNotification": { "roomId": room_id, "level": null },
                "roomMute": { "roomId": room_id, "duration": "minutes15" }
            }),
        )
        .await;
        let before: api::Settings = parse(&inherited);
        let response = write(
            &mut b,
            Method::PUT,
            &format!("/api/v1/rooms/{room_id}/involvement"),
            json!({ "involvement": involvement }),
        )
        .await;
        assert_eq!(response.status, StatusCode::OK, "{}", response.text());
        let response: Value = parse(&response);
        let saved: api::Settings = serde_json::from_value(response["settings"].clone())
            .expect("involvement response must carry the new settings snapshot");
        assert!(saved.revision > before.revision);
        assert_eq!(response["membership"]["involvement"], involvement);
        assert!(
            !saved
                .notifications
                .room_notification_levels
                .contains_key(&room_id.to_string())
        );
        assert_eq!(
            saved.notifications.default_notification_level,
            api::NotificationLevel::Nothing
        );
        assert_eq!(
            saved.notifications.room_mute_until,
            before.notifications.room_mute_until
        );
        assert_eq!(saved, read(&mut b).await);
    }
}

#[tokio::test]
async fn a9_every_settings_write_advances_a_persisted_revision() {
    let Some(app) = app().await else { panic!("restored default seed required") };
    let mut b = app.sign_in(DAVID).await;
    let initial = b.send(get("/api/v1/settings")).await;
    let mut revision = parse::<serde_json::Value>(&initial)["revision"].as_i64()
        .expect("settings GET returns a server revision");
    let initial_count = b.send(get("/api/v1/activity/unread_count")).await;
    let mut activity_revision = parse::<api::ActivityUnreadCount>(&initial_count).unread_revision;
    let allowance = format!("/api/v1/settings/dnd_allowances/{KEVIN}");
    for (method, path, body) in [
        (Method::PATCH, "/api/v1/settings/profile", json!({"bio":"revision test"})),
        (Method::DELETE, "/api/v1/settings/avatar", json!({})),
        (Method::PATCH, "/api/v1/settings/appearance", json!({"theme":"dark"})),
        (Method::PATCH, "/api/v1/settings/calls", json!({"voiceMode":"push_to_talk"})),
        (Method::PATCH, "/api/v1/settings/status", json!({"presenceSetting":"auto"})),
        (Method::PATCH, "/api/v1/settings/notifications", json!({"dndEnabled":false})),
        (Method::POST, allowance.as_str(), json!({})),
        (Method::DELETE, allowance.as_str(), json!({})),
    ] {
        let response = write(&mut b, method, path, body).await;
        assert_eq!(response.status, StatusCode::OK, "{path}: {}", response.text());
        let next = parse::<serde_json::Value>(&response)["revision"].as_i64().unwrap();
        assert!(next > revision, "{path} did not advance revision");
        revision = next;
        let read = b.send(get("/api/v1/settings")).await;
        assert_eq!(parse::<serde_json::Value>(&read)["revision"], revision);
        let persisted = app.booted.app.db.read(|conn| {
            Ok(conn.query_row("SELECT activity_revision FROM users WHERE id=?", [DAVID], |r| r.get::<_, i64>(0))?)
        }).await.unwrap();
        assert_eq!(persisted, revision);
        let count = b.send(get("/api/v1/activity/unread_count")).await;
        let next = parse::<api::ActivityUnreadCount>(&count).unread_revision;
        assert!(next > activity_revision, "{path} did not advance the count revision");
        activity_revision = next;
    }
    let rejected = write(&mut b, Method::PATCH, "/api/v1/settings/notifications",
        json!({"roomMute":{"roomId":i64::MAX,"duration":"forever"}})).await;
    assert_eq!(rejected.status, StatusCode::NOT_FOUND);
    let read = b.send(get("/api/v1/settings")).await;
    assert_eq!(parse::<api::Settings>(&read).revision, revision);
    let count = b.send(get("/api/v1/activity/unread_count")).await;
    assert_eq!(parse::<api::ActivityUnreadCount>(&count).unread_revision, activity_revision);
}

async fn notification_delivery_app() -> (TestApp, Arc<campfire_kit::clock::FrozenClock>) {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let app = TestApp::boot_seed_with_env("default", clock.clone(), &[("SPA_ENABLED", "1")])
        .await
        .expect("restored default seed required")
        .without_job_runner()
        .await;
    (app, clock)
}

async fn assert_room_notification_delivery(
    app: &TestApp,
    client: &mut crate::channels::tests::support::Client,
    browser: &mut Browser<'_>,
    source: &str,
    expected: (bool, i64),
) {
    app.db()
        .write(|tx| {
            let mut membership = campfire_db::Membership::find_by_room_and_user(
                tx.conn(), ALL_TALK, DAVID,
            )?.unwrap();
            membership.read(tx)?;
            tx.conn().execute(
                "UPDATE memberships SET connected_at=NULL WHERE id=?",
                [membership.id],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let source = source.to_owned();
    let message = app.db().write(move |tx| {
        campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: ALL_TALK,
            creator_id: JASON,
            markdown_source: Some(source),
            ..Default::default()
        })
    }).await.unwrap();
    let broadcasts = app.booted.app.broadcasts.clone();
    let rich_text = app.db().env().rich_text.clone();
    app.db().read(move |conn| {
        let room = campfire_db::Room::find(conn, ALL_TALK)?;
        broadcasts.unread_room(conn, &room, &message, &*rich_text)
    }).await.unwrap();

    let unreads = crate::channels::tests::support::identifier(
        json!({"channel":"UnreadRoomsChannel"}),
    );
    let receipt = tokio::time::timeout(Duration::from_secs(1), client.next_text()).await.ok();
    if let Some(receipt) = &receipt {
        assert_eq!(*receipt, crate::channels::tests::support::delivery(
            &unreads, &format!(r#"{{"roomId":{ALL_TALK}}}"#),
        ));
    }
    client.assert_silent().await;
    let unread = app.db().read(|conn| {
        Ok(campfire_db::Membership::find_by_room_and_user(conn, ALL_TALK, DAVID)?
            .unwrap().unread())
    }).await.unwrap();
    // Read through the API again, as a reload does, rather than reusing the settings response.
    let sidebar: api::Sidebar = parse(&browser.send(get("/api/v1/sidebar")).await);
    let row = sidebar.rows.iter().find(|row| row.room.id == ALL_TALK).unwrap();
    assert_eq!((unread, receipt.is_some(), row.notification_count),
        (expected.0, expected.0, expected.1));
}

async fn inherited_notification_delivery(stored: &str, default: &str, notifications: i64) {
    let (app, _) = notification_delivery_app().await;
    let mut browser = app.sign_in(DAVID).await;
    let response = write(&mut browser, Method::PUT,
        &format!("/api/v1/rooms/{ALL_TALK}/involvement"),
        json!({"involvement":stored}),
    ).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let response = write(&mut browser, Method::PATCH,
        "/api/v1/settings/notifications",
        json!({"defaultNotificationLevel":default,
            "roomNotification":{"roomId":ALL_TALK,"level":null}}),
    ).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let (mut client, server) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&app).await;
    client.confirm(&crate::channels::tests::support::identifier(
        json!({"channel":"UnreadRoomsChannel"}),
    )).await;
    assert_room_notification_delivery(&app, &mut client, &mut browser,
        "An ordinary message without a mention", (true, notifications)).await;
    server.abort();
}

#[tokio::test]
async fn a9_unread_delivery_muted_to_default_all() {
    inherited_notification_delivery("muted", "everything", 1).await;
}

#[tokio::test]
async fn a9_unread_delivery_muted_to_default_mentions() {
    inherited_notification_delivery("muted", "mentions", 0).await;
}

#[tokio::test]
async fn a9_unread_delivery_all_to_default_no_notifications() {
    // No notifications retains unread markers, unlike a timed or indefinite room mute.
    inherited_notification_delivery("everything", "nothing", 0).await;
}

#[tokio::test]
async fn a9_unread_delivery_resumes_at_timed_mute_expiry() {
    let (app, clock) = notification_delivery_app().await;
    let mut browser = app.sign_in(DAVID).await;
    let response = write(&mut browser, Method::PUT,
        &format!("/api/v1/rooms/{ALL_TALK}/involvement"),
        json!({"involvement":"muted"}),
    ).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let response = write(&mut browser, Method::PATCH,
        "/api/v1/settings/notifications",
        json!({"defaultNotificationLevel":"everything",
            "roomNotification":{"roomId":ALL_TALK,"level":null},
            "roomMute":{"roomId":ALL_TALK,"duration":"minutes15"}}),
    ).await;
    assert_eq!(response.status, StatusCode::OK, "{}", response.text());
    let (mut client, server) =
        crate::controllers::messages::attachment_processing_tests::subscribe(&app).await;
    client.confirm(&crate::channels::tests::support::identifier(
        json!({"channel":"UnreadRoomsChannel"}),
    )).await;
    assert_room_notification_delivery(&app, &mut client, &mut browser,
        &format!("A muted mention of <@{DAVID}>"), (false, 0)).await;
    clock.advance(jiff::SignedDuration::from_secs(900));
    assert_room_notification_delivery(&app, &mut client, &mut browser,
        "The first ordinary message after expiry", (true, 1)).await;
    server.abort();
}

#[tokio::test]
async fn a9_badges_resume_when_injected_clock_reaches_mute_expiry() {
    async fn push_badge(app: &TestApp) -> i64 {
        let now = app.booted.app.db.env().now();
        app.booted
            .app
            .db
            .read(move |conn| {
                let subscription = campfire_db::PushSubscription::new(
                    DAVID,
                    Some("https://fcm.googleapis.com/fcm/send/a9"),
                    None,
                    None,
                    None,
                );
                let payload = campfire_db::PushPayload::new(
                    "Mute expiry".into(),
                    "Badge".into(),
                    "/app".into(),
                    None,
                );
                let notification = campfire_app::integrations::web_push::Notification::build(
                    conn,
                    &subscription,
                    &payload,
                    now,
                )?;
                Ok(notification.badge)
            })
            .await
            .unwrap()
    }

    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        "2035-01-01T12:00:00Z".parse().unwrap(),
    ));
    let app = TestApp::boot_seed_with_env("default", clock.clone(), &[("SPA_ENABLED", "1")])
        .await
        .expect("restored default seed required");
    let mut browser = app.sign_in(DAVID).await;
    let (room, message) = app.booted.app.db.write(|tx| {
        tx.conn().execute("UPDATE memberships SET unread_at=NULL WHERE user_id=?", [DAVID])?;
        tx.conn().execute("DELETE FROM activity_items WHERE user_id=?", [DAVID])?;
        let (room, message, created_at): (i64,i64,campfire_db::Timestamp) = tx.conn().query_row("SELECT m.room_id,m.id,m.created_at FROM messages m JOIN memberships ms ON ms.room_id=m.room_id WHERE ms.user_id=? AND m.thread_id IS NULL AND m.creator_id!=? AND NOT m.system_note LIMIT 1", [DAVID,DAVID], |row| Ok((row.get(0)?,row.get(1)?,row.get(2)?)))?;
        tx.conn().execute("UPDATE memberships SET involvement='everything', unread_at=?,last_read_message_id=NULL WHERE user_id=? AND room_id=?", rusqlite::params![created_at,DAVID,room])?;
        campfire_db::ActivityItem::refresh_unread(tx, DAVID, "Message", message, "mention")?;
        Ok((room, message))
    }).await.unwrap();
    let before = parse::<api::Sidebar>(&browser.send(get("/api/v1/sidebar")).await);
    let before = before
        .rows
        .iter()
        .find(|row| row.room.id == room)
        .unwrap()
        .notification_count;
    assert!(before > 0, "message {message} contributes a notification");
    assert_eq!(push_badge(&app).await, 1);
    assert_eq!(
        parse::<api::ActivityUnreadCount>(
            &browser.send(get("/api/v1/activity/unread_count")).await
        )
        .unread_count,
        1
    );
    let muted = write(
        &mut browser,
        Method::PATCH,
        "/api/v1/settings/notifications",
        json!({"roomMute":{"roomId":room,"duration":"minutes15"}}),
    )
    .await;
    assert_eq!(muted.status, StatusCode::OK, "{}", muted.text());
    let settings_before = parse::<Value>(&muted);
    let count_before = parse::<Value>(&browser.send(get("/api/v1/activity/unread_count")).await);
    assert_eq!(settings_before["evaluatedAt"], "2035-01-01T12:00:00.000000000Z");
    assert_eq!(count_before["evaluatedAt"], settings_before["evaluatedAt"]);
    assert_eq!(push_badge(&app).await, 0);
    let favorite_path = format!("/api/v1/rooms/{room}/favorite");
    let favorite = write(&mut browser, Method::POST, &favorite_path, json!({})).await;
    assert_eq!(favorite.status, StatusCode::OK, "{}", favorite.text());
    assert_eq!(parse::<api::SidebarRow>(&favorite).notification_count, 0);
    let sidebar = parse::<api::Sidebar>(&browser.send(get("/api/v1/sidebar")).await);
    assert_eq!(
        sidebar
            .rows
            .iter()
            .find(|row| row.room.id == room)
            .unwrap()
            .notification_count,
        0
    );
    assert_eq!(
        parse::<api::ActivityUnreadCount>(
            &browser.send(get("/api/v1/activity/unread_count")).await
        )
        .unread_count,
        0
    );
    clock.advance(jiff::SignedDuration::from_secs(900));
    let settings_after = parse::<Value>(&browser.send(get("/api/v1/settings")).await);
    let count_after = parse::<Value>(&browser.send(get("/api/v1/activity/unread_count")).await);
    let list_after = parse::<Value>(&browser.send(get("/api/v1/activity")).await);
    assert_eq!(settings_after["revision"], settings_before["revision"]);
    assert_eq!(count_after["unreadRevision"], count_before["unreadRevision"]);
    assert_eq!(settings_after["evaluatedAt"], "2035-01-01T12:15:00.000000000Z");
    assert_eq!(count_after["evaluatedAt"], settings_after["evaluatedAt"]);
    assert_eq!(list_after["evaluatedAt"], settings_after["evaluatedAt"]);
    assert_eq!(settings_after["notifications"]["roomMuteUntil"], json!({}));
    assert_eq!(push_badge(&app).await, 1);
    let favorite = write(&mut browser, Method::POST, &favorite_path, json!({})).await;
    assert_eq!(favorite.status, StatusCode::OK, "{}", favorite.text());
    assert_eq!(
        parse::<api::SidebarRow>(&favorite).notification_count,
        before
    );
    let sidebar = parse::<api::Sidebar>(&browser.send(get("/api/v1/sidebar")).await);
    assert_eq!(
        sidebar
            .rows
            .iter()
            .find(|row| row.room.id == room)
            .unwrap()
            .notification_count,
        before
    );
    assert_eq!(
        parse::<api::ActivityUnreadCount>(
            &browser.send(get("/api/v1/activity/unread_count")).await
        )
        .unread_count,
        1
    );
}
