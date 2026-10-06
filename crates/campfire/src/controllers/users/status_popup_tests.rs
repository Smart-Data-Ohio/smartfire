//! #163 bodies and real route authorization/state. No response normalization.
use crate::controllers::presenters::test_support::*;
use askama::Template;
use campfire_views::users;
fn vectors() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/users_status_popup.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn complete_popup_bodies_match_post_pin_rails() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    for case in vectors()["popup"].as_array().unwrap() {
        let fields = serde_json::from_value(case["fields"].clone()).unwrap();
        let actual = super::people_tests::render(&app, |_| {
            users::StatusEdit {
                user_id: DAVID,
                fields,
            }
            .render()
            .unwrap()
        });
        if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(
                format!("{dir}/{}.actual", case["name"].as_str().unwrap()),
                &actual,
            )
            .unwrap();
            std::fs::write(
                format!("{dir}/{}.expected", case["name"].as_str().unwrap()),
                case["html"].as_str().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(
            actual,
            case["html"].as_str().unwrap(),
            "{}: complete status-popup bytes",
            case["name"]
        );
    }
}
#[tokio::test]
async fn edit_is_current_user_only_and_requires_sign_in() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let response = app.anonymous().get("/users/me/status/edit").await;
    assert_eq!(response.status, axum::http::StatusCode::FOUND);
    assert!(response.location().unwrap().ends_with("/session/new"));
    for path in ["/users/me/status/edit", "/users/149087659/status/edit"] {
        let response = app.david().get(path).await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        assert!(
            response
                .text()
                .starts_with("<turbo-frame id=\"user_card\">")
        );
        assert!(response.text().contains("Shipping Rust"));
        assert!(response.text().contains("status_popup_presence_setting"));
        assert!(!response.text().contains("<html"));
        assert!(response.text().contains(&format!("/users/{DAVID}/card")));
    }
}
#[tokio::test]
async fn popup_update_matches_rails_redirects_errors_and_current_user_state() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.david();
    let token = browser.authenticity_token().await;
    for case in vectors()["http"].as_array().unwrap() {
        app.db().write(|tx| {
            tx.conn().execute("UPDATE users SET presence_setting='auto',custom_status_emoji='🚀',custom_status_text='Shipping Rust',custom_status_expires_at=NULL WHERE id=?",[DAVID])?;
            Ok(())
        }).await.unwrap();
        let before_other = app
            .db()
            .read(|c| campfire_db::models::user::status_form::StatusForm::load(c, 149087659))
            .await
            .unwrap()
            .attributes();
        let mut request = Req::new(axum::http::Method::PATCH, case["path"].as_str().unwrap())
            .header("content-type", "application/json")
            .body(serde_json::to_vec(&serde_json::json!({"user":case["params"]})).unwrap())
            .header("x-csrf-token", &token);
        if case["frame"] == true {
            request = request.header("turbo-frame", "user_card");
        }
        let reply = browser.send(request).await;
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{}",
            case["name"]
        );
        assert_eq!(
            reply.location(),
            case["location"].as_str(),
            "{}",
            case["name"]
        );
        let actual = app
            .db()
            .read(|c| campfire_db::models::user::status_form::StatusForm::load(c, DAVID))
            .await
            .unwrap()
            .attributes();
        let mut expected = case["state"].clone();
        if let Some(expiry) = expected["custom_status_expires_at"].as_str() {
            expected["custom_status_expires_at"] =
                serde_json::Value::String(expiry.strip_suffix(" UTC").unwrap().into());
        }
        assert_eq!(actual, expected, "{}", case["name"]);
        assert_eq!(
            app.db()
                .read(|c| campfire_db::models::user::status_form::StatusForm::load(c, 149087659))
                .await
                .unwrap()
                .attributes(),
            before_other
        );
        if reply.status == axum::http::StatusCode::UNPROCESSABLE_ENTITY {
            assert!(reply.text().starts_with("<turbo-frame id=\"user_card\">"));
            assert!(reply.text().contains("field_with_errors"));
            assert!(!reply.text().contains("<html"));
        }
    }
}
#[tokio::test]
async fn status_mutations_require_csrf_and_roll_back_on_write_failure() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.david();
    let request = || {
        Req::new(axum::http::Method::PATCH, "/users/me/status")
            .header("content-type", "application/json")
            .body(
                serde_json::to_vec(&serde_json::json!({"user":{"presence_setting":"invisible"}}))
                    .unwrap(),
            )
    };
    assert_eq!(
        browser.send(request()).await.status,
        axum::http::StatusCode::UNPROCESSABLE_ENTITY
    );
    let token = browser.authenticity_token().await;
    let before = app
        .db()
        .read(|c| campfire_db::models::user::status_form::StatusForm::load(c, DAVID))
        .await
        .unwrap()
        .attributes();
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_status BEFORE UPDATE OF presence_setting ON users BEGIN SELECT RAISE(ABORT,'reject status'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        browser
            .send(request().header("x-csrf-token", &token))
            .await
            .status,
        axum::http::StatusCode::INTERNAL_SERVER_ERROR
    );
    assert_eq!(
        app.db()
            .read(|c| campfire_db::models::user::status_form::StatusForm::load(c, DAVID))
            .await
            .unwrap()
            .attributes(),
        before
    );
}
