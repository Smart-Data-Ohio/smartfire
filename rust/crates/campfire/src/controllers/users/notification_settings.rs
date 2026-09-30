//! Users::NotificationSettingsController: atomic settings and keyword-list replacement.
use crate::app::AppCtx;
use crate::concerns::{self, Before};
use campfire_db::{UserStatusSettings, models::user_status_settings::clock_time_to_minutes};
use campfire_kit::{Ctx, Error, Param, Redirect, Result, StatusCode, permit_keys};

pub async fn update(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let id = concerns::require_current_user(c)?.id;
    let raw = c.params.require("user")?.as_hash().ok_or_else(|| {
        Error::internal(anyhow::anyhow!(
            "undefined method permit for user parameter"
        ))
    })?;
    let params = raw.permit(&permit_keys(&[
        "dnd_enabled",
        "quiet_hours_enabled",
        "quiet_hours_start",
        "quiet_hours_end",
        "meeting_dnd_enabled",
        "ooo_notify_enabled",
    ]));
    let mut user = c
        .app()
        .db
        .read(move |conn| UserStatusSettings::find(conn, id))
        .await
        .map_err(Error::internal)?;
    let mut nulls = Vec::new();
    for (key, field) in [
        ("dnd_enabled", &mut user.dnd_enabled),
        ("quiet_hours_enabled", &mut user.quiet_hours_enabled),
        ("meeting_dnd_enabled", &mut user.meeting_dnd_enabled),
        ("ooo_notify_enabled", &mut user.ooo_notify_enabled),
    ] {
        if let Some(value) = params.get(key) {
            let value = value
                .to_s()
                .map(|s| campfire_db::models::account::cast_boolean(&s))
                .unwrap_or(Some(true));
            *field = value.unwrap_or(false);
            if value.is_none() {
                nulls.push(key);
            }
        }
    }
    if let Some(value) = params.get("quiet_hours_start") {
        user.quiet_hours_start_minute = value.to_s().and_then(|s| clock_time_to_minutes(&s));
    }
    if let Some(value) = params.get("quiet_hours_end") {
        user.quiet_hours_end_minute = value.to_s().and_then(|s| clock_time_to_minutes(&s));
    }
    if raw.get("dnd_enabled").is_some() {
        user.reconcile_dnd_timer(c.app().db.env().now());
    }
    let keywords = raw.get("keyword_alerts").map(keyword_lines).transpose()?;
    let submitted = user.clone();
    let result = c
        .app()
        .db
        .write(move |tx| {
            user.save_with_keywords(tx, keywords.as_deref())?;
            // Boolean casts of nil/empty remain nil in Rails and hit the NOT NULL column after
            // validation; preserve that failure and roll back the replacement, rather than turn off.
            for key in nulls {
                tx.conn()
                    .execute(&format!("UPDATE users SET {key}=NULL WHERE id=?"), [id])?;
            }
            Ok(())
        })
        .await;
    match result {
        Ok(()) => {
            let location = c.url_for(&campfire_routes::user_profile());
            c.redirect_to_with(
                &location,
                Redirect {
                    notice: Some("✓".into()),
                    ..Redirect::default()
                },
            )
        }
        Err(campfire_db::Error::RecordInvalid(errors)) => {
            super::profiles::render_settings(c, StatusCode::UNPROCESSABLE_ENTITY, submitted, errors)
                .await
        }
        Err(error) => Err(Error::internal(error)),
    }
}

fn keyword_lines(value: &Param) -> Result<Vec<String>> {
    match value {
        Param::Null => Ok(Vec::new()),
        Param::Array(values) => Ok(values.iter().map(ruby_to_s).collect()),
        value => Ok(vec![ruby_to_s(value)]),
    }
}

pub(super) fn ruby_to_s(value: &Param) -> String {
    value.to_s().unwrap_or_else(|| ruby_inspect(value))
}
fn ruby_inspect(value: &Param) -> String {
    match value {
        Param::Null => "nil".into(),
        Param::Str(s) => {
            let mut result = String::from("\"");
            let mut chars = s.chars().peekable();
            while let Some(c) = chars.next() {
                match c {
                    '\\' => result.push_str("\\\\"),
                    '"' => result.push_str("\\\""),
                    '\u{7}' => result.push_str("\\a"),
                    '\u{8}' => result.push_str("\\b"),
                    '\t' => result.push_str("\\t"),
                    '\n' => result.push_str("\\n"),
                    '\u{b}' => result.push_str("\\v"),
                    '\u{c}' => result.push_str("\\f"),
                    '\r' => result.push_str("\\r"),
                    '\u{1b}' => result.push_str("\\e"),
                    '#' if chars.peek().is_some_and(|c| matches!(c, '{' | '@' | '$')) => {
                        result.push_str("\\#")
                    }
                    c if c.is_control() || matches!(c, '\u{2028}' | '\u{2029}') => {
                        result.push_str(&format!("\\u{:04X}", c as u32))
                    }
                    c => result.push(c),
                }
            }
            result.push('"');
            result
        }
        Param::Array(values) => format!(
            "[{}]",
            values
                .iter()
                .map(ruby_inspect)
                .collect::<Vec<_>>()
                .join(", ")
        ),
        Param::Hash(values) => format!(
            "{{{}}}",
            values
                .iter()
                .map(|(key, value)| format!(
                    "{} => {}",
                    ruby_inspect(&Param::Str(key.clone())),
                    ruby_inspect(value)
                ))
                .collect::<Vec<_>>()
                .join(", ")
        ),
        value => value.to_s().unwrap_or_default(),
    }
}

#[cfg(test)]
mod tests {
    use crate::controllers::presenters::test_support::{DAVID, Req, TestApp};
    use axum::http::{Method, StatusCode};
    use campfire_db::{KeywordAlert, UserStatusSettings};
    use campfire_kit::Param;
    use jiff::SignedDuration;
    const PATH: &str = "/users/me/notification_settings";
    async fn boot() -> TestApp {
        let app = TestApp::boot()
            .await
            .expect("WS17 requires the parity seed");
        app.db().write(|tx| {
            // Rails named controller tests use users.yml without confirmed 2FA. The actual
            // seeded enabled-credential failure is replayed separately by the status tests.
            tx.conn().execute("UPDATE two_factor_credentials SET confirmed_at=NULL WHERE user_id=?",[DAVID])?;
            tx.conn().execute("UPDATE users SET dnd_enabled=0,dnd_until=NULL,quiet_hours_enabled=0,quiet_hours_start_minute=NULL,quiet_hours_end_minute=NULL,meeting_dnd_enabled=0,ooo_notify_enabled=0 WHERE id=?", [DAVID])?;
            tx.conn().execute("DELETE FROM keyword_alerts WHERE user_id=?", [DAVID])?;
            Ok(())
        }).await.unwrap();
        app
    }
    async fn settings(app: &TestApp) -> UserStatusSettings {
        app.db()
            .read(|c| UserStatusSettings::find(c, DAVID))
            .await
            .unwrap()
    }
    async fn phrases(app: &TestApp) -> Vec<String> {
        app.db()
            .read(|c| {
                Ok(KeywordAlert::for_user(c, DAVID)?
                    .into_iter()
                    .map(|a| a.phrase)
                    .collect())
            })
            .await
            .unwrap()
    }
    #[tokio::test]
    async fn ws17_enables_dnd_with_quiet_hours_and_keywords() {
        let app = boot().await;
        let mut browser = app.david();
        let reply = browser
            .write(Req::new(Method::PATCH, PATH).form(&[
                ("user[dnd_enabled]", "1"),
                ("user[quiet_hours_enabled]", "1"),
                ("user[quiet_hours_start]", "22:00"),
                ("user[quiet_hours_end]", "07:00"),
                ("user[keyword_alerts]", "deploy\nlaunch day"),
            ]))
            .await;
        assert_eq!(
            reply.location(),
            Some("http://campfire.test/users/me/profile")
        );
        let user = settings(&app).await;
        assert!(user.dnd_enabled && user.quiet_hours_enabled);
        assert_eq!(
            (user.quiet_hours_start_minute, user.quiet_hours_end_minute),
            (Some(1320), Some(420))
        );
        assert_eq!(phrases(&app).await, ["deploy", "launch day"]);
    }
    #[tokio::test]
    async fn ws17_disables_dnd_and_clears_keywords() {
        let app = boot().await;
        app.db()
            .write(|tx| {
                tx.conn()
                    .execute("UPDATE users SET dnd_enabled=1 WHERE id=?", [DAVID])?;
                KeywordAlert::create(tx, DAVID, "deploy")?;
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.david();
        let reply = browser
            .write(Req::new(Method::PATCH, PATH).form(&[
                ("user[dnd_enabled]", "0"),
                ("user[quiet_hours_enabled]", "0"),
                ("user[keyword_alerts]", ""),
            ]))
            .await;
        assert_eq!(reply.status, StatusCode::FOUND);
        assert!(!settings(&app).await.dnd_enabled);
        assert!(phrases(&app).await.is_empty());
    }
    #[tokio::test]
    async fn ws17_enables_quiet_during_meetings() {
        let app = boot().await;
        let mut browser = app.david();
        assert_eq!(
            browser
                .write(Req::new(Method::PATCH, PATH).form(&[("user[meeting_dnd_enabled]", "1")]))
                .await
                .status,
            StatusCode::FOUND
        );
        assert!(settings(&app).await.meeting_dnd_enabled);
    }
    #[tokio::test]
    async fn ws17_toggles_keep_notifying_while_out_of_office_defaulting_off() {
        let app = boot().await;
        assert!(!settings(&app).await.ooo_notify_enabled);
        let mut browser = app.david();
        for (input, expected) in [("1", true), ("0", false)] {
            assert_eq!(
                browser
                    .write(
                        Req::new(Method::PATCH, PATH).form(&[("user[ooo_notify_enabled]", input)])
                    )
                    .await
                    .status,
                StatusCode::FOUND
            );
            assert_eq!(settings(&app).await.ooo_notify_enabled, expected);
        }
    }
    #[tokio::test]
    async fn ws17_quiet_hours_without_a_window_render_errors_and_submitted_values() {
        let app = boot().await;
        let mut browser = app.david();
        let reply = browser
            .write(Req::new(Method::PATCH, PATH).form(&[
                ("user[quiet_hours_enabled]", "1"),
                ("user[quiet_hours_start]", ""),
                ("user[quiet_hours_end]", ""),
            ]))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!settings(&app).await.quiet_hours_enabled);
        let body = reply.text();
        assert!(body.contains("Quiet hours needs a start and an end while quiet hours are on."));
        assert!(body.contains("id=\"user_quiet_hours_enabled\" value=\"1\" class=\"switch__input\" checked=\"checked\""));
    }
    #[tokio::test]
    async fn ws17_a_failed_save_keeps_the_previous_keywords() {
        let app = boot().await;
        app.db()
            .write(|tx| KeywordAlert::create(tx, DAVID, "deploy"))
            .await
            .unwrap();
        let mut browser = app.david();
        let reply = browser
            .write(Req::new(Method::PATCH, PATH).form(&[
                ("user[quiet_hours_enabled]", "1"),
                ("user[quiet_hours_start]", ""),
                ("user[quiet_hours_end]", ""),
                ("user[keyword_alerts]", "launch"),
            ]))
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert_eq!(phrases(&app).await, ["deploy"]);
        assert!(reply.text().contains("\ndeploy</textarea>"));
    }
    #[tokio::test]
    async fn ws17_notification_settings_requires_sign_in() {
        let app = boot().await;
        let mut browser = app.anonymous();
        assert_eq!(
            browser
                .write(Req::new(Method::PATCH, PATH).form(&[("user[dnd_enabled]", "1")]))
                .await
                .location(),
            Some("http://campfire.test/session/new")
        );
    }
    async fn timer_request(delta: i64, enabled: bool) -> (TestApp, campfire_db::Timestamp) {
        let app = boot().await;
        let end = app
            .db()
            .env()
            .now()
            .since(SignedDuration::from_hours(delta));
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE users SET dnd_enabled=1,dnd_until=? WHERE id=?",
                    rusqlite::params![end, DAVID],
                )?;
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.david();
        assert_eq!(
            browser
                .write(
                    Req::new(Method::PATCH, PATH)
                        .form(&[("user[dnd_enabled]", if enabled { "1" } else { "0" })])
                )
                .await
                .status,
            StatusCode::FOUND
        );
        (app, end)
    }
    #[tokio::test]
    async fn ws17_enabling_dnd_after_a_timed_expiry_starts_it_indefinitely() {
        let (app, _) = timer_request(-1, true).await;
        let user = settings(&app).await;
        assert!(user.dnd_enabled);
        assert_eq!(user.dnd_until, None);
    }
    #[tokio::test]
    async fn ws17_disabling_dnd_clears_a_running_timer() {
        let (app, _) = timer_request(1, false).await;
        let user = settings(&app).await;
        assert!(!user.dnd_enabled);
        assert_eq!(user.dnd_until, None);
    }
    #[tokio::test]
    async fn ws17_saving_settings_preserves_a_running_dnd_timer() {
        let (app, end) = timer_request(1, true).await;
        assert_eq!(settings(&app).await.dnd_until, Some(end));
    }
    #[tokio::test]
    async fn ws17_notification_settings_sql_failure_rolls_back_prior_keyword_replacement() {
        let app = boot().await;
        app.db().write(|tx| {KeywordAlert::create(tx,DAVID,"deploy")?;tx.conn().execute_batch("CREATE TRIGGER ws17_settings_failure BEFORE UPDATE OF dnd_enabled ON users BEGIN SELECT RAISE(ABORT,'ws17 deliberate write failure'); END;")?;Ok(())}).await.unwrap();
        let mut browser = app.david();
        let reply = browser
            .write(Req::new(Method::PATCH, PATH).form(&[
                ("user[dnd_enabled]", "1"),
                ("user[keyword_alerts]", "launch"),
            ]))
            .await;
        assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(phrases(&app).await, ["deploy"]);
        assert!(!settings(&app).await.dnd_enabled);
    }
    #[tokio::test]
    async fn ws17_keyword_input_shapes_match_actual_rails_params_and_writes() {
        let golden: serde_json::Value = serde_json::from_str(include_str!(
            "../../../../../vectors/ws17_keyword_input.json"
        ))
        .unwrap();
        let app = boot().await;
        for row in golden["rows"].as_array().unwrap() {
            let input = Param::from_json(row["input"].clone());
            let lines = super::keyword_lines(&input).unwrap();
            assert_eq!(serde_json::json!(lines), row["lines"], "{row}");
            let mut browser = app.david();
            let reply = browser
                .write(
                    Req::new(Method::PATCH, PATH)
                        .header("content-type", "application/json")
                        .body(
                            serde_json::json!({"user":{"keyword_alerts": row["input"]}})
                                .to_string(),
                        ),
                )
                .await;
            assert_eq!(reply.status, StatusCode::FOUND, "{row}: {}", reply.text());
            assert_eq!(
                serde_json::json!(phrases(&app).await),
                row["phrases"],
                "{row}"
            );
        }
    }
    #[tokio::test]
    async fn ws17_notification_form_sorts_keywords_and_keeps_invalid_replacements_out() {
        let app = boot().await;
        app.db()
            .write(|tx| {
                KeywordAlert::create(tx, DAVID, "production")?;
                KeywordAlert::create(tx, DAVID, "deploy <freeze> & ready")?;
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.david();
        let reply = browser
            .send(Req::new(Method::GET, "/users/me/profile"))
            .await;
        assert!(
            reply
                .text()
                .contains("\ndeploy &lt;freeze&gt; &amp; ready\nproduction</textarea>")
        );
        let long = "x".repeat(81);
        let reply = browser
            .write(
                Req::new(Method::PATCH, PATH)
                    .form(&[("user[dnd_enabled]", "1"), ("user[keyword_alerts]", &long)]),
            )
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
        assert!(!settings(&app).await.dnd_enabled);
        assert!(
            reply
                .text()
                .contains("\ndeploy &lt;freeze&gt; &amp; ready\nproduction</textarea>")
        );
        assert!(
            reply
                .text()
                .contains("Phrase is too long (maximum is 80 characters).")
        );
    }
    #[tokio::test]
    async fn ws17_nil_boolean_fails_and_rolls_back_keywords_like_rails() {
        let app = boot().await;
        app.db()
            .write(|tx| KeywordAlert::create(tx, DAVID, "deploy"))
            .await
            .unwrap();
        let mut browser = app.david();
        let reply = browser
            .write(
                Req::new(Method::PATCH, PATH)
                    .header("content-type", "application/json")
                    .body(r#"{"user":{"dnd_enabled":null,"keyword_alerts":"launch"}}"#),
            )
            .await;
        assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
        assert_eq!(phrases(&app).await, ["deploy"]);
        assert!(!settings(&app).await.dnd_enabled);
    }
    #[tokio::test]
    async fn ws17_notification_settings_requires_a_user_hash_like_rails() {
        let app = boot().await;
        let mut browser = app.david();
        for (value, expected) in [
            (serde_json::json!(false), StatusCode::INTERNAL_SERVER_ERROR),
            (serde_json::json!(true), StatusCode::INTERNAL_SERVER_ERROR),
            (
                serde_json::json!("person"),
                StatusCode::INTERNAL_SERVER_ERROR,
            ),
            (serde_json::json!([1]), StatusCode::INTERNAL_SERVER_ERROR),
            (serde_json::json!(null), StatusCode::BAD_REQUEST),
            (serde_json::json!({}), StatusCode::BAD_REQUEST),
        ] {
            let reply = browser
                .write(
                    Req::new(Method::PATCH, PATH)
                        .header("content-type", "application/json")
                        .body(serde_json::json!({"user": value}).to_string()),
                )
                .await;
            assert_eq!(reply.status, expected, "{value}: {}", reply.text());
        }
    }
}
