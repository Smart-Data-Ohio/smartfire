//! Board-scoped ports of the inherited RoomsController destroy declarations.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Room, RoomType};
use rusqlite::OptionalExtension;
use serde_json::json;
const JZ: i64 = 773523953;

async fn app() -> TestApp {
    TestApp::boot_frozen()
        .await
        .unwrap()
        .without_job_runner()
        .await
}
async fn board(app: &TestApp, creator: i64, members: &[i64]) -> i64 {
    let members = members.to_vec();
    app.db()
        .write(move |tx| {
            Ok(Room::create_for(tx, RoomType::Board, Some("Launch"), creator, &members)?.id)
        })
        .await
        .unwrap()
}
fn delete(id: i64, format: &str) -> Req {
    Req::new(Method::DELETE, &format!("/rooms/boards/{id}{format}"))
}
async fn unchanged(app: &TestApp, id: i64, members: usize) {
    app.db().read(move |conn| {
        let room = Room::find(conn, id)?;
        assert!(room.deleted_at.is_none() && room.destroy_enqueued_at.is_none());
        assert_eq!(room.user_ids(conn)?.len(), members);
        let writes: i64 = conn.query_row("SELECT (SELECT count(*) FROM audit_logs WHERE action='room.destroy' AND target_id=?1) + (SELECT count(*) FROM background_jobs WHERE job_class='Room::DestroyJob' AND json_extract(arguments,'$.room_id')=?1)", [id], |r| r.get(0))?;
        assert_eq!(writes, 0);
        Ok(())
    }).await.unwrap();
}

#[tokio::test]
async fn board_destroy_matches_inherited_rails_action_bytes_state_and_broadcasts() {
    // The oracle records the pinned board-route callback crash explicitly, and exercises
    // the inherited action through /rooms/:id. It does not pretend Rails' board route works.
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/board_destroy.json")).unwrap();
    let app = app().await;
    for case in oracle["cases"].as_array().unwrap() {
        assert_eq!(case["board_route_status"], 500);
        let id = case["id"].as_i64().unwrap();
        let creator = case["creator"].as_i64().unwrap();
        let members: Vec<i64> = case["members"]
            .as_array()
            .unwrap()
            .iter()
            .map(|id| id.as_i64().unwrap())
            .collect();
        app.db().write(move|tx| {
            tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(?1,'Rooms::Board','Launch',?2,?3,?3)",rusqlite::params![id,creator,tx.now()])?;
            Room::find(tx.conn(),id)?.grant_to(tx,&members)?;
            Ok(())
        }).await.unwrap();
        let mut browser = app.sign_in(case["actor"].as_i64().unwrap()).await;
        let (mut cable, _server) =
            super::opens_rails_cases::stream_for(&app, &browser, &["rooms"]).await;
        let accept = match case["format"].as_str().unwrap() {
            "json" => "application/json",
            "turbo_stream" => "text/vnd.turbo-stream.html, text/html",
            _ => "text/html",
        };
        let reply = browser
            .write(
                Req::new(Method::DELETE, case["path"].as_str().unwrap()).header("accept", accept),
            )
            .await;
        assert_eq!(
            reply.status.as_u16() as u64,
            case["response"]["status"].as_u64().unwrap(),
            "{}",
            case["name"]
        );
        assert_eq!(
            reply.text(),
            case["response"]["body"].as_str().unwrap(),
            "{}",
            case["name"]
        );
        for (name, value) in case["response"]["headers"].as_object().unwrap() {
            assert_eq!(
                reply.header(name),
                value.as_str(),
                "{}: {name}",
                case["name"]
            );
        }
        assert_eq!(
            super::direct_selection_tests::next_flash(&app, &reply, &mut None),
            case["flash"]
        );
        let state = app.db().read(move|conn| {
            let room = Room::find(conn,id)?;
            let audit = conn.query_row("SELECT action,actor_id,target_type,target_id,target_label,details FROM audit_logs WHERE action='room.destroy' AND target_id=? ORDER BY id DESC LIMIT 1",[id],|r| Ok(json!({"action":r.get::<_,String>(0)?,"actor_id":r.get::<_,i64>(1)?,"target_type":r.get::<_,String>(2)?,"target_id":r.get::<_,i64>(3)?,"target_label":r.get::<_,String>(4)?,"details":serde_json::from_str::<serde_json::Value>(&r.get::<_,String>(5)?).unwrap()}))).optional()?;
            Ok(json!({"deleted":room.deleted_at.is_some(),"claimed":room.destroy_enqueued_at.is_some(),"memberships":room.user_ids(conn)?.len(),"audit":audit}))
        }).await.unwrap();
        assert_eq!(state, case["state"], "{}", case["name"]);
        let jobs = app.db().read(move|conn| {
            let mut statement = conn.prepare("SELECT job_class,arguments FROM background_jobs WHERE job_class='Room::DestroyJob' AND json_extract(arguments,'$.room_id')=? ORDER BY id")?;
            Ok(statement.query_map([id],|r| {
                let arguments: serde_json::Value = serde_json::from_str(&r.get::<_,String>(1)?).unwrap();
                Ok(json!([r.get::<_,String>(0)?,[arguments["room_id"]]]))
            })?.collect::<rusqlite::Result<Vec<_>>>()?)
        }).await.unwrap();
        assert_eq!(json!(jobs), case["jobs"], "{}", case["name"]);
        for broadcast in case["broadcasts"].as_array().unwrap() {
            assert_eq!(broadcast["broadcasting"], "rooms");
            assert_eq!(
                super::opens_rails_cases::frame(&mut cable).await,
                broadcast["message"].as_str().unwrap()
            );
        }
        cable.assert_silent().await;
    }
}

#[tokio::test]
async fn board_destroy_requires_creator_or_administrator_after_membership() {
    let app = app().await;
    let id = board(&app, DAVID, &[DAVID, JZ]).await;
    let reply = app.sign_in(JZ).await.write(delete(id, ".json")).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(reply.text(), "");
    unchanged(&app, id, 2).await;
    let inaccessible = board(&app, JZ, &[JZ]).await;
    let reply = app.david().write(delete(inaccessible, ".json")).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
    assert_eq!(
        super::direct_selection_tests::next_flash(&app, &reply, &mut None),
        json!({"alert":"Room not found or inaccessible"})
    );
    unchanged(&app, inaccessible, 1).await;
    let reply = app.david().write(delete(id, ".json")).await;
    assert_eq!(reply.status, StatusCode::OK);
    super::directs_rails_cases::pending_destroy(&app, id).await;
}

#[tokio::test]
async fn board_destroy_creator_html_and_turbo_redirect_flash_and_broadcast() {
    for accept in ["text/html", "text/vnd.turbo-stream.html, text/html"] {
        let app = app().await;
        let id = board(&app, JZ, &[DAVID, JZ]).await;
        let mut browser = app.sign_in(JZ).await;
        let (mut cable, _server) =
            super::opens_rails_cases::stream_for(&app, &browser, &["rooms"]).await;
        let reply = browser.write(delete(id, "").header("accept", accept)).await;
        assert_eq!(reply.status, StatusCode::FOUND, "{}", reply.text());
        assert_eq!(reply.location(), Some("http://campfire.test/"));
        assert_eq!(
            super::direct_selection_tests::next_flash(&app, &reply, &mut None),
            json!({"notice":"Deleted #Launch"})
        );
        super::directs_rails_cases::pending_destroy(&app, id).await;
        assert_eq!(
            super::opens_rails_cases::frame(&mut cable).await,
            format!(
                "<turbo-stream action=\"remove\" target=\"list_rooms_board_{id}\"></turbo-stream>"
            )
        );
        cable.assert_silent().await;
        let (actor,target,details): (i64,String,String) = app.db().read(move|conn| Ok(conn.query_row("SELECT actor_id,target_type,details FROM audit_logs WHERE action='room.destroy' AND target_id=?",[id],|r|Ok((r.get(0)?,r.get(1)?,r.get(2)?)))?)).await.unwrap();
        assert_eq!(actor, JZ);
        assert_eq!(target, "Room");
        assert_eq!(
            serde_json::from_str::<serde_json::Value>(&details).unwrap(),
            json!({"name":"Launch"})
        );
        assert_eq!(
            browser.get(&format!("/rooms/{id}")).await.location(),
            Some("http://campfire.test/")
        );
    }
}

#[tokio::test]
async fn board_destroy_scope_rejects_missing_deleted_and_other_room_types() {
    let app = app().await;
    for kind in [
        RoomType::Closed,
        RoomType::Open,
        RoomType::Direct,
        RoomType::Voice,
        RoomType::Stage,
    ] {
        let id = app
            .db()
            .write(move |tx| {
                let room = Room::create(tx, kind, Some("Other"), DAVID)?;
                room.grant_to(tx, &[DAVID])?;
                Ok(room.id)
            })
            .await
            .unwrap();
        let before_members = app
            .db()
            .read(move |conn| Ok(Room::find(conn, id)?.user_ids(conn)?.len()))
            .await
            .unwrap();
        let reply = app.david().write(delete(id, ".json")).await;
        assert_eq!(
            reply.location(),
            Some("http://campfire.test/"),
            "{kind:?}: {}",
            reply.text()
        );
        unchanged(&app, id, before_members).await;
    }
    assert_eq!(
        app.david()
            .write(delete(-1999999, ".json"))
            .await
            .location(),
        Some("http://campfire.test/")
    );
    let id = board(&app, DAVID, &[DAVID]).await;
    app.db()
        .write(move |tx| {
            tx.conn()
                .execute("UPDATE rooms SET deleted_at=? WHERE id=?", (tx.now(), id))?;
            Ok(())
        })
        .await
        .unwrap();
    assert_eq!(
        app.david().write(delete(id, ".json")).await.location(),
        Some("http://campfire.test/")
    );
}

#[tokio::test]
async fn board_destroy_requires_session_and_csrf_and_rejected_queue_rolls_back() {
    let app = app().await;
    let id = board(&app, DAVID, &[DAVID, JZ]).await;
    assert_eq!(
        app.anonymous().send(delete(id, ".json")).await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        app.david().send(delete(id, ".json")).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    unchanged(&app, id, 2).await;
    app.db().write(|tx| {tx.conn().execute_batch("CREATE TRIGGER reject_board_destroy BEFORE INSERT ON background_jobs WHEN NEW.job_class='Room::DestroyJob' BEGIN SELECT RAISE(ABORT,'injected board queue failure'); END;")?;Ok(())}).await.unwrap();
    assert_eq!(
        app.david().write(delete(id, ".json")).await.status,
        StatusCode::INTERNAL_SERVER_ERROR
    );
    unchanged(&app, id, 2).await;
}

#[tokio::test]
async fn board_destroy_durable_worker_cleans_board_rows_and_purges_attachment_files() {
    let app = app().await;
    // The seed intentionally includes a message whose author has already been deleted.
    // Compare the complete FK result before/after, so cleanup introduces no orphan rows.
    let before_foreign_keys = app.db().read(foreign_keys).await.unwrap();
    let id = board(&app, DAVID, &[DAVID, JZ]).await;
    let storage = app.booted.app.storage.clone();
    let blob = app.db().write(move |tx| {
        let thread = campfire_db::ChannelThread::create(tx,campfire_db::NewChannelThread {room_id:id,creator_id:DAVID,name:Some("Cleanup".into()),work_status:Some("planned".into()),..Default::default()})?;
        let message = campfire_db::Message::create(tx,campfire_db::NewMessage {room_id:id,thread_id:Some(thread.id),board_post_opener:true,creator_id:DAVID,body:Some("Cleanup".into()),..Default::default()})?;
        let blob = storage.create_and_upload(tx.conn(),b"board attachment",campfire_storage::Filename::new("cleanup.txt"),None,tx.now().jiff()).map_err(|e|campfire_db::Error::Other(e.to_string()))?;
        campfire_db::Attachment::create(tx,"Message",message.id,"attachment",blob.id)?;
        tx.conn().execute("INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(?1,'cleanup',?2,?2,?3,?3)",rusqlite::params![id,DAVID,tx.now()])?;
        tx.conn().execute("INSERT INTO board_sla_rules(room_id,work_status,nudge_after_minutes,escalate_after_minutes,created_at,updated_at) VALUES(?1,'planned',10,20,?2,?2)",rusqlite::params![id,tx.now()])?;
        tx.conn().execute("INSERT INTO board_sla_nudges(room_id,channel_thread_id,recipient_id,work_status,stage,status_entered_at,created_at,updated_at) VALUES(?1,?2,?3,'planned','nudge',?4,?4,?4)",rusqlite::params![id,thread.id,DAVID,tx.now()])?;
        tx.conn().execute("INSERT INTO board_stale_digests(room_id,message_id,digest_on,created_at,updated_at) VALUES(?1,?2,'2026-03-02',?3,?3)",rusqlite::params![id,message.id,tx.now()])?;
        Ok(blob)
    }).await.unwrap();
    let path = app.booted.app.storage.path_for(&blob);
    assert!(path.exists());
    assert_eq!(
        app.david().write(delete(id, ".json")).await.status,
        StatusCode::OK
    );
    super::directs_rails_cases::pending_destroy(&app, id).await;
    // Start a new real durable runner after the request, consuming the committed job.
    let registry = crate::jobs::registry();
    let config = crate::jobs::runner_config(&app.booted.app.config);
    let queue = campfire_jobs::JobQueue::new(&registry, &config).unwrap();
    let runner = campfire_jobs::start(
        app.db().clone(),
        queue,
        registry,
        app.booted.app.clone(),
        config,
    );
    crate::test_support::eventually("board destroy and attachment purge", || async {
        app.db()
            .read(move |conn| {
                Ok(
                    conn.query_row("SELECT count(*) FROM rooms WHERE id=?", [id], |r| {
                        r.get::<_, i64>(0)
                    })? == 0,
                )
            })
            .await
            .unwrap()
            && !path.exists()
    })
    .await;
    runner.shutdown(crate::test_support::WAIT).await;
    app.db()
        .read(move |conn| {
            for table in [
                "messages",
                "channel_threads",
                "memberships",
                "board_tag_assignments",
                "board_sla_rules",
                "board_sla_nudges",
                "board_stale_digests",
            ] {
                let remaining: i64 = conn.query_row(
                    &format!("SELECT count(*) FROM {table} WHERE room_id=?"),
                    [id],
                    |r| r.get(0),
                )?;
                assert_eq!(remaining, 0, "{table}");
            }
            let attachments: i64 = conn.query_row(
                "SELECT count(*) FROM active_storage_attachments WHERE blob_id=?",
                [blob.id],
                |r| r.get(0),
            )?;
            let blobs: i64 = conn.query_row(
                "SELECT count(*) FROM active_storage_blobs WHERE id=?",
                [blob.id],
                |r| r.get(0),
            )?;
            assert_eq!((attachments, blobs), (0, 0));
            assert_eq!(foreign_keys(conn)?, before_foreign_keys);
            Ok(())
        })
        .await
        .unwrap();
}

fn foreign_keys(
    conn: &campfire_db::Connection,
) -> campfire_db::Result<Vec<(String, i64, String, i64)>> {
    let mut statement = conn.prepare("PRAGMA foreign_key_check")?;
    Ok(statement
        .query_map([], |row| {
            Ok((row.get(0)?, row.get(1)?, row.get(2)?, row.get(3)?))
        })?
        .collect::<rusqlite::Result<Vec<_>>>()?)
}
