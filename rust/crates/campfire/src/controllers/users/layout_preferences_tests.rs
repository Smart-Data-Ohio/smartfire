//! Pinned Rails helper bytes through the real signed-in profile/layout path.
use crate::controllers::presenters::test_support::*;
use serde_json::Value;

async fn check_cases(names: &[&str]) {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let vectors: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_layout_preferences.json"
    ))
    .unwrap();
    for name in names {
        let case = vectors["cases"]
            .as_array()
            .unwrap()
            .iter()
            .find(|case| case["name"] == *name)
            .unwrap()
            .clone();
        let setup = case.clone();
        app.db()
            .write(move |tx| {
                let conn = tx.conn();
                let attrs = setup["attributes"].as_object().unwrap();
                let mut values = Vec::new();
                for (key, value) in attrs {
                    values.push(match value {
                        Value::Null => rusqlite::types::Value::Null,
                        Value::Bool(v) => rusqlite::types::Value::Integer(i64::from(*v)),
                        Value::Number(v) => rusqlite::types::Value::Integer(v.as_i64().unwrap()),
                        Value::String(v) if key.ends_with("_until") => {
                            let time: jiff::Timestamp = v.parse().unwrap();
                            rusqlite::types::Value::Text(campfire_db::Timestamp::from_jiff(time).to_db())
                        }
                        Value::String(v) => rusqlite::types::Value::Text(v.clone()),
                        _ => panic!("unexpected committed fixture field"),
                    });
                }
                values.push(rusqlite::types::Value::Integer(DAVID));
                let columns = attrs.keys().map(|key| format!("{key}=?")).collect::<Vec<_>>().join(",");
                conn.execute(&format!("UPDATE users SET {columns} WHERE id=?"), rusqlite::params_from_iter(values))?;
                conn.execute("DELETE FROM calendar_meeting_caches WHERE user_id=?", [DAVID])?;
                conn.execute("DELETE FROM google_accounts WHERE user_id=?", [DAVID])?;
                if !setup["cache"].is_null() {
                    let cache = &setup["cache"];
                    conn.execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?,?,?,?,?)",
                        rusqlite::params![DAVID, cache.get("busy").unwrap_or(&serde_json::json!([])).to_string(), cache.get("ooo").unwrap_or(&serde_json::json!([])).to_string(), tx.now(), tx.now()])?;
                }
                if !setup["google"].is_null() {
                    conn.execute("INSERT INTO google_accounts(user_id,email,scopes,disconnected_reason,created_at,updated_at) VALUES (?,?,?,?,?,?)",
                        rusqlite::params![DAVID,"layout@example.test", setup["google"]["scopes"].as_str(), setup["google"]["reason"].as_str(), tx.now(), tx.now()])?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let page = app.david().get("/users/me/profile").await;
        assert_eq!(page.status, axum::http::StatusCode::OK, "{name}");
        let body = page.text();
        // ProfilesControllerTest's light theme also pins the exact color-scheme cardinality.
        if *name == "manual_dnd" {
            assert_eq!(
                body.matches("<meta name=\"color-scheme\" content=\"light\">")
                    .count(),
                1
            );
        }
        // Read the whole helper's rendered line verbatim; tokens/nonces elsewhere stay real.
        let actual = body
            .split_once("<meta name=\"time-zone-url\" content=\"/users/me/time_zone\">\n      ")
            .unwrap()
            .1
            .split_once('\n')
            .unwrap()
            .0;
        assert_eq!(
            actual,
            case["sound_meta"].as_str().unwrap(),
            "{name}: complete sound meta bytes"
        );
        let drive = body
            .find("<meta name=\"google-drive-previews\"")
            .map(|start| {
                let end = body[start..].find('>').unwrap();
                &body[start..=start + end]
            })
            .unwrap_or("");
        assert_eq!(
            drive,
            case["drive_meta"].as_str().unwrap(),
            "{name}: complete Drive meta bytes"
        );
        assert!(body.contains(&format!(
            "data-theme=\"{}\"",
            case["attributes"]["theme"].as_str().unwrap()
        )));
        assert!(body.contains(&format!(
            "data-text-size=\"{}\"",
            case["attributes"]["text_size"].as_str().unwrap()
        )));
        let zone_start = body.find("<meta name=\"current-user-time-zone\"").unwrap();
        let zone_end = zone_start + body[zone_start..].find('>').unwrap();
        assert_eq!(
            &body[zone_start..=zone_end],
            case["time_zone_meta"].as_str().unwrap(),
            "{name}: complete zone meta bytes"
        );
    }
}

macro_rules! cases {
    ($($test:ident => [$($case:literal),+]),+ $(,)?) => {
        $(#[tokio::test] async fn $test() { check_cases(&[$($case),+]).await; })+
    };
}
cases!(
    layout_theme_zone_and_manual_sound_state => ["default", "manual_dnd", "expired_dnd", "boundary_dnd", "running_dnd"],
    layout_dnd_presence_mutes_sounds => ["dnd_presence"],
    layout_quiet_hours_window_and_zone => ["quiet_hours", "quiet_hours_off", "quiet_hours_incomplete", "quiet_hours_equal", "quiet_hours_zone_default"],
    layout_meeting_windows_include_all_cached_pairs => ["meeting_current", "cache_order_and_offsets", "cache_malformed_pairs"],
    layout_future_meeting_windows_before_start => ["meeting_future"],
    layout_empty_or_missing_meeting_cache => ["meeting_empty", "meeting_missing"],
    layout_meeting_status_off_sends_no_windows => ["meeting_status_off"],
    layout_manual_ooo_windows => ["manual_ooo", "expired_ooo", "boundary_ooo", "ooo_manual_and_calendar"],
    layout_future_calendar_ooo_windows => ["calendar_ooo_future", "calendar_ooo_off"],
    layout_ooo_notifications_kept_sends_no_windows => ["ooo_notifications_kept"],
    layout_meeting_quiet_off_sends_no_windows => ["meeting_quiet_off"],
    layout_drive_previews_uses_exact_scope => ["drive_missing", "drive_calendar_only", "drive_current_scope", "drive_retired_scope", "drive_disconnected", "drive_ascii_separators", "drive_wrong_case", "drive_unicode_separator"],
);
