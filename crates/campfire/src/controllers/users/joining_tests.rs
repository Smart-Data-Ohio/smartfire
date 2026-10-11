use super::people_tests::render_with;
use crate::controllers::presenters::{self, test_support::*};
use askama::Template;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_joining.json")).unwrap()
}
#[tokio::test]
async fn join_page_preserves_rails_form_and_access_checks() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let v = vectors();
    let (account, help) = app
        .db()
        .read(|c| {
            Ok((
                campfire_db::Account::first(c)?,
                presenters::accounts::help_contact(c)?,
            ))
        })
        .await
        .unwrap();
    let actual = render_with(
        &app,
        |ctx| {
            ctx.current_user = None;
            ctx.account = presenters::view_context::account_summary(account.as_ref(), false);
        },
        |ctx| {
            campfire_retained::users::New {
                ctx,
                join_path: campfire_routes::join(v["join"].as_str().unwrap()),
                description: String::new(),
                help_contact: help,
                invite_error: None,
            }
            .as_content()
            .render()
            .unwrap()
        },
    );
    crate::form_contracts::assert_forms("join", &actual, v["html"].as_str().unwrap());
    crate::form_contracts::assert_text(&actual, &account.as_ref().unwrap().name);
    let path = format!("/join/{}", v["join"].as_str().unwrap());
    assert_eq!(
        app.anonymous().get(&path).await.status.as_u16(),
        v["valid_get"]["status"].as_u64().unwrap() as u16
    );
    assert_eq!(
        app.anonymous().get("/join/wrong-code").await.status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.david().get(&path).await.location(),
        Some("http://campfire.test/")
    );
}
#[tokio::test]
async fn join_writes_match_rails_duplicate_scope_open_rooms_and_sessions() {
    for attempt_role_escalation in [false, true] {
        let app = TestApp::boot_frozen().await.expect("seed required");
        let v = vectors();
        let path = format!("/join/{}", v["join"].as_str().unwrap());
        let mut browser = app.anonymous();
        browser.get(&path).await;
        let count = app.db().read(campfire_db::User::count).await.unwrap();
        let duplicate = browser
            .write(Req::new(Method::POST, &path).form(&[
                ("user[name]", "Another David"),
                ("user[email_address]", "david@37signals.com"),
                ("user[password]", "secret123456"),
            ]))
            .await;
        assert_eq!(duplicate.status, StatusCode::FOUND);
        assert_eq!(duplicate.location(), v["duplicate"]["location"].as_str());
        assert_eq!(
            app.db().read(campfire_db::User::count).await.unwrap(),
            count
        );
        let wrong = browser
            .write(Req::new(Method::POST, "/join/wrong-code").form(&[
                ("user[name]", "Must not save"),
                ("user[password]", "secret123456"),
            ]))
            .await;
        assert_eq!(wrong.status, StatusCode::NOT_FOUND);
        assert_eq!(
            app.db().read(campfire_db::User::count).await.unwrap(),
            count
        );
        let mut signup = vec![
            ("user[name]", "New Person"),
            ("user[email_address]", "new@37signals.com"),
            ("user[password]", "secret123456"),
        ];
        if attempt_role_escalation {
            signup.push(("user[role]", "administrator"));
        }
        let valid = browser
            .write(Req::new(Method::POST, &path).form(&signup))
            .await;
        assert_eq!(valid.status, StatusCode::FOUND);
        assert_eq!(valid.location(), v["valid"]["location"].as_str());
        assert!(browser.cookie_header().contains("session_token="));
        // Authenticate the freshly issued cookie through the real middleware, rather than
        // equating the existence of some session row with ownership of this browser.
        let self_profile = browser.get("/users/me/profile").await;
        // The new unenrolled human is required to complete 2FA by the approved WS9 drift.
        assert_eq!(
            self_profile.location(),
            Some("http://campfire.test/two_factor_setup")
        );
        let state=app.db().read(move |c| {
            let id=c.query_row("SELECT id FROM users WHERE email_address='new@37signals.com'",[],|r|r.get::<_,i64>(0))?;let user=campfire_db::User::find(c,id)?;
            let rooms=c.prepare("SELECT room_id FROM memberships WHERE user_id=? ORDER BY room_id")?.query_map([id],|r|r.get::<_,i64>(0))?.collect::<Result<Vec<_>,_>>()?;
            let sessions=campfire_db::Session::for_user(c,id)?;
            Ok(json!({"user_delta":campfire_db::User::count(c)?-count,"role":user.role.name(),"room_ids":rooms,"session_count":sessions.len()}))
        }).await.unwrap();
        for key in ["user_delta", "role", "room_ids", "session_count"] {
            assert_eq!(state[key], v["valid"][key], "{key}");
        }
        assert_eq!(state["room_ids"], v["valid"]["open_ids"]);
        let enrolled_page = browser.get("/two_factor_setup").await;
        assert_eq!(enrolled_page.status, StatusCode::OK);
        let id = app
            .db()
            .read(|c| {
                Ok(c.query_row(
                    "SELECT id FROM users WHERE email_address='new@37signals.com'",
                    [],
                    |r| r.get::<_, i64>(0),
                )?)
            })
            .await
            .unwrap();
        assert!(
            enrolled_page
                .text()
                .contains(&format!("name=\"current-user-id\" content=\"{id}\"")),
            "the issued cookie authenticates the newly created user"
        );
        assert_eq!(
            browser.get(&path).await.location(),
            v["signed_get"]["location"].as_str()
        );
    }
}
