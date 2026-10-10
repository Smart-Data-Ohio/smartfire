//! The four exact original profile clauses reopened by the #244 assertion audit.
use crate::controllers::presenters::test_support::*;
use axum::http::Method;
use serde_json::{Value, json};
async fn run(name: &str) {
    let app = TestApp::boot_with_clock_and_env(
        std::sync::Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
        &[
            ("GOOGLE_CLIENT_ID", "parity-client"),
            ("GOOGLE_CLIENT_SECRET", "parity-secret"),
        ],
    )
    .await
    .expect("CI seed required")
    .without_job_runner()
    .await;
    let oracle: Value = serde_json::from_str(include_str!(
        "../../../../../vectors/profile_gap_originals.json"
    ))
    .unwrap();
    let case = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|c| c["name"] == name)
        .unwrap();
    if matches!(name, "meeting_error" | "connected_email") {
        let error = case["notice_html"].as_str().is_some();
        app.db().write(move|tx|{
            tx.conn().execute("DELETE FROM google_accounts WHERE user_id=?",[DAVID])?;
            tx.conn().execute("INSERT INTO google_accounts(user_id,email,created_at,updated_at) VALUES (?,'david@gmail.test',?,?)",rusqlite::params![DAVID,tx.now(),tx.now()])?;
            if error {
                tx.conn().execute("UPDATE users SET meeting_status_enabled=1 WHERE id=?",[DAVID])?;
                tx.conn().execute("DELETE FROM calendar_meeting_caches WHERE user_id=?",[DAVID])?;
                tx.conn().execute("INSERT INTO calendar_meeting_caches(user_id,fetched_at,fetch_error,created_at,updated_at) VALUES (?,?,?,?,?)",rusqlite::params![DAVID,tx.now(),"Google Calendar couldn't be reached; calendar status will retry.",tx.now(),tx.now()])?;
            }
            Ok(())
        }).await.unwrap();
    }
    let mut b = app.david();
    let reply = if name.starts_with("email_") {
        let pairs = case["params"]
            .as_object()
            .unwrap()
            .iter()
            .map(|(k, v)| (format!("user[{k}]"), v.as_str().unwrap().to_string()))
            .collect::<Vec<_>>();
        let form = pairs
            .iter()
            .map(|(k, v)| (k.as_str(), v.as_str()))
            .collect::<Vec<_>>();
        b.write(Req::new(Method::PUT, "/users/me/profile").form(&form))
            .await
    } else {
        b.get("/users/me/profile").await
    };
    let mut actual = json!({"status":reply.status.as_u16()});
    match name {
        "meeting_error" => {
            actual["notice"] = json!(reply.text().contains(case["notice_html"].as_str().unwrap()))
        }
        "connected_email" => {
            actual["connected_email"] =
                json!(reply.text().contains("Connected as david@gmail.test"));
            actual["disconnect"] = json!(reply.text().contains("Disconnect"));
        }
        _ => {
            let (user, marker) = app
                .db()
                .read(|c| {
                    Ok((
                        campfire_db::User::find(c, DAVID)?,
                        c.query_row(
                            "SELECT email_self_changed_at FROM users WHERE id=?",
                            [DAVID],
                            |r| r.get::<_, Option<campfire_db::Timestamp>>(0),
                        )?,
                    ))
                })
                .await
                .unwrap();
            actual["location"] = json!(reply.location());
            actual["email"] = json!(user.email_address);
            actual["name"] = json!(user.name);
            actual["marker"] = json!(marker.map(|t| t.jiff().to_string()));
        }
    }
    assert_eq!(
        actual, case["response"],
        "{name}: exact original profile gap"
    );
}
#[tokio::test]
async fn original_email_change_with_current_password_marks_now() {
    run("email_change").await;
}
#[tokio::test]
async fn original_case_only_email_and_dave_name_leave_marker_null() {
    run("email_case").await;
}
