//! Message-to-Fizzy JSON parity, using the same HTTP stubs as the classic matrix.
use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        fizzy::accounts::{Account, Input},
        test_support::{
            FakeServer, ResponseGate, Route, ws15e_http_case, ws15e_http_case_listener,
        },
    },
};
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::json;
use std::sync::Arc;

#[tokio::test]
async fn spa_api_fizzy_message_creation_http_matrix() {
    if let Ok(case) = std::env::var("SPA_API_FIZZY_MESSAGE_CASE") {
        let api = run(&case, true).await;
        if matches!(case.as_str(), "create" | "thread" | "direct_bots") {
            let classic = run(&case, false).await;
            assert_eq!(api.0, classic.0, "reply and bot jobs must match");
        } else if matches!(case.as_str(), "enqueue_rollback" | "locked_after_create") {
            let classic = run(&case, false).await;
            assert_eq!(api.0, classic.0, "failed replies must leave the same rows");
            assert!(api.1.is_empty());
            assert!(classic.1.is_empty());
        }
        return;
    }
    for case in [
        "new",
        "thread_new",
        "locked_new",
        "long_new",
        "wrong_room_thread",
        "wrong_room_thread_post",
        "nonmember_post",
        "scope_post",
        "refused",
        "create_failure",
        "create_unreachable",
        "fallback_url",
        "missing_title",
        "both_missing",
        "invalid_rejected",
        "invalid_unreachable",
        "no_account",
        "new_failure",
        "new_rejected",
        "create",
        "thread",
        "invalid",
        "readonly",
        "revoked",
        "probe_failure",
        "locked",
        "locked_after_create",
        "long_reply",
        "nonmember",
        "scope",
        "enqueue_rollback",
        "no_connection",
        "direct_bots",
    ] {
        let output = ws15e_http_case(
            "SPA_API_FIZZY_MESSAGE_CASE",
            case,
            "controllers::spa::api_fizzy_tests::spa_api_fizzy_message_creation_http_matrix",
        )
        .await;
        let stdout = String::from_utf8_lossy(&output.stdout);
        assert!(
            output.status.success(),
            "{case}: {stdout}\n{}",
            String::from_utf8_lossy(&output.stderr)
        );
        assert!(
            stdout.contains("1 passed; 0 failed"),
            "must execute: {stdout}"
        );
        println!("Fizzy message cards API case {case}: 1 passed; 0 failed");
    }
}
async fn run(case: &str, api_mode: bool) -> (serde_json::Value, Vec<(String, String)>) {
    let listener = ws15e_http_case_listener();
    let base = crate::integrations::fizzy::client::api_base_url();
    let mut app = TestApp::boot_frozen_with_env(&[("SPA_ENABLED", "1")])
        .await
        .expect("pinned seeds required");
    app.booted
        .jobs
        .stop(std::time::Duration::from_secs(1))
        .await;
    app.db()
        .write(|tx| Account::disconnect(tx, DAVID))
        .await
        .unwrap();
    let crypto = ArEncryption::new(&app.booted.app.secrets);
    let room_id = if case == "direct_bots" {
        DIRECT_DAVID_JASON
    } else {
        ALL_TALK
    };
    let threaded = matches!(
        case,
        "thread"
            | "thread_new"
            | "locked_new"
            | "locked"
            | "locked_after_create"
            | "scope"
            | "scope_post"
            | "wrong_room_thread"
            | "wrong_room_thread_post"
    );
    let long_source = case == "long_new";
    let (message, thread) = app
        .db()
        .write(move |tx| {
            let mut message = Message::create(
                tx,
                NewMessage {
                    room_id,
                    creator_id: KEVIN,
                    markdown_source: Some(if long_source {
                        format!("{}\n{}", "A".repeat(150), "B".repeat(360))
                    } else {
                        "The deploy is broken\nTrack the fix".into()
                    }),
                    ..Default::default()
                },
            )?;
            let thread = if threaded {
                let mut thread = ChannelThread::create(
                    tx,
                    NewChannelThread {
                        room_id,
                        creator_id: DAVID,
                        parent_message_id: Some(message.id),
                        ..Default::default()
                    },
                )?;
                message = thread.post_message(
                    tx,
                    DAVID,
                    NewMessage {
                        markdown_source: Some("Threaded problem".into()),
                        ..Default::default()
                    },
                )?;
                Some(thread)
            } else {
                None
            };
            Ok((message, thread))
        })
        .await
        .unwrap();
    if !matches!(case, "no_account" | "no_connection") {
        app.db()
            .write(move |tx| {
                Account::relink(
                    tx,
                    &crypto,
                    &Input {
                        user_id: DAVID,
                        account_id: "897362094",
                        account_name: Some("Smart Data"),
                        fizzy_user_id: Some("03user1"),
                        fizzy_user_name: Some("David"),
                        token: "david-token",
                    },
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
    if matches!(case, "locked" | "locked_new") {
        let id = thread.as_ref().unwrap().id;
        app.db()
            .write(move |tx| {
                tx.conn().execute(
                    "UPDATE channel_threads SET locked_at=? WHERE id=?",
                    rusqlite::params![tx.now(), id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
    let legacy_bot = if matches!(case, "direct_bots" | "thread") {
        Some(app.db().write(move |tx| {
            // Bender is agent-backed in the seed: Rails excludes it from legacy delivery.
            let bot = campfire_db::User::create_integration_bot(tx, "Fizzy legacy fixture")?;
            campfire_db::Webhook::create(tx, bot.id, Some("https://example.test/legacy"))?;
            for user in [BENDER, bot.id] {
                tx.conn().execute("INSERT OR IGNORE INTO memberships (room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![room_id,user,tx.now(),tx.now()])?;
            }
            Ok(bot.id)
        }).await.unwrap())
    } else {
        None
    };
    if case == "enqueue_rollback" {
        app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER ws15e_reject_fizzy BEFORE INSERT ON background_jobs WHEN NEW.job_class='Fizzy::FetchCardJob' BEGIN SELECT RAISE(ABORT,'queue rejected'); END;")?;Ok(())}).await.unwrap();
    }
    let url = if case == "long_reply" {
        format!("https://example.com/{}", "x".repeat(60000))
    } else {
        format!("{base}/897362094/cards/580")
    };
    let create_status = if matches!(case, "readonly" | "revoked" | "probe_failure") {
        401
    } else {
        match case {
            "refused" => 422,
            "create_failure" => 500,
            _ => 201,
        }
    };
    let probe_status = match case {
        "revoked" => 401,
        "probe_failure" => 500,
        _ => 200,
    };
    let board_status = match case {
        "new_failure" | "invalid_unreachable" => 500,
        "new_rejected" | "invalid_rejected" => 401,
        _ => 200,
    };
    let disconnected = if case == "create_unreachable" {
        let listener = ws15e_http_case_listener();
        Some(tokio::spawn(async move {
            loop {
                let (socket, _) = listener.accept().await.unwrap();
                drop(socket);
            }
        }))
    } else {
        None
    };
    drop(listener);
    let gate = (case == "locked_after_create").then(|| Arc::new(ResponseGate::default()));
    let mut create_route = Route::new(
        "POST",
        "127.0.0.1",
        "/897362094/boards/03board1/cards.json",
        create_status,
    )
    .body(
        match case {
            "refused" => json!({"error":"Board denied"}),
            "fallback_url" => json!({"number":"580"}),
            _ => json!({"number":580,"url":url}),
        }
        .to_string(),
    );
    create_route.response_gate = gate.clone();
    let server = if case == "create_unreachable" {
        None
    } else {
        Some(
            FakeServer::on_listener(
                vec![
            Route::new("GET", "127.0.0.1", "/897362094/boards.json", board_status).body(
                json!([{"id":"03board1","name":"Engineering"},{"id":"03board2","name":"Support"}])
                    .to_string(),
            ),
            create_route,
            Route::new("GET", "127.0.0.1", "/my/identity.json", probe_status).body("{}"),
        ],
                None,
                ws15e_http_case_listener(),
            )
            .await,
        )
    };
    let before = app
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let path = if matches!(case, "scope" | "scope_post") {
        format!("/rooms/{room_id}/messages/{}/fizzy_cards", message.id)
    } else if let Some(thread) = &thread {
        format!(
            "/rooms/{room_id}/threads/{}/messages/{}/fizzy_cards",
            thread.id, message.id
        )
    } else {
        format!("/rooms/{room_id}/messages/{}/fizzy_cards", message.id)
    };
    let path = if matches!(case, "wrong_room_thread" | "wrong_room_thread_post") {
        path.replacen(&format!("/rooms/{room_id}"), &format!("/rooms/{HQ}"), 1)
    } else {
        path
    };
    let path = if api_mode {
        format!("/api/v1{path}")
    } else {
        path
    };
    let get = matches!(
        case,
        "new"
            | "thread_new"
            | "locked_new"
            | "long_new"
            | "no_account"
            | "new_failure"
            | "new_rejected"
            | "nonmember"
            | "scope"
            | "wrong_room_thread"
    );
    let success = matches!(case, "create" | "thread" | "direct_bots" | "fallback_url");
    let capture = app.booted.app.cable.capture_every_publication();
    let mut browser = if matches!(case, "nonmember" | "nonmember_post") {
        app.sign_in(KEVIN).await
    } else {
        app.david()
    };
    let (mut sync, live) = if api_mode && success {
        browser.authenticity_token().await;
        let (address, live) = super::api_tests::serve(&app).await;
        let topic = thread.as_ref().map_or_else(
            || format!("room:{room_id}"),
            |thread| format!("thread:{}", thread.id),
        );
        let mut sync =
            super::api_tests::Sync::connect(address, &browser.cookie_header(), &[topic]).await;
        sync.welcome().await;
        (Some(sync), Some(live))
    } else {
        (None, None)
    };
    let input = json!({
        "boardId": if matches!(case, "invalid" | "both_missing" | "invalid_rejected" | "invalid_unreachable") { "" } else { "03board1" },
        "title": if matches!(case, "missing_title" | "both_missing") { "  " } else { "  The deploy is broken  " },
        "description": "Fix it\n\nSource: x",
    });
    let locker = gate.map(|gate| {
        let db = app.db().clone();
        let id = thread.as_ref().unwrap().id;
        tokio::spawn(async move {
            gate.entered.notified().await;
            db.write(move |tx| {
                tx.conn().execute(
                    "UPDATE channel_threads SET locked_at=? WHERE id=?",
                    rusqlite::params![tx.now(), id],
                )?;
                Ok(())
            })
            .await
            .unwrap();
            gate.released.notify_one();
        })
    });
    let response = if get {
        browser.send(super::api_tests::get(&(path + "/new"))).await
    } else if api_mode {
        browser
            .write(super::api_tests::json_body(Method::POST, &path, &input))
            .await
    } else {
        browser
            .write(Req::new(Method::POST, &path).form(&[
                ("board_id", input["boardId"].as_str().unwrap()),
                ("title", input["title"].as_str().unwrap()),
                ("description", input["description"].as_str().unwrap()),
            ]))
            .await
    };
    if let Some(locker) = locker {
        locker.await.unwrap();
    }
    let expected = if !api_mode {
        if case == "enqueue_rollback" {
            StatusCode::INTERNAL_SERVER_ERROR
        } else {
            StatusCode::FOUND
        }
    } else {
        match case {
            "new" | "thread_new" | "locked_new" | "long_new" | "no_account" => StatusCode::OK,
            "nonmember"
            | "nonmember_post"
            | "scope"
            | "scope_post"
            | "wrong_room_thread"
            | "wrong_room_thread_post" => StatusCode::NOT_FOUND,
            "locked" => StatusCode::CONFLICT,
            "new_failure" | "probe_failure" | "create_unreachable" | "invalid_unreachable" => {
                StatusCode::SERVICE_UNAVAILABLE
            }
            _ if success => StatusCode::CREATED,
            _ => StatusCode::UNPROCESSABLE_ENTITY,
        }
    };
    assert_eq!(response.status, expected, "{case}: {}", response.text());
    if matches!(
        case,
        "new" | "thread_new" | "locked_new" | "long_new" | "no_account"
    ) {
        let form: campfire_api_types::FizzyMessageCardForm = super::api_tests::parse(&response);
        assert_eq!(form.connected, case != "no_account");
        assert_eq!(form.boards.len(), if form.connected { 2 } else { 0 });
        assert_eq!(
            form.title,
            if case == "long_new" {
                format!("{}...", "A".repeat(117))
            } else if matches!(case, "thread_new" | "locked_new") {
                "Threaded problem".to_string()
            } else {
                "The deploy is broken".to_string()
            }
        );
        assert_eq!(
            form.excerpt,
            if case == "long_new" {
                format!("{}\n{}...", "A".repeat(150), "B".repeat(126))
            } else if matches!(case, "thread_new" | "locked_new") {
                "Threaded problem".to_string()
            } else {
                "The deploy is broken\nTrack the fix".to_string()
            }
        );
        let source_path = if matches!(case, "thread_new" | "locked_new") {
            format!(
                "?thread={}&message_id={}",
                thread.as_ref().unwrap().id,
                message.id
            )
        } else {
            format!("/@{}", message.id)
        };
        assert!(form.description.ends_with(&source_path));
        assert_eq!(
            form.account_name,
            if form.connected { "Smart Data" } else { "" }
        );
        assert_eq!(
            form.fizzy_user_name,
            if form.connected { "David" } else { "" }
        );
        assert!(!form.room_display_name.is_empty());
        assert_eq!(
            form.author_name,
            if matches!(case, "thread_new" | "locked_new") {
                "David"
            } else {
                "Kevin"
            }
        );
    } else if api_mode && !success {
        let error: campfire_api_types::ApiErrorResponse = super::api_tests::parse(&response);
        let tag = super::api_tests::tag(&response);
        let expected_tag = match case {
            "invalid" | "missing_title" | "both_missing" => "Validation",
            "nonmember"
            | "nonmember_post"
            | "scope"
            | "scope_post"
            | "wrong_room_thread"
            | "wrong_room_thread_post" => "NotFound",
            "new_failure" | "probe_failure" | "create_unreachable" | "invalid_unreachable" => {
                "FizzyUnreachable"
            }
            "new_rejected" | "revoked" | "invalid_rejected" => "FizzyTokenRejected",
            "readonly" => "FizzyReadOnly",
            "no_connection" => "FizzyNotConnected",
            "locked" => "FizzyThreadLocked",
            "long_reply" | "enqueue_rollback" | "locked_after_create" => "FizzyReplyFailed",
            "refused" | "create_failure" => "FizzyRefused",
            _ => panic!("unexpected case: {case}"),
        };
        assert_eq!(tag, expected_tag, "{error:?}");
        let json = serde_json::to_value(&error.error).unwrap();
        let message = json["message"].as_str().unwrap();
        match case {
            "new_failure" | "invalid_unreachable" => {
                assert_eq!(message, "Could not reach Fizzy. Try again.")
            }
            "new_rejected" | "revoked" | "invalid_rejected" => assert_eq!(
                message,
                "Fizzy rejected the linked token. Reconnect on your profile."
            ),
            "readonly" => assert_eq!(
                message,
                "That Fizzy token is read-only. Generate a Read + Write token to create cards."
            ),
            "probe_failure" => assert_eq!(
                message,
                "Could not reach Fizzy to verify the token. Try again."
            ),
            "no_connection" => assert_eq!(message, "Connect Fizzy on your profile first."),
            "locked" => assert_eq!(message, "This thread is locked"),
            "refused" => assert_eq!(
                message,
                "Fizzy refused the new card (Fizzy refused: Board denied)."
            ),
            "create_failure" => {
                assert_eq!(message, "Fizzy refused the new card (Fizzy returned 500).")
            }
            "create_unreachable" => {
                assert!(message.starts_with("Fizzy refused the new card (Could not reach Fizzy ("))
            }
            "long_reply" | "enqueue_rollback" | "locked_after_create" => {
                assert!(
                    message.starts_with(
                        "Fizzy card #580 created, but the reply could not be posted ("
                    )
                );
                assert_eq!(json["number"], "580");
                assert_eq!(json["url"], url);
                if case != "long_reply" {
                    let details = if case == "locked_after_create" {
                        "This thread is locked"
                    } else {
                        "A local error prevented posting the reply"
                    };
                    assert_eq!(
                        message,
                        format!(
                            "Fizzy card #580 created, but the reply could not be posted ({details})."
                        )
                    );
                    let received = server.as_ref().unwrap().received.lock().unwrap();
                    assert_eq!(received.len(), 1, "the remote card must exist");
                    assert_eq!(received[0].method, "POST");
                }
            }
            _ => {}
        }
        if let campfire_api_types::ApiError::Validation { message, fields } = error.error {
            assert_eq!(message, "Choose a board and enter a title.");
            assert_eq!(fields.contains_key("boardId"), case != "missing_title");
            assert_eq!(fields.contains_key("title"), case != "invalid");
        }
    }
    let after = app
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    assert_eq!(after, before + i64::from(success));
    if success {
        let original = message.id;
        let (reply,refs,jobs)=app.db().read(|c| {let id=c.query_row("SELECT MAX(id) FROM messages",[],|r|r.get::<_,i64>(0))?;Ok((Message::find(c,id)?,c.query_row("SELECT COUNT(*) FROM fizzy_card_references WHERE message_id=?",[id],|r|r.get::<_,i64>(0))?,c.query_row("SELECT COUNT(*) FROM background_jobs WHERE job_class='Fizzy::FetchCardJob'",[],|r|r.get::<_,i64>(0))?))}).await.unwrap();
        assert_eq!(reply.reply_to_message_id, Some(original));
        assert_eq!(reply.thread_id, thread.as_ref().map(|t| t.id));
        if api_mode {
            let created: campfire_api_types::CreatedFizzyCard = super::api_tests::parse(&response);
            assert_eq!(created.number, "580");
            assert_eq!(created.notice, "Fizzy card #580 created.");
            assert_eq!(created.message.id, reply.id);
            assert_eq!(
                created.message.reply_to_message_id,
                reply.reply_to_message_id
            );
            assert_eq!(created.message.thread_id, reply.thread_id);
            assert_eq!(created.message.markdown_source, reply.markdown_source);
            assert!(created.url.ends_with("/cards/580"));
            let event = sync.as_mut().unwrap().until(
                |event| matches!(&event.payload, campfire_api_types::SyncPayload::MessageCreated(message) if message.id == created.message.id),
                |_| false,
            ).await;
            assert_eq!(
                event.payload,
                campfire_api_types::SyncPayload::MessageCreated(created.message)
            );
        }
        assert_eq!(refs, 1);
        assert_eq!(jobs, 1);
        assert!(reply.markdown_source.unwrap().contains("/cards/580"));
        let received = server.as_ref().unwrap().received.lock().unwrap();
        let request = received.iter().find(|r| r.method == "POST").unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
            json!({"card":{"title":"The deploy is broken","description":"Fix it\n\nSource: x"}})
        );
    }
    if matches!(
        case,
        "no_account"
            | "no_connection"
            | "locked"
            | "nonmember"
            | "nonmember_post"
            | "scope"
            | "scope_post"
            | "wrong_room_thread"
            | "wrong_room_thread_post"
    ) {
        assert!(server.as_ref().unwrap().received.lock().unwrap().is_empty());
    }
    if matches!(case, "new_rejected" | "revoked" | "invalid_rejected") {
        assert!(
            !app.db()
                .read(|c| Account::for_user(c, DAVID))
                .await
                .unwrap()
                .unwrap()
                .connected()
        );
    }
    if matches!(case, "readonly" | "probe_failure") {
        assert!(
            app.db()
                .read(|c| Account::for_user(c, DAVID))
                .await
                .unwrap()
                .unwrap()
                .connected()
        );
        assert_eq!(server.as_ref().unwrap().received.lock().unwrap().len(), 2);
    }
    if case == "direct_bots" {
        let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
        let jobs: Vec<_> = jobs
            .iter()
            .filter(|j| j.class == "Bot::WebhookJob")
            .collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].arguments["bot_id"], legacy_bot.unwrap());
        assert!(
            jobs.iter().all(|j| j.arguments["bot_id"] != BENDER),
            "agent-backed bot bypassed its ledger"
        );
    }
    if case == "thread" {
        assert!(legacy_bot.is_some());
        let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
        assert!(
            jobs.iter().all(|job| job.class != "Bot::WebhookJob"),
            "thread replies do not deliver legacy bot webhooks"
        );
    }
    if case == "enqueue_rollback" {
        app.db().read(|c| {assert_eq!(c.query_row("SELECT COUNT(*) FROM fizzy_card_references WHERE message_id NOT IN (SELECT id FROM messages)",[],|r|r.get::<_,i64>(0))?,0);assert_eq!(c.query_row("SELECT COUNT(*) FROM fizzy_card_caches",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
    }
    let mut frames = Vec::new();
    let mut quiet = 0;
    while quiet < 5 {
        tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        let more = capture.take();
        quiet = if more.is_empty() { quiet + 1 } else { 0 };
        frames.extend(more);
    }
    // The two paths must produce identical messages and webhook jobs. Ignore runtime claims.
    let source_id = message.id;
    let snapshot = app.db().read(move |conn| {
        let mut messages = conn.prepare("SELECT id,room_id,thread_id,creator_id,reply_to_message_id,markdown_source FROM messages WHERE id >= ? ORDER BY id")?;
        let messages = messages.query_map([source_id], |row| Ok(json!([
            row.get::<_, i64>(0)?, row.get::<_, i64>(1)?, row.get::<_, Option<i64>>(2)?,
            row.get::<_, i64>(3)?, row.get::<_, Option<i64>>(4)?, row.get::<_, Option<String>>(5)?
        ])))?.collect::<rusqlite::Result<Vec<_>>>()?;
        let jobs = campfire_jobs::inspect::all(conn)?.into_iter()
            .filter(|job| job.class == "Bot::WebhookJob").map(|job| job.arguments).collect::<Vec<_>>();
        Ok(json!({"messages": messages, "webhooks": jobs}))
    }).await.unwrap();
    if let Some(live) = live {
        live.abort();
    }
    if let Some(task) = disconnected {
        task.abort();
    }
    (snapshot, frames)
}
