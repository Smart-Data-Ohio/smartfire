use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};

const BOARD: i64 = 699448332;

#[tokio::test]
async fn create_board_post_assigns_owner_and_records_creation_audit() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,?,'mentions',?,?)",(BOARD,KEVIN,tx.now(),tx.now()))?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let response = browser
        .write(
            Req::new(Method::POST, &format!("/rooms/{BOARD}/threads.json")).form(&[
                ("thread[name]", "Release plan"),
                ("thread[work_status]", "planned"),
                ("thread[work_owner_id]", &KEVIN.to_string()),
                ("thread[tags]", "Launch, api"),
            ]),
        )
        .await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let body: serde_json::Value = serde_json::from_str(&response.text()).unwrap();
    assert_eq!(body["thread"]["name"], "Release plan");
    let id = body["thread"]["id"].as_i64().unwrap();
    let (owner, events, members, tags) = app
        .db()
        .read(move |conn| {
            let row = campfire_db::ChannelThread::find(conn, id)?;
            let count = conn.query_row(
                "SELECT COUNT(*) FROM work_thread_events WHERE channel_thread_id=?",
                [id],
                |r| r.get::<_, i64>(0),
            )?;
            let members = conn.query_row(
                "SELECT COUNT(*) FROM thread_memberships WHERE thread_id=? AND user_id=?",
                [id, DAVID],
                |r| r.get::<_, i64>(0),
            )?;
            Ok((row.work_owner_id, count, members, row.tag_names(conn)?))
        })
        .await
        .unwrap();
    assert_eq!(owner, Some(KEVIN));
    assert_eq!(events, 1);
    assert_eq!(members, 1);
    assert_eq!(tags, ["api", "launch"]);
}

#[tokio::test]
async fn board_writes_match_complete_rails_responses_without_masks() {
    let env = std::fs::read_to_string(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../parity/.env.reference"
    ))
    .unwrap();
    let vapid = env
        .lines()
        .filter_map(|line| line.split_once('='))
        .filter(|(key, _)| matches!(*key, "VAPID_PUBLIC_KEY" | "VAPID_PRIVATE_KEY"))
        .collect::<Vec<_>>();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/boards_write.json")).unwrap();
    let mut mismatches = Vec::new();
    for row in oracle["rows"].as_array().unwrap() {
        let app = TestApp::boot_frozen_with_env(&vapid)
            .await
            .expect("default seed required")
            .without_job_runner()
            .await;
        let setup = row["setup"]
            .as_array()
            .unwrap()
            .iter()
            .map(|value| value.as_str().unwrap().to_string())
            .collect::<Vec<_>>();
        app.db()
            .write(move |tx| {
                for sql in setup {
                    tx.conn().execute_batch(&sql)?;
                }
                Ok(())
            })
            .await
            .unwrap();
        let mut browser = app.sign_in(row["user_id"].as_i64().unwrap()).await;
        let method = row["method"]
            .as_str()
            .unwrap()
            .to_ascii_uppercase()
            .parse::<Method>()
            .unwrap();
        let request =
            Req::new(method, row["path"].as_str().unwrap()).header("user-agent", "Mozilla");
        let request = if let Some(accept) = row["accept"].as_str() {
            request.header("accept", accept)
        } else {
            request
        };
        let request = if row["encoding"] == "json" {
            request
                .header("content-type", "application/json")
                .body(row["input"].to_string())
        } else {
            let pairs = row["input"]["thread"]
                .as_object()
                .map(|fields| {
                    fields
                        .iter()
                        .map(|(key, value)| {
                            (
                                format!("thread[{key}]"),
                                campfire_richtext::ruby::json_value_to_s(value),
                            )
                        })
                        .collect::<Vec<_>>()
                })
                .unwrap_or_default();
            request.form(
                &pairs
                    .iter()
                    .map(|(key, value)| (key.as_str(), value.as_str()))
                    .collect::<Vec<_>>(),
            )
        };
        let response = with_fixed_render_secrets(browser.write(request)).await;
        assert_eq!(
            response.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}: {}",
            row["name"],
            response.text()
        );
        assert_eq!(
            response.location(),
            row["location"].as_str(),
            "{}",
            row["name"]
        );
        if row["content_type"].as_str().is_some_and(|kind| kind.starts_with("application/json")) {
            assert_eq!(response.content_type(), row["content_type"].as_str());
        }
        let expected = if row["name"] == "create-invalid-tags-json-preferred" { "{\"error\":\"Tags use lowercase letters, digits, and hyphens\"}" } else if response.status == StatusCode::UNPROCESSABLE_ENTITY && !response.content_type().is_some_and(|kind| kind.starts_with("application/json")) { "" } else { row["body"].as_str().unwrap() };
        if response.text() != expected {
            let directory = std::env::temp_dir().join("ws12-write-diffs");
            std::fs::create_dir_all(&directory).unwrap();
            let name = row["name"].as_str().unwrap();
            std::fs::write(
                directory.join(format!("{name}-actual.txt")),
                response.text(),
            )
            .unwrap();
            std::fs::write(directory.join(format!("{name}-expected.txt")), expected).unwrap();
            mismatches.push(name.to_string());
        }
    }
    assert!(
        mismatches.is_empty(),
        "complete Rails response mismatches: {mismatches:?}"
    );
}

async fn counts(app: &TestApp) -> Vec<i64> {
    app.db()
        .read(|conn| {
            [
                "channel_threads",
                "messages",
                "thread_memberships",
                "thread_tags",
                "work_thread_events",
                "agent_events",
                "activity_items",
                "background_jobs",
            ]
            .iter()
            .map(|table| {
                Ok(conn.query_row(&format!("SELECT COUNT(*) FROM {table}"), [], |r| r.get(0))?)
            })
            .collect()
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn board_creation_rolls_back_every_row_when_the_atomic_job_insert_fails() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let mut browser = app.sign_in(DAVID).await;
    app.db().write(|tx| {
        tx.conn().execute("UPDATE memberships SET involvement='everything' WHERE room_id=? AND user_id=?",(BOARD,JASON))?;
        tx.conn().execute_batch("CREATE TRIGGER ws12_reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'WS12 rejects atomic job'); END")?;
        Ok(())
    }).await.unwrap();
    let before = counts(&app).await;
    let response=browser.write(Req::new(Method::POST,&format!("/rooms/{BOARD}/threads.json"))
        .header("content-type","application/json").body(serde_json::json!({"thread":{"name":"Rollback post","work_owner_id":BENDER,"tags":"atomic","first_message":"Brief"}}).to_string())).await;
    assert_eq!(
        response.status,
        StatusCode::INTERNAL_SERVER_ERROR,
        "{}",
        response.text()
    );
    assert_eq!(counts(&app).await, before);
}

#[tokio::test]
async fn opening_message_notifies_room_followers_and_assignee_with_notifications_off() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    app.db().write(|tx| {
        tx.conn().execute("INSERT INTO memberships(room_id,user_id,involvement,created_at,updated_at) VALUES(?,?,'nothing',?,?)",(BOARD,KEVIN,tx.now(),tx.now()))?;
        tx.conn().execute("UPDATE memberships SET involvement='everything' WHERE room_id=? AND user_id=?",(BOARD,JASON))?;Ok(())
    }).await.unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let response=browser.write(Req::new(Method::POST,&format!("/rooms/{BOARD}/threads.json")).header("content-type","application/json")
        .body(serde_json::json!({"thread":{"name":"Brief","work_owner_id":KEVIN,"first_message":"## Context"}}).to_string())).await;
    assert_eq!(response.status, StatusCode::CREATED, "{}", response.text());
    let id = response.json()["thread"]["id"].as_i64().unwrap();
    let recipients=app.db().read(move |conn| {
        let message=campfire_db::Message::in_thread(conn,id)?.remove(0);
        assert!(message.board_post_opener);
        let mut statement=conn.prepare("SELECT user_id,event_type FROM activity_items WHERE source_type='Message' AND source_id=? ORDER BY user_id")?;
        Ok(statement.query_map([message.id],|r|Ok((r.get::<_,i64>(0)?,r.get::<_,String>(1)?)))?.collect::<std::result::Result<Vec<_>,_>>()?)
    }).await.unwrap();
    assert_eq!(
        recipients,
        [
            (JASON, "thread_activity".into()),
            (KEVIN, "thread_activity".into())
        ]
    );
}

#[tokio::test]
async fn null_title_rejects_result_without_persisting_any_write() {
    let app = TestApp::boot_frozen()
        .await
        .expect("default seed required")
        .without_job_runner()
        .await;
    let before = counts(&app).await;
    let original = app
        .db()
        .read(|conn| campfire_db::ChannelThread::find(conn, 4))
        .await
        .unwrap();
    let mut browser = app.sign_in(DAVID).await;
    let response = browser
        .write(
            Req::new(Method::PATCH, &format!("/rooms/{BOARD}/threads/4.json"))
                .header("content-type", "application/json")
                .body(
                    serde_json::json!({"thread":{"name":null,"result_markdown":"x"}}).to_string(),
                ),
        )
        .await;
    assert_eq!(
        response.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        response.text()
    );
    assert_eq!(response.text(), "{\"error\":\"Name can't be blank\"}");
    assert_eq!(counts(&app).await, before);
    let persisted = app
        .db()
        .read(|conn| campfire_db::ChannelThread::find(conn, 4))
        .await
        .unwrap();
    assert_eq!(persisted.name, original.name);
    assert_eq!(persisted.result_markdown, original.result_markdown);
    assert_eq!(persisted.updated_at, original.updated_at);
}
