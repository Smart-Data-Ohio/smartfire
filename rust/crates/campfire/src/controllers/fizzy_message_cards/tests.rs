use crate::{
    controllers::presenters::test_support::*,
    integrations::{
        fizzy::accounts::{Account, Input},
        test_support::{FakeServer, Route, ws15e_http_case, ws15e_http_case_listener},
    },
};
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};
use rails_compat::ar_encryption::ArEncryption;
use serde_json::json;

#[tokio::test]
async fn ws15e_fizzy_message_creation_http_matrix() {
    if let Ok(case) = std::env::var("WS15E_FIZZY_MESSAGE_CASE") {
        return run(&case).await;
    }
    for case in [
        "new",
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
        "long_reply",
        "nonmember",
        "scope",
        "enqueue_rollback",
        "no_connection",
        "direct_bots",
    ] {
        let output = ws15e_http_case("WS15E_FIZZY_MESSAGE_CASE", case,
            "controllers::fizzy_message_cards::tests::ws15e_fizzy_message_creation_http_matrix").await;
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
        println!("Fizzy message cards Rails case {case}: 1 passed; 0 failed");
    }
}
async fn run(case: &str) {
    let listener = ws15e_http_case_listener();
    let base = crate::integrations::fizzy::client::api_base_url();
    let mut app = TestApp::boot().await.expect("pinned seeds required");
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
    let threaded = matches!(case, "thread" | "locked" | "scope");
    let (message, thread) = app
        .db()
        .write(move |tx| {
            let mut message = Message::create(
                tx,
                NewMessage {
                    room_id,
                    creator_id: KEVIN,
                    markdown_source: Some("The deploy is broken\nTrack the fix".into()),
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
    if case == "locked" {
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
    let legacy_bot = if case == "direct_bots" {
        Some(app.db().write(move |tx| {
            // Bender is agent-backed in the seed: Rails excludes it from legacy delivery.
            let bot = campfire_db::User::create_integration_bot(tx, "Fizzy legacy fixture")?;
            campfire_db::Webhook::create(tx, bot.id, Some("https://example.test/legacy"))?;
            for user in [BENDER, bot.id] {
                tx.conn().execute("INSERT OR IGNORE INTO memberships (room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)", rusqlite::params![room_id,user,tx.now(),tx.now()])?;
            }
            Ok(bot.id)
        }).await.unwrap())
    } else { None };
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
        201
    };
    let probe_status = match case {
        "revoked" => 401,
        "probe_failure" => 500,
        _ => 200,
    };
    let board_status = match case {
        "new_failure" => 500,
        "new_rejected" => 401,
        _ => 200,
    };
    let server = FakeServer::on_listener(
        vec![
            Route::new("GET", "127.0.0.1", "/897362094/boards.json", board_status).body(
                json!([{"id":"03board1","name":"Engineering"},{"id":"03board2","name":"Support"}])
                    .to_string(),
            ),
            Route::new(
                "POST",
                "127.0.0.1",
                "/897362094/boards/03board1/cards.json",
                create_status,
            )
            .body(json!({"number":580,"url":url}).to_string()),
            Route::new("GET", "127.0.0.1", "/my/identity.json", probe_status).body("{}"),
        ],
        None,
        listener,
    )
    .await;
    let before = app
        .db()
        .read(|c| Ok(c.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let path = if case == "scope" {
        format!("/rooms/{room_id}/messages/{}/fizzy_cards", message.id)
    } else if let Some(thread) = &thread {
        format!(
            "/rooms/{room_id}/threads/{}/messages/{}/fizzy_cards",
            thread.id, message.id
        )
    } else {
        format!("/rooms/{room_id}/messages/{}/fizzy_cards", message.id)
    };
    let get = matches!(
        case,
        "new" | "no_account" | "new_failure" | "new_rejected" | "nonmember" | "scope"
    );
    let mut browser = if case == "nonmember" {
        app.sign_in(KEVIN).await
    } else {
        app.david()
    };
    let response = if get {
        browser.get(&(path + "/new")).await
    } else {
        browser
            .write(Req::new(Method::POST, &path).form(&[
                ("board_id", if case == "invalid" { "" } else { "03board1" }),
                ("title", "  The deploy is broken  "),
                ("description", "Fix it\n\nSource: x"),
            ]))
            .await
    };
    let expected = match case {
        "new" | "no_account" => StatusCode::OK,
        "invalid" => StatusCode::UNPROCESSABLE_ENTITY,
        "nonmember" | "scope" => StatusCode::NOT_FOUND,
        "enqueue_rollback" => StatusCode::INTERNAL_SERVER_ERROR,
        _ => StatusCode::FOUND,
    };
    assert_eq!(response.status, expected, "{case}: {}", response.text());
    if case == "new" {
        for text in [
            "Engineering",
            "Support",
            "The deploy is broken",
            "Track the fix",
        ] {
            assert!(response.text().contains(text));
        }
    }
    if case == "no_account" {
        assert!(response.text().contains("Connect Fizzy on your profile"));
    }
    let success = matches!(case, "create" | "thread" | "direct_bots");
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
        assert_eq!(reply.thread_id, thread.map(|t| t.id));
        assert_eq!(refs, 1);
        assert_eq!(jobs, 1);
        assert!(reply.markdown_source.unwrap().contains("/cards/580"));
        let received = server.received.lock().unwrap();
        let request = received.iter().find(|r| r.method == "POST").unwrap();
        assert_eq!(
            serde_json::from_slice::<serde_json::Value>(&request.body).unwrap(),
            json!({"card":{"title":"The deploy is broken","description":"Fix it\n\nSource: x"}})
        );
    }
    if matches!(
        case,
        "no_account" | "no_connection" | "locked" | "nonmember" | "scope"
    ) {
        assert!(server.received.lock().unwrap().is_empty());
    }
    if matches!(case, "new_rejected" | "revoked") {
        assert!(
            !app.db()
                .read(|c| Account::for_user(c, DAVID))
                .await
                .unwrap()
                .unwrap()
                .connected()
        );
        assert!(response.location().unwrap().ends_with("/users/me/profile"));
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
        assert_eq!(server.received.lock().unwrap().len(), 2);
    }
    if case == "direct_bots" {
        let jobs = app.db().read(campfire_jobs::inspect::all).await.unwrap();
        let jobs: Vec<_> = jobs.iter().filter(|j| j.class == "Bot::WebhookJob").collect();
        assert_eq!(jobs.len(), 1);
        assert_eq!(jobs[0].arguments["bot_id"], legacy_bot.unwrap());
        assert!(jobs.iter().all(|j| j.arguments["bot_id"] != BENDER), "agent-backed bot bypassed its ledger");
    }
    if case == "enqueue_rollback" {
        app.db().read(|c| {assert_eq!(c.query_row("SELECT COUNT(*) FROM fizzy_card_references WHERE message_id NOT IN (SELECT id FROM messages)",[],|r|r.get::<_,i64>(0))?,0);assert_eq!(c.query_row("SELECT COUNT(*) FROM fizzy_card_caches",[],|r|r.get::<_,i64>(0))?,0);Ok(())}).await.unwrap();
    }
}
#[tokio::test]
async fn ws15e_fizzy_message_form_matches_pinned_rails_bytes() {
    use askama::Template;
    let app = TestApp::boot().await.expect("pinned seeds required");
    let vector: serde_json::Value = serde_json::from_str(include_str!(concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/../../vectors/ws15e_fizzy_message_form.json"
    )))
    .unwrap();
    for case in vector["frames"].as_array().unwrap() {
        let back = case["thread"]
            .as_i64()
            .map_or("/rooms/42".to_owned(), |id| {
                format!("/rooms/42/threads/{id}")
            });
        let view = campfire_views::fizzy_message_cards::FormView {
            action: format!("{back}/messages/99/fizzy_cards"),
            back_path: back,
            room_name: "Engineering <&>".into(),
            plain: "The deploy <&> is broken\nTrack the fix".into(),
            creator: "David <&>".into(),
            connected: case["connected"].as_bool().unwrap(),
            boards: json!([{"id":"03board1","name":"Engineering <&>"},{"id":"03board2","name":"Support"}]),
            board_id: case["board"].as_str().unwrap_or("").into(),
            title: "Deploy <&>".into(),
            description: "Description <&>\nTwo".into(),
            user_name: "David <&>".into(),
            account_name: "Smart Data <&>".into(),
        };
        let html =
            crate::controllers::presenters::page::render_detached(&app.booted.app, None, |ctx| {
                campfire_views::fizzy_message_cards::New { ctx, view: &view }
                    .as_content()
                    .render()
                    .unwrap()
            });
        assert_eq!(html, case["html"].as_str().unwrap(), "{}", case["name"]);
    }
}
