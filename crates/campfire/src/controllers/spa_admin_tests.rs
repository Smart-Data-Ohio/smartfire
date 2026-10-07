//! `/api/v1/admin` (S7) over the seeded app with `SPA_ENABLED`. Each write runs twice, on two apps
//! frozen at the same instant: once through the classic admin form and once through the API. The
//! whole database and every publication (followed or not) must come out the same.

use std::sync::Arc;
use std::time::Duration;

use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::{Map, Value, json};

use crate::controllers::presenters::test_support::{
    BENDER, Browser, DAVID, JASON, KEVIN, Reply, Req, SEED_NOW, TestApp,
};

/// David's password in the seed.
pub(super) const PASSWORD: &str = "secret123456";
/// A deactivated member in the seed.
const RITA: i64 = 773523954;
/// A banned member in the seed.
const MALLORY: i64 = 773523955;

/// The seeded app with `SPA_ENABLED`, its job runner stopped: a write's enqueued jobs stay as it
/// left them, so a whole-database comparison never sees a runner claim one on one side only.
pub(super) async fn app() -> Option<TestApp> {
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let app = TestApp::boot_seed_with_env("default", clock, &[("SPA_ENABLED", "1")]).await?;
    Some(app.without_job_runner().await)
}

pub(super) fn get(path: &str) -> Req {
    Req::new(Method::GET, path).header("accept", "application/json")
}

pub(super) fn json_body(method: Method, path: &str, body: &Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(body.to_string())
}

pub(super) fn parse<T: serde::de::DeserializeOwned>(reply: &Reply) -> T {
    serde_json::from_slice(&reply.body).unwrap_or_else(|error| panic!("{error}: {}", reply.text()))
}

pub(super) fn error(reply: &Reply) -> Value {
    let envelope: api::ApiErrorResponse = parse(reply);
    serde_json::to_value(&envelope.error).unwrap()
}

pub(super) async fn write(b: &mut Browser<'_>, method: Method, path: &str, body: Value) -> Reply {
    b.write(json_body(method, path, &body)).await
}

/// An API write that must succeed, as `T`.
pub(super) async fn spa<T: serde::de::DeserializeOwned>(
    b: &mut Browser<'_>,
    method: Method,
    path: &str,
    body: Value,
) -> T {
    let reply = write(b, method, path, body).await;
    assert_eq!(reply.status, StatusCode::OK, "{path}: {}", reply.text());
    parse(&reply)
}

/// A classic form post that must redirect, as the admin pages' writes do.
pub(super) async fn classic(b: &mut Browser<'_>, method: Method, path: &str, fields: &[(&str, &str)]) {
    let reply = b.write(Req::new(method, path).form(fields)).await;
    assert!(
        reply.status.is_redirection(),
        "{path}: {}: {}",
        reply.status,
        reply.text()
    );
}

/// Columns that differ between two runs by design: secrets drawn afresh (session tokens, storage
/// keys, the join code, bot keys, signing secrets, credentials and the ciphertext of stored
/// tokens and secrets), the digests salted afresh, and a system note's random client id.
const VOLATILE: &[&str] = &[
    "token",
    "key",
    "join_code",
    "password_digest",
    "client_message_id",
    "bot_token",
    "bot_token_digest",
    "webhook_signing_secret",
    "signing_secret",
    "token_digest",
    "token_last_four",
    "access_token",
    "refresh_token",
    "client_secret",
];

/// `text` with the random part of a deactivated address (`kevin-deactivated-<uuid>@...`) and a
/// new credential's last four characters masked.
pub(super) fn unrandom(text: &str) -> String {
    const MARK: &str = "-deactivated-";
    let text = match text.find(MARK) {
        Some(at) if text.len() >= at + MARK.len() + 36 => {
            let start = at + MARK.len();
            format!("{}<uuid>{}", &text[..start], &text[start + 36..])
        }
        _ => text.to_string(),
    };
    // A new credential's audit names the last four characters of its fresh secret.
    const LAST_FOUR: &str = "\"last_four\":\"";
    match text.find(LAST_FOUR) {
        Some(at) if text.len() >= at + LAST_FOUR.len() + 4 => {
            let start = at + LAST_FOUR.len();
            format!("{}<four>{}", &text[..start], &text[start + 4..])
        }
        _ => text,
    }
}

/// `text` with every UUID (a system note's random client id in its DOM id) masked.
pub(super) fn uuids_masked(text: &str) -> String {
    let shape = |window: &[u8]| {
        window.len() == 36
            && window.iter().enumerate().all(|(index, byte)| match index {
                8 | 13 | 18 | 23 => *byte == b'-',
                _ => byte.is_ascii_hexdigit(),
            })
    };
    let bytes = text.as_bytes();
    let mut out = String::with_capacity(text.len());
    let mut at = 0;
    while at < bytes.len() {
        if at + 36 <= bytes.len() && shape(&bytes[at..at + 36]) {
            out.push_str("<uuid>");
            at += 36;
        } else {
            let character = text[at..].chars().next().unwrap();
            out.push(character);
            at += character.len_utf8();
        }
    }
    out
}

/// A time SQLite's own clock stamped (`insert_all` writes `CURRENT_TIMESTAMP`, as Rails does, not
/// the app's frozen clock): within the hour of the real now.
fn sqlite_now(text: &str) -> bool {
    campfire_db::Timestamp::parse_db(text).is_some_and(|at| {
        let gap = jiff::Timestamp::now().as_second() - at.jiff().as_second();
        gap.abs() < 3600
    })
}

/// Every row of every table, as JSON, each table's rows sorted (with times SQLite stamped
/// masked).
pub(super) async fn dump(a: &TestApp) -> Value {
    a.db()
        .read(|conn| {
            let tables: Vec<String> = conn
                .prepare(
                    "SELECT name FROM sqlite_master WHERE type='table' AND name NOT LIKE 'sqlite_%' ORDER BY name",
                )?
                .query_map([], |row| row.get(0))?
                .collect::<rusqlite::Result<_>>()?;
            let mut out = Map::new();
            for table in tables {
                let mut statement = conn.prepare(&format!("SELECT * FROM \"{table}\""))?;
                let names: Vec<String> = statement
                    .column_names()
                    .into_iter()
                    .map(str::to_string)
                    .collect();
                let mut rows: Vec<String> = statement
                    .query_map([], |row| {
                        let mut columns = Map::new();
                        for (index, name) in names.iter().enumerate() {
                            if VOLATILE.contains(&name.as_str()) {
                                continue;
                            }
                            let value: rusqlite::types::Value = row.get(index)?;
                            columns.insert(
                                name.clone(),
                                match value {
                                    rusqlite::types::Value::Null => Value::Null,
                                    rusqlite::types::Value::Integer(number) => json!(number),
                                    rusqlite::types::Value::Real(number) => json!(number),
                                    rusqlite::types::Value::Text(text) if sqlite_now(&text) => {
                                        json!("<sqlite now>")
                                    }
                                    rusqlite::types::Value::Text(text)
                                        if table == "background_jobs" && name == "arguments"
                                            && row.get::<_, String>("job_class")? == "Calendar::DisconnectCleanupJob" =>
                                    {
                                        let mut arguments: Value = serde_json::from_str(&text).unwrap();
                                        // Only this encrypted credential snapshot is random. Keep
                                        // the remote event IDs and account ID in the comparison.
                                        if arguments[1].is_string() {
                                            arguments[1] = json!("<encrypted snapshot>");
                                        }
                                        json!(arguments.to_string())
                                    }
                                    rusqlite::types::Value::Text(text) => json!(unrandom(&text)),
                                    rusqlite::types::Value::Blob(bytes) => json!(bytes),
                                },
                            );
                        }
                        Ok(Value::Object(columns).to_string())
                    })?
                    .collect::<rusqlite::Result<_>>()?;
                rows.sort();
                out.insert(table, json!(rows));
            }
            Ok(Value::Object(out))
        })
        .await
        .unwrap()
}

/// Every publication until they stop coming (some go out after the response).
pub(super) async fn settle(capture: &campfire_cable::pubsub::PublicationCapture) -> Vec<(String, String)> {
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

/// A PNG uploaded the way the browser's direct upload leaves it: its signed blob id.
pub(super) async fn upload(a: &TestApp, name: &str) -> String {
    let png = include_bytes!("../../../../fixtures/files/workspace_icons/square_64.png");
    let staged = a
        .booted
        .app
        .storage
        .stage_bytes(
            png,
            campfire_storage::Filename::new(name),
            Some("image/png"),
        )
        .unwrap();
    let blob = a
        .db()
        .write(move |tx| crate::controllers::messages::save_staged(tx, staged))
        .await
        .unwrap();
    campfire_storage::paths::signed_blob_id(&*a.booted.app.storage.verifier, blob.id, None)
}

/// What one write did: the database afterwards and everything it published.
pub(super) struct Outcome {
    pub(super) before: Value,
    pub(super) rows: Value,
    pub(super) frames: Vec<(String, String)>,
}

/// Signs David in (with sudo) on a fresh app, runs `prepare` (its answer goes to `exercise`),
/// then records what `exercise` leaves behind.
async fn outcome_with_app<B, P, F>(boot: &B, prepare: P, exercise: F) -> Option<Outcome>
where
    B: AsyncFn() -> Option<TestApp>,
    P: AsyncFnOnce(&TestApp, &mut Browser<'_>) -> Value,
    F: AsyncFnOnce(&mut Browser<'_>, Value),
{
    let a = boot().await?;
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    let context = prepare(&a, &mut b).await;
    let before = dump(&a).await;
    let capture = a.booted.app.cable.capture_every_publication();
    exercise(&mut b, context).await;
    let frames = settle(&capture)
        .await
        .into_iter()
        .map(|(stream, frame)| (stream, uuids_masked(&frame)))
        .collect();
    Some(Outcome {
        before,
        rows: dump(&a).await,
        frames,
    })
}

/// The classic form and the API write leave the same database and publish the same frames.
pub(super) async fn assert_parity<P, C, S>(prepare: P, classic: C, spa: S) -> Option<Outcome>
where
    P: AsyncFn(&TestApp, &mut Browser<'_>) -> Value,
    C: AsyncFnOnce(&mut Browser<'_>, Value),
    S: AsyncFnOnce(&mut Browser<'_>, Value),
{
    assert_parity_with_app(app, prepare, classic, spa).await
}

/// The same whole-database comparison with caller-owned service stubs installed at boot.
pub(super) async fn assert_parity_with_app<B, P, C, S>(
    boot: B,
    prepare: P,
    classic: C,
    spa: S,
) -> Option<Outcome>
where
    B: AsyncFn() -> Option<TestApp>,
    P: AsyncFn(&TestApp, &mut Browser<'_>) -> Value,
    C: AsyncFnOnce(&mut Browser<'_>, Value),
    S: AsyncFnOnce(&mut Browser<'_>, Value),
{
    let classic = outcome_with_app(&boot, &prepare, classic).await?;
    let spa = outcome_with_app(&boot, &prepare, spa).await?;
    for (table, rows) in classic.rows.as_object().unwrap() {
        if &spa.rows[table] != rows {
            let side = |rows: &Value| -> Vec<String> {
                rows.as_array()
                    .into_iter()
                    .flatten()
                    .map(|row| row.as_str().unwrap_or_default().to_string())
                    .collect()
            };
            let (classic_rows, spa_rows) = (side(rows), side(&spa.rows[table]));
            let only = |a: &[String], b: &[String]| -> Vec<String> {
                a.iter().filter(|row| !b.contains(row)).cloned().collect()
            };
            panic!(
                "{table} differs\nclassic only: {:#?}\nspa only: {:#?}",
                only(&classic_rows, &spa_rows),
                only(&spa_rows, &classic_rows)
            );
        }
    }
    assert_eq!(spa.frames, classic.frames, "frames");
    Some(spa)
}

pub(super) async fn nothing(_: &TestApp, _: &mut Browser<'_>) -> Value {
    Value::Null
}

/// The audit rows a write left, as `[action, details]`.
pub(super) fn audits(outcome: &Outcome) -> String {
    outcome.rows["audit_logs"].to_string()
}

// --- The gates ---------------------------------------------------------------------------------

#[tokio::test]
async fn admin_exists_only_with_the_spa() {
    let clock = crate::controllers::presenters::test_support::seed_clock();
    let Some(a) = TestApp::boot_seed_with_env("default", clock, &[]).await else {
        return;
    };
    let mut b = a.sign_in(DAVID).await;
    let unknown = b.send(get("/no-such-page")).await.status;
    for path in [
        "/api/v1/admin/workspace",
        "/api/v1/admin/people",
        "/api/v1/admin/audit_log",
        "/api/v1/admin/integrations_health",
    ] {
        assert_eq!(b.send(get(path)).await.status, unknown, "{path}");
    }
}

#[tokio::test]
async fn members_read_the_workspace_and_people_but_change_nothing() {
    let Some(a) = app().await else { return };
    let mut kevin = a.sign_in(KEVIN).await;
    let workspace: api::Workspace = parse(&kevin.send(get("/api/v1/admin/workspace")).await);
    assert!(!workspace.can_administer);
    assert_eq!(workspace.name, "37signals");

    let page: api::PeoplePage = parse(&kevin.send(get("/api/v1/admin/people")).await);
    assert!(
        page.people
            .iter()
            .any(|person| person.you && person.id == KEVIN)
    );
    for person in &page.people {
        assert_eq!(person.email_address, None, "{}", person.name);
        assert!(!person.two_factor_enabled && !person.offer_google_email_link);
        assert!(!person.banned, "members don't see banned people");
    }

    for path in [
        "/api/v1/admin/custom_styles",
        "/api/v1/admin/icons",
        "/api/v1/admin/audit_log",
        "/api/v1/admin/integrations_health",
    ] {
        let reply = kevin.send(get(path)).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{path}");
        assert_eq!(error(&reply)["_tag"], "Forbidden", "{path}");
    }
    let before = dump(&a).await;
    let writes = [
        (
            Method::PATCH,
            "/api/v1/admin/workspace".to_string(),
            json!({"name": "Mine", "restrictRoomCreationToAdministrators": null}),
        ),
        (
            Method::POST,
            "/api/v1/admin/workspace/join_code".to_string(),
            Value::Null,
        ),
        (
            Method::PATCH,
            format!("/api/v1/admin/people/{JASON}"),
            json!({"role": "member"}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/people/{JASON}"),
            Value::Null,
        ),
        (
            Method::PATCH,
            "/api/v1/admin/custom_styles".to_string(),
            json!({"css": "body{}"}),
        ),
        (
            Method::POST,
            format!("/api/v1/admin/people/{JASON}/two_factor_reset"),
            Value::Null,
        ),
        (
            Method::POST,
            format!("/api/v1/admin/people/{JASON}/google_link"),
            Value::Null,
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/people/{JASON}/google_link"),
            Value::Null,
        ),
        (
            Method::PUT,
            "/api/v1/admin/workspace/logo".to_string(),
            json!({"signedId": "anything"}),
        ),
        (
            Method::DELETE,
            "/api/v1/admin/workspace/logo".to_string(),
            Value::Null,
        ),
        (
            Method::POST,
            "/api/v1/admin/icons".to_string(),
            json!({"name": "mine", "title": "Mine", "signedId": null}),
        ),
        (
            Method::DELETE,
            "/api/v1/admin/icons/1".to_string(),
            Value::Null,
        ),
    ];
    for (method, path, body) in writes {
        let reply = write(&mut kevin, method.clone(), &path, body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(error(&reply)["_tag"], "Forbidden", "{method} {path}");
    }
    assert_eq!(dump(&a).await, before);
}

#[tokio::test]
async fn writes_need_the_csrf_token_and_a_valid_body() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let forged = b
        .send(json_body(
            Method::PATCH,
            "/api/v1/admin/workspace",
            &json!({"name": "Forged", "restrictRoomCreationToAdministrators": null}),
        ))
        .await;
    assert_eq!(error(&forged)["_tag"], "InvalidAuthenticityToken");
    let wrong = write(
        &mut b,
        Method::PATCH,
        &format!("/api/v1/admin/people/{KEVIN}"),
        json!({"role": "owner"}),
    )
    .await;
    // Sudo comes first, as the classic before-actions order it.
    assert_eq!(error(&wrong)["_tag"], "SudoRequired");
    b.grant_sudo().await;
    let wrong = write(
        &mut b,
        Method::PATCH,
        &format!("/api/v1/admin/people/{KEVIN}"),
        json!({"role": "owner"}),
    )
    .await;
    assert_eq!(
        (wrong.status, error(&wrong)["_tag"].clone()),
        (StatusCode::UNPROCESSABLE_ENTITY, json!("Validation"))
    );
    let signed_out = a.anonymous().send(get("/api/v1/admin/workspace")).await;
    assert_eq!(signed_out.status, StatusCode::UNAUTHORIZED);
}

/// A write that needs sudo answers `SudoRequired` and leaves a visit to the SPA page that asked:
/// confirming the password on `/sudo` comes back there.
#[tokio::test]
async fn sudo_comes_back_to_the_spa_page() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    let asked = b
        .write(
            json_body(
                Method::POST,
                "/api/v1/admin/workspace/join_code",
                &Value::Null,
            )
            .header("referer", "http://campfire.test/app/admin/workspace"),
        )
        .await;
    assert_eq!(asked.status, StatusCode::FORBIDDEN, "{}", asked.text());
    assert_eq!(error(&asked)["_tag"], "SudoRequired");
    assert_eq!(dump(&a).await, before, "nothing changed");

    let confirmed = b
        .write(Req::new(Method::POST, "/sudo").form(&[("password", PASSWORD)]))
        .await;
    assert_eq!(confirmed.status, StatusCode::FOUND, "{}", confirmed.text());
    assert_eq!(
        confirmed.location(),
        Some("http://campfire.test/app/admin/workspace")
    );
    let reset = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/workspace/join_code",
        Value::Null,
    )
    .await;
    assert_eq!(reset.status, StatusCode::OK, "{}", reset.text());

    // A referer elsewhere comes back to the SPA's home.
    let mut other = a.sign_in(DAVID).await;
    let asked = other
        .write(
            json_body(
                Method::PATCH,
                "/api/v1/admin/custom_styles",
                &json!({"css": null}),
            )
            .header("referer", "https://elsewhere.example/app/admin"),
        )
        .await;
    assert_eq!(error(&asked)["_tag"], "SudoRequired");
    let confirmed = other
        .write(Req::new(Method::POST, "/sudo").form(&[("password", PASSWORD)]))
        .await;
    assert_eq!(confirmed.location(), Some("http://campfire.test/app/"));

    // So does a page of this host outside the SPA.
    let mut classic_page = a.sign_in(DAVID).await;
    let asked = classic_page
        .write(
            json_body(
                Method::POST,
                "/api/v1/admin/workspace/join_code",
                &Value::Null,
            )
            .header("referer", "http://campfire.test/account/edit"),
        )
        .await;
    assert_eq!(error(&asked)["_tag"], "SudoRequired");
    let confirmed = classic_page
        .write(Req::new(Method::POST, "/sudo").form(&[("password", PASSWORD)]))
        .await;
    assert_eq!(confirmed.location(), Some("http://campfire.test/app/"));
}

/// Without a fresh password confirmation, the writes the classic pages guard answer
/// `SudoRequired` and change nothing.
#[tokio::test]
async fn guarded_writes_change_nothing_without_the_password() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    let writes = [
        (
            Method::PATCH,
            format!("/api/v1/admin/people/{KEVIN}"),
            json!({"role": "administrator"}),
        ),
        (
            Method::DELETE,
            format!("/api/v1/admin/people/{KEVIN}"),
            Value::Null,
        ),
        (
            Method::PATCH,
            "/api/v1/admin/custom_styles".to_string(),
            json!({"css": "body { color: red }"}),
        ),
        (
            Method::POST,
            "/api/v1/admin/workspace/join_code".to_string(),
            Value::Null,
        ),
    ];
    for (method, path, body) in writes {
        let reply = write(&mut b, method.clone(), &path, body).await;
        assert_eq!(reply.status, StatusCode::FORBIDDEN, "{method} {path}");
        assert_eq!(error(&reply)["_tag"], "SudoRequired", "{method} {path}");
        assert_eq!(dump(&a).await, before, "{method} {path} changed something");
    }
}

// --- Workspace ---------------------------------------------------------------------------------

#[tokio::test]
async fn the_workspace_saves_as_the_classic_form_does() {
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| {
            classic(
                b,
                Method::PATCH,
                "/account",
                &[
                    ("account[name]", "Smart Data"),
                    (
                        "account[settings][restrict_room_creation_to_administrators]",
                        "true",
                    ),
                ],
            )
            .await
        },
        async |b, _| {
            let workspace: api::Workspace = spa(
                b,
                Method::PATCH,
                "/api/v1/admin/workspace",
                json!({"name": "Smart Data", "restrictRoomCreationToAdministrators": true}),
            )
            .await;
            assert_eq!(workspace.name, "Smart Data");
            assert!(workspace.restrict_room_creation_to_administrators);
            assert!(workspace.can_administer);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("account.settings.change"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn a_logo_attaches_and_goes_as_the_classic_forms_do() {
    let prepare = async |a: &TestApp, _: &mut Browser<'_>| json!(upload(a, "logo.png").await);
    assert_parity(
        prepare,
        async |b, signed| {
            let signed = signed.as_str().unwrap().to_string();
            classic(b, Method::PATCH, "/account", &[("account[logo]", &signed)]).await;
            classic(b, Method::DELETE, "/account/logo", &[]).await;
        },
        async |b, signed| {
            let attached: api::Workspace = spa(
                b,
                Method::PUT,
                "/api/v1/admin/workspace/logo",
                json!({"signedId": signed}),
            )
            .await;
            assert!(attached.logo_attached);
            let removed: api::Workspace = spa(
                b,
                Method::DELETE,
                "/api/v1/admin/workspace/logo",
                Value::Null,
            )
            .await;
            assert!(!removed.logo_attached);
        },
    )
    .await;

    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let forged = write(
        &mut b,
        Method::PUT,
        "/api/v1/admin/workspace/logo",
        json!({"signedId": "forged"}),
    )
    .await;
    assert_eq!(forged.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(error(&forged)["fields"]["signedId"].is_array());
}

#[tokio::test]
async fn a_new_join_link_matches_the_classic_reset() {
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| classic(b, Method::POST, "/account/join_code", &[]).await,
        async |b, _| {
            let workspace: api::Workspace = spa(
                b,
                Method::POST,
                "/api/v1/admin/workspace/join_code",
                Value::Null,
            )
            .await;
            assert!(!workspace.join_url.ends_with("/CRMu-l8Ge-KB9B"));
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("account.join_code.reset"),
        "{}",
        audits(&spa_side)
    );
}

// --- People ------------------------------------------------------------------------------------

#[tokio::test]
async fn people_list_as_the_account_page_does() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let page: api::PeoplePage = parse(&b.send(get("/api/v1/admin/people")).await);
    assert_eq!(page.next_page, None);
    let ids: Vec<i64> = page.people.iter().map(|person| person.id).collect();
    assert!(!ids.contains(&RITA), "deactivated people aren't listed");
    let mallory = page.people.iter().find(|person| person.id == MALLORY);
    assert!(
        mallory.is_some_and(|person| person.banned),
        "administrators see banned people"
    );
    let first_member = page
        .people
        .iter()
        .position(|person| person.role == api::PersonRole::Member)
        .unwrap();
    assert!(
        page.people[first_member..]
            .iter()
            .all(|person| person.role == api::PersonRole::Member),
        "administrators first"
    );
    let kevin = page
        .people
        .iter()
        .find(|person| person.id == KEVIN)
        .unwrap();
    assert!(kevin.two_factor_enabled);
    assert!(kevin.email_address.is_some());
    assert!(
        page.people
            .iter()
            .any(|person| person.you && person.id == DAVID)
    );

    // Everyone listed is on the classic page too.
    let html = b.get("/account/edit").await.text();
    for person in &page.people {
        assert!(html.contains(&person.name), "{}", person.name);
    }
}

#[tokio::test]
async fn a_role_change_saves_as_the_classic_form_does() {
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| {
            classic(
                b,
                Method::PATCH,
                &format!("/account/users/{KEVIN}"),
                &[("user[role]", "administrator")],
            )
            .await
        },
        async |b, _| {
            let change: api::PersonChange = spa(
                b,
                Method::PATCH,
                &format!("/api/v1/admin/people/{KEVIN}"),
                json!({"role": "administrator"}),
            )
            .await;
            assert_eq!(change.person.role, api::PersonRole::Administrator);
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("user.role.change"),
        "{}",
        audits(&spa_side)
    );
}

#[tokio::test]
async fn removing_someone_matches_the_classic_page() {
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| classic(b, Method::DELETE, &format!("/account/users/{KEVIN}"), &[]).await,
        async |b, _| {
            let removed: api::PersonRemoved = spa(
                b,
                Method::DELETE,
                &format!("/api/v1/admin/people/{KEVIN}"),
                Value::Null,
            )
            .await;
            assert_eq!(removed.id, KEVIN);
        },
    )
    .await
    else {
        return;
    };
    assert!(audits(&spa_side).contains("user.deactivate"));
    // Their open sockets are told to disconnect, as the classic removal tells them.
    let stream = format!("action_cable/gid://campfire/User/{KEVIN}");
    assert!(
        spa_side.frames.iter().any(|(name, _)| *name == stream),
        "{:?}",
        spa_side.frames
    );

    // Someone already gone, a bot, or nobody: not found.
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    for id in [RITA, 999_999_999] {
        let reply = write(
            &mut b,
            Method::DELETE,
            &format!("/api/v1/admin/people/{id}"),
            Value::Null,
        )
        .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{id}");
    }
}

/// The classic page doesn't stop an administrator demoting or removing themselves, or changing a
/// bot's role or removing it (`User.active` includes bots); neither does the API.
#[tokio::test]
async fn yourself_and_bots_change_as_the_classic_page_allows() {
    for target in [DAVID, BENDER] {
        let path = format!("/account/users/{target}");
        let api_path = format!("/api/v1/admin/people/{target}");
        let (role, wire) = if target == DAVID {
            ("member", "member")
        } else {
            ("administrator", "administrator")
        };
        assert_parity(
            nothing,
            async |b, _| classic(b, Method::PATCH, &path, &[("user[role]", role)]).await,
            async |b, _| {
                let change: api::PersonChange =
                    spa(b, Method::PATCH, &api_path, json!({"role": wire})).await;
                assert_eq!(change.person.id, target);
            },
        )
        .await;
        assert_parity(
            nothing,
            async |b, _| classic(b, Method::DELETE, &path, &[]).await,
            async |b, _| {
                let removed: api::PersonRemoved =
                    spa(b, Method::DELETE, &api_path, Value::Null).await;
                assert_eq!(removed.id, target);
            },
        )
        .await;
    }
}

#[tokio::test]
async fn a_two_step_reset_matches_the_classic_page() {
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| {
            classic(
                b,
                Method::POST,
                &format!("/account/users/{KEVIN}/two_factor_reset"),
                &[],
            )
            .await
        },
        async |b, _| {
            let change: api::PersonChange = spa(
                b,
                Method::POST,
                &format!("/api/v1/admin/people/{KEVIN}/two_factor_reset"),
                Value::Null,
            )
            .await;
            assert!(!change.person.two_factor_enabled);
            assert!(
                change
                    .notice
                    .unwrap()
                    .starts_with("Two-step sign-in reset for ")
            );
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("two_factor.reset"),
        "{}",
        audits(&spa_side)
    );
    // Their sockets disconnect: the reset signs them out everywhere.
    let stream = format!("action_cable/gid://campfire/User/{KEVIN}");
    assert!(
        spa_side.frames.iter().any(|(name, _)| *name == stream),
        "{:?}",
        spa_side.frames
    );
}

#[tokio::test]
async fn a_two_step_reset_refuses_as_the_classic_page_does() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let before = dump(&a).await;
    for (id, alert) in [
        (
            DAVID,
            "Reset someone else's two-step sign-in from here. To change your own, use Disable on your profile.".to_string(),
        ),
        {
            // Someone else without two-step sign-in.
            let people: api::PeoplePage = parse(&b.send(get("/api/v1/admin/people")).await);
            let person = people
                .people
                .iter()
                .find(|p| !p.you && !p.banned && !p.two_factor_enabled)
                .expect("someone without two-step sign-in");
            (
                person.id,
                format!("{} doesn't have two-step sign-in enabled.", person.name),
            )
        },
    ] {
        let reply = write(
            &mut b,
            Method::POST,
            &format!("/api/v1/admin/people/{id}/two_factor_reset"),
            Value::Null,
        )
        .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{id}");
        assert_eq!(
            error(&reply),
            json!({"_tag": "Validation", "message": alert, "fields": {}})
        );
        // The classic page says the same.
        let classic = b
            .write(Req::new(
                Method::POST,
                &format!("/account/users/{id}/two_factor_reset"),
            ))
            .await;
        assert!(classic.status.is_redirection(), "{}", classic.status);
        let page = b.get(classic.location().unwrap()).await.text();
        let escaped = alert.replace('\'', "&#39;");
        assert!(page.contains(&escaped), "{alert}");
    }
    for id in [BENDER, RITA] {
        let reply = write(
            &mut b,
            Method::POST,
            &format!("/api/v1/admin/people/{id}/two_factor_reset"),
            Value::Null,
        )
        .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND, "{id}");
    }
    assert_eq!(dump(&a).await["audit_logs"], before["audit_logs"]);
}

#[tokio::test]
async fn a_google_link_allows_and_unlinks_as_the_classic_page_does() {
    assert_parity(
        nothing,
        async |b, _| {
            let path = format!("/account/users/{KEVIN}/google_link");
            classic(b, Method::POST, &path, &[]).await;
            classic(b, Method::DELETE, &path, &[]).await;
        },
        async |b, _| {
            let path = format!("/api/v1/admin/people/{KEVIN}/google_link");
            let allowed: api::PersonChange = spa(b, Method::POST, &path, Value::Null).await;
            assert!(
                allowed
                    .notice
                    .unwrap()
                    .contains("can now link Google sign-in for")
            );
            let unlinked: api::PersonChange = spa(b, Method::DELETE, &path, Value::Null).await;
            assert!(
                unlinked
                    .notice
                    .unwrap()
                    .starts_with("Google sign-in unlinked from ")
            );
        },
    )
    .await;
}

// --- Custom styles and icons -------------------------------------------------------------------

#[tokio::test]
async fn custom_styles_save_as_the_classic_form_does() {
    let css = ".message { color: rebeccapurple }";
    let Some(spa_side) = assert_parity(
        nothing,
        async |b, _| {
            classic(
                b,
                Method::PATCH,
                "/account/custom_styles",
                &[("account[custom_styles]", css)],
            )
            .await
        },
        async |b, _| {
            let saved: api::CustomStyles = spa(
                b,
                Method::PATCH,
                "/api/v1/admin/custom_styles",
                json!({"css": css}),
            )
            .await;
            assert_eq!(saved.css.as_deref(), Some(css));
        },
    )
    .await
    else {
        return;
    };
    assert!(
        audits(&spa_side).contains("account.custom_styles.change"),
        "{}",
        audits(&spa_side)
    );
}

/// A body without `css` leaves the styles alone, as a classic post without `custom_styles` does;
/// `null` clears them.
#[tokio::test]
async fn custom_styles_change_only_when_given() {
    let css = ".message { color: rebeccapurple }";
    let prepare = async |a: &TestApp, _: &mut Browser<'_>| {
        a.db()
            .write(move |tx| {
                let mut account = campfire_db::Account::first(tx.conn())?.unwrap();
                account.update(tx, None, Some(Some(css)), None)
            })
            .await
            .unwrap();
        Value::Null
    };
    assert_parity(
        prepare,
        async |b, _| {
            classic(
                b,
                Method::PATCH,
                "/account/custom_styles",
                &[("account[unrelated]", "1")],
            )
            .await
        },
        async |b, _| {
            let kept: api::CustomStyles =
                spa(b, Method::PATCH, "/api/v1/admin/custom_styles", json!({})).await;
            assert_eq!(kept.css.as_deref(), Some(css));
        },
    )
    .await;

    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    let cleared: api::CustomStyles = spa(
        &mut b,
        Method::PATCH,
        "/api/v1/admin/custom_styles",
        json!({"css": null}),
    )
    .await;
    assert_eq!(cleared.css, None);
}

#[tokio::test]
async fn icons_come_and_go_as_the_classic_forms_do() {
    let prepare = async |a: &TestApp, _: &mut Browser<'_>| json!(upload(a, "acme.png").await);
    let Some(spa_side) = assert_parity(
        prepare,
        async |b, signed| {
            let signed = signed.as_str().unwrap().to_string();
            classic(
                b,
                Method::POST,
                "/account/icons",
                &[
                    ("workspace_icon[name]", "acme"),
                    ("workspace_icon[title]", "Acme Corp"),
                    ("workspace_icon[image]", &signed),
                ],
            )
            .await;
            let list: api::WorkspaceIconList = parse(&b.send(get("/api/v1/admin/icons")).await);
            let id = list
                .icons
                .iter()
                .find(|icon| icon.name == "acme")
                .unwrap()
                .id;
            classic(b, Method::DELETE, &format!("/account/icons/{id}"), &[]).await;
        },
        async |b, signed| {
            let created: api::WorkspaceIconList = spa(
                b,
                Method::POST,
                "/api/v1/admin/icons",
                json!({"name": "acme", "title": "Acme Corp", "signedId": signed}),
            )
            .await;
            let icon = created
                .icons
                .iter()
                .find(|icon| icon.name == "acme")
                .unwrap();
            assert_eq!(icon.title, "Acme Corp");
            assert_eq!(icon.image_url, "/icons/acme");
            let id = icon.id;
            let deleted: api::WorkspaceIconList = spa(
                b,
                Method::DELETE,
                &format!("/api/v1/admin/icons/{id}"),
                Value::Null,
            )
            .await;
            assert!(deleted.icons.iter().all(|icon| icon.name != "acme"));
        },
    )
    .await
    else {
        return;
    };
    let audits = audits(&spa_side);
    assert!(audits.contains("workspace_icon.create") && audits.contains("workspace_icon.destroy"));

    // An icon needs a name the classic form accepts.
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let refused = write(
        &mut b,
        Method::POST,
        "/api/v1/admin/icons",
        json!({"name": "", "title": "", "signedId": null}),
    )
    .await;
    assert_eq!(
        (refused.status, error(&refused)["_tag"].clone()),
        (StatusCode::UNPROCESSABLE_ENTITY, json!("Validation")),
        "{}",
        refused.text()
    );
}

// --- Audit log and integrations health ---------------------------------------------------------

#[tokio::test]
async fn the_audit_log_filters_as_the_classic_page_does() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    b.grant_sudo().await;
    classic(
        &mut b,
        Method::PATCH,
        &format!("/account/users/{KEVIN}"),
        &[("user[role]", "administrator")],
    )
    .await;
    classic(&mut b, Method::POST, "/account/join_code", &[]).await;

    let all: api::AuditLogPage = parse(&b.send(get("/api/v1/admin/audit_log")).await);
    let total: i64 = a
        .db()
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |row| row.get(0))?))
        .await
        .unwrap();
    assert!(!all.entries.is_empty());
    assert!(all.entries.len() as i64 <= total);
    let ids: Vec<i64> = all.entries.iter().map(|entry| entry.id).collect();
    let mut newest_first = ids.clone();
    newest_first.sort_unstable_by(|a, b| b.cmp(a));
    assert_eq!(ids, newest_first, "newest first");
    assert!(
        all.actions
            .iter()
            .any(|action| action == "user.role.change")
    );
    assert_eq!(all.export_url, "/account/audit_log.csv");
    // The zone the dates are read in, for the client to show times in.
    assert!(
        jiff::tz::TimeZone::get(&all.time_zone).is_ok(),
        "{}",
        all.time_zone
    );

    let filtered: api::AuditLogPage = parse(
        &b.send(get(
            "/api/v1/admin/audit_log?action=user.role.change&targetType=User",
        ))
        .await,
    );
    assert_eq!(filtered.filters.action.as_deref(), Some("user.role.change"));
    assert_eq!(filtered.filters.target_type.as_deref(), Some("User"));
    assert!(!filtered.entries.is_empty());
    assert!(
        filtered
            .entries
            .iter()
            .all(|entry| entry.action == "user.role.change")
    );
    assert!(filtered.export_url.contains("action=user.role.change"));

    // The classic page lists the same rows for the same filter.
    let html = b
        .get("/account/audit_log?action=user.role.change&target_type=User")
        .await
        .text();
    for entry in &filtered.entries {
        assert!(html.contains(&entry.action));
    }
}

#[tokio::test]
async fn integrations_health_reads_for_administrators() {
    let Some(a) = app().await else { return };
    let mut b = a.sign_in(DAVID).await;
    let reply = b.send(get("/api/v1/admin/integrations_health")).await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.header("cache-control"), Some("no-store"));
    let health: api::IntegrationsHealth = parse(&reply);
    assert!(health.github.connected >= 0);
    assert!(health.agent_delivery.pending >= 0);
    // The classic page renders from the same snapshot.
    let page = b.get("/account/integrations_health").await;
    assert_eq!(page.status, StatusCode::OK);
}
