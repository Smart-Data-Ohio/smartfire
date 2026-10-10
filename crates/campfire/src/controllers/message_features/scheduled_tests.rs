//! Request ports of `test/controllers/scheduled_messages_controller_test.rb`.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{NewScheduledMessage, ScheduledMessage};
use serde_json::{Value, json};

async fn app() -> TestApp {
    TestApp::boot_with_test_clock(std::sync::Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    )))
    .await
    .expect("WS8bm2 requires default seed")
}
fn req(method: Method, path: &str, value: Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&value).unwrap())
}
async fn row(app: &TestApp, user: i64, room: i64, body: &str) -> ScheduledMessage {
    let body = body.to_owned();
    app.db()
        .write(move |tx| {
            ScheduledMessage::create(
                tx,
                NewScheduledMessage {
                    user_id: user,
                    room_id: room,
                    thread_id: None,
                    reply_to_message_id: None,
                    markdown_source: body,
                    send_at: campfire_db::Timestamp::from_jiff(
                        "2026-03-02T17:00:00Z".parse().unwrap(),
                    ),
                },
            )
        })
        .await
        .unwrap()
}
async fn patch(browser: &mut Browser<'_>, id: i64, value: Value) -> Reply {
    browser
        .write(req(
            Method::PATCH,
            &format!("/scheduled_messages/{id}"),
            value,
        ))
        .await
}
async fn delete(browser: &mut Browser<'_>, id: i64) -> Reply {
    browser
        .write(req(
            Method::DELETE,
            &format!("/scheduled_messages/{id}"),
            json!({}),
        ))
        .await
}
async fn send(browser: &mut Browser<'_>, id: i64) -> Reply {
    browser
        .write(req(
            Method::POST,
            &format!("/scheduled_messages/{id}/send_now"),
            json!({}),
        ))
        .await
}
async fn claim(app: &TestApp, id: i64, time: &str) {
    let time = campfire_db::Timestamp::from_jiff(time.parse().unwrap());
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE scheduled_messages SET claimed_at=? WHERE id=?",
                (time, id),
            )?;
            Ok(())
        })
        .await
        .unwrap();
}
async fn reload(app: &TestApp, id: i64) -> ScheduledMessage {
    app.db()
        .read(move |conn| ScheduledMessage::find(conn, id))
        .await
        .unwrap()
}

#[tokio::test]
async fn index_lists_upcoming_and_past_rows() {
    let app = app().await;
    let upcoming = row(&app, DAVID, ALL_TALK, "Soon").await;
    let past = row(&app, DAVID, ALL_TALK, "Gone").await;
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE scheduled_messages SET sent_at=? WHERE id=?",
                (tx.now(), past.id),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let response = app.david().get("/scheduled_messages").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(
        response
            .text()
            .contains(&format!("id=\"scheduled_message_{}\"", upcoming.id))
    );
    assert!(
        response
            .text()
            .contains(&format!("id=\"scheduled_message_{}\"", past.id))
    );
}
#[tokio::test]
async fn index_hides_other_peoples_rows() {
    let app = app().await;
    let other = row(&app, JASON, ALL_TALK, "Theirs").await;
    let response = app.david().get("/scheduled_messages").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(
        !response
            .text()
            .contains(&format!("id=\"scheduled_message_{}\"", other.id))
    );
}
#[tokio::test]
async fn index_shows_stranded_rows_so_they_can_be_cancelled() {
    let app = app().await;
    let stranded = row(&app, DAVID, ALL_TALK, "Stranded").await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let response = app.david().get("/scheduled_messages").await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("No longer sendable"));
    assert!(
        response
            .text()
            .contains(&format!("id=\"scheduled_message_{}\"", stranded.id))
    );
    assert_eq!(
        delete(&mut app.david(), stranded.id).await.status,
        StatusCode::NO_CONTENT
    );
}
#[tokio::test]
async fn creates_a_scheduled_message() {
    let app = app().await;
    let response=app.david().write(req(Method::POST,&format!("/rooms/{ALL_TALK}/scheduled_messages"),json!({"scheduled_message":{"markdown_source":"Morning!","send_at":"2026-03-02T17:00:00Z"}}))).await;
    assert_eq!(response.status, StatusCode::CREATED);
    assert_eq!(response.json()["markdown_source"], "Morning!");
    assert_eq!(response.json()["send_at"], "2026-03-02T17:00:00.000Z");
}
#[tokio::test]
async fn create_rejects_past_times() {
    let app = app().await;
    let response=app.david().write(req(Method::POST,&format!("/rooms/{ALL_TALK}/scheduled_messages"),json!({"scheduled_message":{"markdown_source":"Late","send_at":"2026-03-02T15:00:00Z"}}))).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        response.json(),
        json!({"errors":{"send_at":["must be in the future"]}})
    );
}
#[tokio::test]
async fn create_is_404_outside_membership() {
    let app = app().await;
    let response=app.sign_in(KEVIN).await.write(req(Method::POST,&format!("/rooms/{ALL_TALK}/scheduled_messages"),json!({"scheduled_message":{"markdown_source":"Hidden","send_at":"2026-03-02T17:00:00Z"}}))).await;
    assert_eq!(response.status, StatusCode::NOT_FOUND);
}
#[tokio::test]
async fn updates_text_and_time() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    let response = patch(
        &mut app.david(),
        scheduled.id,
        json!({"scheduled_message":{"markdown_source":"Sooner!","send_at":"2026-03-02T18:00"}}),
    )
    .await;
    assert_eq!(response.status, StatusCode::OK);
    assert_eq!(response.json()["markdown_source"], "Sooner!");
    assert_eq!(
        reload(&app, scheduled.id).await.send_at.to_db(),
        "2026-03-02 18:00:00"
    );
}
#[tokio::test]
async fn update_during_an_active_claim_is_refused() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    claim(&app, scheduled.id, SEED_NOW).await;
    let response = patch(
        &mut app.david(),
        scheduled.id,
        json!({"scheduled_message":{"markdown_source":"Edited!"}}),
    )
    .await;
    assert_eq!(response.status, StatusCode::CONFLICT);
    assert!(
        response.json()["error"]
            .as_str()
            .unwrap()
            .contains("sending right now")
    );
    assert_eq!(reload(&app, scheduled.id).await.markdown_source, "Soon");
}
#[tokio::test]
async fn update_during_an_active_claim_redirects_with_an_alert_in_html() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    claim(&app, scheduled.id, SEED_NOW).await;
    let mut browser = app.david();
    let response = browser
        .write(
            Req::new(
                Method::PATCH,
                &format!("/scheduled_messages/{}", scheduled.id),
            )
            .form(&[("scheduled_message[markdown_source]", "Edited!")]),
        )
        .await;
    assert_eq!(response.status, StatusCode::FOUND);
    assert_eq!(
        response.location(),
        Some("http://campfire.test/scheduled_messages")
    );
    assert!(
        browser
            .get("/scheduled_messages")
            .await
            .text()
            .contains("sending right now")
    );
}
#[tokio::test]
async fn update_after_the_claim_goes_stale_is_allowed() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    claim(&app, scheduled.id, "2026-03-02T15:54:00Z").await;
    assert_eq!(
        patch(
            &mut app.david(),
            scheduled.id,
            json!({"scheduled_message":{"markdown_source":"Edited!"}})
        )
        .await
        .status,
        StatusCode::OK
    );
    assert_eq!(reload(&app, scheduled.id).await.markdown_source, "Edited!");
}
#[tokio::test]
async fn update_is_404_for_sent_rows_and_other_peoples_rows() {
    let app = app().await;
    let sent = row(&app, DAVID, ALL_TALK, "Gone").await;
    let other = row(&app, JASON, ALL_TALK, "Theirs").await;
    app.db()
        .write(move |tx| {
            tx.conn().execute(
                "UPDATE scheduled_messages SET sent_at=? WHERE id=?",
                (tx.now(), sent.id),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    for id in [sent.id, other.id] {
        assert_eq!(
            patch(
                &mut app.david(),
                id,
                json!({"scheduled_message":{"markdown_source":"Edit"}})
            )
            .await
            .status,
            StatusCode::NOT_FOUND
        );
    }
}
#[tokio::test]
async fn destroy_cancels_pending_rows_only() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    let other = row(&app, JASON, ALL_TALK, "Theirs").await;
    assert_eq!(
        delete(&mut app.david(), scheduled.id).await.status,
        StatusCode::NO_CONTENT
    );
    assert_eq!(
        delete(&mut app.david(), other.id).await.status,
        StatusCode::NOT_FOUND
    );
    assert!(
        app.db()
            .read(move |conn| ScheduledMessage::find_by_id(conn, scheduled.id))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn destroy_during_an_active_claim_is_refused() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    claim(&app, scheduled.id, SEED_NOW).await;
    assert_eq!(
        delete(&mut app.david(), scheduled.id).await.status,
        StatusCode::CONFLICT
    );
    assert!(reload(&app, scheduled.id).await.pending());
}
#[tokio::test]
async fn send_now_posts_immediately() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Now!").await;
    assert_eq!(
        send(&mut app.david(), scheduled.id).await.status,
        StatusCode::OK
    );
    let saved = reload(&app, scheduled.id).await;
    assert!(saved.sent());
    assert_eq!(
        app.db()
            .read(move |conn| Ok(
                campfire_db::Message::find(conn, saved.sent_message_id.unwrap())?.markdown_source
            ))
            .await
            .unwrap()
            .as_deref(),
        Some("Now!")
    );
}
#[tokio::test]
async fn send_now_drops_rows_without_access() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Stranded").await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id=? AND room_id=?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        send(&mut app.david(), scheduled.id).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert!(reload(&app, scheduled.id).await.dropped());
}
#[tokio::test]
async fn send_now_drops_rows_the_model_rejects_with_the_reason() {
    let app = app().await;
    let board = app
        .db()
        .write(|tx| {
            campfire_db::Room::create_for(
                tx,
                campfire_db::RoomType::Board,
                Some("Launch"),
                DAVID,
                &[DAVID],
            )
        })
        .await
        .unwrap();
    let scheduled = row(&app, DAVID, board.id, "Root post").await;
    let response = send(&mut app.david(), scheduled.id).await;
    assert_eq!(response.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(response.json()["error"].as_str().unwrap().contains("board"));
    assert!(reload(&app, scheduled.id).await.dropped());
}
#[tokio::test]
async fn bots_are_forbidden() {
    let app = app().await;
    assert_eq!(
        app.sign_in(BENDER)
            .await
            .get("/scheduled_messages")
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}
async fn races_send_before_lock(method: Method) {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    let mut browser = app.david();
    let token = browser.authenticity_token().await;
    let db = app.db().clone();
    let (started, ready) = tokio::sync::oneshot::channel();
    let (release, wait) = std::sync::mpsc::channel();
    let blocker = tokio::spawn(async move {
        db.write(move |tx| {
            tx.conn().execute(
                "UPDATE scheduled_messages SET sent_at=? WHERE id=?",
                (tx.now(), scheduled.id),
            )?;
            started.send(()).unwrap();
            wait.recv()
                .map_err(|error| campfire_db::Error::Other(error.to_string()))?;
            Ok(())
        })
        .await
        .unwrap()
    });
    ready.await.unwrap();
    let request = browser.send(
        req(
            method,
            &format!("/scheduled_messages/{}", scheduled.id),
            json!({"scheduled_message":{"markdown_source":"Edited!"}}),
        )
        .header(campfire_kit::csrf::HEADER, &token),
    );
    let release = async {
        tokio::time::timeout(std::time::Duration::from_secs(5), async {
            while app.db().queued_writes() == 0 {
                tokio::task::yield_now().await;
            }
        })
        .await
        .expect("HTTP write queued after its pending-row read");
        release.send(()).unwrap();
    };
    let (response, ()) = tokio::join!(request, release);
    blocker.await.unwrap();
    assert_eq!(response.status, StatusCode::CONFLICT);
    let saved = reload(&app, scheduled.id).await;
    assert!(saved.sent());
    assert_eq!(saved.markdown_source, "Soon");
}
#[tokio::test]
async fn a_send_between_lookup_and_lock_refuses_the_cancel() {
    races_send_before_lock(Method::DELETE).await;
}
#[tokio::test]
async fn a_send_between_lookup_and_lock_refuses_the_edit() {
    races_send_before_lock(Method::PATCH).await;
}
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/scheduled.json"
    ))
    .unwrap()
}
#[tokio::test]
async fn scheduled_http_matches_rails_json_offsets_and_claim_outcomes() {
    let app = app().await;
    let draft = row(&app, DAVID, ALL_TALK, "Draft <body> & example").await;
    assert_eq!(draft.id, oracle()["draft_id"]);
    let mut browser = app.david();
    let mut id = None;
    for (index, step) in oracle()["steps"].as_array().unwrap().iter().enumerate() {
        if index == 5 {
            claim(&app, id.unwrap(), SEED_NOW).await;
        }
        if index == 7 {
            claim(&app, id.unwrap(), "2026-03-02T15:54:00Z").await;
        }
        let zone = step["zone"].as_str().unwrap().to_owned();
        app.db()
            .write(move |tx| {
                tx.conn()
                    .execute("UPDATE users SET time_zone=? WHERE id=?", (zone, DAVID))?;
                Ok(())
            })
            .await
            .unwrap();
        let response = browser
            .write(req(
                step["method"]
                    .as_str()
                    .unwrap()
                    .to_uppercase()
                    .parse()
                    .unwrap(),
                step["path"].as_str().unwrap(),
                step["input"].clone(),
            ))
            .await;
        assert_eq!(
            response.status.as_u16(),
            step["status"].as_u64().unwrap() as u16,
            "step {index}"
        );
        assert_eq!(
            response.text(),
            step["body"].as_str().unwrap(),
            "step {index}"
        );
        assert_eq!(
            response.header("cache-control"),
            step["cache_control"].as_str()
        );
        assert_eq!(response.header("pragma"), step["pragma"].as_str());
        if index == 0 {
            id = response.json()["id"].as_i64();
        }
    }
}
#[tokio::test]
async fn scheduled_row_partials_match_rails_and_empty_page_bytes() {
    use askama::Template;
    let app = app().await;
    let draft = row(&app, DAVID, ALL_TALK, "Draft <body> & example").await;
    for case in oracle()["html"].as_array().unwrap() {
        let case = case.clone();
        let expected = case["html"].as_str().unwrap().to_owned();
        let draft = draft.clone();
        let runtime = app.booted.app.clone();
        let actual = app
            .db()
            .read(move |conn| {
                let mut draft = draft;
                let state = case["state"].as_str().unwrap();
                draft.sent_at = (state == "sent")
                    .then_some(campfire_db::Timestamp::from_jiff(SEED_NOW.parse().unwrap()));
                let mut presenter = crate::controllers::presenters::Presenter::new(conn, &runtime, None);
                presenter.render_zone = campfire_views::time::Zone::lookup(case["zone"].as_str().unwrap()).unwrap();
                let view = crate::controllers::scheduled_messages::view(
                    &presenter,
                    conn,
                    &campfire_db::User::find(conn, DAVID)?,
                    &draft,
                )?;
                let account = campfire_db::Account::first(conn)?;
                Ok(crate::controllers::presenters::page::render_detached_at(
                    &runtime,
                    account.as_ref(),
                    "http://campfire.test",
                    |ctx| {
                        let ctx = super::saved_tests::zoned(ctx, case["zone"].as_str().unwrap());
                        if ["sent", "dropped"].contains(&state) {
                            campfire_views::scheduled_messages::PastPartial {
                                ctx: &ctx,
                                item: &view,
                            }
                            .render()
                            .unwrap()
                        } else {
                            campfire_views::scheduled_messages::ItemPartial {
                                ctx: &ctx,
                                item: &view,
                                stranded: state == "stranded",
                            }
                            .render()
                            .unwrap()
                        }
                    },
                ))
            })
            .await
            .unwrap();
        assert_eq!(actual, expected);
    }
    let actual = crate::controllers::presenters::page::render_detached_at(
        &app.booted.app,
        None,
        "http://campfire.test",
        |ctx| {
            campfire_views::scheduled_messages::Index {
                ctx,
                upcoming: &[],
                stranded: &[],
                past: &[],
            }
            .as_content()
            .render()
            .unwrap()
        },
    );
    assert_eq!(actual, oracle()["empty"].as_str().unwrap());
}
#[tokio::test]
async fn scheduled_mutations_require_csrf_and_busy_claims_accept_no_parameters() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Soon").await;
    let mut browser = app.david();
    for (method, path, value) in [
        (
            Method::POST,
            format!("/rooms/{ALL_TALK}/scheduled_messages"),
            json!({"scheduled_message":{"markdown_source":"draft","send_at":"2026-03-02T17:00:00Z"}}),
        ),
        (
            Method::PATCH,
            format!("/scheduled_messages/{}", scheduled.id),
            json!({"scheduled_message":{"markdown_source":"Edit"}}),
        ),
        (
            Method::DELETE,
            format!("/scheduled_messages/{}", scheduled.id),
            json!({}),
        ),
        (
            Method::POST,
            format!("/scheduled_messages/{}/send_now", scheduled.id),
            json!({}),
        ),
    ] {
        assert_eq!(
            browser.send(req(method, &path, value)).await.status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
    }
    claim(&app, scheduled.id, SEED_NOW).await;
    assert_eq!(
        patch(&mut browser, scheduled.id, json!({})).await.status,
        StatusCode::CONFLICT
    );
}
#[tokio::test]
async fn scheduled_send_rolls_back_claim_post_and_history_when_job_insert_fails() {
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Atomic scheduled post").await;
    let before: i64 = app
        .db()
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER reject_scheduled_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT,'reject scheduled job'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        send(&mut app.david(), scheduled.id).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    let saved = reload(&app, scheduled.id).await;
    assert!(saved.pending() && saved.claimed_at.is_none() && saved.sent_message_id.is_none());
    let after: i64 = app
        .db()
        .read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get(0))?))
        .await
        .unwrap();
    assert_eq!(before, after);
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_batch("DROP TRIGGER reject_scheduled_job")?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        send(&mut app.david(), scheduled.id).await.status,
        StatusCode::OK
    );
}


#[tokio::test]
async fn scheduled_composer_controls_match_rails_for_room_and_thread() {
    use askama::Template;
    let app = app().await;
    for case in oracle()["composer"].as_array().unwrap() {
        let actual = crate::controllers::presenters::page::render_detached_at(
            &app.booted.app,
            None,
            "http://campfire.test",
            |ctx| {
                campfire_views::scheduled_messages::ComposerButton {
                    ctx,
                    room_id: ALL_TALK,
                    thread_id: case["thread_id"].as_i64(),
                }
                .render()
                .unwrap()
            },
        );
        assert_eq!(actual, case["html"].as_str().unwrap());
    }
}
#[tokio::test]
async fn review_regression_scheduled_send_emits_unread_room_frame() {
    use crate::channels::tests::support::{Client, bind_listener, identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let app = app().await;
    let scheduled = row(&app, DAVID, ALL_TALK, "Scheduled socket example").await;
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let _server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut request = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    request
        .headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    request
        .headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(request).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    client
        .confirm(&identifier(json!({"channel":"UnreadRoomsChannel"})))
        .await;
    assert_eq!(
        send(&mut app.david(), scheduled.id).await.status,
        StatusCode::OK
    );
    let got = tokio::time::timeout(std::time::Duration::from_secs(1), client.next_text()).await;
    println!("REVIEW_SCHEDULED_UNREAD frame={got:?}");
    assert!(
        got.is_ok(),
        "scheduled sends never reach the subscribed user_*_unreads stream"
    );
    let frame: Value = serde_json::from_str(&got.unwrap()).unwrap();
    assert_eq!(frame["message"], json!({"roomId":ALL_TALK}));
    assert_eq!(
        frame["identifier"],
        identifier(json!({"channel":"UnreadRoomsChannel"}))
    );

}
