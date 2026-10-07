//! Auth form/behavior contracts against Rails vectors; unrelated classic pages keep byte goldens.
use crate::controllers::presenters::test_support::TestApp;
use askama::Template;
use campfire_views::{helpers as h, layouts, sessions, sudos, two_factor, users};
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
#[tokio::test]
async fn complete_auth_templates_preserve_rails_forms_and_visible_behaviour() {
    let a = TestApp::boot()
        .await
        .expect("build the pinned WS19 default seed");
    let vectors: Value =
        serde_json::from_str(include_str!("../../../../vectors/auth_full_pages.json")).unwrap();
    let id = vectors["user_id"].as_i64().unwrap();
    let (user, account, searches) = a
        .db()
        .read(move |c| {
            Ok((
                campfire_db::User::find(c, id)?,
                campfire_db::Account::first(c)?,
                campfire_db::Search::ordered_for_user(c, id)?,
            ))
        })
        .await
        .unwrap();
    let help = a
        .db()
        .read(crate::controllers::presenters::accounts::help_contact)
        .await
        .unwrap();
    let qr = crate::controllers::qr_code::two_factor_svg(vectors["uri"].as_str().unwrap()).unwrap();
    let now = "2026-03-02T16:00:00Z".parse().unwrap();
    let env = include_str!("../../../../parity/.env.reference");
    let vapid = env
        .lines()
        .find_map(|l| l.strip_prefix("VAPID_PUBLIC_KEY="))
        .unwrap();
    let prefs = &vectors["user"];
    assert_eq!(vectors["pages"].as_object().unwrap().len(), 15);
    let mut mismatches = Vec::new();
    for (name, expected) in vectors["pages"].as_object().unwrap() {
        let actual = h::request_forgery::rendering_with(
            h::request_forgery::RequestSecrets {
                tokens: Box::new(Tokens),
                csp_nonce: Some("NONCE".into()),
            },
            || {
                crate::controllers::presenters::page::render_detached_at(
                    &a.booted.app,
                    account.as_ref(),
                    "http://campfire.test",
                    |ctx| {
                        let mut ctx = ctx_copy(ctx);
                        ctx.app_version = "parity".into();
                        ctx.vapid_public_key = Some(vapid.into());
                        ctx.platform = campfire_views::Platform {
                            mac: true,
                            chrome: true,
                            desktop: true,
                            browser: "Chrome".into(),
                            operating_system: "macOS".into(),
                            ..Default::default()
                        };
                        ctx.chrome.service_worker_auto_register = true;
                        ctx.chrome.brand_icon_names = vectors["brand_icon_names"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_str().unwrap().into())
                            .collect();
                        if !["challenge", "sign_in", "incompatible_browser", "transfer"]
                            .contains(&name.as_str())
                        {
                            let mut current =
                                crate::controllers::presenters::view_context::current_user(
                                    &a.booted.app.secrets,
                                    &user,
                                );
                            assert_eq!(current.avatar_url, prefs["avatar_path"].as_str().unwrap());
                            current.preferences = layouts::UserPreferences {
                                theme: prefs["theme"].as_str().map(str::to_string),
                                text_size: prefs["text_size"].as_str().map(str::to_string),
                                time_zone: prefs["time_zone"].as_str().map(str::to_string),
                                time_zone_explicit: prefs["time_zone_explicit"].as_bool().unwrap(),
                                tour_completed: prefs["tour_completed"].as_bool().unwrap(),
                                voice_mode: prefs["voice_mode"].as_str().map(str::to_string),
                                push_to_talk_key: prefs["push_to_talk_key"]
                                    .as_str()
                                    .map(str::to_string),
                                ..Default::default()
                            };
                            ctx.current_user = Some(current);
                            ctx.chrome.recent_searches = searches
                                .iter()
                                .take(10)
                                .map(|s| layouts::RecentSearch {
                                    id: s.id,
                                    query: s.query.clone(),
                                })
                                .collect();
                        }
                        match name.as_str() {
                            "sign_in" => sessions::New {
                                                        ctx: &ctx,
                                email_address: None,
                                help_contact: help.clone(),
                                google_sign_in_domains: Vec::new(),
                            }
                            .render()
                            .unwrap(),
                            "incompatible_browser" => sessions::IncompatibleBrowser { ctx: &ctx }
                                .render()
                                .unwrap(),
                            "transfer" => sessions::TransferShow {
                                ctx: &ctx,
                                action: "/session/transfers/some-token".into(),
                            }
                            .render()
                            .unwrap(),
                            "setup" => two_factor::Setup {
                                ctx: &ctx,
                                key: vectors["key"].as_str().unwrap().into(),
                                qr: qr.clone(),
                            }
                            .render()
                            .unwrap(),
                            "challenge" => two_factor::Challenge { ctx: &ctx }.render().unwrap(),
                            "backups" | "backups_signed_out" => two_factor::BackupCodes {
                                ctx: &ctx,
                                codes: vectors["codes"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .map(|v| v.as_str().unwrap().into())
                                    .collect(),
                                signed_out: usize::from(name == "backups_signed_out") * 2,
                                continue_url: if name == "backups" {
                                    "http://campfire.test/users/me/profile"
                                } else {
                                    "http://campfire.test/rooms/486777696"
                                }
                                .into(),
                            }
                            .render()
                            .unwrap(),
                            "sessions_one" | "sessions_two" => users::SessionsIndex {
                                ctx: &ctx,
                                now,
                                sessions: vectors["sessions"]
                                    .as_array()
                                    .unwrap()
                                    .iter()
                                    .take(if name == "sessions_one" { 1 } else { 2 })
                                    .map(|s| users::UserSession {
                                        id: s["id"].as_i64().unwrap(),
                                        current: s["current"].as_bool().unwrap(),
                                        description: s["description"].as_str().unwrap().into(),
                                        ip_address: s["ip_address"].as_str().map(str::to_string),
                                        created_at: s["created_at"]
                                            .as_str()
                                            .unwrap()
                                            .parse()
                                            .unwrap(),
                                        last_active_at: s["last_active_at"]
                                            .as_str()
                                            .unwrap()
                                            .parse()
                                            .unwrap(),
                                    })
                                    .collect(),
                            }
                            .render()
                            .unwrap(),
                            "sudo_continue" => sudos::Continue {
                                ctx: &ctx,
                                method: "patch".into(),
                                path: "/account/users/127326141?x=1&y=2".into(),
                                params: vectors["replay"].clone(),
                            }
                            .render()
                            .unwrap(),
                            "sudo_"
                            | "sudo_password"
                            | "sudo_totp"
                            | "sudo_google"
                            | "sudo_password_totp_google" => sudos::New {
                                ctx: &ctx,
                                password: name.contains("password"),
                                totp: name.contains("totp"),
                                google: name.contains("google"),
                            }
                            .render()
                            .unwrap(),
                            other => panic!("unexpected full-page case {other}"),
                        }
                    },
                )
            },
        );
        let expected = expected.as_str().unwrap();
        if ["incompatible_browser", "sessions_one", "sessions_two"].contains(&name.as_str()) {
            if !super::asset_goldens::compare(name, &actual, expected) {
                let at = actual
                    .bytes()
                    .zip(expected.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                eprintln!(
                    "{name}: byte {at}; Rust {} bytes, Rails {} bytes",
                    actual.len(),
                    expected.len()
                );
                if let Ok(dir) = std::env::var("WS9_PAGE_DIFF_DIR") {
                    std::fs::create_dir_all(&dir).unwrap();
                    std::fs::write(format!("{dir}/{name}.actual"), &actual).unwrap();
                    std::fs::write(format!("{dir}/{name}.expected"), expected).unwrap();
                }
                mismatches.push(name);
            }
        } else {
            crate::form_contracts::assert_head(name, &actual, expected);
            let content = crate::form_contracts::page_content(&actual);
            crate::form_contracts::assert_forms(
                name, content, crate::form_contracts::page_content(expected),
            );
            if name == "sign_in" {
                crate::form_contracts::assert_text(content, &account.as_ref().unwrap().name);
            }
            if name == "setup" {
                crate::form_contracts::assert_text(content, vectors["key"].as_str().unwrap());
                assert!(content.contains(&qr));
            }
            if name.starts_with("backups") {
                for code in vectors["codes"].as_array().unwrap() {
                    crate::form_contracts::assert_text(content, code.as_str().unwrap());
                }
                crate::form_contracts::assert_text(content, "They will not be shown again.");
                if name == "backups_signed_out" {
                    crate::form_contracts::assert_text(content, "Signed out your other devices");
                }
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "full-page mismatches: {mismatches:?}"
    );
}
/// Borrow detached asset/signing functions while supplying the real seed's request state.
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
