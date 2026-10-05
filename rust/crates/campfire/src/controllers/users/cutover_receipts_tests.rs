//! Missing individual clauses from d7c7de92 ProfilesControllerTest, through the real router.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_richtext::dom::Dom;
use serde_json::{Value, json};

fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_original_receipts.json"
    ))
    .unwrap()
}

fn profile_observation(reply: &Reply) -> Value {
    let body = reply.text();
    let mut dom = Dom::new();
    let root = dom.parse_fragment(&body).unwrap();
    let nodes = dom.descendants(root);
    let inputs: Vec<_> = nodes
        .iter()
        .copied()
        .filter(|id| dom.name(*id) == "input")
        .collect();
    let selected = |name| {
        nodes
            .iter()
            .copied()
            .filter(|id| {
                dom.name(*id) == "select"
                    && dom.attr(*id, "name") == Some(name)
                    && (name != "user[time_zone]" || dom.attr(*id, "id") == Some("user_time_zone"))
            })
            .flat_map(|id| dom.descendants(id))
            .filter(|id| dom.name(*id) == "option" && dom.attr(*id, "selected").is_some())
            .collect::<Vec<_>>()
    };
    json!({
        "status": reply.status.as_u16(),
        "inbox": inputs.iter().filter_map(|id| {
            let name = dom.attr(*id, "name")?;
            (dom.attr(*id, "type") == Some("checkbox") && name.starts_with("user[inbox_preferences]") && dom.attr(*id, "checked").is_some()).then_some(name)
        }).collect::<Vec<_>>(),
        "notification_explanations": (["GitHub review requests", "The incoming-call banner still shows."].map(|s| body.contains(s))),
        "voice_mode": selected("user[voice_mode]").iter().map(|id| json!([dom.attr(*id, "value"), dom.text_content(*id)])).collect::<Vec<_>>(),
        "push_to_talk_key": inputs.iter().filter(|id| dom.attr(**id, "name") == Some("user[push_to_talk_key]")).map(|id| dom.attr(*id, "value")).collect::<Vec<_>>(),
        "current_password": inputs.iter().filter(|id| dom.attr(**id, "name") == Some("user[current_password]")).map(|id| dom.attr(*id, "autocomplete")).collect::<Vec<_>>(),
        "time_zone": selected("user[time_zone]").iter().map(|id| dom.attr(*id, "value")).collect::<Vec<_>>(),
        "meeting_dnd_checkbox": inputs.iter().filter(|id| dom.attr(**id, "name") == Some("user[meeting_dnd_enabled]") && dom.attr(**id, "type") == Some("checkbox")).count(),
        "meeting_dnd_explanation": body.contains("Do not disturb during meetings"),
        "calendar_not_configured": body.contains("Google Calendar is not configured for this workspace"),
        "calendar_connect": body.contains("Connect Google Calendar"),
        "drive_row": (["Enable Drive previews", "Drive previews enabled"].iter().any(|s| body.contains(s))),
    })
}

#[tokio::test]
async fn original_profile_defaults_and_zone_options_match_rails_through_http() {
    let app = TestApp::boot_frozen()
        .await
        .expect("CI default seed required");
    app.db().write(|tx| {
        tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?", [DAVID])?;
        tx.conn().execute("UPDATE users SET inbox_preferences=NULL,voice_mode=NULL,push_to_talk_key=NULL,time_zone=NULL WHERE id=?", [DAVID])?;
        Ok(())
    }).await.unwrap();
    for page in oracle()["pages"].as_array().unwrap() {
        let zone = page["zone"].as_str().map(str::to_owned);
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET time_zone=? WHERE id=?",
                    rusqlite::params![zone, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let reply = app.david().get("/users/me/profile").await;
        assert_eq!(
            profile_observation(&reply),
            page["response"],
            "zone {}",
            page["zone"]
        );
    }
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
        let mut dom = Dom::new();
        let root = dom.parse_fragment(&reply.text()).unwrap();
        assert_eq!(
            dom.descendants(root)
                .into_iter()
                .any(|id| dom.name(id) == "p"
                    && dom
                        .text_content(id)
                        .contains("already linked to another user")),
            case["duplicate_error"].as_bool().unwrap(),
            "{}",
            case["name"]
        );
        if reply.status == StatusCode::UNPROCESSABLE_ENTITY {
            assert!(reply.text().contains("<form"));
        }
        let read = browser.get("/users/me/profile").await;
        assert_eq!(
            profile_observation(&read)["inbox"],
            case["enabled_notifications"],
            "{}: real typed preference read",
            case["name"]
        );
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
