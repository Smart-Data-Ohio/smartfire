use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Message, NewMessage, NewPoll, Poll};
use serde_json::json;

async fn fixture() -> (TestApp, i64, i64) {
    let app = TestApp::boot()
        .await
        .expect("WS8bm2 requires the default parity seed");
    let (poll, option) = app
        .db()
        .write(|tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("Lunch?".into()),
                    client_message_id: Some("feature-poll".into()),
                    ..Default::default()
                },
            )?;
            let poll = Poll::create_for_message(
                tx,
                &message,
                NewPoll {
                    labels: vec!["Tacos".into(), "Pizza".into()],
                    ..Default::default()
                },
            )?;
            Ok((poll.id, poll.options(tx.conn())?[0].id))
        })
        .await
        .unwrap();
    (app, poll, option)
}

fn request(method: Method, path: &str, value: serde_json::Value) -> Req {
    Req::new(method, path)
        .header("accept", "application/json")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&value).unwrap())
}

#[tokio::test]
async fn poll_membership_and_human_gates() {
    let (app, poll, option) = fixture().await;
    let mut kevin = app.sign_in(KEVIN).await;
    let mut bot = app.sign_in(BENDER).await;
    for (method, path, value) in [
        (
            Method::GET,
            format!("/rooms/{ALL_TALK}/polls/{poll}"),
            json!({}),
        ),
        (
            Method::POST,
            format!("/rooms/{ALL_TALK}/polls"),
            json!({"poll":{"question":"Lunch?","options":["A","B"]}}),
        ),
        (
            Method::POST,
            format!("/rooms/{ALL_TALK}/polls/{poll}/vote"),
            json!({"option_ids":[option]}),
        ),
    ] {
        assert_eq!(
            kevin
                .write(request(method.clone(), &path, value.clone()))
                .await
                .status,
            StatusCode::NOT_FOUND,
            "{path}"
        );
        assert_eq!(
            bot.write(request(method, &path, value)).await.status,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
    let mut david = app.david();
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE user_id = ? AND room_id = ?",
                (DAVID, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        david
            .get(&format!("/rooms/{ALL_TALK}/polls/{poll}"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let mut jason = app.sign_in(JASON).await;
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET deleted_at = ? WHERE id = ?",
                (tx.now(), ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        jason
            .get(&format!("/rooms/{ALL_TALK}/polls/{poll}"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn pins_require_reachable_messages_and_rooms() {
    let (app, poll, _) = fixture().await;
    let message = app
        .db()
        .read(move |conn| Ok(Poll::find(conn, poll)?.message_id))
        .await
        .unwrap();
    let mut kevin = app.sign_in(KEVIN).await;
    for method in [Method::POST, Method::DELETE] {
        assert_eq!(
            kevin
                .write(request(
                    method,
                    &format!("/messages/{message}/pin"),
                    json!({})
                ))
                .await
                .status,
            StatusCode::NOT_FOUND
        );
    }
    assert_eq!(
        kevin.get(&format!("/rooms/{ALL_TALK}/pins")).await.status,
        StatusCode::NOT_FOUND
    );
}

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/messaging/features.json"
    ))
    .unwrap()
}

async fn oracle_fixture() -> (TestApp, Vec<i64>) {
    use std::sync::Arc;
    let clock = Arc::new(campfire_kit::clock::FrozenClock::new(
        SEED_NOW.parse().unwrap(),
    ));
    let app = TestApp::boot_with_test_clock(clock)
        .await
        .expect("WS8bm2 requires the default parity seed");
    let data = oracle();
    let ids = app
        .db()
        .write(move |tx| {
            let mut ids = vec![];
            for (index, input) in data["inputs"].as_array().unwrap().iter().enumerate() {
                let message = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some(input["question"].as_str().unwrap().into()),
                        client_message_id: Some(format!("features-{index}")),
                        ..Default::default()
                    },
                )?;
                let poll = Poll::create_for_message(
                    tx,
                    &message,
                    NewPoll {
                        labels: input["labels"]
                            .as_array()
                            .unwrap()
                            .iter()
                            .map(|v| v.as_str().unwrap().to_owned())
                            .collect(),
                        multiple: input["multiple"].as_bool().unwrap_or(false),
                        anonymous: input["anonymous"].as_bool().unwrap_or(false),
                        closes_at: input["closes_at"]
                            .as_str()
                            .map(|time| campfire_db::Timestamp::from_jiff(time.parse().unwrap())),
                    },
                )?;
                ids.push(poll.id);
            }
            Ok(ids)
        })
        .await
        .unwrap();
    assert_eq!(json!(ids), oracle()["poll_ids"]);
    (app, ids)
}

async fn render_poll(app: &TestApp, id: i64, error: Option<String>) -> String {
    let runtime = app.booted.app.clone();
    app.db()
        .read(move |conn| {
            let view = super::poll_view(conn, &runtime, id, error)?;
            let account = campfire_db::Account::first(conn)?;
            Ok(crate::controllers::presenters::page::render_detached_at(
                &runtime,
                account.as_ref(),
                "http://campfire.test",
                |ctx| campfire_views::messages::parts::poll(ctx, &view).0,
            ))
        })
        .await
        .unwrap()
}

#[tokio::test]
async fn poll_partials_match_rails_open_voted_anonymous_closed_and_error_bytes() {
    let (app, ids) = oracle_fixture().await;
    for (index, row) in oracle()["html"].as_array().unwrap().iter().enumerate() {
        if index > 0 {
            let id = ids[row["index"].as_u64().unwrap() as usize];
            app.db()
                .write(move |tx| {
                    let mut poll = Poll::find(tx.conn(), id)?;
                    if index == 3 {
                        poll.close(tx, tx.now())?;
                    } else if index < 3 {
                        poll.cast_vote(tx, DAVID, &[poll.options(tx.conn())?[0].id])?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
        }
        let id = ids[row["index"].as_u64().unwrap() as usize];
        let html = render_poll(&app, id, row["error"].as_str().map(str::to_owned)).await;
        assert_eq!(html, row["html"].as_str().unwrap(), "state {index}");
        assert!(!html.contains("authenticity_token") && !html.contains("nonce=\""));
        if row["index"] == 1 {
            assert!(!html.contains(&format!("data-voter-ids=\"{DAVID}")));
        }
    }
}

async fn ballot_states(app: &TestApp, ids: &[i64]) {
    let ids = ids.to_vec();
    app.db()
        .write(move |tx| {
            for id in &ids[..2] {
                let mut poll = Poll::find(tx.conn(), *id)?;
                poll.cast_vote(tx, DAVID, &[poll.options(tx.conn())?[0].id])?;
            }
            Poll::find(tx.conn(), ids[2])?.close(tx, tx.now())?;
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn poll_http_matches_real_rails_responses_and_transactional_create_errors() {
    let (app, ids) = oracle_fixture().await;
    ballot_states(&app, &ids).await;
    let mut david = app.david();
    for row in oracle()["steps"].as_array().unwrap() {
        let id = ids[row["index"].as_u64().unwrap() as usize];
        let method = if row["method"] == "get" {
            Method::GET
        } else {
            Method::POST
        };
        let path = format!(
            "/rooms/{ALL_TALK}/polls/{id}{}",
            if method == Method::POST { "/vote" } else { "" }
        );
        let value = row
            .get("option_ids")
            .map(|ids| json!({"option_ids":ids}))
            .unwrap_or_else(|| json!({}));
        let reply = david.write(request(method, &path, value)).await;
        assert_eq!(
            reply.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}",
            reply.text()
        );
        assert_eq!(reply.json(), row["json"]);
        assert_eq!(reply.text(), row["body"].as_str().unwrap());
    }
    for row in oracle()["creates"].as_array().unwrap() {
        let before = app
            .db()
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap();
        let reply = david
            .write(request(
                Method::POST,
                &format!("/rooms/{ALL_TALK}/polls"),
                row["input"].clone(),
            ))
            .await;
        assert_eq!(
            reply.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}",
            reply.text()
        );
        assert_eq!(reply.json(), row["json"]);
        assert_eq!(reply.text(), row["body"].as_str().unwrap());
        let after = app
            .db()
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap();
        assert_eq!(after - before, row["message_delta"].as_i64().unwrap());
    }
}

#[test]
fn builder_dates_match_rails_zones_and_dst_gap_fold() {
    for row in oracle()["dates"].as_array().unwrap() {
        let zone = campfire_views::time::Zone::lookup(row["zone"].as_str().unwrap()).unwrap();
        let time = super::parse_time(
            row["raw"].as_str().unwrap(),
            &zone,
            SEED_NOW.parse().unwrap(),
        )
        .unwrap()
        .map(campfire_db::poll::json_time);
        assert_eq!(json!(time), row["time"], "{row}");
    }
}

#[tokio::test]
async fn pin_requests_match_rails_and_keep_the_note_quiet_and_idempotent() {
    let (app, ids) = oracle_fixture().await;
    let message_id = app
        .db()
        .read(move |conn| Ok(Poll::find(conn, ids[0])?.message_id))
        .await
        .unwrap();
    let mut david = app.david();
    for row in oracle()["pins"].as_array().unwrap() {
        let before = app
            .db()
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap();
        let method = if row["method"] == "post" {
            Method::POST
        } else {
            Method::DELETE
        };
        let reply = david
            .write(request(
                method,
                &format!("/messages/{message_id}/pin"),
                json!({}),
            ))
            .await;
        assert_eq!(
            reply.status.as_u16(),
            row["status"].as_u64().unwrap() as u16,
            "{}",
            reply.text()
        );
        assert_eq!(reply.text(), row["body"].as_str().unwrap());
        let after = app
            .db()
            .read(|conn| {
                Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
            })
            .await
            .unwrap();
        assert_eq!(after - before, row["message_delta"].as_i64().unwrap());
    }
    let notes = app
        .db()
        .read(|conn| Message::for_room(conn, ALL_TALK))
        .await
        .unwrap()
        .into_iter()
        .filter(|message| {
            message
                .markdown_source
                .as_deref()
                .is_some_and(|source| source.starts_with("pinned a message:"))
        })
        .collect::<Vec<_>>();
    assert_eq!(notes.len(), 1);
    assert!(notes[0].system_note);
    let indexed: i64 = app
        .db()
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT COUNT(*) FROM message_search_index WHERE rowid = ?",
                [notes[0].id],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap();
    assert_eq!(indexed, 0);
    let reply = david
        .write(
            Req::new(Method::DELETE, &format!("/messages/{message_id}/pin"))
                .header("accept", "text/vnd.turbo-stream.html"),
        )
        .await;
    assert_eq!(reply.status, StatusCode::SEE_OTHER);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{ALL_TALK}/pins").as_str())
    );
}

#[tokio::test]
async fn poll_and_pin_writes_require_csrf_and_reject_bot_keys() {
    let (app, poll, option) = fixture().await;
    let message = app
        .db()
        .read(move |conn| Ok(Poll::find(conn, poll)?.message_id))
        .await
        .unwrap();
    let mut david = app.david();
    let mut anonymous = app.anonymous();
    for (path, value) in [
        (
            format!("/rooms/{ALL_TALK}/polls"),
            json!({"poll":{"question":"Lunch?","options":["A","B"]}}),
        ),
        (
            format!("/rooms/{ALL_TALK}/polls/{poll}/vote"),
            json!({"option_ids":[option]}),
        ),
        (format!("/messages/{message}/pin"), json!({})),
    ] {
        assert_eq!(
            david
                .send(request(Method::POST, &path, value.clone()))
                .await
                .status,
            StatusCode::UNPROCESSABLE_ENTITY
        );
        assert_eq!(
            anonymous
                .send(request(
                    Method::POST,
                    &format!("{path}?bot_key={BENDER_KEY}"),
                    value.clone()
                ))
                .await
                .status,
            StatusCode::FORBIDDEN
        );
        let authorization = format!("{} {}", "Bearer", "fixture-revoked-agent-credential");
        assert_eq!(
            anonymous
                .send(request(Method::POST, &path, value).header("authorization", &authorization))
                .await
                .status,
            StatusCode::UNAUTHORIZED
        );
    }
}

#[tokio::test]
async fn poll_creation_rolls_back_when_the_durable_job_insert_is_rejected() {
    let (app, _, _) = fixture().await;
    app.db().write(|tx| { tx.conn().execute_batch("CREATE TRIGGER feature_reject_job BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'rejected job'); END;")?; Ok(()) }).await.unwrap();
    let before = app
        .db()
        .read(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
        })
        .await
        .unwrap();
    let reply = app
        .david()
        .write(request(
            Method::POST,
            &format!("/rooms/{ALL_TALK}/polls"),
            json!({"poll":{"question":"Lunch?","options":["A","B"]}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    let after = app
        .db()
        .read(|conn| {
            Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)
        })
        .await
        .unwrap();
    assert_eq!(before, after);
}

#[tokio::test]
async fn pin_partials_match_rails_list_count_badge_frame_and_empty_bytes() {
    use askama::Template;
    let (app, ids) = oracle_fixture().await;
    let message_id = app
        .db()
        .read(move |conn| Ok(Poll::find(conn, ids[0])?.message_id))
        .await
        .unwrap();
    app.db()
        .write(move |tx| {
            let message = Message::find(tx.conn(), message_id)?;
            campfire_db::MessagePin::create(tx, &message, ALL_TALK, DAVID)?;
            Ok(())
        })
        .await
        .unwrap();
    for empty in [false, true] {
        if empty {
            app.db()
                .write(move |tx| {
                    campfire_db::MessagePin::find_by_message(tx.conn(), message_id)?
                        .unwrap()
                        .unpin(tx)
                })
                .await
                .unwrap();
        }
        let runtime = app.booted.app.clone();
        let actual = app.db().read(move |conn| {
            let room = campfire_db::Room::find(conn, ALL_TALK)?;
            let list = crate::controllers::rooms::pins::list(conn, &runtime, &room)?;
            let message = Message::find(conn, message_id)?;
            let message = campfire_views::pins::Badge::new(
                message.client_message_id,
                campfire_db::MessagePin::pinned(conn, message_id)?,
            );
            let account = campfire_db::Account::first(conn)?;
            crate::controllers::presenters::page::render_detached_at(&runtime, account.as_ref(), "http://campfire.test", |ctx| {
                Ok(json!({
                    "list": campfire_views::pins::ListPartial { ctx, list: &list }.render().unwrap(),
                    "count": campfire_views::pins::CountPartial { room_id: ALL_TALK, room_param_key: list.room_param_key.clone(), count: list.pins.len() as i64 }.render().unwrap(),
                    "badge": campfire_views::pins::BadgePartial { ctx, message: &message }.render().unwrap(),
                    "index": campfire_views::pins::Index { ctx, list: &list }.render().unwrap(),
                }))
            })
        }).await.unwrap();
        let expected = &oracle()["pin_html"];
        assert_eq!(
            actual["list"],
            expected[if empty { "empty_list" } else { "list" }]
        );
        assert_eq!(
            actual["badge"],
            expected[if empty { "hidden_badge" } else { "badge" }]
        );
        assert!(
            !actual["list"]
                .as_str()
                .unwrap()
                .contains("authenticity_token")
        );
        if !empty {
            assert_eq!(actual["count"], expected["count"]);
            assert_eq!(actual["index"], expected["index"]);
        }
    }
    let mut david = app.david();
    let reply = david
        .send(
            Req::new(Method::GET, &format!("/rooms/{ALL_TALK}/pins")).header(
                "turbo-frame",
                &format!("pins_frame_rooms_closed_{ALL_TALK}"),
            ),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert!(reply.text().contains("No pinned messages yet"));
    assert!(reply.text().contains(&format!(
        "<turbo-frame id=\"pins_frame_rooms_closed_{ALL_TALK}\">"
    )));
}

#[tokio::test]
async fn poll_and_pin_frames_reach_a_real_websocket_without_session_values() {
    use crate::channels::tests::support::{Client, bind_listener, identifier};
    use tokio_tungstenite::tungstenite::client::IntoClientRequest;
    let (app, ids) = oracle_fixture().await;
    let listener = bind_listener().await;
    let address = listener.local_addr().unwrap();
    let router = app.booted.router.clone();
    let server = tokio::spawn(async move { axum::serve(listener, router).await.unwrap() });
    let mut ws = format!("ws://{address}/cable")
        .into_client_request()
        .unwrap();
    ws.headers_mut()
        .insert("host", "campfire.test".parse().unwrap());
    ws.headers_mut()
        .insert("origin", "http://campfire.test".parse().unwrap());
    ws.headers_mut()
        .insert("cookie", david_cookie().parse().unwrap());
    let (socket, _) = tokio_tungstenite::connect_async(ws).await.unwrap();
    let mut client = Client { socket };
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    let room = app
        .db()
        .read(|conn| campfire_db::Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    let stream = rails_compat::turbo::signed_stream_name(
        &app.booted.app.secrets,
        &[&crate::channels::room_gid(&room).to_param(), "messages"],
    );
    client
        .confirm(&identifier(
            json!({"channel":"RoomMessagesChannel", "signed_stream_name":stream}),
        ))
        .await;
    let id = ids[0];
    let (message, option) = app
        .db()
        .read(move |conn| {
            let poll = Poll::find(conn, id)?;
            Ok((poll.message_id, poll.options(conn)?[0].id))
        })
        .await
        .unwrap();
    let mut david = app.david();
    let reply = david
        .write(request(
            Method::POST,
            &format!("/rooms/{ALL_TALK}/polls/{id}/vote"),
            json!({"option_ids":[option]}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    assert_eq!(
        frame["message"],
        format!(
            "<turbo-stream maintain_scroll=\"true\" action=\"replace\" target=\"card_poll_{id}\"><template>{}</template></turbo-stream>",
            oracle()["html"][1]["html"].as_str().unwrap()
        )
    );
    assert_eq!(
        david
            .write(request(
                Method::POST,
                &format!("/messages/{message}/pin"),
                json!({})
            ))
            .await
            .status,
        StatusCode::CREATED
    );
    for key in ["badge", "count", "list"] {
        let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
        let html = frame["message"].as_str().unwrap();
        let expected = oracle()["pin_html"][key].as_str().unwrap().to_owned();
        assert!(
            html.contains(&format!("<template>{expected}</template>")),
            "{key}: {html}"
        );
        assert!(!html.contains("authenticity_token") && !html.contains("nonce=\""));
    }
    let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
    let html = frame["message"].as_str().unwrap();
    assert!(
        html.contains("action=\"append\"") && html.contains("message--system-note"),
        "{html}"
    );
    assert_eq!(
        david
            .write(request(
                Method::DELETE,
                &format!("/messages/{message}/pin"),
                json!({})
            ))
            .await
            .status,
        StatusCode::OK
    );
    for key in ["hidden_badge", "empty_count", "empty_list"] {
        let frame: serde_json::Value = serde_json::from_str(&client.next_text().await).unwrap();
        let html = frame["message"].as_str().unwrap();
        let expected = if key == "empty_count" {
            oracle()["pin_html"]["count"]
                .as_str()
                .unwrap()
                .replace("1 pinned", "0 pinned")
                .replace(">1<", ">0<")
        } else {
            oracle()["pin_html"][key].as_str().unwrap().to_owned()
        };
        assert!(
            html.contains(&format!("<template>{expected}</template>")),
            "{key}: {html}"
        );
    }
    client.assert_silent().await;
    server.abort();
}

#[tokio::test]
async fn pins_enforce_the_cap_but_repinning_a_full_room_succeeds() {
    let (app, poll, _) = fixture().await;
    let (message, first) = app
        .db()
        .write(move |tx| {
            let message = Poll::find(tx.conn(), poll)?.message_id;
            let mut first = 0;
            for n in 0..campfire_db::message_pin::MAX_PER_ROOM {
                let candidate = Message::create(
                    tx,
                    NewMessage {
                        room_id: ALL_TALK,
                        creator_id: DAVID,
                        markdown_source: Some(format!("Pinnable {n}")),
                        ..Default::default()
                    },
                )?;
                campfire_db::MessagePin::create(tx, &candidate, ALL_TALK, DAVID)?;
                if n == 0 {
                    first = candidate.id;
                }
            }
            Ok((message, first))
        })
        .await
        .unwrap();
    let mut david = app.david();
    let reply = david
        .write(request(
            Method::POST,
            &format!("/messages/{message}/pin"),
            json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert_eq!(
        reply.json(),
        json!({"error":"This channel already has 50 pinned messages"})
    );
    let reply = david
        .write(request(
            Method::POST,
            &format!("/messages/{first}/pin"),
            json!({}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED);
    assert_eq!(reply.json(), json!({"pinned":true,"pin_count":50}));
}

#[tokio::test]
async fn ballots_replace_and_retract_in_turbo_and_reject_foreign_rooms_and_odd_shapes() {
    let (app, id, option) = fixture().await;
    let mut david = app.david();
    assert_eq!(
        david
            .get(&format!("/rooms/{QUIET_CORNER}/polls/{id}"))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let path = format!("/rooms/{ALL_TALK}/polls/{id}/vote");
    let reply = david
        .write(
            Req::new(Method::POST, &path)
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("option_ids[]", &option.to_string())]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(reply.text().starts_with(&format!(
        "<turbo-stream action=\"replace\" target=\"card_poll_{id}\">"
    )));
    assert!(reply.text().contains("1 · 100%"));
    let reply = david
        .write(
            Req::new(Method::POST, &path)
                .header("accept", "text/vnd.turbo-stream.html")
                .form(&[("option_ids[]", "0")]),
        )
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    assert!(
        reply
            .text()
            .contains("That option is not part of this poll")
    );
    for shape in [json!(true), json!({"id":option}), json!([[option]])] {
        assert_eq!(
            david
                .write(request(Method::POST, &path, json!({"option_ids":shape})))
                .await
                .status,
            StatusCode::INTERNAL_SERVER_ERROR
        );
    }
    let votes = app
        .db()
        .read(move |conn| Ok(Poll::find(conn, id)?.votes(conn)?.len()))
        .await
        .unwrap();
    assert_eq!(
        votes, 1,
        "an invalid shape must not silently retract the ballot"
    );
    let reply = david
        .write(request(Method::POST, &path, json!({"option_ids":null})))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    assert_eq!(reply.json()["total_votes"], 0);
}

#[tokio::test]
async fn boards_reject_root_polls_and_pin_lists_keep_their_sti_dom_identity() {
    let app = TestApp::boot().await.expect("WS8bm2 requires default seed");
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
    let mut david = app.david();
    let reply = david
        .write(request(
            Method::POST,
            &format!("/rooms/{}/polls", board.id),
            json!({"poll":{"question":"Lunch?","options":["A","B"]}}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::UNPROCESSABLE_ENTITY);
    let reply = david.get(&format!("/rooms/{}/pins", board.id)).await;
    assert_eq!(reply.status, StatusCode::OK);
    assert!(
        reply
            .text()
            .contains(&format!("id=\"pins_list_rooms_board_{}\"", board.id))
    );
}

#[tokio::test]
async fn a_zone_local_multiple_anonymous_poll_shows_only_each_viewers_ballot() {
    let app = TestApp::boot_with_test_clock(std::sync::Arc::new(
        campfire_kit::clock::FrozenClock::new(SEED_NOW.parse().unwrap()),
    ))
    .await
    .expect("WS8bm2 requires default seed");
    app.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE users SET time_zone = 'Hawaii' WHERE id = ?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    let reply = david.write(request(Method::POST, &format!("/rooms/{ALL_TALK}/polls"), json!({"poll":{"question":"Snacks?","options":["A","B"],"multiple":"1","anonymous":"1","closes_at":"2026-03-03T09:00"}}))).await;
    assert_eq!(reply.status, StatusCode::CREATED);
    let payload = reply.json();
    assert_eq!(payload["closes_at"], oracle()["dates"][2]["time"]);
    assert_eq!(payload["multiple"], true);
    assert_eq!(payload["anonymous"], true);
    let id = payload["id"].as_i64().unwrap();
    let options = payload["options"]
        .as_array()
        .unwrap()
        .iter()
        .map(|option| option["id"].as_i64().unwrap())
        .collect::<Vec<_>>();
    assert_eq!(
        david
            .write(request(
                Method::POST,
                &format!("/rooms/{ALL_TALK}/polls/{id}/vote"),
                json!({"option_ids":options})
            ))
            .await
            .status,
        StatusCode::OK
    );
    let mut jason = app.sign_in(JASON).await;
    let payload = jason
        .get(&format!("/rooms/{ALL_TALK}/polls/{id}"))
        .await
        .json();
    assert_eq!(payload["total_votes"], 2);
    for option in payload["options"].as_array().unwrap() {
        assert_eq!(option["voted"], false);
        assert!(option.get("voters").is_none());
    }
    let payload = david
        .get(&format!("/rooms/{ALL_TALK}/polls/{id}"))
        .await
        .json();
    for option in payload["options"].as_array().unwrap() {
        assert_eq!(option["voted"], true);
    }
}

#[tokio::test]
async fn pins_list_orders_newest_first_and_keeps_thread_jump_links() {
    let (app, id, _) = fixture().await;
    let (root, reply) = app
        .db()
        .write(move |tx| {
            let root = Message::find(tx.conn(), Poll::find(tx.conn(), id)?.message_id)?;
            let thread = campfire_db::ChannelThread::create(
                tx,
                campfire_db::NewChannelThread {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    parent_message_id: Some(root.id),
                    name: Some("Lunch replies".into()),
                    ..Default::default()
                },
            )?;
            let reply = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: JASON,
                    thread_id: Some(thread.id),
                    markdown_source: Some("Thread pin".into()),
                    ..Default::default()
                },
            )?;
            campfire_db::MessagePin::create(tx, &root, ALL_TALK, DAVID)?;
            campfire_db::MessagePin::create(tx, &reply, ALL_TALK, JASON)?;
            Ok((root, reply))
        })
        .await
        .unwrap();
    let body = app
        .david()
        .get(&format!("/rooms/{ALL_TALK}/pins"))
        .await
        .text();
    assert!(body.find("Thread pin").unwrap() < body.find("Lunch?").unwrap());
    let url = campfire_db::message_pin::message_path(&reply).replace('&', "&amp;");
    assert!(body.contains(&format!("href=\"http://campfire.test{url}\"")));
    assert!(body.contains(&format!("action=\"/messages/{}/pin\"", root.id)));
    assert!(body.contains("Pinned by Jason") && body.contains("Pinned by David"));
}
