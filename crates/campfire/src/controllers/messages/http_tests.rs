//! Fork-specific request contracts; the Rails-built parity seed is mandatory here.
use axum::http::{Method, StatusCode};
use campfire_db::{ChannelThread, Message, NewChannelThread, NewMessage};

use crate::controllers::presenters::test_support::*;

async fn boot() -> TestApp {
    TestApp::boot()
        .await
        .expect("WS8bm HTTP tests require the default seed (python3 parity/bin/frozen-seeds restore)")
}

async fn create(app: &TestApp, creator_id: i64, system_note: bool) -> Message {
    app.db()
        .write(move |tx| {
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id,
                    system_note,
                    markdown_source: Some("original".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn author_only_edits_even_for_an_administrator() {
    let app = boot().await;
    let message = create(&app, JASON, false).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut david = app.david();
    assert_eq!(david.get(&format!("{path}/edit")).await.status, StatusCode::FORBIDDEN);
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")]))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let original = app
        .db()
        .read(move |conn| Message::find(conn, message.id)?.body_html(conn))
        .await
        .unwrap();
    assert_eq!(original.as_deref(), Some("<p>original</p>"));
}

#[tokio::test]
async fn system_notes_are_immutable_for_author_and_administrator() {
    let app = boot().await;
    let mut david = app.david();
    for creator in [DAVID, JASON] {
        let note = create(&app, creator, true).await;
        let path = format!("/rooms/{ALL_TALK}/messages/{}", note.id);
        assert_eq!(david.get(&format!("{path}/edit")).await.status, StatusCode::FORBIDDEN);
        assert_eq!(
            david
                .write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")]))
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        assert_eq!(
            david
                .write(Req::new(Method::DELETE, &path).header("accept", "text/vnd.turbo-stream.html"))
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        assert!(
            app.db()
                .read(move |conn| Message::find_by_id(conn, note.id))
                .await
                .unwrap()
                .is_some()
        );
    }
}

#[tokio::test]
async fn non_author_non_admin_cannot_edit_or_delete() {
    let app = boot().await;
    app.db()
        .write(|tx| {
            tx.conn().execute("UPDATE users SET role = 0 WHERE id = ?", [JASON])?;
            Ok(())
        })
        .await
        .unwrap();
    let message = create(&app, DAVID, false).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut jason = app.sign_in(JASON).await;
    assert_eq!(jason.get(&format!("{path}/edit")).await.status, StatusCode::FORBIDDEN);
    let before=app.db().read(move|conn|Message::find(conn,message.id)).await.unwrap();
    assert_eq!(jason.write(Req::new(Method::PATCH,&path).form(&[("message[markdown_source]","Other member edit")])).await.status,StatusCode::FORBIDDEN);
    let id=before.id;
    assert_eq!(app.db().read(move|conn|Message::find(conn,id)).await.unwrap(),before);
    assert_eq!(
        jason
            .write(Req::new(Method::DELETE, &path).header("accept", "text/vnd.turbo-stream.html"))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn preview_matches_real_rails_http_without_writing() {
    let app = boot().await;
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/messaging/preview.json")).unwrap();
    let before = app
        .db()
        .read(|conn| Ok(conn.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    let mut david = app.david();
    for row in oracle["previews"].as_array().unwrap() {
        let body = serde_json::to_string(&serde_json::json!({"message": {"markdown_source": row["source"]}})).unwrap();
        let reply = david
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages/preview"))
                    .header("content-type", "application/json")
                    .header("accept", "application/json")
                    .body(body),
            )
            .await;
        assert_eq!(
            reply.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}",
            reply.text()
        );
        assert_eq!(
            reply.json(),
            row["json"],
            "source prefix: {:?}",
            row["source"].as_str().unwrap().chars().take(40).collect::<String>()
        );
        assert_eq!(reply.text(), row["json_text"].as_str().unwrap());
    }
    let after = app
        .db()
        .read(|conn| Ok(conn.query_row("SELECT count(*) FROM messages", [], |r| r.get::<_, i64>(0))?))
        .await
        .unwrap();
    assert_eq!(before, after);
}

#[tokio::test]
async fn preview_requires_membership_and_a_valid_source_parameter() {
    let app = boot().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages/preview");
    for body in [
        "{}",
        r#"{"message":{"body":"wrong"}}"#,
        r#"{"message":{"markdown_source":{}}}"#,
    ] {
        assert_eq!(
            david
                .write(
                    Req::new(Method::POST, &path)
                        .header("content-type", "application/json")
                        .body(body)
                )
                .await
                .status,
            StatusCode::BAD_REQUEST
        );
    }
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(
        kevin
            .write(Req::new(Method::POST, &path).form(&[("message[markdown_source]", "private")]))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn preview_rejects_cross_site_submissions_without_a_csrf_token() {
    let app = boot().await;
    let mut david = app.david();
    let reply = david
        .send(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages/preview"))
                .header("origin", "https://other.example")
                .form(&[("message[markdown_source]", "cross site")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
}

#[tokio::test]
async fn deleted_room_is_inaccessible_even_with_a_lingering_membership() {
    let app = boot().await;
    let message = create(&app, DAVID, false).await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE rooms SET deleted_at = ? WHERE id = ?", (tx.now(), ALL_TALK))?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    for path in [
        format!("/rooms/{ALL_TALK}/messages"),
        format!("/rooms/{ALL_TALK}/messages/{}", message.id),
    ] {
        assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    }
}

#[tokio::test]
async fn removed_and_non_members_cannot_read_or_edit_messages() {
    let app = boot().await;
    let message = create(&app, DAVID, false).await;
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut kevin = app.sign_in(KEVIN).await;
    assert_eq!(kevin.get(&path).await.status, StatusCode::NOT_FOUND);
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id = ? AND user_id = ?",
                (ALL_TALK, DAVID),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")]))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn root_endpoint_cannot_read_edit_or_delete_thread_messages() {
    let app = boot().await;
    let message = create(&app, DAVID, false).await;
    let message = app
        .db()
        .write(move |tx| {
            let thread = ChannelThread::create(
                tx,
                NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    parent_message_id: Some(message.id),
                    ..Default::default()
                },
            )?;
            Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    thread_id: Some(thread.id),
                    markdown_source: Some("thread reply".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    let path = format!("/rooms/{ALL_TALK}/messages/{}", message.id);
    let mut david = app.david();
    assert_eq!(david.get(&path).await.status, StatusCode::NOT_FOUND);
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &path).form(&[("message[body]", "changed")]))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        david
            .write(Req::new(Method::DELETE, &path).header("accept", "text/vnd.turbo-stream.html"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn markdown_message_fragments_match_real_rails_records_and_cache_hits() {
    use crate::controllers::presenters::{Presenter, page};
    let app = boot().await;
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/messaging/fragments.json")).unwrap();
    for row in oracle["messages"].as_array().unwrap() {
        let row = row.clone();
        let input = row["input"].clone();
        let message = app
            .db()
            .write(move |tx| {
                let message = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        client_message_id: input["client_message_id"].as_str().map(str::to_owned),
                        markdown_source: input["markdown_source"].as_str().map(str::to_owned),
                        body: input["body"].as_str().map(str::to_owned),
                        system_note: input["system_note"].as_bool().unwrap_or(false),
                        forwarded_markdown: input["forwarded_markdown"].as_bool().unwrap_or(false),
                        forwarded_at: input
                            .get("forwarded_at")
                            .map(|_| campfire_db::Timestamp::from_jiff(SEED_NOW.parse::<jiff::Timestamp>().unwrap())),
                        forwarded_from_message_id: input["forwarded_from_message_id"].as_i64(),
                        ..Default::default()
                    },
                )?;
                let fixed = campfire_db::Timestamp::from_jiff(SEED_NOW.parse::<jiff::Timestamp>().unwrap());
                tx.conn().execute(
                    "UPDATE messages SET created_at = ?, updated_at = ? WHERE id = ?",
                    (fixed, fixed, message.id),
                )?;
                Message::find(tx.conn(), message.id)
            })
            .await
            .unwrap();
        assert_eq!(
            message.id,
            row["message"]["id"].as_i64().unwrap(),
            "same Rails seed and insert order"
        );
        for expected in row["html_by_viewer"].as_array().unwrap() {
            let runtime = app.booted.app.clone();
            let message = message.clone();
            let actual = app
                .db()
                .read(move |conn| {
                    let view = Presenter::new(conn, &runtime, None).message(&message)?;
                    let account = campfire_db::Account::first(conn)?;
                    Ok(page::render_detached_at(
                        &runtime,
                        account.as_ref(),
                        "http://campfire.test",
                        |ctx| campfire_views::messages::message(ctx, &view),
                    ))
                })
                .await
                .unwrap();
            let expected = expected.as_str().unwrap();
            let first = actual
                .bytes()
                .zip(expected.bytes())
                .position(|(a, b)| a != b)
                .unwrap_or(actual.len().min(expected.len()));
            assert!(
                actual == expected,
                "{} differs at byte {first}: actual {:?}, Rails {:?}",
                row["input"]["client_message_id"],
                &actual.as_bytes()[first.saturating_sub(40)..(first + 100).min(actual.len())],
                &expected.as_bytes()[first.saturating_sub(40)..(first + 100).min(expected.len())]
            );
            assert!(!actual.contains("authenticity_token"));
            assert!(!actual.contains("nonce=\""));
        }
    }
}

#[tokio::test]
async fn root_create_preserves_markdown_numeric_client_id_and_deduplicates_retries() {
    let app = boot().await.without_job_runner().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages");
    let body = serde_json::json!({"message": {
        "markdown_source": "# Release\n\n**Ready**", "body": "<script>wrong source</script>", "client_message_id": 999
    }})
    .to_string();
    let reply = david
        .write(
            Req::new(Method::POST, &path)
                .header("accept", "text/vnd.turbo-stream.html")
                .header("content-type", "application/json")
                .body(body),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert!(reply.text().is_empty());
    let message = app
        .db()
        .read(|conn| Message::find_duplicate(conn, ALL_TALK, DAVID, "999"))
        .await
        .unwrap()
        .expect("numeric id is cast to a string");
    assert_eq!(message.markdown_source.as_deref(), Some("# Release\n\n**Ready**"));
    let runtime = app.booted.app.clone();
    let message_id = message.id;
    let plain = app
        .db()
        .read(move |conn| Message::find(conn, message_id)?.plain_text_body(conn, &*runtime.db.env().rich_text))
        .await
        .unwrap();
    assert_eq!(plain, "Release\n\nReady");
    let counts = |conn: &campfire_db::Connection| -> campfire_db::Result<(i64, i64)> {
        Ok((
            conn.query_row("SELECT count(*) FROM messages", [], |r| r.get(0))?,
            conn.query_row("SELECT count(*) FROM background_jobs", [], |r| r.get(0))?,
        ))
    };
    let before = app.db().read(counts).await.unwrap();
    let retry =
        serde_json::json!({"message": {"client_message_id": 999, "markdown_source": "", "attachment": 42}}).to_string();
    let reply = david
        .write(
            Req::new(Method::POST, &path)
                .header("accept", "text/vnd.turbo-stream.html")
                .header("content-type", "application/json")
                .body(retry),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    assert_eq!(
        app.db().read(counts).await.unwrap(),
        before,
        "a retry saves and enqueues nothing"
    );
}

#[tokio::test]
async fn root_create_scalar_columns_match_real_rails_casts() {
    let app = boot().await;
    let mut david = app.david();
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/messaging/preview.json")).unwrap();
    let rows = oracle["scalar_casts"].as_array().unwrap();
    for row in rows.iter().filter(|row| !row["value"].is_null() && row["value"] != "") {
        let input =
            serde_json::json!({"message": {"markdown_source": row["value"], "client_message_id": row["value"]}})
                .to_string();
        let reply = david
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                    .header("accept", "text/vnd.turbo-stream.html")
                    .header("content-type", "application/json")
                    .body(input),
            )
            .await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let expected_id = row["client_message_id"].as_str().unwrap().to_owned();
        let message = app
            .db()
            .read(move |conn| Message::find_duplicate(conn, ALL_TALK, DAVID, &expected_id))
            .await
            .unwrap()
            .expect("Rails scalar column cast");
        assert_eq!(message.markdown_source.as_deref(), row["markdown_source"].as_str());
    }
    let before = app
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT count(*) FROM messages WHERE client_message_id = 'f'", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    let input = serde_json::json!({"message": {"markdown_source": "false is blank for the retry check", "client_message_id": false}}).to_string();
    assert_eq!(
        david
            .write(
                Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                    .header("accept", "text/vnd.turbo-stream.html")
                    .header("content-type", "application/json")
                    .body(input)
            )
            .await
            .status,
        StatusCode::CREATED
    );
    let after = app
        .db()
        .read(|conn| {
            Ok(
                conn.query_row("SELECT count(*) FROM messages WHERE client_message_id = 'f'", [], |r| {
                    r.get::<_, i64>(0)
                })?,
            )
        })
        .await
        .unwrap();
    assert_eq!(
        after,
        before + 1,
        "Rails checks raw false.blank? before casting the client id"
    );
}

#[tokio::test]
async fn root_create_validates_markdown_and_drive_ids_with_rails_error_shapes() {
    let app = boot().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/messages.json");
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/messaging/preview.json")).unwrap();
    for row in oracle["invalid_creates"].as_array().unwrap() {
        let body = serde_json::json!({"message": row["input"]}).to_string();
        let reply = david
            .write(
                Req::new(Method::POST, &path)
                    .header("content-type", "application/json")
                    .body(body),
            )
            .await;
        assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY, "{}", reply.text());
        assert_eq!(reply.json(), row["json"]);
    }
}

#[tokio::test]
async fn root_create_resolves_replies_and_normalizes_drive_sets() {
    let app = boot().await;
    let source = create(&app, JASON, false).await;
    let mut david = app.david();
    let body = serde_json::json!({"message": {
        "markdown_source": "a reply", "client_message_id": "ws8bm-reply", "reply_to_message_id": source.id,
        "reply_notify_author": "false", "drive_file_ids": ["", " abcdefghij ", "abcdefghij", "klmnopqrst"]
    }})
    .to_string();
    let reply = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .header("content-type", "application/json")
                .body(body),
        )
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let row = app
        .db()
        .read(|conn| {
            let message = Message::find_duplicate(conn, ALL_TALK, DAVID, "ws8bm-reply")?.unwrap();
            Ok((
                message.reply_to_message_id,
                message.reply_notify_author,
                message.drive_file_ids(conn)?,
            ))
        })
        .await
        .unwrap();
    assert_eq!(
        row,
        (
            Some(source.id),
            false,
            vec!["abcdefghij".to_owned(), "klmnopqrst".to_owned()]
        )
    );
}

#[tokio::test]
async fn root_create_rolls_back_when_the_atomic_job_insert_is_rejected() {
    let app = boot().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws8bm_reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'WS8bm atomic enqueue rejection'); END;")?;
        Ok(())
    }).await.unwrap();
    let mut david = app.david();
    let reply = david
        .write(
            Req::new(Method::POST, &format!("/rooms/{ALL_TALK}/messages"))
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[
                    ("message[markdown_source]", "must roll back"),
                    ("message[client_message_id]", "ws8bm-rollback"),
                ]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    assert!(
        app.db()
            .read(|conn| Message::find_duplicate(conn, ALL_TALK, DAVID, "ws8bm-rollback"))
            .await
            .unwrap()
            .is_none()
    );
}

#[tokio::test]
async fn administrator_can_delete_another_authors_ordinary_root_message() {
    let app=boot().await;
    let message=create(&app,JASON,false).await;
    let id=message.id;
    assert_eq!(app.david().write(Req::new(Method::DELETE,&format!("/rooms/{ALL_TALK}/messages/{id}.turbo_stream"))).await.status,StatusCode::NO_CONTENT);
    assert!(app.db().read(move|conn|Message::find_by_id(conn,id)).await.unwrap().is_none());
}

use campfire_web::controllers::presenters::Rendering;
