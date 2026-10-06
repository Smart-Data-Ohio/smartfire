//! Rails-produced layout inputs, read from persisted state rather than supplied to the view.
use super::{
    test_support::{DAVID, TestApp},
    view_context,
};
use campfire_db::{CachedStatements, Timestamp};
use serde_json::{Value, json};
#[tokio::test]
async fn runtime_chrome_reads_twenty_three_recorded_rails_sound_and_drive_states() {
    let vectors: Value = serde_json::from_str(include_str!("chrome_vectors.json")).unwrap();
    assert_eq!(vectors["cases"].as_array().unwrap().len(), 23);
    let now = Timestamp::from_second(vectors["now"].as_i64().unwrap());
    let Some(test) = TestApp::boot_with_huddle_and_clock(
        Default::default(),
        std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(now.jiff())),
    )
    .await
    else {
        return;
    };
    for case in vectors["cases"].as_array().unwrap() {
        let input = case["input"].clone();
        test.db().write(move |tx| {
            for (column,value) in input["user"].as_object().unwrap() {
                let value=match value {
                    Value::Null=>rusqlite::types::Value::Null,
                    Value::Bool(b)=>rusqlite::types::Value::Integer(i64::from(*b)),
                    Value::Number(n)=>rusqlite::types::Value::Integer(n.as_i64().unwrap()),
                    Value::String(s)=>rusqlite::types::Value::Text(Timestamp::parse_db(s).map(|t|t.to_db()).unwrap_or_else(||s.clone())),
                    _=>panic!("invalid fixture column")
                };
                tx.conn().execute(&format!("UPDATE users SET \"{column}\"=? WHERE id=?"),rusqlite::params![value,DAVID])?;
            }
            tx.conn().execute_cached("DELETE FROM calendar_meeting_caches WHERE user_id=?",[DAVID])?;
            tx.conn().execute_cached("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES(?,?,?,?,?)",rusqlite::params![DAVID,input["busy"].to_string(),input["ooo"].to_string(),tx.now(),tx.now()])?;
            tx.conn().execute_cached("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;
            if let Some(scopes)=input["scopes"].as_str() {
                tx.conn().execute_cached("INSERT INTO google_accounts(user_id,email,scopes,disconnected_reason,created_at,updated_at) VALUES(?,'fixture@example.test',?,?,?,?)",rusqlite::params![DAVID,scopes,if input["disconnected"]==true {Some("fixture")} else {None},tx.now(),tx.now()])?;
            }
            tx.conn().execute_cached("DELETE FROM searches WHERE user_id=?",[DAVID])?;
            for search in input["searches"].as_array().unwrap() {
                let at=Timestamp::parse_db(search["updated_at"].as_str().unwrap()).unwrap();
                tx.conn().execute_cached("INSERT INTO searches(id,user_id,query,created_at,updated_at) VALUES(?,?,?,?,?)",rusqlite::params![search["id"].as_i64(),DAVID,search["query"].as_str(),at,at])?;
            }
            Ok(())
        }).await.unwrap();
        let preferences = test
            .db()
            .read(move |conn| view_context::user_preferences_at(conn, DAVID, now))
            .await
            .unwrap();
        let sounds = &preferences.notification_sounds;
        let actual = json!({"muted":sounds.muted,"quiet_hours":sounds.quiet_hours,"meeting_quiet":sounds.meeting_quiet,"ooo_quiet":sounds.ooo_quiet,"google_drive":preferences.google_drive});
        let mut expected = case["expected"].clone();
        expected.as_object_mut().unwrap().remove("recent_searches");
        assert_eq!(actual, expected, "{}", case["name"]);
        let recent = test
            .db()
            .read(|conn| super::runtime_chrome::recent_searches(conn, Some(DAVID)))
            .await
            .unwrap();
        assert_eq!(
            recent
                .iter()
                .map(|r| json!({"id":r.id,"query":r.query}))
                .collect::<Vec<_>>(),
            case["expected"]["recent_searches"]
                .as_array()
                .unwrap()
                .clone()
        );
        let mut browser = test.sign_in(DAVID).await;
        let response = browser.get("/rooms/voices/new").await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        let html = response.text();
        let meta = case["meta"].as_str().unwrap();
        assert!(
            html.contains(meta),
            "{} missing exact meta {meta}",
            case["name"]
        );
        for name in [
            "notification-dnd",
            "quiet-hours",
            "meeting-quiet",
            "ooo-quiet",
        ] {
            assert_eq!(
                html.contains(&format!("name=\"{name}\"")),
                meta.contains(&format!("name=\"{name}\"")),
                "{} {name}",
                case["name"]
            );
        }
        assert_eq!(
            html.contains("name=\"google-drive-previews\""),
            preferences.google_drive
        );
        for (position, search) in recent.iter().enumerate() {
            let marker = format!("id=\"global_search_option_search_{}\"", search.id);
            assert!(html.contains(&marker));
            if position > 0 {
                assert!(
                    html.find(&format!(
                        "id=\"global_search_option_search_{}\"",
                        recent[position - 1].id
                    ))
                    .unwrap()
                        < html.find(&marker).unwrap()
                );
            }
        }
        for omitted in 9010..9012 {
            assert!(!html.contains(&format!("id=\"global_search_option_search_{omitted}\"")));
        }
    }
}

#[tokio::test]
async fn runtime_chrome_picker_requires_all_three_public_settings_and_renders_escaped_metadata() {
    let vectors: Value = serde_json::from_str(include_str!("chrome_vectors.json")).unwrap();
    for case in vectors["picker_cases"].as_array().unwrap() {
        let settings: Vec<_> = case["input"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(key, value)| (key.as_str(), value.as_str().unwrap()))
            .collect();
        let Some(test) = TestApp::boot_with_settings(
            Default::default(),
            super::test_support::seed_clock(),
            &settings,
        )
        .await
        else {
            return;
        };
        let mut browser = test.sign_in(DAVID).await;
        let response = browser.get("/rooms/voices/new").await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        let html = response.text();
        let configured = case["configured"].as_bool().unwrap();
        assert_eq!(test.booted.app.config.google_picker.is_some(), configured);
        for name in [
            "google-drive-share",
            "google-picker-client-id",
            "google-picker-api-key",
            "google-cloud-project-number",
        ] {
            assert_eq!(
                html.contains(&format!("name=\"{name}\"")),
                configured,
                "{name} {case}"
            );
        }
        if configured {
            assert!(html.contains("content=\"public-client&lt;&amp;&gt;\""));
            assert!(html.contains("content=\"public-picker-key\""));
            assert!(html.contains("content=\"12345\""));
        }
        let response = test.anonymous().get("/session/new").await;
        assert_eq!(response.status, axum::http::StatusCode::OK);
        assert!(!response.text().contains("name=\"google-drive-share\""));
    }
}
