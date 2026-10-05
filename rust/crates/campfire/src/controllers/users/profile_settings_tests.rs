use crate::controllers::presenters::test_support::*;
use askama::Template;
use axum::http::{Method, StatusCode};

#[tokio::test]
async fn invalid_profile_settings_roll_back_security_and_core_changes() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let (before, before_devices) = app
        .db()
        .write(|tx| {
            campfire_db::TwoFactorRememberedDevice::create_for(tx, DAVID, Some("fixture"), None)?;
            Ok((
                campfire_db::User::find(tx.conn(), DAVID)?,
                campfire_db::TwoFactorRememberedDevice::for_user(tx.conn(), DAVID)?.len(),
            ))
        })
        .await
        .unwrap();
    let audits = app
        .db()
        .read(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .await
        .unwrap();
    let mut browser = app.david();
    let response=browser.write(Req::new(Method::PATCH,"/users/me/profile").header("content-type","application/json").header("accept","text/html").body(serde_json::to_vec(&serde_json::json!({"user":{"name":"must not save","email_address":"fixture@smartdata.net","current_password":"secret123456","password":"new-fixture-password","theme":"neon"}})).unwrap())).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        response
            .text()
            .contains("Theme is not included in the list.")
    );
    let (after, after_audits, devices, marker) = app
        .db()
        .read(|conn| {
            Ok((
                campfire_db::User::find(conn, DAVID)?,
                conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                    r.get::<_, i64>(0)
                })?,
                campfire_db::TwoFactorRememberedDevice::for_user(conn, DAVID)?.len(),
                conn.query_row(
                    "SELECT email_self_changed_at FROM users WHERE id=?",
                    [DAVID],
                    |r| r.get::<_, Option<String>>(0),
                )?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(after, before);
    assert_eq!(after_audits, audits);
    assert_eq!(devices, before_devices);
    assert_eq!(marker, None);
}

#[tokio::test]
async fn manual_timezone_choice_is_rendered_and_blocks_browser_detection() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut browser = app.david();
    let response = browser
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[
            ("user[theme]", "dark"),
            ("user[text_size]", "larger"),
            ("user[time_zone]", "America/New_York"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    let profile = browser.get("/users/me/profile").await;
    assert_eq!(profile.status, StatusCode::OK);
    assert!(
        profile
            .text()
            .contains("<option selected=\"selected\" value=\"America/New_York\">")
    );
    assert!(profile.text().contains("data-theme=\"dark\""));
    assert!(profile.text().contains("data-text-size=\"larger\""));
    let detected = browser
        .write(
            Req::new(Method::PATCH, "/users/me/time_zone").form(&[("time_zone", "Europe/London")]),
        )
        .await;
    assert_eq!(detected.status, StatusCode::OK);
    let zone = app
        .db()
        .read(|conn| campfire_db::User::saved_time_zone(conn, DAVID))
        .await
        .unwrap();
    assert_eq!(zone.as_deref(), Some("America/New_York"));
    let clear = browser
        .write(Req::new(Method::PATCH, "/users/me/profile").form(&[("user[time_zone]", "")]))
        .await;
    assert_eq!(clear.status, StatusCode::FOUND);
    let cleared: (Option<String>, bool) = app.db().read(|conn| {
        Ok(conn.query_row("SELECT time_zone,time_zone_explicit FROM users WHERE id=?", [DAVID],
            |r| Ok((r.get(0)?, r.get(1)?)))?)
    }).await.unwrap();
    assert_eq!(cleared, (None, true), "clear the previously selected zone and retain the explicit choice");
    let profile = browser.get("/users/me/profile").await;
    assert_eq!(profile.status, StatusCode::OK);
    assert!(
        profile
            .text()
            .contains("<meta name=\"current-user-time-zone\" content=\"\"")
    );
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(&profile.text()).unwrap();
    let unset_zones = dom
        .descendants(root)
        .into_iter()
        .filter(|id| {
            dom.name(*id) == "meta"
                && dom.attr(*id, "name") == Some("current-user-time-zone")
                && dom.attr(*id, "content") == Some("")
        })
        .count();
    assert_eq!(unset_zones, 1);
    let detected = browser
        .write(
            Req::new(Method::PATCH, "/users/me/time_zone").form(&[("time_zone", "Europe/London")]),
        )
        .await;
    assert_eq!(detected.status, StatusCode::OK);
    assert_eq!(
        app.db()
            .read(|conn| campfire_db::User::saved_time_zone(conn, DAVID))
            .await
            .unwrap(),
        None
    );
}

#[tokio::test]
async fn appearance_partial_matches_all_pinned_rails_bytes() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/users_appearance.json")).unwrap();
    let catalogue: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../views/src/users/profile_time_zones.json"
    ))
    .unwrap();
    assert_eq!(catalogue, vectors["choices"]);
    for case in vectors["cases"].as_array().unwrap() {
        let a = &case["attributes"];
        let errors = |field: &str| {
            case["errors"][field]
                .as_array()
                .map(|v| v.iter().map(|s| s.as_str().unwrap().to_owned()).collect())
                .unwrap_or_default()
        };
        let data = campfire_views::users::AppearanceData {
            theme: a["theme"].as_str().unwrap().into(),
            text_size: a["text_size"].as_str().unwrap().into(),
            zone_identifier: a["time_zone"]
                .as_str()
                .and_then(campfire_db::models::user::profile_settings::zone_identifier),
            theme_errors: errors("theme"),
            text_size_errors: errors("text_size"),
            time_zone_errors: errors("time_zone"),
        };
        let actual = super::people_tests::render(&app, |ctx| {
            campfire_views::users::Appearance { ctx, data }
                .render()
                .unwrap()
        });
        if let Ok(dir) = std::env::var("WS8BR2_DIFF_DIR") {
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(format!("{dir}/appearance.actual"), &actual).unwrap();
            std::fs::write(
                format!("{dir}/appearance.expected"),
                case["html"].as_str().unwrap(),
            )
            .unwrap();
        }
        assert_eq!(
            actual,
            case["html"].as_str().unwrap(),
            "full appearance bytes: {a}"
        );
    }
}

#[tokio::test]
async fn manual_profile_settings_match_pinned_rails_patch_vectors() {
    run_profile_vectors(None).await;
}

#[tokio::test]
async fn original_github_login_normalizes_david_gh() {
    run_profile_vectors(Some("original_github_normalize")).await;
}

#[tokio::test]
async fn original_linked_github_login_is_cleared() {
    run_profile_vectors(Some("original_github_unlink")).await;
}

async fn run_profile_vectors(selected: Option<&str>) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let cases: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_profile_settings.json"
    ))
    .unwrap();
    let mut browser = app.david();
    for case in cases["profiles"].as_array().unwrap().iter().filter(|case| selected.is_none_or(|name| case["name"] == name)) {
        let setup = case.clone();
        app.db().write(move |tx|{
            let case=&setup;
            tx.conn().execute("UPDATE users SET name='David',bio=NULL,theme='system',text_size='default',time_zone=NULL,time_zone_explicit=0,voice_mode=NULL,push_to_talk_key=NULL,inbox_preferences=NULL,github_login=NULL,updated_at='2026-03-02 15:00:00' WHERE id=?",[DAVID])?;
            tx.conn().execute("DELETE FROM github_connected_accounts WHERE user_id=?",[DAVID])?;
            if let Some(before)=case["before"].as_object() {
                for (key,value) in before {
                    assert!(["theme","inbox_preferences","github_login","time_zone","time_zone_explicit"].contains(&key.as_str()));
                    let value = match value {
                        serde_json::Value::Bool(value) => rusqlite::types::Value::Integer(i64::from(*value)),
                        serde_json::Value::String(value) => rusqlite::types::Value::Text(value.clone()),
                        value => rusqlite::types::Value::Text(value.to_string()),
                    };
                    tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"),rusqlite::params![value,DAVID])?;
                }
            }
            if case["connection"]==true {
                tx.conn().execute("INSERT INTO github_connected_accounts (user_id,github_login,access_token,disconnected_reason,created_at,updated_at) VALUES (?,'verified','ws8br2-fixture',?,?,?)",rusqlite::params![DAVID,case["reason"].as_str(),tx.now(),tx.now()])?;
            }
            tx.conn().execute("UPDATE users SET github_login=? WHERE id=?",rusqlite::params![if case["duplicate"]==true {Some("shared-login")} else {None},JASON])?;
            Ok(())
        }).await.unwrap();
        let response = browser
            .write(
                Req::new(Method::PATCH, case["path"].as_str().unwrap())
                    .header("content-type", "application/json")
                    .header("accept", "text/html")
                    .body(serde_json::to_vec(&serde_json::json!({"user":case["params"]})).unwrap()),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{}",
            case["name"]
        );
        if response.status == StatusCode::FOUND {
            assert_eq!(
                response.location(),
                Some("http://campfire.test/users/me/profile"),
                "{}",
                case["name"]
            );
        }
        let state=app.db().read(|conn| {
            assert_eq!(campfire_db::User::find(conn, DAVID)?.email_address.as_deref(), Some("david@37signals.com"));
            let other_name:String=conn.query_row("SELECT name FROM users WHERE id=?",[JASON],|r|r.get(0))?;
            let mut stmt=conn.prepare("SELECT theme,text_size,time_zone,time_zone_explicit,voice_mode,push_to_talk_key,inbox_preferences,github_login,name,updated_at,bio FROM users WHERE id=?")?;
            let state=stmt.query_row([DAVID],|r|{
                let text=|i| r.get::<_,Option<String>>(i);
                let raw:Option<String>=text(6)?;
                let updated:campfire_db::Timestamp=r.get(9)?;
                Ok(serde_json::json!({"theme":text(0)?,"text_size":text(1)?,"time_zone":text(2)?,"time_zone_explicit":r.get::<_,bool>(3)?,"voice_mode":text(4)?,"push_to_talk_key":text(5)?,"inbox_preferences":raw.map(|s|serde_json::from_str::<serde_json::Value>(&s).unwrap()),"github_login":text(7)?,"name":text(8)?,"bio":text(10)?,"other_name":other_name,"updated_at":updated.jiff().strftime("%Y-%m-%dT%H:%M:%S%.6fZ").to_string()}))
            })?;
            Ok(state)
        }).await.unwrap();
        assert_eq!(state, case["state"], "{}", case["name"]);
        if response.status == StatusCode::UNPROCESSABLE_ENTITY {
            assert!(response.text().contains("<form"));
        }
    }
}
