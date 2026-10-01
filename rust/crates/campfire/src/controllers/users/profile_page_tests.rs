//! Complete profile renderer bytes. HTTP authorization and CSRF remain real in the route test.
use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use campfire_views::{helpers as h, users};
use serde_json::Value;
struct Tokens;
impl h::request_forgery::AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
fn vectors() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_page.json"
    ))
    .unwrap()
}
async fn profile_case(markup:bool) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let v = vectors();
    if markup {
        let setup=v["markup"]["setup"].clone();
        app.db().write(move |tx| {
            for table in ["users","rooms"] {for (id,attrs) in setup[table].as_object().unwrap() {for (key,value) in attrs.as_object().unwrap() {
                tx.conn().execute(&format!("UPDATE {table} SET {key}=? WHERE id=?"),rusqlite::params![value.as_str().unwrap(),id.parse::<i64>().unwrap()])?;
            }}}
            for (key,value) in setup["account"].as_object().unwrap() {tx.conn().execute(&format!("UPDATE accounts SET {key}=?"),[value.as_str().unwrap()])?;}
            Ok(())
        }).await.unwrap();
    }
    let now = SEED_NOW.parse().unwrap();
    let (user, account, memberships, appearance, mut sections, avatar) = app
        .db()
        .read(move |c| {
            let user = campfire_db::User::find(c, DAVID)?;
            Ok((
                user.clone(),
                campfire_db::Account::first(c)?,
                presenters::accounts::profile_memberships(c, &user)?,
                campfire_db::models::user::profile_settings::appearance(c, DAVID)?,
                presenters::profile_sections::load(c, DAVID, now)?,
                presenters::attachments::attached_blob(c, "User", DAVID, "avatar")?.is_some(),
            ))
        })
        .await
        .unwrap();
    let data = users::AppearanceData {
        theme: appearance.theme,
        text_size: appearance.text_size,
        zone_identifier: appearance
            .time_zone
            .as_deref()
            .and_then(campfire_db::models::user::profile_settings::zone_identifier),
        theme_errors: vec![],
        text_size_errors: vec![],
        time_zone_errors: vec![],
    };
    let security = app
        .db()
        .read(move |c| {
            Ok(campfire_views::two_factor::ProfileData {
                confirmed_at: campfire_db::TwoFactorCredential::for_user(c, DAVID)?
                    .and_then(|c| c.confirmed_at)
                    .map(|t| t.jiff()),
                devices: campfire_db::TwoFactorRememberedDevice::for_user(c, DAVID)?
                    .into_iter()
                    .filter(|d| d.expires_at.jiff() > now)
                    .map(|d| campfire_views::two_factor::Device {
                        id: d.id,
                        user_agent: d.user_agent,
                        ip_address: d.ip_address,
                        last_used_at: d.last_used_at.map(|t| t.jiff()),
                    })
                    .collect(),
                google: false,
            })
        })
        .await
        .unwrap();
    let (preferences, chrome) = app.db().read(move |c| Ok((
        presenters::view_context::user_preferences(c, DAVID, now)?,
        presenters::view_context::chrome(c, Some(DAVID))?,
    ))).await.unwrap();
    let github = presenters::github::connection(&app.booted.app, DAVID).await.unwrap();
    let settings = app.db().read(move |conn| {
        let user = campfire_db::UserStatusSettings::find(conn, DAVID)?;
        presenters::status_settings::forms(conn, &user, campfire_db::Errors::default(), campfire_db::Timestamp::from_jiff(now), false)
    }).await.unwrap();
    sections.fizzy = presenters::fizzy_profile::connection(&app.booted.app, DAVID).await.unwrap();
    let expected=if markup {&v["markup"]["html"]} else {&v["html"]};
    let summary = presenters::user_summary(&app.booted.app.secrets, &user);
    let transfer = presenters::accounts::transfer_id(&app.booted.app.secrets, DAVID, now);
    let mut current = presenters::view_context::current_user(&app.booted.app.secrets, &user);
    current.preferences = preferences;
    let actual = h::request_forgery::rendering_with(
        h::request_forgery::RequestSecrets {
            tokens: Box::new(Tokens),
            csp_nonce: Some("NONCE".into()),
        },
        || {
            presenters::page::render_detached_at(
                &app.booted.app,
                account.as_ref(),
                "http://campfire.test",
                |ctx| {
                    let mut ctx = ctx_copy(ctx);
                    ctx.current_user = Some(current);
                    ctx.app_version = "parity".into();
                    ctx.platform = campfire_views::Platform {
                        mac: true,
                        chrome: true,
                        desktop: true,
                        browser: "Chrome".into(),
                        operating_system: "macOS".into(),
                        ..Default::default()
                    };
                    ctx.vapid_public_key = include_str!("../../../../../parity/.env.reference")
                        .lines()
                        .find_map(|l| l.strip_prefix("VAPID_PUBLIC_KEY="))
                        .map(str::to_owned);
                    ctx.chrome = chrome;
                    users::ProfileShow {
                        ctx: &ctx,
                        user: summary,
                        sections,
                        github,
                        settings,
                        appearance: data,
                        has_password: true,
                        current_password_error: None,
                        security,
                        now,
                        avatar_attached: avatar,
                        transfer_id: transfer,
                        direct_memberships: memberships.0,
                        shared_memberships: memberships.1,
                    }
                    .render()
                    .unwrap()
                },
            )
        },
    );
    if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
        std::fs::create_dir_all(&dir).unwrap();
        std::fs::write(format!("{dir}/profile.actual"), &actual).unwrap();
        std::fs::write(
            format!("{dir}/profile.expected"),
            expected.as_str().unwrap(),
        )
        .unwrap();
    }
    assert!(
        actual == expected.as_str().unwrap(),
        "complete Rails profile bytes differ; set WS8BR2_DIFF_DIR for the raw diff"
    );
}
#[tokio::test]
async fn whole_profile_matches_rails_seed_without_masks() {profile_case(false).await;}
#[tokio::test]
async fn review_whole_profile_markup_matches_rails() {profile_case(true).await;}
#[tokio::test]
async fn profile_route_renders_owner_sections_and_ws9_security_directly() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let page = app.david().get("/users/me/profile").await;
    assert_eq!(page.status, axum::http::StatusCode::OK);
    for section in [
        "Notifications",
        "Calls",
        "Status",
        "Do not disturb",
        "Appearance",
        "Google Calendar",
        "GitHub",
        "Fizzy",
        "Slack import",
        "Two-step sign-in",
        "Your sessions",
    ] {
        assert!(page.text().contains(section), "missing {section}");
    }
    assert!(page.text().contains("Shipping Rust"));
    assert!(
        page.text()
            .contains("Connected as David (Parity workspace)")
    );
}
fn ctx_copy<'a>(ctx: &campfire_views::ViewContext<'a>) -> campfire_views::ViewContext<'a> {
    campfire_views::ViewContext {
        current_user: ctx.current_user.clone(),
        account: ctx.account.clone(),
        flash_notice: ctx.flash_notice.clone(),
        flash_alert: ctx.flash_alert.clone(),
        platform: ctx.platform.clone(),
        vapid_public_key: ctx.vapid_public_key.clone(),
        asset_path: ctx.asset_path,
        importmap_tags: ctx.importmap_tags,
        stylesheet_tags: ctx.stylesheet_tags,
        custom_styles: ctx.custom_styles.clone(),
        cable_url: ctx.cable_url.clone(),
        base_url: ctx.base_url.clone(),
        request_url: ctx.request_url.clone(),
        referrer: ctx.referrer.clone(),
        last_room_visited_id: ctx.last_room_visited_id,
        app_version: ctx.app_version.clone(),
        signed_stream_name: ctx.signed_stream_name,
        time_zone: ctx.time_zone.clone(),
        chrome: ctx.chrome.clone(),
    }
}
#[tokio::test]
async fn edge_only_user_agent_gets_rails_edge_install_instructions() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let page = app
        .david()
        .send(
            Req::new(axum::http::Method::GET, "/users/me/profile")
                .header("user-agent", "Edge/124.0.0.0"),
        )
        .await;
    assert_eq!(page.status, axum::http::StatusCode::OK);
    assert!(page.text().contains("install-edge-"));
    assert!(page.text().contains("Click <em>Install</em>"));
}
#[tokio::test]
async fn dnd_switch_tracks_expired_and_live_timers() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for (expiry, checked) in [
        ("2026-03-02 15:00:00", false),
        ("2026-03-02 17:00:00", true),
    ] {
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET dnd_enabled=1,dnd_until=? WHERE id=?",
                    rusqlite::params![expiry, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let page = app.david().get("/users/me/profile").await;
        assert_eq!(page.status, axum::http::StatusCode::OK);
        let body = page.text();
        let field = body
            .split("id=\"user_dnd_enabled\"")
            .nth(1)
            .unwrap()
            .split('>')
            .next()
            .unwrap();
        assert_eq!(field.contains("checked=\"checked\""), checked);
    }
}
#[tokio::test]
async fn rejected_owner_settings_show_errors_without_persisting_changes() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET github_login='shared-login' WHERE id=?",
                [JASON],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for fields in [
        [("user[github_login]", "Shared-Login")],
        [("user[voice_mode]", "shout")],
        [("user[inbox_preferences][github_review_requests]", "banana")],
    ] {
        let page = app
            .david()
            .write(Req::new(axum::http::Method::PATCH, "/users/me/profile").form(&fields))
            .await;
        assert_eq!(page.status, axum::http::StatusCode::UNPROCESSABLE_ENTITY);
        if fields[0].0 == "user[github_login]" {
            assert!(page.text().contains("already linked to another user"));
        } else {
            assert!(page.text().contains("color: var(--color-negative)"));
        }
    }
    let facts = app
        .db()
        .read(|c| presenters::profile_sections::load(c, DAVID, SEED_NOW.parse().unwrap()))
        .await
        .unwrap();
    assert_eq!(facts.github_login, None);
    assert_eq!(facts.voice_mode, "voice_activity");
    assert!(facts.inbox[0].enabled);
}
