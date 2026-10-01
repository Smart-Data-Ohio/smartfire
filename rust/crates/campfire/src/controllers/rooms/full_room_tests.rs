use crate::controllers::presenters::{page, test_support::TestApp, view_context};
use askama::Template;
use campfire_views::{
    helpers::request_forgery::{RequestSecrets, rendering_with},
    rooms::{Show, ShowView},
};
#[tokio::test]
async fn full_room_pages_match_thirty_eight_complete_rails_pages() {
    let Some(test) = TestApp::boot().await else {
        return;
    };
    let v: serde_json::Value =
        serde_json::from_str(include_str!("full_room_vectors.json")).unwrap();
    assert_eq!(v["cases"].as_array().unwrap().len(), 38);
    assert!(v["deferred"].as_array().unwrap().is_empty());
    let mut mismatches = Vec::new();
    for case in v["cases"].as_array().unwrap() {
        let mut show: ShowView = serde_json::from_value(case["input"].clone()).unwrap();
        if let Some(at) = case["membership_unread_at"].as_str() {
            let at = campfire_db::Timestamp::parse_db(at).unwrap();
            let room_id = show.room.id;
            let user_id = show.user.id;
            test.db().write(move |tx| {
                tx.conn().execute("UPDATE memberships SET unread_at=?,last_read_message_id=NULL WHERE room_id=? AND user_id=?",rusqlite::params![at,room_id,user_id])?;
                Ok(())
            }).await.unwrap();
        }
        if let Some(ids) = case["message_ids"].as_array() {
            let expected_ids: Vec<i64> = ids.iter().map(|id| id.as_i64().unwrap()).collect();
            let app = test.booted.app.clone();
            let room_id = show.room.id;
            let user_id = show.user.id;
            let (room, navigation, shell, items) = test
                .db()
                .read(move |conn| {
                    use campfire_db::{Message, Room, Timeline, User};
                    let room = Room::find(conn, room_id)?;
                    let user = User::find(conn, user_id)?;
                    let messages = Message::last_page(conn, Timeline::Room(room_id))?;
                    assert_eq!(
                        messages.iter().map(|m| m.id).collect::<Vec<_>>(),
                        expected_ids
                    );
                    let mut presenter = crate::controllers::presenters::Presenter::new(
                        conn,
                        &app,
                        Some("campfire.test".into()),
                    );
                    presenter.cache_base_url = Some("http://campfire.test/".into());
                    Ok((
                        presenter.room_view(&room, &user)?,
                        super::call_navigation::model(&app, conn, &room, &user)?,
                        super::shell::load(
                            conn,
                            &room,
                            user_id,
                            &messages,
                            campfire_db::Timestamp::from_second(1772467200),
                        )?,
                        presenter.messages(&messages)?,
                    ))
                })
                .await
                .unwrap();
            assert_eq!(room, show.room, "{} room adapter", case["name"]);
            assert_eq!(
                Some(&navigation),
                show.navigation.as_ref(),
                "{} header adapter",
                case["name"]
            );
            assert_eq!(shell, show.shell, "{} shell adapter", case["name"]);
            show.messages = items;
        }
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
                let label = format!("{}-{part}", case["name"].as_str().unwrap());
                std::fs::write(dir.join(format!("{label}-actual.html")), &actual).unwrap();
                std::fs::write(dir.join(format!("{label}-expected.html")), expected).unwrap();
                mismatches.push(label);
            }
        }
    }
    assert!(
        mismatches.is_empty(),
        "whole-page mismatches: {mismatches:?}"
    );
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
