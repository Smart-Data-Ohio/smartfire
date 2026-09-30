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
