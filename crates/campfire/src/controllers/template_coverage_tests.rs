//! Coverage gaps use real controllers over the default Rails seed, with fixed render entropy.
use super::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use serde_json::Value;

#[tokio::test]
async fn uncovered_controller_branches_match_fresh_pinned_rails_bytes() {
    let oracle: Value = serde_json::from_str(
        &std::fs::read_to_string(
            std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
                .join("../../vectors/template_coverage_http.json"),
        )
        .unwrap(),
    )
    .unwrap();
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let mut browser = app.david();
    let mut differences = Vec::new();
    for case in oracle["cases"].as_array().unwrap() {
        let name = case["name"].as_str().unwrap();
        if let Some(level) = name.strip_prefix("involvement_") {
            let level = level.to_owned();
            app.db()
                .write(move |tx| {
                    tx.conn().execute(
                        "UPDATE memberships SET involvement=? WHERE user_id=? AND room_id=?",
                        (level, DAVID, ALL_TALK),
                    )?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        if let Some(status) = case["setup"]["user_status"].as_i64() {
            app.db()
                .write(move |tx| {
                    tx.conn()
                        .execute("UPDATE users SET status=? WHERE id=?", (status, KEVIN))?;
                    Ok(())
                })
                .await
                .unwrap();
        }
        if name == "missing_creator" {
            // The oracle deliberately disables referential integrity for this corrupt-row
            // rescue branch. Alter only this test's private database, outside a transaction.
            let conn = rusqlite::Connection::open(&app.booted.app.config.storage.database).unwrap();
            conn.pragma_update(None, "foreign_keys", false).unwrap();
            conn.execute(
                "UPDATE messages SET creator_id=2100000000 WHERE id=?",
                [case["setup"]["missing_creator_message"].as_i64().unwrap()],
            )
            .unwrap();
        }
        if let Some(room_id) = case["setup"]["empty_original_room"].as_i64() {
            // Mirror the oracle's callback-free corrupt-row cleanup in this private DB.
            let conn = rusqlite::Connection::open(&app.booted.app.config.storage.database).unwrap();
            conn.pragma_update(None, "foreign_keys", false).unwrap();
            conn.execute("DELETE FROM messages WHERE room_id=?", [room_id])
                .unwrap();
        }
        if name == "users_page_one" {
            let users = oracle["inserted_users"].as_array().unwrap().clone();
            app.db().write(move |tx| {
                for user in users {
                    tx.conn().execute("INSERT INTO users(id,name,role,status,created_at,updated_at) VALUES(?,?,0,0,?,?)",rusqlite::params![user["id"].as_i64().unwrap(),user["name"].as_str().unwrap(),tx.now(),tx.now()])?;
                }
                Ok(())
            }).await.unwrap();
        }
        if name == "root_create" {
            app.db().write(|tx| {
                tx.conn().execute("INSERT INTO channel_threads(id,room_id,creator_id,name,last_activity_at,created_at,updated_at) VALUES(1900500000,486777696,127326141,'Coverage thread',?,?,?)", (tx.now(),tx.now(),tx.now()))?;
                Ok(())
            }).await.unwrap();
        }
        let mut req = Req::new(
            if case["method"] == "POST" {
                Method::POST
            } else {
                Method::GET
            },
            case["path"].as_str().unwrap(),
        );
        if name == "thread_create" {
            req = req.header("accept", "application/json");
        } else if case["region"] == "stream" {
            req = req.header("accept", "text/vnd.turbo-stream.html");
        }
        let reply = if case["method"] == "POST" {
            req = req
                .header("content-type", "application/json")
                .body(serde_json::json!({"message":case["input"]}).to_string());
            with_fixed_render_secrets(browser.write(req)).await
        } else {
            with_fixed_render_secrets(browser.send(req)).await
        };
        if matches!(name, "users_page_one" | "users_page_two") {
            assert_eq!(reply.status, StatusCode::FOUND);
            assert_eq!(reply.location(), Some("http://campfire.test/app/admin/people"));
            assert!(reply.body.is_empty());
            continue;
        }
        if matches!(name, "root_create" | "thread_create" | "original_create") {
            assert_eq!(reply.status, StatusCode::CREATED, "{name}: {}", reply.text());
            let client_id = case["input"]["client_message_id"]
                .as_str().unwrap().to_owned();
            let message = app.db().read(move |conn| {
                Ok(conn.query_row(
                    "SELECT id, thread_id FROM messages WHERE client_message_id=?",
                    [client_id],
                    |row| Ok((row.get::<_, i64>(0)?, row.get::<_, Option<i64>>(1)?)),
                )?)
            }).await.unwrap();
            if name == "thread_create" {
                assert_eq!(reply.json()["id"], message.0);
                assert_eq!(message.1, Some(1900500000));
            } else {
                assert!(reply.body.is_empty());
                assert_eq!(message.1, None);
            }
            continue;
        }
        assert_eq!(
            reply.status.as_u16(),
            case["status"].as_u64().unwrap() as u16,
            "{name}: {}",
            reply.text()
        );
        let text = reply.text();
        let actual = if case["region"] == "main" {
            text.split_once("<main id=\"main-content\">")
                .unwrap()
                .1
                .split_once("</main>")
                .unwrap()
                .0
        } else {
            &text
        };
        let expected = case["body"].as_str().unwrap();
        if actual != expected {
            let dir = std::env::temp_dir().join("template-coverage-diffs");
            std::fs::create_dir_all(&dir).unwrap();
            std::fs::write(dir.join(format!("{name}.actual")), actual).unwrap();
            std::fs::write(dir.join(format!("{name}.expected")), expected).unwrap();
            differences.push(name.to_owned());
        }
    }
    assert!(
        differences.is_empty(),
        "Rails byte differences: {differences:?}"
    );
}
