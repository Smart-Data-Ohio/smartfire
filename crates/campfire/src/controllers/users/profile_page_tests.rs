//! Complete profile renderer bytes. HTTP authorization and CSRF remain real in the route test.
use crate::controllers::presenters::{self, test_support::*};

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
