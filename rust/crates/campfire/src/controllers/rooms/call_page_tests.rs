use crate::controllers::presenters::test_support::{DAVID, KEVIN, TestApp};
use axum::http::StatusCode;
use campfire_db::{Room, RoomType};

#[tokio::test]
async fn stage_page_composes_listener_permissions_and_sti_targets() {
    let Some(test) = TestApp::boot_with_huddle(super::call_channel_tests::configured()).await
    else {
        return;
    };
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Town Hall <&>"),
                DAVID,
                &[DAVID, KEVIN],
            )
        })
        .await
        .unwrap();
    let mut listener = test.sign_in(KEVIN).await;
    let response = listener.get(&format!("/rooms/{}", room.id)).await;
    assert_eq!(response.status, StatusCode::OK);
    let html = response.text();
    assert!(html.contains("data-huddle-can-publish-param=\"false\""));
    assert!(html.contains(&format!("id=\"header_rooms_stage_{}\"", room.id)));
    assert!(html.contains(&format!("href=\"/rooms/stages/{}/edit\"", room.id)));
    assert!(html.contains("Town Hall &lt;&amp;&gt;"));
    assert!(!html.contains("Town Hall <&>"));
    assert!(html.contains("You are in the audience"));
    assert!(!html.contains("Invite to speak"));
    assert!(!html.contains("Go live"));
}

pub(super) struct Tokens;
impl campfire_views::helpers::request_forgery::AuthenticityTokens for Tokens {
    fn global(&self) -> String {
        "GLOBAL".into()
    }
    fn for_form(&self, action: &str, method: &str) -> String {
        format!("{method}:{action}")
    }
}
#[tokio::test]
async fn complete_call_headers_match_twenty_eight_rails_renders() {
    use crate::controllers::presenters::{page, view_context};
    use campfire_views::{
        helpers::request_forgery::{RequestSecrets, rendering_with},
        rooms::navigation::Navigation,
    };
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let user = test
        .db()
        .read(|conn| campfire_db::User::find(conn, DAVID))
        .await
        .unwrap();
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("page_view_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 28);
    for case in vectors["cases"].as_array().unwrap() {
        let nav: Navigation = serde_json::from_value(case["input"].clone()).unwrap();
        let actual =
            page::render_detached_at(&test.booted.app, None, "http://campfire.test", |ctx| {
                let mut ctx = campfire_views::ViewContext {
                    current_user: Some(view_context::current_user(&test.booted.app.secrets, &user)),
                    account: ctx.account.clone(),
                    flash_notice: None,
                    flash_alert: None,
                    platform: campfire_views::Platform {
                        browser: "Mozilla".into(),
                        ..Default::default()
                    },
                    vapid_public_key: ctx.vapid_public_key.clone(),
                    asset_path: ctx.asset_path,
                    importmap_tags: ctx.importmap_tags,
                    stylesheet_tags: ctx.stylesheet_tags,
                    custom_styles: None,
                    cable_url: ctx.cable_url.clone(),
                    base_url: ctx.base_url.clone(),
                    request_url: ctx.request_url.clone(),
                    referrer: None,
                    last_room_visited_id: None,
                    app_version: ctx.app_version.clone(),
                    signed_stream_name: ctx.signed_stream_name,
                    time_zone: campfire_views::time::Zone::utc(),
                    chrome: Default::default(),
                };
                ctx.chrome.huddle_configured = case["input"]["configured"].as_bool().unwrap();
                rendering_with(
                    RequestSecrets {
                        tokens: Box::new(Tokens),
                        csp_nonce: Some("NONCE".into()),
                    },
                    || nav.render(&ctx),
                )
            });
        let expected = case["html"].as_str().unwrap();
        if actual != expected {
            let scratch =
                std::path::PathBuf::from(std::env::var_os("TMPDIR").expect("worker scratch"));
            std::fs::write(scratch.join("nav-actual.html"), &actual).unwrap();
            std::fs::write(scratch.join("nav-expected.html"), expected).unwrap();
            let offset = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            panic!(
                "{} first difference {offset}: actual {:?} expected {:?}",
                case["name"],
                actual.get(offset.saturating_sub(40)..(offset + 100).min(actual.len())),
                expected.get(offset.saturating_sub(40)..(offset + 100).min(expected.len()))
            );
        }
    }
}

#[tokio::test]
async fn complete_voice_and_stage_form_pages_match_fourteen_rails_renders() {
    use crate::controllers::presenters::{self, page, view_context};
    use askama::Template;
    use campfire_views::{
        helpers::request_forgery::{RequestSecrets, rendering_with},
        rooms::{
            FormRoom,
            calls::{CallForm, StagesEdit, StagesNew, VoicesEdit, VoicesNew},
        },
    };
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("form_page_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 14);
    for case in vectors["cases"].as_array().unwrap() {
        let input = &case["input"];
        let actor_id = input["actor_id"].as_i64().unwrap();
        let now = test.booted.app.clock.now();
        let (user, mut preferences, account, has_logo, last_room, icons) = test
            .db()
            .read(move |conn| {
                let account = campfire_db::Account::first(conn)?;
                let has_logo = account
                    .as_ref()
                    .map(|a| {
                        presenters::attachments::attached_blob(conn, "Account", a.id, "logo")
                            .map(|b| b.is_some())
                    })
                    .transpose()?
                    .unwrap_or(false);
                Ok((
                    campfire_db::User::find(conn, actor_id)?,
                    view_context::user_preferences(conn, actor_id, now)?,
                    account,
                    has_logo,
                    Room::original_for_user(conn, actor_id)?.map(|r| r.id),
                    presenters::client_icon_names(conn)?,
                ))
            })
            .await
            .unwrap();
        let providers = &input["providers"];
        preferences.google_drive = providers["google_drive"].as_bool().unwrap();
        preferences.notification_sounds = campfire_views::layouts::NotificationSounds {
            muted: providers["notification_muted"].as_bool().unwrap(),
            quiet_hours: serde_json::from_value(providers["quiet_hours"].clone()).unwrap(),
            meeting_quiet: serde_json::from_value(providers["meeting_quiet"].clone()).unwrap(),
            ooo_quiet: serde_json::from_value(providers["ooo_quiet"].clone()).unwrap(),
        };
        let recent_searches: Vec<_> = providers["recent_searches"]
            .as_array()
            .unwrap()
            .iter()
            .map(|r| campfire_views::layouts::RecentSearch {
                id: r["id"].as_i64().unwrap(),
                query: r["query"].as_str().unwrap().into(),
            })
            .collect();
        let form = CallForm {
            room: FormRoom {
                id: input["id"].as_i64(),
                name: input["name"].as_str().map(str::to_string),
            ..Default::default()},
            stage: input["stage"].as_bool().unwrap(),
            can_administer: input["can_administer"].as_bool().unwrap(),
            current_user_id: actor_id,
            selected_users: serde_json::from_value(input["selected_users"].clone()).unwrap(),
            unselected_users: serde_json::from_value(input["unselected_users"].clone()).unwrap(),
            icon_name: None,
            icon: None,
            errors: Vec::new(),
            settings: serde_json::from_value(input["settings"].clone()).unwrap(),
        };
        let (actual, content) = page::render_detached_at(
            &test.booted.app,
            account.as_ref(),
            "http://campfire.test",
            |base| {
                let current = campfire_views::CurrentUser {
                    preferences,
                    ..view_context::current_user(&test.booted.app.secrets, &user)
                };
                let ctx = campfire_views::ViewContext {
                    current_user: Some(current),
                    account: view_context::account_summary(account.as_ref(), has_logo),
                    flash_notice: None,
                    flash_alert: None,
                    platform: campfire_views::Platform {
                        browser: "Mozilla".into(),
                        ..Default::default()
                    },
                    vapid_public_key: providers["vapid_public_key"].as_str().map(str::to_string),
                    asset_path: base.asset_path,
                    importmap_tags: base.importmap_tags,
                    stylesheet_tags: base.stylesheet_tags,
                    custom_styles: account.as_ref().and_then(|a| a.custom_styles.clone()),
                    cable_url: base.cable_url.clone(),
                    base_url: base.base_url.clone(),
                    request_url: base.request_url.clone(),
                    referrer: None,
                    last_room_visited_id: last_room,
                    app_version: base.app_version.clone(),
                    signed_stream_name: base.signed_stream_name,
                    time_zone: campfire_views::time::Zone::utc(),
                    chrome: campfire_views::layouts::Chrome {
                        service_worker_auto_register: true,
                        brand_icon_names: icons,
                        huddle_configured: true,
                        recent_searches,
                        ..Default::default()
                    },
                };
                rendering_with(
                    RequestSecrets {
                        tokens: Box::new(Tokens),
                        csp_nonce: Some("NONCE".into()),
                    },
                    || match (form.stage, form.room.id.is_some()) {
                        (false, false) => {
                            let page = VoicesNew {
                                ctx: &ctx,
                                form: &form,
                            };
                            (page.render().unwrap(), page.as_content().to_string())
                        }
                        (false, true) => {
                            let page = VoicesEdit {
                                ctx: &ctx,
                                form: &form,
                            };
                            (page.render().unwrap(), page.as_content().to_string())
                        }
                        (true, false) => {
                            let page = StagesNew {
                                ctx: &ctx,
                                form: &form,
                            };
                            (page.render().unwrap(), page.as_content().to_string())
                        }
                        (true, true) => {
                            let page = StagesEdit {
                                ctx: &ctx,
                                form: &form,
                            };
                            (page.render().unwrap(), page.as_content().to_string())
                        }
                    },
                )
            },
        );
        for (part, actual) in [("content", content), ("html", actual)] {
            let expected = case[part].as_str().unwrap();
            if !crate::app::asset_goldens::compare(&format!("{} {part}",case["name"]),&actual,expected) {
                let scratch =
                    std::path::PathBuf::from(std::env::var_os("TMPDIR").expect("worker scratch"));
                std::fs::write(scratch.join("form-page-actual.html"), &actual).unwrap();
                std::fs::write(scratch.join("form-page-expected.html"), expected).unwrap();
                let offset = actual
                    .bytes()
                    .zip(expected.bytes())
                    .position(|(a, b)| a != b)
                    .unwrap_or(actual.len().min(expected.len()));
                panic!(
                    "{} {part} first difference {offset}: actual {:?} expected {:?}",
                    case["name"],
                    actual.get(offset.saturating_sub(40)..(offset + 130).min(actual.len())),
                    expected.get(offset.saturating_sub(40)..(offset + 130).min(expected.len()))
                );
            }
        }
    }
}
