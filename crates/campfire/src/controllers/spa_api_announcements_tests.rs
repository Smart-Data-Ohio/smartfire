use axum::http::{Method, StatusCode};
use serde_json::{Value, json};

use super::api_tests::{app, get, json_body, parse};
use crate::controllers::presenters::test_support::{ALL_TALK, DAVID, HQ, JASON, KEVIN};

#[tokio::test]
async fn announcement_follow_returns_422_for_board_targets() {
    let a = app(true)
        .await
        .expect("restored seed required")
        .without_job_runner()
        .await;
    a.db()
        .write(|tx| {
            campfire_db::models::announcement::set_flag(tx, ALL_TALK, true)?;
            tx.conn()
                .execute("UPDATE rooms SET type='Rooms::Board' WHERE id=?", [HQ])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut admin = a.sign_in(JASON).await;
    let reply = admin
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/announcement_follows"),
            &json!({"targetRoomId":HQ}),
        ))
        .await;
    assert_eq!(
        reply.status,
        StatusCode::UNPROCESSABLE_ENTITY,
        "{}",
        reply.text()
    );
    let error = parse::<Value>(&reply);
    assert_eq!(error["error"]["_tag"], "Validation");
    assert_eq!(
        error["error"]["fields"]["targetRoomId"],
        json!(["must show plain timeline messages; boards are not supported"])
    );
    assert!(
        error["error"]["message"]
            .as_str()
            .unwrap()
            .contains("boards are not supported")
    );
    assert!(
        a.db()
            .read(|conn| campfire_db::models::announcement::followers(conn, ALL_TALK))
            .await
            .unwrap()
            .is_empty()
    );
}

async fn replay_query_count(copies: bool) {
    use super::api_tests::{Sync, serve};
    use campfire_api_types as api;
    use campfire_cable::sync::{Audience, SyncPublication};
    use campfire_db::models::announcement;
    use campfire_db::{Message, NewMessage};
    let mut counts = Vec::new();
    for size in [1, 100] {
        let a = app(true)
            .await
            .expect("restored seed required")
            .without_job_runner()
            .await;
        let mut david = a.sign_in(DAVID).await;
        let (addr, server) = serve(&a).await;
        let topics = [format!("room:{HQ}")];
        let mut live = Sync::connect(addr, &david.cookie_header(), &topics).await;
        let Some(api::ServerFrame::Welcome { epoch, seq, .. }) = live.next().await else {
            panic!("a welcome")
        };
        let ids = a
            .db()
            .write(move |tx| {
                if copies {
                    announcement::set_flag(tx, ALL_TALK, true)?;
                    announcement::follow(tx, ALL_TALK, HQ, DAVID)?;
                }
                let mut ids = Vec::new();
                for _ in 0..size {
                    let source = Message::create(
                        tx,
                        NewMessage {
                            room_id: if copies { ALL_TALK } else { HQ },
                            creator_id: DAVID,
                            body: Some("<p>Batch replay</p>".into()),
                            ..Default::default()
                        },
                    )?;
                    if copies {
                        let publication = announcement::publish(tx, &source, DAVID)?;
                        let copy = announcement::deliver(tx, publication)?.remove(0).0;
                        ids.push((copy.id, Some(source.id)));
                    } else {
                        ids.push((source.id, None));
                    }
                }
                Ok(ids)
            })
            .await
            .unwrap();
        let read: api::MessageRead = parse(
            &david
                .send(get(&format!("/api/v1/messages/{}", ids[0].0)))
                .await,
        );
        a.booted.app.broadcasts.settle_sync().await;
        for &(id, _) in &ids {
            let mut message = read.message.clone();
            message.id = id;
            message.announcement_source = None;
            a.booted.app.cable.sync_publish(SyncPublication::new(
                Audience::Topic(topics[0].clone()),
                serde_json::to_string(&api::SyncPayload::MessageCreated(message)).unwrap(),
            ));
        }
        let last_id = ids.last().unwrap().0;
        live.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == last_id), |_| false).await;
        let queries = a.db().capture_read_queries();
        let mut replay = Sync::open(
            addr,
            &david.cookie_header(),
            &topics,
            json!({"epoch":epoch,"seq":seq}),
        )
        .await;
        assert!(matches!(
            replay.next().await,
            Some(api::ServerFrame::Welcome { resumed: true, .. })
        ));
        for &(id, source_id) in &ids {
            let event = replay.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == id), |_| false).await;
            let api::SyncPayload::MessageCreated(message) = event.payload else {
                unreachable!()
            };
            assert_eq!(
                message.announcement_source.map(|source| source.message_id),
                source_id
            );
        }
        a.db().stop_capturing_read_queries();
        let queries = queries.lock().unwrap();
        let selects = queries
            .iter()
            .filter(|sql| sql.trim_start().to_ascii_uppercase().starts_with("SELECT"))
            .count();
        let provenance = queries
            .iter()
            .filter(|sql| sql.contains("JOIN announcement_publications"))
            .count();
        println!(
            "announcement replay: copies={copies}, messages={size}, SELECTs={selects}, provenance={provenance}"
        );
        assert_eq!(
            provenance,
            usize::from(copies),
            "one provenance query only when copies are present"
        );
        counts.push(selects);
        server.abort();
    }
    assert_eq!(
        counts[0], counts[1],
        "replay SELECT count must be constant for 1 and 100 messages"
    );
}

#[tokio::test]
async fn announcement_replay_query_count_is_constant_without_copies() {
    replay_query_count(false).await;
}

#[tokio::test]
async fn announcement_replay_query_count_is_constant_with_copies() {
    replay_query_count(true).await;
}

async fn replay_announcement(revoked_room: Option<i64>) {
    use super::api_tests::{Sync, serve};
    use campfire_api_types as api;
    use campfire_db::models::announcement;
    use campfire_db::{Message, NewMessage};
    let a = app(true)
        .await
        .expect("restored seed required")
        .without_job_runner()
        .await;
    let david = a.sign_in(DAVID).await;
    let (addr, server) = serve(&a).await;
    let topics = [format!("room:{HQ}")];
    let mut sync = Sync::connect(addr, &david.cookie_header(), &topics).await;
    let Some(api::ServerFrame::Welcome { epoch, seq, .. }) = sync.next().await else {
        panic!("a welcome")
    };
    let (publication, source_id) = a
        .db()
        .write(|tx| {
            announcement::set_flag(tx, ALL_TALK, true)?;
            announcement::follow(tx, ALL_TALK, HQ, DAVID)?;
            let source = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("<p>Replay news</p>".into()),
                    ..Default::default()
                },
            )?;
            Ok((announcement::publish(tx, &source, DAVID)?, source.id))
        })
        .await
        .unwrap();
    crate::jobs::announcements::deliver(a.booted.app.clone(), publication)
        .await
        .unwrap();
    let copy_id = a
        .db()
        .read(move |c| Ok(announcement::copies(c, publication)?[0].id))
        .await
        .unwrap();
    let copy_created = |event: &api::SyncEvent| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == copy_id);
    let live = sync.until(copy_created, |_| false).await;
    let api::SyncPayload::MessageCreated(message) = live.payload else {
        unreachable!()
    };
    assert_eq!(message.announcement_source.unwrap().message_id, source_id);
    drop(sync);
    if let Some(room_id) = revoked_room {
        a.db()
            .write(move |tx| {
                tx.conn().execute(
                    "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                    (room_id, DAVID),
                )?;
                Ok(())
            })
            .await
            .unwrap();
    }
    campfire_app::cable::broadcasts::read_room(&a.booted.app.cable, DAVID, ALL_TALK);
    let mut resumed = Sync::open(
        addr,
        &david.cookie_header(),
        &topics,
        json!({"epoch": epoch, "seq": seq}),
    )
    .await;
    assert!(matches!(
        resumed.next().await,
        Some(api::ServerFrame::Welcome { resumed: true, .. })
    ));
    if revoked_room == Some(HQ) {
        resumed.until(
            |event| matches!(&event.payload, api::SyncPayload::RoomRead(read) if read.room_id == ALL_TALK),
            copy_created,
        ).await;
    } else {
        let replay = resumed.until(copy_created, |_| false).await;
        let api::SyncPayload::MessageCreated(message) = replay.payload else {
            unreachable!()
        };
        if revoked_room.is_some() {
            assert!(
                message.announcement_source.is_none(),
                "revoked source access must hide the replayed reference"
            );
        } else {
            assert_eq!(message.announcement_source.unwrap().message_id, source_id);
        }
    }
    server.abort();
}

#[tokio::test]
async fn announcement_replay_hides_a_revoked_source_reference() {
    replay_announcement(Some(ALL_TALK)).await;
}

#[tokio::test]
async fn announcement_replay_excludes_a_revoked_target_room() {
    replay_announcement(Some(HQ)).await;
}

#[tokio::test]
async fn announcement_replay_keeps_an_authorized_source_reference() {
    replay_announcement(None).await;
}

#[tokio::test]
async fn announcement_message_dto_paths_keep_viewer_authorized_references() {
    use campfire_db::models::announcement;
    use campfire_db::{ActivityItem, Message, NewMessage};
    let a = app(true)
        .await
        .expect("restored seed required")
        .without_job_runner()
        .await;
    let (copy_id, source_id) = a
        .db()
        .write(|tx| {
            announcement::set_flag(tx, ALL_TALK, true)?;
            announcement::follow(tx, ALL_TALK, HQ, DAVID)?;
            let source = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    markdown_source: Some("auditnews".into()),
                    ..Default::default()
                },
            )?;
            let publication = announcement::publish(tx, &source, DAVID)?;
            let copy = announcement::deliver(tx, publication)?.remove(0).0;
            campfire_db::MessagePin::pin(tx, &copy, KEVIN)?.unwrap();
            for viewer in [DAVID, KEVIN] {
                ActivityItem::refresh_unread(tx, viewer, "Message", copy.id, "mention")?;
            }
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                (ALL_TALK, KEVIN),
            )?;
            Ok((copy.id, source.id))
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/threads"),
            &json!({
                "parentMessageId": copy_id, "name": null,
                "message": {"clientMessageId": "audit-discussion", "markdownSource": "Discuss"},
            }),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let created = parse::<Value>(&reply);
    assert_eq!(
        created["detail"]["parentMessage"]["announcementSource"]["messageId"],
        source_id
    );
    let thread_id = created["detail"]["thread"]["id"].as_i64().unwrap();
    for (browser, visible) in [(&mut david, true), (&mut kevin, false)] {
        let saved = browser
            .write(json_body(
                Method::POST,
                "/api/v1/saved",
                &json!({"messageId": copy_id}),
            ))
            .await;
        assert_eq!(saved.status, StatusCode::CREATED, "{}", saved.text());
        for path in [
            format!("/api/v1/rooms/{HQ}/pins"),
            "/api/v1/search?q=auditnews".into(),
            "/api/v1/saved".into(),
        ] {
            let reply = browser.send(get(&path)).await;
            assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
            let response = parse::<Value>(&reply);
            let copy = response["messages"]
                .as_array()
                .unwrap()
                .iter()
                .find(|message| message["id"] == copy_id)
                .unwrap();
            assert_eq!(!copy["announcementSource"].is_null(), visible, "{path}");
            if visible {
                assert_eq!(copy["announcementSource"]["messageId"], source_id);
            }
        }
        let detail: Value = parse(
            &browser
                .send(get(&format!("/api/v1/threads/{thread_id}")))
                .await,
        );
        assert_eq!(
            !detail["parentMessage"]["announcementSource"].is_null(),
            visible
        );
        let activity: Value = parse(&browser.send(get("/api/v1/activity")).await);
        let item = activity["items"]
            .as_array()
            .unwrap()
            .iter()
            .find(|item| item["source"]["messageId"] == copy_id)
            .unwrap();
        assert_eq!(item["source"]["roomId"], HQ);
        assert_eq!(item["source"]["body"], "auditnews");
        assert!(
            !item["source"]["title"]
                .as_str()
                .unwrap()
                .contains("All Talk")
        );
    }
}

#[tokio::test]
async fn announcement_flag_reuses_room_management_permissions() {
    let a = app(true).await.expect("restored seed required");
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=0 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE rooms SET creator_id=? WHERE id=?", (DAVID, HQ))?;
            Ok(())
        })
        .await
        .unwrap();
    let path = format!("/api/v1/rooms/{HQ}/announcement");
    let mut kevin = a.sign_in(KEVIN).await;
    assert_eq!(
        kevin
            .write(json_body(Method::PUT, &path, &json!({"announcement":true})))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    let mut david = a.sign_in(DAVID).await;
    let reply = david
        .write(json_body(Method::PUT, &path, &json!({"announcement":true})))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(parse::<Value>(&reply)["announcement"], true);
    let mut admin = a.sign_in(JASON).await;
    assert_eq!(
        admin
            .write(json_body(
                Method::PUT,
                &path,
                &json!({"announcement":false})
            ))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        parse::<Value>(&david.send(get(&format!("/api/v1/rooms/{HQ}"))).await)["room"]["announcement"],
        false
    );
}

#[tokio::test]
async fn announcement_publish_checks_author_or_admin_and_is_idempotent() {
    let a = app(true).await.expect("restored seed required");
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=0 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let posted = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &json!({"clientMessageId":"announcement-source", "markdownSource":"News"}),
        ))
        .await;
    assert_eq!(posted.status, StatusCode::CREATED, "{}", posted.text());
    let id = parse::<Value>(&posted)["id"].as_i64().unwrap();
    let path = format!("/api/v1/messages/{id}/publish");
    assert_eq!(
        david
            .write(json_body(Method::POST, &path, &json!({})))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let mut admin = a.sign_in(JASON).await;
    assert_eq!(
        admin
            .write(json_body(
                Method::PUT,
                &format!("/api/v1/rooms/{ALL_TALK}/announcement"),
                &json!({"announcement":true})
            ))
            .await
            .status,
        StatusCode::OK
    );
    let mut kevin = a.sign_in(KEVIN).await;
    a.db()
        .write(|tx| campfire_db::Room::find(tx.conn(), ALL_TALK)?.grant_to(tx, &[KEVIN]))
        .await
        .unwrap();
    assert_eq!(
        kevin
            .write(json_body(Method::POST, &path, &json!({})))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    for _ in 0..2 {
        let reply = david
            .write(json_body(Method::POST, &path, &json!({})))
            .await;
        assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
        assert_eq!(parse::<Value>(&reply)["published"], true);
    }
    assert_eq!(
        admin
            .write(json_body(Method::POST, &path, &json!({})))
            .await
            .status,
        StatusCode::OK
    );
}

#[tokio::test]
async fn announcement_follows_require_source_access_and_target_management_and_refuse_cycles() {
    let a = app(true).await.expect("restored seed required");
    let mut admin = a.sign_in(JASON).await;
    for room in [HQ, ALL_TALK] {
        assert_eq!(
            admin
                .write(json_body(
                    Method::PUT,
                    &format!("/api/v1/rooms/{room}/announcement"),
                    &json!({"announcement":true})
                ))
                .await
                .status,
            StatusCode::OK
        );
    }
    let path = format!("/api/v1/rooms/{HQ}/announcement_follows");
    let mut kevin = a.sign_in(KEVIN).await;
    assert_eq!(
        kevin
            .write(json_body(
                Method::POST,
                &path,
                &json!({"targetRoomId":ALL_TALK})
            ))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        admin
            .write(json_body(Method::POST, &path, &json!({"targetRoomId":HQ})))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let reply = admin
        .write(json_body(
            Method::POST,
            &path,
            &json!({"targetRoomId":ALL_TALK}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(
        admin
            .write(json_body(
                Method::POST,
                &format!("/api/v1/rooms/{ALL_TALK}/announcement_follows"),
                &json!({"targetRoomId":HQ})
            ))
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    let listing: Value = parse(&admin.send(get(&path)).await);
    assert_eq!(listing["followers"].as_array().unwrap().len(), 1);
    let target: Value = parse(
        &admin
            .send(get(&format!(
                "/api/v1/rooms/{ALL_TALK}/announcement_follows"
            )))
            .await,
    );
    assert_eq!(target["following"].as_array().unwrap().len(), 1);
    let delete = format!("{path}/{ALL_TALK}");
    for _ in 0..2 {
        assert_eq!(
            admin
                .write(json_body(Method::DELETE, &delete, &json!({})))
                .await
                .status,
            StatusCode::NO_CONTENT
        );
    }
}

#[tokio::test]
async fn announcement_jobs_deliver_normal_messages_and_filter_http_and_sync_references() {
    use super::api_tests::{ALL_PETS, Sync, serve};
    use campfire_api_types as api;
    use campfire_db::models::announcement;
    use campfire_db::{Membership, Message};
    let a = app(true)
        .await
        .expect("restored seed required")
        .without_job_runner()
        .await;
    a.db()
        .write(|tx| {
            announcement::set_flag(tx, ALL_TALK, true)?;
            announcement::follow(tx, ALL_TALK, HQ, DAVID)?;
            announcement::follow(tx, ALL_TALK, ALL_PETS, DAVID)?;
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                (ALL_TALK, KEVIN),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let (addr, server) = serve(&a).await;
    let mut visible = Sync::connect(addr, &david.cookie_header(), &[format!("room:{HQ}")]).await;
    visible.welcome().await;
    let mut hidden = Sync::connect(addr, &kevin.cookie_header(), &[format!("room:{HQ}")]).await;
    hidden.welcome().await;
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{ALL_TALK}/messages"),
            &json!({"clientMessageId":"news-with-mention", "markdownSource":format!("News for <@{KEVIN}>")}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let source_id = parse::<Value>(&reply)["id"].as_i64().unwrap();
    assert_eq!(
        david
            .write(json_body(
                Method::POST,
                &format!("/api/v1/messages/{source_id}/publish"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::OK
    );
    let publication = a
        .db()
        .read(move |c| announcement::publication_id(c, source_id))
        .await
        .unwrap()
        .unwrap();
    for _ in 0..2 {
        crate::jobs::announcements::deliver(a.booted.app.clone(), publication)
            .await
            .unwrap();
    }
    let copies = a
        .db()
        .read(move |c| announcement::copies(c, publication))
        .await
        .unwrap();
    assert_eq!(copies.len(), 2);
    let copy_id = copies.iter().find(|m| m.room_id == HQ).unwrap().id;
    let created = |event: &api::SyncEvent| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id==copy_id);
    let visible_event = visible.until(created, |_| false).await;
    let hidden_event = hidden.until(created, |_| false).await;
    let api::SyncPayload::MessageCreated(visible_copy) = visible_event.payload else {
        unreachable!()
    };
    let api::SyncPayload::MessageCreated(hidden_copy) = hidden_event.payload else {
        unreachable!()
    };
    assert_eq!(visible_copy.announcement_source.unwrap().room_id, ALL_TALK);
    assert!(hidden_copy.announcement_source.is_none());
    assert_eq!(hidden_copy.creator_id, DAVID);
    let read: Value = parse(
        &kevin
            .send(get(&format!("/api/v1/messages/{copy_id}")))
            .await,
    );
    assert!(read["message"]["announcementSource"].is_null());
    let read: Value = parse(
        &david
            .send(get(&format!("/api/v1/messages/{copy_id}")))
            .await,
    );
    assert_eq!(
        read["message"]["announcementSource"]["roomName"],
        "All Talk"
    );
    assert_eq!(
        kevin
            .write(json_body(
                Method::POST,
                &format!("/api/v1/messages/{copy_id}/pin"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::CREATED
    );
    for (browser, can_see_source) in [(&mut kevin, false), (&mut david, true)] {
        let pins: Value = parse(&browser.send(get(&format!("/api/v1/rooms/{HQ}/pins"))).await);
        let pinned_copy = pins["messages"]
            .as_array()
            .unwrap()
            .iter()
            .find(|message| message["id"] == copy_id)
            .unwrap();
        assert_eq!(!pinned_copy["announcementSource"].is_null(), can_see_source);
    }
    let app = a.booted.app.clone();
    assert!(
        a.db()
            .read(move |c| {
                Ok(Message::find(c, copy_id)?
                    .mentionees(c, &*app.db.env().rich_text)?
                    .iter()
                    .any(|u| u.id == KEVIN))
            })
            .await
            .unwrap()
    );
    // A connected viewer sees the copy live; the regular receive path advances their pointer.
    let member = a
        .db()
        .read(|c| Membership::find_by_room_and_user(c, HQ, KEVIN))
        .await
        .unwrap()
        .unwrap();
    assert!(member.unread_at.is_some() || member.last_read_message_id == Some(copy_id));
    let reply = david
        .write(json_body(
            Method::PATCH,
            &format!("/api/v1/messages/{source_id}"),
            &json!({"markdownSource":"Revised news"}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    for _ in 0..2 {
        crate::jobs::announcements::deliver(a.booted.app.clone(), publication)
            .await
            .unwrap();
    }
    for copy in &copies {
        let id = copy.id;
        assert_eq!(
            a.db()
                .read(move |c| Ok(Message::find(c, id)?.markdown_source))
                .await
                .unwrap(),
            Some("Revised news".into())
        );
    }
    let reply = david
        .write(json_body(
            Method::POST,
            &format!("/api/v1/rooms/{HQ}/threads"),
            &json!({
                "parentMessageId": copy_id,
                "name": null,
                "message": {"clientMessageId":"announcement-discussion", "markdownSource":"Discuss the news"},
            }),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let thread_id = parse::<api::ThreadCreated>(&reply).detail.thread.id;
    assert_eq!(
        david
            .write(json_body(
                Method::DELETE,
                &format!("/api/v1/messages/{source_id}"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
    let rows = a.db().read(campfire_jobs::inspect::all).await.unwrap();
    let job = rows
        .iter()
        .find(|row| row.class == "Announcement::DeleteCopiesJob")
        .unwrap();
    let args: announcement::DeleteCopiesJob =
        serde_json::from_value(job.arguments.clone()).unwrap();
    for _ in 0..2 {
        crate::jobs::announcements::delete_delivered(
            a.booted.app.clone(),
            args.publication_id,
            args.message_ids.clone(),
        )
        .await
        .unwrap();
    }
    let event = tokio::time::timeout(
        std::time::Duration::from_secs(2),
        visible.until(
            |event| matches!(&event.payload, api::SyncPayload::ThreadUnread(thread) if thread.thread_id == thread_id && thread.refresh_only),
            |_| false,
        ),
    )
    .await
    .expect("deleting a published copy refreshes its target-room thread");
    assert!(matches!(event.payload, api::SyncPayload::ThreadUnread(_)));
    let detail: api::ThreadDetail = parse(
        &david.send(get(&format!("/api/v1/threads/{thread_id}"))).await,
    );
    assert!(detail.thread.parent_message_id.is_none());
    assert!(detail.parent_message.is_none());
    for copy in copies {
        assert!(
            a.db()
                .read(move |c| Message::find_by_id(c, copy.id))
                .await
                .unwrap()
                .is_none()
        );
    }
    server.abort();
}

#[tokio::test]
async fn announcement_follows_allow_target_creator_without_source_management() {
    let a = app(true).await.expect("restored seed required");
    a.db()
        .write(|tx| {
            tx.conn()
                .execute("UPDATE users SET role=0 WHERE id=?", [DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    a.db()
        .write(|tx| {
            campfire_db::models::announcement::set_flag(tx, HQ, true)?;
            tx.conn().execute(
                "UPDATE rooms SET creator_id=? WHERE id=?",
                (KEVIN, ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = a.sign_in(DAVID).await;
    let path = format!("/api/v1/rooms/{HQ}/announcement_follows");
    assert_eq!(
        david
            .write(json_body(
                Method::POST,
                &path,
                &json!({"targetRoomId":ALL_TALK})
            ))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "UPDATE rooms SET creator_id=? WHERE id=?",
                (DAVID, ALL_TALK),
            )?;
            tx.conn()
                .execute("UPDATE rooms SET creator_id=? WHERE id=?", (KEVIN, HQ))?;
            Ok(())
        })
        .await
        .unwrap();
    let reply = david
        .write(json_body(
            Method::POST,
            &path,
            &json!({"targetRoomId":ALL_TALK}),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(parse::<Value>(&reply)["followers"], json!([]));
    a.db()
        .write(|tx| {
            tx.conn().execute(
                "DELETE FROM memberships WHERE room_id=? AND user_id=?",
                (HQ, DAVID),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        david
            .write(json_body(
                Method::DELETE,
                &format!("{path}/{ALL_TALK}"),
                &json!({})
            ))
            .await
            .status,
        StatusCode::NO_CONTENT
    );
}

#[tokio::test]
async fn announcement_registered_worker_performs_duplicate_jobs_once() {
    use campfire_db::models::announcement::{self, SyncJob};
    let a = app(true).await.expect("restored seed required");
    let publication = a
        .db()
        .write(|tx| {
            announcement::set_flag(tx, ALL_TALK, true)?;
            announcement::follow(tx, ALL_TALK, HQ, DAVID)?;
            let message = campfire_db::Message::create(
                tx,
                campfire_db::NewMessage {
                    room_id: ALL_TALK,
                    creator_id: DAVID,
                    body: Some("Worker news".into()),
                    ..Default::default()
                },
            )?;
            let publication = announcement::publish(tx, &message, DAVID)?;
            for _ in 0..2 {
                tx.emit_after_commit(campfire_db::Event::job(&SyncJob {
                    publication_id: publication,
                }));
            }
            Ok(publication)
        })
        .await
        .unwrap();
    tokio::time::timeout(std::time::Duration::from_secs(10), async {
        loop {
            let (copies, jobs) = a
                .db()
                .read(move |c| {
                    Ok((
                        announcement::copies(c, publication)?,
                        campfire_jobs::inspect::all(c)?,
                    ))
                })
                .await
                .unwrap();
            if copies.len() == 1 && jobs.iter().all(|row| row.class != "Announcement::SyncJob") {
                assert_eq!(copies[0].creator_id, DAVID);
                break;
            }
            assert!(
                !jobs.iter().any(|row| row.class == "Announcement::SyncJob"
                    && row.status == campfire_jobs::FAILED),
                "{jobs:?}"
            );
            tokio::time::sleep(std::time::Duration::from_millis(20)).await;
        }
    })
    .await
    .expect("registered announcement jobs must finish");
}

#[tokio::test]
async fn announcement_webhook_copies_preserve_ignore_policy_in_http_live_and_replay() {
    use super::api_tests::{Sync, serve};
    use campfire_api_types as api;
    use campfire_db::models::{
        announcement,
        channel_webhook::{ChannelWebhook, WebhookMessage},
        user_block::UserBlock,
    };
    use campfire_db::{Membership, Message, NewMessage};

    let a = app(true)
        .await
        .expect("restored seed required")
        .without_job_runner()
        .await;
    let author = a
        .db()
        .write(|tx| {
            announcement::set_flag(tx, ALL_TALK, true)?;
            announcement::follow(tx, ALL_TALK, HQ, DAVID)?;
            Membership::create_default(tx, ALL_TALK, KEVIN)?;
            let (hook, _) = ChannelWebhook::create(tx, ALL_TALK, DAVID, "Announcements")?;
            UserBlock::set(tx, KEVIN, hook.author_id, true)?;
            tx.conn().execute(
                "UPDATE memberships SET unread_at=NULL WHERE room_id=? AND user_id=?",
                (HQ, KEVIN),
            )?;
            Ok((hook.id, hook.author_id))
        })
        .await
        .unwrap();
    let david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let (addr, server) = serve(&a).await;
    let topics = [format!("room:{HQ}")];
    let mut visible = Sync::connect(addr, &david.cookie_header(), &topics).await;
    visible.welcome().await;
    let mut hidden = Sync::connect(addr, &kevin.cookie_header(), &topics).await;
    let Some(api::ServerFrame::Welcome { epoch, seq, .. }) = hidden.next().await else {
        panic!("a welcome")
    };
    let (publication, source_id) = a
        .db()
        .write(move |tx| {
            let message = Message::create(
                tx,
                NewMessage {
                    room_id: ALL_TALK,
                    creator_id: author.1,
                    markdown_source: Some(format!("News for <@{KEVIN}> and <@{DAVID}>")),
                    webhook: Some(WebhookMessage {
                        webhook_id: author.0,
                        token_digest: None,
                        username: Some("Release bot".into()),
                        embeds: json!([{"title":"Release"}]),
                    }),
                    ..Default::default()
                },
            )?;
            Ok((announcement::publish(tx, &message, DAVID)?, message.id))
        })
        .await
        .unwrap();
    crate::jobs::announcements::deliver(a.booted.app.clone(), publication)
        .await
        .unwrap();
    let copy_id = a
        .db()
        .read(move |conn| {
            let copies = announcement::copies(conn, publication)?;
            assert_eq!(copies.len(), 1);
            let id = copies[0].id;
            let mut statement = conn.prepare(
                "SELECT user_id FROM activity_items WHERE source_type='Message' AND source_id=?",
            )?;
            let recipients = statement
                .query_map([id], |row| row.get::<_, i64>(0))?
                .collect::<rusqlite::Result<Vec<_>>>()?;
            assert!(recipients.contains(&DAVID));
            assert!(!recipients.contains(&KEVIN));
            let unread: Option<campfire_db::Timestamp> = conn.query_row(
                "SELECT unread_at FROM memberships WHERE room_id=? AND user_id=?",
                (HQ, KEVIN),
                |row| row.get(0),
            )?;
            assert!(unread.is_none());
            Ok(id)
        })
        .await
        .unwrap();
    let created = |event: &api::SyncEvent| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.id == copy_id);
    let visible_event = visible.until(created, |_| false).await;
    let hidden_event = hidden.until(created, |_| false).await;
    assert_eq!(visible_event.seq, hidden_event.seq);
    for (event, ignored) in [(visible_event, None), (hidden_event, Some(true))] {
        let api::SyncPayload::MessageCreated(message) = event.payload else {
            unreachable!()
        };
        assert_eq!(message.creator_id, author.1);
        assert_eq!(message.ignored, ignored);
        assert_eq!(message.webhook.unwrap().username.as_deref(), Some("Release bot"));
        assert_eq!(message.embeds.unwrap()[0].title.as_deref(), Some("Release"));
        assert_eq!(message.announcement_source.unwrap().message_id, source_id);
    }
    let read: api::MessageRead = parse(
        &kevin
            .send(get(&format!("/api/v1/messages/{copy_id}")))
            .await,
    );
    assert_eq!(read.message.ignored, Some(true));
    assert_eq!(read.message.webhook.unwrap().name, "Announcements");
    assert_eq!(read.message.announcement_source.unwrap().message_id, source_id);
    let mut replay = Sync::open(
        addr,
        &kevin.cookie_header(),
        &topics,
        json!({"epoch":epoch,"seq":seq}),
    )
    .await;
    assert!(matches!(replay.next().await, Some(api::ServerFrame::Welcome { resumed: true, .. })));
    let event = replay.until(created, |_| false).await;
    let api::SyncPayload::MessageCreated(message) = event.payload else {
        unreachable!()
    };
    assert_eq!(message.ignored, Some(true));
    assert_eq!(message.webhook.unwrap().name, "Announcements");
    assert_eq!(message.announcement_source.unwrap().message_id, source_id);
    server.abort();
}
