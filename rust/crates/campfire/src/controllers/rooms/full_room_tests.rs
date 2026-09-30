use crate::controllers::presenters::{page, test_support::TestApp, view_context};
use askama::Template;
use campfire_views::{
    helpers::request_forgery::{RequestSecrets, rendering_with},
    rooms::{Show, ShowView},
};
#[tokio::test]
async fn full_room_pages_match_twenty_complete_rails_pages() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let v: serde_json::Value =
        serde_json::from_str(include_str!("full_room_vectors.json")).unwrap();
    assert_eq!(v["cases"].as_array().unwrap().len(), 20);
    for case in v["cases"].as_array().unwrap() {
        let show: ShowView = serde_json::from_value(case["input"].clone()).unwrap();
        let id = show.user.id;
        let now = test.booted.app.db.env().now();
        let (actor, account, prefs, icons, last, recents) = test
            .db()
            .read(move |c| {
                Ok((
                    campfire_db::User::find(c, id)?,
                    campfire_db::Account::first(c)?,
                    crate::controllers::presenters::runtime_chrome::preferences(
                        c,
                        id,
                        now,
                        view_context::user_preferences(c, id)?,
                    )?,
                    crate::controllers::presenters::client_icon_names(c)?,
                    RoomLast::get(c, id)?,
                    crate::controllers::presenters::runtime_chrome::recent_searches(c, Some(id))?,
                ))
            })
            .await
            .unwrap();
        let (html, content) = page::render_detached_at(
            &test.booted.app,
            account.as_ref(),
            "http://campfire.test",
            |base| {
                let mut ctx = base.clone();
                ctx.current_user = Some(campfire_views::CurrentUser {
                    preferences: prefs,
                    ..view_context::current_user(&test.booted.app.secrets, &actor)
                });
                ctx.last_room_visited_id = last;
                ctx.custom_styles = account.as_ref().and_then(|a| a.custom_styles.clone());
                ctx.chrome.service_worker_auto_register = true;
                ctx.chrome.brand_icon_names = icons;
                ctx.chrome.recent_searches = recents;
                ctx.chrome.huddle_configured = case["configured"].as_bool().unwrap();
                ctx.vapid_public_key = case["providers"]["vapid_public_key"]
                    .as_str()
                    .map(str::to_string);
                ctx.platform.browser = "Mozilla".into();
                rendering_with(
                    RequestSecrets {
                        tokens: Box::new(super::call_page_tests::Tokens),
                        csp_nonce: Some("NONCE".into()),
                    },
                    || {
                        let p = Show {
                            ctx: &ctx,
                            show: &show,
                        };
                        (p.render().unwrap(), p.as_content().to_string())
                    },
                )
            },
        );
        for (part, actual) in [("content", content), ("html", html)] {
            let expected = case[part].as_str().unwrap();
            if !crate::app::asset_goldens::compare(
                &format!("{} {part}", case["name"]),
                &actual,
                expected,
            ) {
                let dir = std::path::PathBuf::from(std::env::var_os("TMPDIR").unwrap());
                std::fs::write(dir.join("full-room-actual.html"), &actual).unwrap();
                std::fs::write(dir.join("full-room-expected.html"), expected).unwrap();
                panic!("{} {part}: see full-room-actual/expected", case["name"]);
            }
        }
    }
    test.booted
        .jobs
        .shutdown(std::time::Duration::from_secs(1))
        .await;
}
struct RoomLast;
impl RoomLast {
    fn get(c: &campfire_db::Connection, id: i64) -> campfire_db::Result<Option<i64>> {
        Ok(campfire_db::Room::original_for_user(c, id)?.map(|r| r.id))
    }
}
