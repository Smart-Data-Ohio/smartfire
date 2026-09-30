//! Smartfire room HTTP regressions, with the required Rails-built seed.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Membership, Room, RoomType};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/rooms_http.json")).unwrap()
}

async fn app() -> TestApp {
    TestApp::boot()
        .await
        .expect("build parity/.seed/default before running room parity tests")
}

async fn group(app: &TestApp) -> i64 {
    app.db()
        .write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID, JASON, KEVIN], KEVIN)?.id))
        .await
        .unwrap()
}

#[tokio::test]
async fn parity_group_deletion_requires_admin_in_both_namespaces() {
    let app = app().await;
    let id = group(&app).await;
    let mut kevin = app.sign_in(KEVIN).await;
    for (key, path) in [
        ("group_delete_base", format!("/rooms/{id}")),
        ("group_delete_direct", format!("/rooms/directs/{id}")),
    ] {
        let reply = kevin.write(Req::new(Method::DELETE, &path)).await;
        assert_eq!(
            u64::from(reply.status.as_u16()),
            oracle()["cases"][key]["status"].as_u64().unwrap(),
            "{path}: {}",
            reply.text()
        );
        assert!(
            app.db()
                .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
                .await
                .unwrap()
        );
    }
}

#[tokio::test]
async fn parity_conversion_scopes_exclude_voice_stage_and_board() {
    let app = app().await;
    for kind in [RoomType::Voice, RoomType::Stage, RoomType::Board] {
        let id = app
            .db()
            .write(move |tx| {
                Ok(Room::create_for(tx, kind, Some("private history"), DAVID, &[DAVID])?.id)
            })
            .await
            .unwrap();
        let mut david = app.david();
        for namespace in ["opens", "closeds"] {
            let reply = david
                .write(
                    Req::new(Method::PATCH, &format!("/rooms/{namespace}/{id}"))
                        .form(&[("room[name]", "leaked"), ("user_ids[]", &DAVID.to_string())]),
                )
                .await;
            assert_eq!(
                reply.location(),
                Some("http://campfire.test/"),
                "{namespace}/{id}: {}",
                reply.text()
            );
            assert_eq!(
                app.db()
                    .read(move |conn| Ok(Room::find(conn, id)?.room_type))
                    .await
                    .unwrap(),
                kind
            );
        }
    }
}

#[tokio::test]
async fn parity_room_scoped_endpoints_reject_deleted_rooms_even_with_membership() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE rooms SET deleted_at=? WHERE id=?",
                (tx.now(), ALL_TALK),
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    let reply = david.get(&format!("/rooms/{ALL_TALK}/involvement")).await;
    assert_eq!(reply.status, StatusCode::NOT_FOUND);
    let reply = david.get(&format!("/rooms/{ALL_TALK}")).await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}

#[tokio::test]
async fn parity_destroy_marks_enqueues_and_returns_json() {
    let app = app().await;
    let mut david = app.david();
    let reply = david
        .write(Req::new(Method::DELETE, &format!("/rooms/{ALL_TALK}.json")))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.json(), oracle()["cases"]["destroy_json"]["json"]);
    app.db()
        .read(|conn| {
            let room = Room::find(conn, ALL_TALK)?;
            assert!(room.deleted_at.is_some());
            assert!(room.destroy_enqueued_at.is_some());
            assert!(Membership::for_room(conn, ALL_TALK)?.is_empty());
            let jobs: i64 = conn.query_row_cached(
                "SELECT count(*) FROM background_jobs WHERE job_class='Room::DestroyJob'",
                [],
                |r| r.get(0),
            )?;
            assert_eq!(jobs, 1);
            let actor: i64 = conn.query_row_cached(
                "SELECT actor_id FROM audit_logs WHERE action='room.destroy' AND target_id=?",
                [ALL_TALK],
                |r| r.get(0),
            )?;
            assert_eq!(actor, DAVID);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn parity_destroy_queue_failure_rolls_back_full_http_write() {
    let app = app().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_room_job BEFORE INSERT ON background_jobs WHEN NEW.job_class='Room::DestroyJob' BEGIN SELECT RAISE(ABORT,'injected queue failure'); END;")?;
        Ok(())
    }).await.unwrap();
    let reply = app
        .david()
        .write(Req::new(Method::DELETE, &format!("/rooms/{ALL_TALK}.json")))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    app.db()
        .read(|conn| {
            let room = Room::find(conn, ALL_TALK)?;
            assert!(room.deleted_at.is_none());
            assert!(room.destroy_enqueued_at.is_none());
            assert!(!Membership::for_room(conn, ALL_TALK)?.is_empty());
            let logs: i64 = conn.query_row_cached(
                "SELECT count(*) FROM audit_logs WHERE action='room.destroy' AND target_id=?",
                [ALL_TALK],
                |r| r.get(0),
            )?;
            assert_eq!(logs, 0);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn parity_closed_and_direct_nonmembers_cannot_mutate_or_read_settings() {
    let app = app().await;
    let mut kevin = app.sign_in(KEVIN).await;
    for id in [ALL_TALK, DIRECT_DAVID_JASON] {
        let reply = kevin.get(&format!("/rooms/{id}/involvement")).await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
        let reply = kevin
            .write(Req::new(Method::DELETE, &format!("/rooms/{id}.json")))
            .await;
        assert_eq!(reply.location(), Some("http://campfire.test/"));
    }
    for path in ["/account/custom_styles/edit"] {
        assert_eq!(
            kevin.get(path).await.status,
            StatusCode::FORBIDDEN,
            "{path}"
        );
    }
}

async fn category(app: &TestApp, user: i64, name: &'static str) -> i64 {
    app.db()
        .write(move |tx| Ok(campfire_db::RoomCategory::create(tx, user, name, 1, false)?.id))
        .await
        .unwrap()
}

#[tokio::test]
async fn parity_category_crud_is_scoped_and_preserves_validation_behavior() {
    let app = app().await;
    app.db()
        .write(|tx| {
            for category in campfire_db::RoomCategory::ordered_for_user(tx.conn(), DAVID)? {
                category.destroy(tx)?;
            }
            Ok(())
        })
        .await
        .unwrap();
    let theirs = category(&app, JASON, "Theirs").await;
    let mut david = app.david();
    let create = david
        .write(Req::new(Method::POST, "/room_categories").form(&[
            ("room_category[name]", "Team"),
            ("room_category[position]", "99"),
        ]))
        .await;
    assert_eq!(
        create.location(),
        Some("http://campfire.test/users/me/sidebar")
    );
    let mine = app
        .db()
        .read(|conn| {
            Ok(campfire_db::RoomCategory::ordered_for_user(conn, DAVID)?
                .last()
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let update = david
        .write(
            Req::new(Method::PATCH, &format!("/room_categories/{mine}")).form(&[
                ("room_category[name]", "Squad"),
                ("room_category[collapsed]", "true"),
            ]),
        )
        .await;
    assert_eq!(update.status, StatusCode::FOUND);
    let row = app
        .db()
        .read(move |conn| campfire_db::RoomCategory::find(conn, mine))
        .await
        .unwrap();
    assert_eq!(
        (row.name.as_str(), row.collapsed, row.position),
        ("Squad", true, 1)
    );
    let list = david.get("/room_categories.json").await;
    assert_eq!(list.json(), serde_json::json!([{"id":mine,"name":"Squad"}]));
    for method in [Method::PATCH, Method::DELETE] {
        let reply = david
            .write(
                Req::new(method, &format!("/room_categories/{theirs}"))
                    .form(&[("room_category[name]", "stolen")]),
            )
            .await;
        assert_eq!(reply.status, StatusCode::NOT_FOUND);
    }
    let invalid = david
        .write(
            Req::new(Method::PATCH, &format!("/room_categories/{mine}"))
                .form(&[("room_category[name]", "")]),
        )
        .await;
    assert_eq!(
        invalid.status,
        StatusCode::FOUND,
        "invalid Rails update reloads sidebar"
    );
    assert_eq!(
        app.db()
            .read(move |conn| Ok(campfire_db::RoomCategory::find(conn, mine)?.name))
            .await
            .unwrap(),
        "Squad"
    );
    let invalid = david
        .write(Req::new(Method::POST, "/room_categories").form(&[("room_category[name]", "")]))
        .await;
    assert_eq!(invalid.status, StatusCode::FOUND);
    assert_eq!(
        app.db()
            .read(|conn| Ok(campfire_db::RoomCategory::ordered_for_user(conn, DAVID)?.len()))
            .await
            .unwrap(),
        1
    );
}

#[tokio::test]
async fn parity_category_assignment_owns_category_and_requires_channel_membership() {
    let app = app().await;
    let mine = category(&app, DAVID, "Team").await;
    let theirs = category(&app, JASON, "Theirs").await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/category_assignment.json");
    assert_eq!(
        david
            .write(
                Req::new(Method::PATCH, &path).form(&[("room_category_id", &theirs.to_string())])
            )
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        david
            .write(
                Req::new(
                    Method::PATCH,
                    &format!("/rooms/{DIRECT_DAVID_JASON}/category_assignment.json")
                )
                .form(&[("room_category_id", &mine.to_string())])
            )
            .await
            .status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(
        david
            .write(Req::new(Method::PATCH, &path).form(&[("room_category_id", &mine.to_string())]))
            .await
            .status,
        StatusCode::OK
    );
    let row = app
        .db()
        .read(|conn| Membership::find_by_room_and_user(conn, ALL_TALK, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert_eq!(row.room_category_id, Some(mine));
    assert_eq!(
        david
            .write(Req::new(
                Method::DELETE,
                &format!("/room_categories/{mine}")
            ))
            .await
            .status,
        StatusCode::FOUND
    );
    assert!(
        app.db()
            .read(
                |conn| Ok(Membership::find_by_room_and_user(conn, ALL_TALK, DAVID)?
                    .unwrap()
                    .room_category_id
                    .is_none())
            )
            .await
            .unwrap()
    );
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .write(Req::new(Method::PATCH, &path))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn parity_favorites_append_reorder_clamp_and_are_private() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn().execute_cached(
                "UPDATE memberships SET favorite_position=NULL WHERE user_id=?",
                [DAVID],
            )?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    for id in [ALL_TALK, QUIET_CORNER, DIRECT_DAVID_JASON, ALL_TALK] {
        assert_eq!(
            david
                .write(Req::new(
                    Method::POST,
                    &format!("/rooms/{id}/favorite.json")
                ))
                .await
                .status,
            StatusCode::OK
        );
    }
    assert_eq!(
        david
            .write(
                Req::new(
                    Method::PATCH,
                    &format!("/rooms/{DIRECT_DAVID_JASON}/favorite.json")
                )
                .form(&[("position", "-20")])
            )
            .await
            .status,
        StatusCode::OK
    );
    let rows = app
        .db()
        .read(|conn| Membership::favorites_for_user(conn, DAVID))
        .await
        .unwrap();
    assert_eq!(
        rows.iter()
            .map(|m| (m.room_id, m.favorite_position))
            .collect::<Vec<_>>(),
        vec![
            (DIRECT_DAVID_JASON, Some(0)),
            (ALL_TALK, Some(1)),
            (QUIET_CORNER, Some(2))
        ]
    );
    assert_eq!(
        david
            .write(
                Req::new(
                    Method::PATCH,
                    &format!("/rooms/{DIRECT_DAVID_JASON}/favorite.json")
                )
                .form(&[("position", "99")])
            )
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        david
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/{ALL_TALK}/favorite.json")
            ))
            .await
            .status,
        StatusCode::OK
    );
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{ALL_TALK}/favorite.json")
            ))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
}

#[tokio::test]
async fn parity_muting_clears_unread_and_json_does_not_redirect() {
    let app = app().await;
    app.db().write(|tx| { tx.conn().execute_cached("UPDATE memberships SET unread_at=?,last_read_message_id=NULL WHERE user_id=? AND room_id=?", (tx.now(), DAVID, ALL_TALK))?; Ok(()) }).await.unwrap();
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/involvement.json");
    let reply = david
        .write(Req::new(Method::PATCH, &path).form(&[("involvement", "muted")]))
        .await;
    assert_eq!(reply.status, StatusCode::OK);
    let row = app
        .db()
        .read(|conn| Membership::find_by_room_and_user(conn, ALL_TALK, DAVID))
        .await
        .unwrap()
        .unwrap();
    assert!(!row.unread());
    assert!(row.last_read_message_id.is_some());
    let missing = david.write(Req::new(Method::PATCH, &path)).await;
    assert_eq!(missing.status, StatusCode::BAD_REQUEST);
}

#[tokio::test]
async fn parity_inbound_address_rotation_requires_emailable_room_and_admin_or_creator() {
    let app = app().await;
    let mut david = app.david();
    let path = format!("/rooms/{ALL_TALK}/inbound_email_address");
    for _ in 0..2 {
        assert_eq!(
            david.write(Req::new(Method::POST, &path)).await.location(),
            Some(format!("http://campfire.test/rooms/{ALL_TALK}/edit").as_str())
        );
    }
    assert!(
        app.db()
            .read(|conn| Ok(Room::find(conn, ALL_TALK)?.inbound_email_token.is_some()))
            .await
            .unwrap()
    );
    assert_eq!(
        david
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{DIRECT_DAVID_JASON}/inbound_email_address")
            ))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .write(Req::new(Method::POST, &path))
            .await
            .status,
        StatusCode::NOT_FOUND
    );
    let id = app
        .db()
        .write(|tx| {
            Ok(Room::create_for(
                tx,
                RoomType::Closed,
                Some("secret"),
                JASON,
                &[DAVID, JASON, KEVIN],
            )?
            .id)
        })
        .await
        .unwrap();
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .write(Req::new(
                Method::POST,
                &format!("/rooms/{id}/inbound_email_address")
            ))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
}

#[tokio::test]
async fn parity_direct_create_filters_inactive_caps_before_query_and_supports_huddle() {
    let app = app().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE users SET status=1 WHERE id=?", [KEVIN])?;
            Ok(())
        })
        .await
        .unwrap();
    let mut david = app.david();
    let reply = david
        .write(Req::new(Method::POST, "/rooms/directs").form(&[
            ("user_ids[]", &JASON.to_string()),
            ("user_ids[]", &KEVIN.to_string()),
            ("start_huddle", "1"),
        ]))
        .await;
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/{DIRECT_DAVID_JASON}?huddle=start").as_str())
    );
    let mut pairs = vec![("user_ids[]", "missing"); 10];
    let kevin = KEVIN.to_string();
    pairs.push(("user_ids[]", &kevin));
    let reply = david
        .write(Req::new(Method::POST, "/rooms/directs").form(&pairs))
        .await;
    let id: i64 = reply
        .location()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap();
    assert_eq!(
        app.db()
            .read(move |conn| Room::find(conn, id)?.user_ids(conn))
            .await
            .unwrap(),
        vec![DAVID]
    );
    let count: i64 = app
        .db()
        .read(|conn| {
            conn.query_row_cached(
                "SELECT count(*) FROM audit_logs WHERE action='room.create'",
                [],
                |r| r.get(0),
            )
            .map_err(Into::into)
        })
        .await
        .unwrap();
    david
        .write(Req::new(Method::POST, "/rooms/directs").form(&pairs))
        .await;
    let after: i64 = app
        .db()
        .read(|conn| {
            conn.query_row_cached(
                "SELECT count(*) FROM audit_logs WHERE action='room.create'",
                [],
                |r| r.get(0),
            )
            .map_err(Into::into)
        })
        .await
        .unwrap();
    assert_eq!(
        after, count,
        "reusing a DM must not emit another creation audit"
    );
}

#[tokio::test]
async fn parity_direct_create_rejects_eleven_active_people() {
    let app = app().await;
    let ids = app
        .db()
        .write(|tx| {
            (0..10)
                .map(|i| {
                    Ok(campfire_db::User::create(
                        tx,
                        campfire_db::NewUser {
                            name: format!("Cap {i}"),
                            email_address: Some(format!("cap{i}@example.test")),
                            ..Default::default()
                        },
                    )?
                    .id
                    .to_string())
                })
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    let pairs = ids
        .iter()
        .map(|id| ("user_ids[]", id.as_str()))
        .collect::<Vec<_>>();
    let reply = app
        .david()
        .write(Req::new(Method::POST, "/rooms/directs").form(&pairs))
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/rooms/directs/new")
    );
}

#[tokio::test]
async fn parity_group_rename_add_and_leave_preserve_notes_and_history() {
    let app = app().await;
    let id = group(&app).await;
    let extra = app
        .db()
        .write(|tx| {
            Ok(campfire_db::User::create(
                tx,
                campfire_db::NewUser {
                    name: "Extra".into(),
                    email_address: Some("extra@example.test".into()),
                    ..Default::default()
                },
            )?
            .id)
        })
        .await
        .unwrap();
    let mut kevin = app.sign_in(KEVIN).await;
    let renamed = kevin
        .write(
            Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                .form(&[("room[name]", "  Weekend  ")]),
        )
        .await;
    assert_eq!(
        renamed.location(),
        Some(format!("http://campfire.test/rooms/directs/{id}/edit").as_str())
    );
    assert_eq!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.name))
            .await
            .unwrap()
            .as_deref(),
        Some("Weekend")
    );
    let added = kevin
        .write(
            Req::new(Method::POST, &format!("/rooms/directs/{id}/add_members"))
                .form(&[("user_ids[]", &extra.to_string())]),
        )
        .await;
    assert_eq!(added.status, StatusCode::FOUND);
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.user_ids(conn)?.contains(&extra)))
            .await
            .unwrap()
    );
    let left = kevin
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/directs/{id}/leave.json"),
        ))
        .await;
    assert_eq!(left.json(), serde_json::json!({"left":true,"room_id":id}));
    let renderer = app.db().env().rich_text.clone();
    let notes = app
        .db()
        .read(move |conn| {
            campfire_db::Message::for_room(conn, id)?
                .into_iter()
                .filter(|m| m.system_note)
                .map(|m| m.plain_text_body(conn, &*renderer))
                .collect::<campfire_db::Result<Vec<_>>>()
        })
        .await
        .unwrap();
    assert_eq!(
        serde_json::to_value(notes).unwrap(),
        oracle()["cases"]["group_note_texts"]
    );
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}

#[tokio::test]
async fn parity_pair_cannot_be_widened_and_group_writes_require_membership() {
    let app = app().await;
    let mut david = app.david();
    let reply = david
        .write(
            Req::new(
                Method::POST,
                &format!("/rooms/directs/{DIRECT_DAVID_JASON}/add_members"),
            )
            .form(&[("user_ids[]", &KEVIN.to_string())]),
        )
        .await;
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/directs/{DIRECT_DAVID_JASON}/edit").as_str())
    );
    assert!(
        !app.db()
            .read(|conn| Ok(Room::find(conn, DIRECT_DAVID_JASON)?
                .user_ids(conn)?
                .contains(&KEVIN)))
            .await
            .unwrap()
    );
    let reply = app
        .sign_in(KEVIN)
        .await
        .write(
            Req::new(
                Method::PATCH,
                &format!("/rooms/directs/{DIRECT_DAVID_JASON}"),
            )
            .form(&[("room[name]", "stolen")]),
        )
        .await;
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}

#[tokio::test]
async fn parity_plain_leave_keeps_room_and_last_direct_leave_enqueues_destroy() {
    let app = app().await;
    let mut david = app.david();
    let left = david
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/{ALL_TALK}/leave.json"),
        ))
        .await;
    assert_eq!(
        left.json(),
        serde_json::json!({"left":true,"room_id":ALL_TALK})
    );
    assert!(
        app.db()
            .read(|conn| Ok(Room::find(conn, ALL_TALK)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
    let solo = app
        .db()
        .write(|tx| Ok(Room::find_or_create_direct_for(tx, &[DAVID], DAVID)?.id))
        .await
        .unwrap();
    let left = david
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/directs/{solo}/leave.json"),
        ))
        .await;
    assert_eq!(left.json(), serde_json::json!({"left":true,"room_id":solo}));
    let room = app
        .db()
        .read(move |conn| Room::find(conn, solo))
        .await
        .unwrap();
    assert!(room.deleted_at.is_some());
    assert!(room.destroy_enqueued_at.is_some());
    assert_eq!(
        david
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{QUIET_CORNER}/leave.json")
            ))
            .await
            .location(),
        Some("http://campfire.test/")
    );
}

#[tokio::test]
async fn parity_audit_failure_does_not_undo_completed_room_deletion() {
    let app = app().await;
    app.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER reject_room_audit BEFORE INSERT ON audit_logs WHEN NEW.action='room.destroy' BEGIN SELECT RAISE(ABORT,'injected audit failure'); END;")?;
        Ok(())
    }).await.unwrap();
    let reply = app
        .david()
        .write(Req::new(Method::DELETE, &format!("/rooms/{ALL_TALK}.json")))
        .await;
    assert_eq!(reply.status, StatusCode::INTERNAL_SERVER_ERROR);
    let room = app
        .db()
        .read(|conn| Room::find(conn, ALL_TALK))
        .await
        .unwrap();
    assert!(
        room.deleted_at.is_some(),
        "Rails records the audit after the marking transaction commits"
    );
    assert!(
        room.destroy_enqueued_at.is_some(),
        "the durable destroy job stays atomic with marking"
    );
}

#[tokio::test]
async fn parity_category_name_uses_active_model_string_cast() {
    let app = app().await;
    let reply = app.david().write(Req::new(Method::POST, "/room_categories")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"room_category":{"name":false}})).unwrap())).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    let name = app.db().read(|conn| Ok(campfire_db::RoomCategory::ordered_for_user(conn, DAVID)?.last().unwrap().name.clone())).await.unwrap();
    assert_eq!(serde_json::json!(name), oracle()["cases"]["category_false_name_state"]);
}

#[tokio::test]
async fn unread_shell_facts_match_rails_pointer_cases() {
    let app=TestApp::boot_frozen().await.expect("seed required");
    let cases:Vec<serde_json::Value>=serde_json::from_str(include_str!("../../../../../vectors/room_shell_unread.json")).unwrap();
    for case in cases {
        let name=case["name"].as_str().unwrap().to_string();
        let stamp=case["unread_at"].as_str().map(|t|campfire_db::Timestamp::from_jiff(t.parse::<jiff::Timestamp>().unwrap()));
        let pointer=case["last_read_message_id"].as_i64();
        app.db().write(move |tx| { tx.conn().execute("UPDATE memberships SET unread_at=?,last_read_message_id=? WHERE room_id=486777696 AND user_id=127326141",rusqlite::params![stamp,pointer])?;Ok(()) }).await.unwrap();
        let result=app.db().read(|conn| {
            let membership=campfire_db::Membership::find_by_room_and_user(conn,486777696,127326141)?.unwrap();
            let messages=crate::controllers::presenters::room_shell::find_messages(conn,486777696,None)?;
            crate::controllers::presenters::room_shell::unread_divider(conn,&membership,&messages)
        }).await.unwrap();
        assert_eq!(result.message_id,case["divider"].as_i64(),"{name}");
        assert_eq!(result.count,case["count"].as_i64().unwrap(),"{name}");
        assert_eq!(result.scroll,case["scroll"].as_bool(),"{name}");
        assert_eq!(result.jump_url.as_deref(),case["jump_url"].as_str(),"{name}");
    }
}
