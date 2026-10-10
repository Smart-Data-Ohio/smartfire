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
    for (field, value, _message) in [
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
