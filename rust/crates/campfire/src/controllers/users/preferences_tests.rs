use crate::controllers::presenters::test_support::{
    BENDER_KEY, DAVID, JASON, Req, SEED_NOW, TestApp,
};
use axum::http::{Method, StatusCode};
use campfire_db::Timestamp;

#[tokio::test]
async fn time_zone_detection_matches_rails_validation_and_saved_choice_vectors() {
    let Some(app) = TestApp::boot_frozen().await else {
        return;
    };
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_preferences.json"
    ))
    .unwrap();
    let mut browser = app.david();
    for vector in vectors["time_zones"].as_array().unwrap() {
        let initial = vector["initial"].as_object().unwrap().clone();
        app.db().write(move |tx| {
            tx.conn().execute("UPDATE users SET time_zone=NULL,time_zone_explicit=0,theme='system',voice_mode=NULL,quiet_hours_enabled=0,quiet_hours_start_minute=NULL,quiet_hours_end_minute=NULL,updated_at='2026-03-02 15:00:00' WHERE id=?", [DAVID])?;
            for (key, value) in initial {
                let value = match value {
                    serde_json::Value::String(s) => rusqlite::types::Value::Text(s),
                    serde_json::Value::Bool(b) => rusqlite::types::Value::Integer(i64::from(b)),
                    _ => panic!("unsupported initial state"),
                };
                tx.conn().execute(&format!("UPDATE users SET {key}=? WHERE id=?"), rusqlite::params![value, DAVID])?;
            }
            Ok(())
        }).await.unwrap();
        let params = serde_json::json!({"time_zone": vector["zone"], "user_id": JASON});
        let response = browser
            .write(
                Req::new(Method::PATCH, vector["path"].as_str().unwrap())
                    .header("content-type", "application/json")
                    .body(serde_json::to_vec(&params).unwrap()),
            )
            .await;
        assert_eq!(
            response.status.as_u16(),
            vector["status"].as_u64().unwrap() as u16,
            "{}",
            vector["name"]
        );
        assert_eq!(response.json(), vector["json"], "{}", vector["name"]);
        let (zone, explicit, updated) = app
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT time_zone,time_zone_explicit,updated_at FROM users WHERE id=?",
                    [DAVID],
                    |r| {
                        Ok((
                            r.get::<_, Option<String>>(0)?,
                            r.get::<_, bool>(1)?,
                            r.get::<_, Timestamp>(2)?,
                        ))
                    },
                )?)
            })
            .await
            .unwrap();
        assert_eq!(
            serde_json::to_value(zone).unwrap(),
            vector["state"]["time_zone"]
        );
        assert_eq!(
            explicit,
            vector["state"]["time_zone_explicit"].as_bool().unwrap()
        );
        assert_eq!(
            updated.jiff(),
            vector["state"]["updated_at"]
                .as_str()
                .unwrap()
                .parse::<jiff::Timestamp>()
                .unwrap()
        );
    }
}

#[tokio::test]
async fn preference_writes_require_session_and_csrf_and_scope_to_current_user() {
    let Some(app) = TestApp::boot().await else {
        return;
    };
    for path in ["/users/me/time_zone", "/users/me/tour"] {
        assert_eq!(
            app.anonymous()
                .send(Req::new(Method::PATCH, path))
                .await
                .status,
            StatusCode::FOUND
        );
        assert_eq!(
            app.david().send(Req::new(Method::PATCH, path)).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        let mut bot = app.anonymous();
        assert_eq!(
            bot.send(Req::new(Method::PATCH, path).form(&[("bot_key", BENDER_KEY)]))
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        let authorization = ["Bearer", "ws8br2-invalid-agent-fixture"].join(" ");
        assert_eq!(
            app.anonymous()
                .send(Req::new(Method::PATCH, path).header("authorization", &authorization))
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
    app.db().write(|tx| { tx.conn().execute("UPDATE users SET tour_completed_at=NULL,time_zone=NULL,time_zone_explicit=0 WHERE id IN (?,?)", [DAVID, JASON])?; Ok(()) }).await.unwrap();
    let mut member = app.sign_in(JASON).await;
    let response = member
        .write(
            Req::new(Method::PATCH, &format!("/users/{DAVID}/time_zone")).form(&[
                ("time_zone", "America/New_York"),
                ("user_id", &DAVID.to_string()),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::OK);
    let response = member
        .write(
            Req::new(Method::PATCH, &format!("/users/{DAVID}/tour"))
                .form(&[("user_id", &DAVID.to_string())]),
        )
        .await;
    assert_eq!(response.status, StatusCode::NO_CONTENT);
    app.db()
        .read(|conn| {
            let other: (Option<String>, Option<Timestamp>) = conn.query_row(
                "SELECT time_zone,tour_completed_at FROM users WHERE id=?",
                [DAVID],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(other, (None, None));
            let member: (Option<String>, Option<Timestamp>) = conn.query_row(
                "SELECT time_zone,tour_completed_at FROM users WHERE id=?",
                [JASON],
                |r| Ok((r.get(0)?, r.get(1)?)),
            )?;
            assert_eq!(member.0.as_deref(), Some("America/New_York"));
            assert!(member.1.is_some());
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn tour_touch_matches_rails_and_refreshes_on_repeated_completion() {
    use std::sync::{
        Arc,
        atomic::{AtomicI64, Ordering},
    };
    #[derive(Debug)]
    struct Clock(AtomicI64);
    impl campfire_kit::Clock for Clock {
        fn now(&self) -> jiff::Timestamp {
            jiff::Timestamp::from_second(self.0.load(Ordering::SeqCst)).unwrap()
        }
    }
    let now: jiff::Timestamp = SEED_NOW.parse().unwrap();
    let clock = Arc::new(Clock(AtomicI64::new(now.as_second())));
    let Some(app) = TestApp::boot_seed_with_env("default", clock.clone(), &[]).await else {
        return;
    };
    app.db().write(|tx| { tx.conn().execute("UPDATE users SET tour_completed_at=NULL,updated_at='2026-03-02 15:00:00',theme='invalid' WHERE id=?", [DAVID])?; Ok(()) }).await.unwrap();
    let mut browser = app.david();
    let vectors: serde_json::Value = serde_json::from_str(include_str!(
        "../../../../../vectors/users_preferences.json"
    ))
    .unwrap();
    for offset in [0, 3600] {
        clock.0.store(now.as_second() + offset, Ordering::SeqCst);
        let response = browser
            .write(Req::new(Method::PATCH, "/users/me/tour"))
            .await;
        assert_eq!(
            response.status.as_u16(),
            vectors["tour"]["status"].as_u64().unwrap() as u16
        );
        assert!(response.body.is_empty());
        let timestamps: (Timestamp, Timestamp) = app
            .db()
            .read(|conn| {
                Ok(conn.query_row(
                    "SELECT tour_completed_at,updated_at FROM users WHERE id=?",
                    [DAVID],
                    |r| Ok((r.get(0)?, r.get(1)?)),
                )?)
            })
            .await
            .unwrap();
        let expected = now + jiff::SignedDuration::from_secs(offset);
        assert_eq!(timestamps.0.jiff(), expected);
        assert_eq!(timestamps.1.jiff(), expected);
    }
}

#[tokio::test]
async fn tour_stamp_controls_the_room_layout_auto_start() {
    let assert_shell = |body: &str, auto_start: &str| {
        let mut dom = campfire_richtext::dom::Dom::new();
        let root = dom.parse_fragment(body).unwrap();
        let nodes = dom.descendants(root);
        let tours = nodes
            .iter()
            .filter(|id| {
                dom.attr(**id, "id") == Some("tour")
                    && dom.attr(**id, "data-controller") == Some("tour")
                    && dom.attr(**id, "data-tour-auto-start-value") == Some(auto_start)
            })
            .count();
        assert_eq!(tours, 1, "exact original tour shell selector");
        let help = nodes
            .iter()
            .filter(|id| dom.attr(**id, "id") == Some("help-menu-button"))
            .count();
        assert_eq!(help, 1, "exact Help control cardinality");
    };
    let Some(app) = TestApp::boot().await else {
        return;
    };
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET tour_completed_at=NULL WHERE id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut browser = app.david();
    let before = browser.get("/rooms/486777696").await;
    assert_eq!(before.status, StatusCode::OK);
    assert_shell(&before.text(), "true");
    assert!(
        before
            .text()
            .contains("data-tour-auto-start-value=\"true\"")
    );
    assert!(before.text().contains("id=\"help-menu-button\""));
    assert_eq!(
        browser
            .write(Req::new(Method::PATCH, "/users/me/tour"))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let after = browser.get("/rooms/486777696").await;
    assert_eq!(after.status, StatusCode::OK);
    assert_shell(&after.text(), "false");
    assert!(after.text().contains("id=\"help-menu-button\""));
    assert!(
        after
            .text()
            .contains("data-tour-auto-start-value=\"false\"")
    );
}
