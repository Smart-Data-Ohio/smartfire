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
