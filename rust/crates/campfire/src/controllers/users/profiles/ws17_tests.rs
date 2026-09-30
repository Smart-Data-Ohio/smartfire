use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
use axum::http::{Method, StatusCode};
const PATH: &str = "/users/me/profile";
async fn appearance(app: &TestApp) -> (String, String, Option<String>, bool) {
    app.db()
        .read(|c| {
            Ok(c.query_row(
                "SELECT theme,text_size,time_zone,time_zone_explicit FROM users WHERE id=?",
                [DAVID],
                |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?, r.get(3)?)),
            )?)
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn ws17_update_saves_theme_and_time_zone() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    let reply = browser
        .write(Req::new(Method::PUT, PATH).form(&[
            ("user[theme]", "dark"),
            ("user[time_zone]", "Pacific Time (US & Canada)"),
        ]))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    let stored = appearance(&app).await;
    assert_eq!(stored.0, "dark");
    assert_eq!(stored.2.as_deref(), Some("Pacific Time (US & Canada)"));
    assert!(stored.3);
}
#[tokio::test]
async fn ws17_update_saves_text_size() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    let reply = browser
        .write(Req::new(Method::PUT, PATH).form(&[("user[text_size]", "smaller")]))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(appearance(&app).await.1, "smaller");
}
#[tokio::test]
async fn ws17_choosing_zone_or_not_set_records_explicit_choice() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    for zone in ["America/New_York", ""] {
        assert_eq!(
            browser
                .write(Req::new(Method::PUT, PATH).form(&[("user[time_zone]", zone)]))
                .await
                .status,
            StatusCode::FOUND
        );
        let stored = appearance(&app).await;
        assert!(stored.3);
        assert_eq!(stored.2.as_deref(), (!zone.is_empty()).then_some(zone));
    }
}
#[tokio::test]
async fn ws17_update_rejects_unknown_appearance_and_rolls_back_other_fields() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    let before = appearance(&app).await;
    for (field, value, message) in [
        ("theme", "neon", "Theme is not included in the list."),
        ("time_zone", "Narnia", "Time zone is not a valid time zone."),
        (
            "text_size",
            "huge",
            "Text size is not included in the list.",
        ),
    ] {
        let reply = browser
            .write(Req::new(Method::PUT, PATH).form(&[
                ("user[name]", "Unsaved"),
                (&format!("user[{field}]"), value),
            ]))
            .await;
        assert_eq!(
            reply.status,
            StatusCode::UNPROCESSABLE_ENTITY,
            "{}",
            reply.text()
        );
        assert!(reply.text().contains(message));
        assert_eq!(appearance(&app).await, before);
        assert_eq!(
            app.db()
                .read(|c| Ok(c
                    .query_row("SELECT name FROM users WHERE id=?", [DAVID], |r| r
                        .get::<_, String>(0))?))
                .await
                .unwrap(),
            "David"
        );
    }
}

#[tokio::test]
async fn ws17_iana_and_legacy_zones_round_trip_selected_form_options() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    for (stored, selected) in [
        ("America/New_York", "America/New_York"),
        ("Pacific Time (US & Canada)", "America/Los_Angeles"),
    ] {
        assert_eq!(
            browser
                .write(Req::new(Method::PUT, PATH).form(&[("user[time_zone]", stored)]))
                .await
                .status,
            StatusCode::FOUND
        );
        let reply = browser.get(PATH).await;
        assert_eq!(reply.status, StatusCode::OK);
        assert!(reply.text().contains(&format!(
            "<option selected=\"selected\" value=\"{selected}\">"
        )));
    }
}
#[tokio::test]
async fn ws17_layout_carries_appearance_sound_and_explicit_blank_zone() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    app.db().write(|tx| {tx.conn().execute("UPDATE users SET theme='light',text_size='large',time_zone='UTC',dnd_enabled=1,quiet_hours_enabled=1,quiet_hours_start_minute=1320,quiet_hours_end_minute=420 WHERE id=?",[DAVID])?;Ok(())}).await.unwrap();
    let html = browser.get(PATH).await.text();
    for expected in [
        "<html data-theme=\"light\" data-text-size=\"large\"",
        "<meta name=\"notification-dnd\" content=\"muted\">",
        "<meta name=\"quiet-hours\" content=\"1320-420\">",
        "<meta name=\"quiet-hours-zone\" content=\"UTC\">",
    ] {
        assert!(html.contains(expected), "{expected}");
    }
    browser
        .write(Req::new(Method::PUT, PATH).form(&[("user[time_zone]", "")]))
        .await;
    assert!(
        browser
            .get(PATH)
            .await
            .text()
            .contains("<meta name=\"current-user-time-zone\" content=\"\">")
    );
    app.db().write(|tx|{tx.conn().execute("UPDATE users SET dnd_enabled=0,presence_setting='dnd',quiet_hours_enabled=0 WHERE id=?",[DAVID])?;Ok(())}).await.unwrap();
    let html = browser.get(PATH).await.text();
    assert!(html.contains("<meta name=\"notification-dnd\" content=\"muted\">"));
    assert!(!html.contains("name=\"quiet-hours\""));
}
#[tokio::test]
async fn ws17_layout_carries_current_and_future_meeting_and_ooo_windows() {
    let app = TestApp::boot().await.expect("parity seed");
    let mut browser = app.david();
    app.db().write(|tx| {
        tx.conn().execute("UPDATE users SET meeting_status_enabled=1,meeting_dnd_enabled=1,ooo_calendar_enabled=1,ooo_until='2026-03-03 16:00:00.000000',ooo_notify_enabled=0 WHERE id=?",[DAVID])?;
        tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,busy_intervals,ooo_intervals,created_at,updated_at) VALUES (?, ?, ?, ?, ?)",rusqlite::params![DAVID,r#"[["2026-03-02T15:55:00Z","2026-03-02T16:55:00Z"],["2026-03-02T17:00:00Z","2026-03-02T18:00:00Z"]]"#,r#"[["2026-03-02T17:00:00Z","2026-03-02T18:00:00Z"]]"#,tx.now(),tx.now()])?;Ok(())
    }).await.unwrap();
    let html = browser.get(PATH).await.text();
    assert!(html.contains(
        "<meta name=\"meeting-quiet\" content=\"1772466900-1772470500,1772470800-1772474400\">"
    ));
    assert!(
        html.contains("<meta name=\"ooo-quiet\" content=\"0-1772553600,1772470800-1772474400\">")
    );
    for sql in [
        "UPDATE users SET meeting_dnd_enabled=0,ooo_notify_enabled=1",
        "UPDATE users SET meeting_dnd_enabled=1,meeting_status_enabled=0",
    ] {
        app.db()
            .write(move |tx| {
                tx.conn().execute(&format!("{sql} WHERE id=?"), [DAVID])?;
                Ok(())
            })
            .await
            .unwrap();
        let html = browser.get(PATH).await.text();
        assert!(!html.contains("name=\"meeting-quiet\""));
        assert!(!html.contains("name=\"ooo-quiet\""));
    }
}

/// Integration with WS9: appearance and authentication writes share the original save transaction.
#[tokio::test]
async fn ws17_profile_auth_rejection_keeps_submitted_appearance_without_saving_it() {
    let app = TestApp::boot().await.expect("parity seed");
    let before = appearance(&app).await;
    let mut browser = app.david();
    let response = browser
        .write(Req::new(Method::PATCH, PATH).form(&[
            ("user[email_address]", "ws17-new@example.test"),
            ("user[name]", "Unsaved name"),
            ("user[theme]", "dark"),
            ("user[text_size]", "large"),
            ("user[time_zone]", "America/New_York"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let html = response.text();
    assert!(html.contains("<html data-theme=\"dark\" data-text-size=\"large\""));
    assert!(html.contains("<meta name=\"current-user-time-zone\" content=\"America/New_York\">"));
    assert!(html.contains("Current password is required to change your email address."));
    assert!(html.contains("value=\"Unsaved name\""));
    assert!(
        html.contains("id=\"user_theme_dark\" type=\"radio\" value=\"dark\" checked=\"checked\"")
    );
    assert!(html.contains(
        "id=\"user_text_size_large\" type=\"radio\" value=\"large\" checked=\"checked\""
    ));
    assert!(html.contains("<option selected=\"selected\" value=\"America/New_York\">"));
    assert_eq!(appearance(&app).await, before);
    assert_eq!(
        app.db()
            .read(|conn| Ok(campfire_db::User::find(conn, DAVID)?.name))
            .await
            .unwrap(),
        "David"
    );
}

#[tokio::test]
async fn ws17_profile_auth_audit_failure_rolls_back_the_earlier_appearance_save() {
    let app = TestApp::boot().await.expect("parity seed");
    let before = appearance(&app).await;
    let user_before = app
        .db()
        .read(|conn| campfire_db::User::find(conn, DAVID))
        .await
        .unwrap();
    let marker_before = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT email_self_changed_at FROM users WHERE id=?",
                [DAVID],
                |r| r.get::<_, Option<String>>(0),
            )?)
        })
        .await
        .unwrap();
    let audits_before = app
        .db()
        .read(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| {
                r.get::<_, i64>(0)
            })?)
        })
        .await
        .unwrap();
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws17_refuse_profile_audit BEFORE INSERT ON audit_logs WHEN NEW.action='user.email.change' BEGIN SELECT RAISE(ABORT, 'audit unavailable'); END")?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.david();
    let response = browser
        .write(Req::new(Method::PATCH, PATH).form(&[
            ("user[email_address]", "ws17-atomic@example.test"),
            ("user[current_password]", "secret123456"),
            ("user[name]", "Submitted name"),
            ("user[theme]", "dark"),
            ("user[text_size]", "large"),
            ("user[time_zone]", "America/New_York"),
        ]))
        .await;
    assert_eq!(response.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert_eq!(appearance(&app).await, before);
    let user_after = app
        .db()
        .read(|conn| campfire_db::User::find(conn, DAVID))
        .await
        .unwrap();
    assert_eq!(user_after.name, user_before.name);
    assert_eq!(user_after.email_address, user_before.email_address);
    let marker_after = app
        .db()
        .read(|conn| {
            Ok(conn.query_row(
                "SELECT email_self_changed_at FROM users WHERE id=?",
                [DAVID],
                |r| r.get::<_, Option<String>>(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(marker_after, marker_before);
    assert_eq!(
        app.db()
            .read(
                |conn| Ok(conn.query_row("SELECT COUNT(*) FROM audit_logs", [], |r| r
                    .get::<_, i64>(0))?)
            )
            .await
            .unwrap(),
        audits_before
    );
}

#[tokio::test]
async fn ws17_rejected_profile_layout_metadata_matches_loaded_unsaved_rails_values() {
    let app = TestApp::boot().await.expect("parity seed");
    app.db().write(|tx| {
        tx.conn().execute("UPDATE users SET theme='system',text_size='default',time_zone=NULL,time_zone_explicit=0,dnd_enabled=0,quiet_hours_enabled=0,meeting_status_enabled=0,ooo_calendar_enabled=0,ooo_until=NULL WHERE id=?",[DAVID])?;
        Ok(())
    }).await.unwrap();
    let before = appearance(&app).await;
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../views/tests/golden/ws17-profile-ui.json"
    )))
    .unwrap();
    let mut browser = app.david();
    for row in vectors["rows"].as_array().unwrap() {
        let theme = row["data"]["theme"].as_str().unwrap();
        let size = row["data"]["text_size"].as_str().unwrap();
        let mut fields = vec![
            ("user[email_address]", "ws17-metadata@example.test"),
            ("user[theme]", theme),
            ("user[text_size]", size),
        ];
        if let Some(zone) = row["data"]["time_zone"].as_str() {
            fields.push(("user[time_zone]", zone));
        }
        let response = browser
            .write(Req::new(Method::PATCH, PATH).form(&fields))
            .await;
        assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
        let html = response.text();
        for (name, fragment) in row["metadata"].as_object().unwrap() {
            let fragment = fragment.as_str().unwrap();
            if name == "time_zone" && fragment.trim().is_empty() {
                assert!(
                    !html.contains("name=\"current-user-time-zone\""),
                    "{theme}/{size}: Rails omits time-zone metadata on this rejection"
                );
            } else {
                assert!(
                    html.contains(fragment),
                    "{theme}/{size} {name}: expected {fragment:?}"
                );
            }
        }
        assert_eq!(appearance(&app).await, before);
    }
}

#[tokio::test]
async fn ws17_invalid_notification_form_layout_keeps_unsaved_sound_gate() {
    let app = TestApp::boot().await.expect("parity seed");
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE two_factor_credentials SET confirmed_at=NULL WHERE user_id=?",
                [DAVID],
            )?;
            tx.conn().execute(
                "UPDATE users SET dnd_enabled=0,quiet_hours_enabled=0 WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = app.david();
    let response = browser
        .write(
            Req::new(Method::PATCH, "/users/me/notification_settings").form(&[
                ("user[dnd_enabled]", "1"),
                ("user[quiet_hours_enabled]", "1"),
                ("user[quiet_hours_start]", ""),
                ("user[quiet_hours_end]", ""),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    let vectors: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../views/tests/golden/ws17-profile-ui.json"
    )))
    .unwrap();
    assert!(
        response
            .text()
            .contains(vectors["notification_error_sounds"].as_str().unwrap())
    );
    assert!(!response.text().contains("name=\"quiet-hours\""));
    assert!(
        !app.db()
            .read(|conn| Ok(conn.query_row(
                "SELECT dnd_enabled FROM users WHERE id=?",
                [DAVID],
                |r| r.get::<_, bool>(0)
            )?))
            .await
            .unwrap()
    );
}
