use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};
fn vectors() -> Value {
    serde_json::from_str(include_str!("../../../../../vectors/users_joining.json")).unwrap()
}
/// The join fields as the SPA's page posts them (`POST /api/v1/join/{code}`).
fn join_json(code: &str, fields: Value) -> Req {
    Req::new(Method::POST, &format!("/api/v1/join/{code}")).json_body(&fields)
}
#[tokio::test]
async fn join_page_preserves_rails_access_checks() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let v = vectors();
    // The retained page goes to the SPA's, which answers with Rails' statuses; the code is only
    // checked there.
    let path = format!("/join/{}", v["join"].as_str().unwrap());
    for (retained, status) in [(path.as_str(), &v["valid_get"]), ("/join/wrong-code", &v["wrong_get"])] {
        let reply = app.anonymous().get(retained).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{retained}");
        let spa = format!("http://campfire.test/app{retained}");
        assert_eq!(reply.location(), Some(spa.as_str()), "{retained}");
        assert_eq!(
            app.anonymous().get(&format!("/app{retained}")).await.status.as_u16(),
            status["status"].as_u64().unwrap() as u16,
            "{retained}"
        );
    }
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
        let code = v["join"].as_str().unwrap();
        let path = format!("/join/{code}");
        let mut browser = app.anonymous();
        browser.get(&format!("/app{path}")).await;
        let count = app.db().read(campfire_db::User::count).await.unwrap();
        // The retained form goes, unread, to the SPA's page: nothing is created.
        let retained = browser
            .write(Req::new(Method::POST, &path).form(&[
                ("user[name]", "New Person"),
                ("user[email_address]", "new@37signals.com"),
                ("user[password]", "secret123456"),
            ]))
            .await;
        assert_eq!(retained.status, StatusCode::SEE_OTHER);
        let spa = format!("http://campfire.test/app{path}");
        assert_eq!(retained.location(), Some(spa.as_str()));
        assert_eq!(
            app.db().read(campfire_db::User::count).await.unwrap(),
            count
        );
        // Rails' writes, through the SPA's JSON contract.
        let duplicate = browser
            .write(join_json(code, json!({
                "name": "Another David",
                "emailAddress": "david@37signals.com",
                "password": "secret123456",
            })))
            .await;
        assert_eq!(duplicate.status, StatusCode::OK, "{}", duplicate.text());
        assert_eq!(
            duplicate.json(),
            json!({"kind": "navigate", "location": spa_auth_location(v["duplicate"]["location"].as_str().unwrap())})
        );
        assert_eq!(
            app.db().read(campfire_db::User::count).await.unwrap(),
            count
        );
        let wrong = browser
            .write(join_json("wrong-code", json!({
                "name": "Must not save",
                "emailAddress": "nobody@example.com",
                "password": "secret123456",
            })))
            .await;
        assert_eq!(wrong.status.as_u16(), v["wrong_post"]["status"].as_u64().unwrap() as u16);
        assert_eq!(
            app.db().read(campfire_db::User::count).await.unwrap(),
            count
        );
        let mut signup = json!({
            "name": "New Person",
            "emailAddress": "new@37signals.com",
            "password": "secret123456",
        });
        if attempt_role_escalation {
            signup["role"] = json!("administrator");
        }
        let valid = browser.write(join_json(code, signup)).await;
        assert_eq!(valid.status, StatusCode::OK, "{}", valid.text());
        assert_eq!(
            valid.json(),
            json!({"kind": "signedIn", "location": v["valid"]["location"]})
        );
        assert!(browser.cookie_header().contains("session_token="));
        // Authenticate the freshly issued cookie through the real middleware, rather than
        // equating the existence of some session row with ownership of this browser.
        let self_profile = browser.get("/users/me/profile").await;
        // The new unenrolled human is required to complete 2FA by the approved WS9 drift.
        assert_eq!(
            self_profile.location(),
            Some("http://campfire.test/app/two_factor/setup?required=true")
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
        let enrolled_page = browser.get("/app/two_factor/setup").await;
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
        assert_eq!(
            crate::controllers::spa::signed_out_contracts_tests::inline_boot(&enrolled_page)["user"]["id"],
            id,
            "the issued cookie authenticates the newly created user"
        );
        assert_eq!(
            browser.get(&path).await.location().map(str::to_owned),
            v["signed_get"]["location"].as_str().map(spa_auth_location)
        );
    }
}
