//! Missing individual clauses from d7c7de92 ProfilesControllerTest, through the real router.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_original_receipts.json"
    ))
    .unwrap()
}

async fn original_mutation(name: &str) {
    let app = TestApp::boot_frozen()
        .await
        .expect("CI default seed required");
    let mut browser = app.david();
    for case in oracle()["mutations"]
        .as_array()
        .unwrap()
        .iter()
        .filter(|case| case["name"] == name)
    {
        app.db().write(|tx| {
            tx.conn().execute("UPDATE users SET text_size='default',voice_mode='voice_activity',inbox_preferences=NULL,github_login=NULL WHERE id=?", [DAVID])?;
            tx.conn().execute("UPDATE users SET github_login='shared-login' WHERE id=?", [JASON])?;
            Ok(())
        }).await.unwrap();
        let reply = browser
            .write(
                Req::new(Method::PUT, "/users/me/profile")
                    .header("content-type", "application/json")
                    .header("accept", "text/html")
                    .body(json!({"user": case["params"]}).to_string()),
            )
            .await;
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
        let state = app.db().read(|conn| {
            Ok(conn.query_row("SELECT text_size,voice_mode,inbox_preferences,github_login FROM users WHERE id=?", [DAVID], |r| {
                let raw: Option<String> = r.get(2)?;
                Ok(json!({"text_size":r.get::<_,String>(0)?,"voice_mode":r.get::<_,Option<String>>(1)?,"inbox_preferences":raw.map(|s|serde_json::from_str::<Value>(&s).unwrap()),"github_login":r.get::<_,Option<String>>(3)?}))
            })?)
        }).await.unwrap();
        assert_eq!(state, case["state"], "{}", case["name"]);
        if reply.status == StatusCode::UNPROCESSABLE_ENTITY { assert!(reply.body.is_empty()); }
    }
}


macro_rules! original_mutations {
    ($($test:ident => $name:literal),+ $(,)?) => { $(
        #[tokio::test]
        async fn $test() { original_mutation($name).await; }
    )+ };
}
original_mutations!(
    original_profile_text_size_save_matches_rails => "original_text_size",
    original_profile_notification_switch_save_matches_rails => "original_notifications",
    original_profile_bad_notification_preserves_saved_state => "original_bad_notification",
    original_profile_bad_voice_preserves_saved_state => "original_bad_voice",
    original_profile_duplicate_login_is_rejected => "original_github_duplicate",
);
