//! `DELETE /rooms/boards/:id` deletes the board the way `DELETE /rooms/:id` does.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{Room, RoomType};
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
// All fields and rows are compared, rather than only row counts or soft-delete flags.
const UNCHANGED_TABLES: &[&str] = &[
    "rooms",
    "memberships",
    "messages",
    "channel_threads",
    "audit_logs",
    "background_jobs",
    "active_storage_attachments",
    "active_storage_blobs",
    "active_storage_variant_records",
    "activity_items",
    "board_tag_assignments",
    "board_sla_rules",
    "board_sla_nudges",
    "board_stale_digests",
    "boosts",
];
type DomainSnapshot = Vec<Vec<Vec<rusqlite::types::Value>>>;

async fn snapshot(app: &TestApp) -> DomainSnapshot {
    app.db()
        .read(|conn| {
            UNCHANGED_TABLES
                .iter()
                .map(|table| {
                    let mut statement =
                        conn.prepare(&format!("SELECT * FROM {table} ORDER BY id"))?;
                    let columns = statement.column_count();
                    Ok(statement
                        .query_map([], |row| {
                            (0..columns)
                                .map(|index| row.get(index))
                                .collect::<rusqlite::Result<Vec<_>>>()
                        })?
                        .collect::<rusqlite::Result<Vec<_>>>()?)
                })
                .collect::<campfire_db::Result<_>>()
        })
        .await
        .unwrap()
}

async fn board_route(name: &str) {
    let oracle: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../vectors/board_destroy.json")).unwrap();
    assert_eq!(oracle["cases"].as_array().unwrap().len(), 7);
    let case = oracle["cases"]
        .as_array()
        .unwrap()
        .iter()
        .find(|case| case["name"] == name)
        .unwrap();
    let app = app().await;
    let id = case["id"].as_i64().unwrap();
    let creator = case["creator"].as_i64().unwrap();
    let members: Vec<i64> = case["members"]
        .as_array()
        .unwrap()
        .iter()
        .map(|id| id.as_i64().unwrap())
        .collect();
    let kind = if name == "wrong_type" {
        "Rooms::Closed"
    } else {
        "Rooms::Board"
    };
    app.db().write(move |tx| {
        tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES(?1,?2,'Launch',?3,?4,?4)", rusqlite::params![id,kind,creator,tx.now()])?;
        Room::find(tx.conn(),id)?.grant_to(tx,&members)?;
        Ok(())
    }).await.unwrap();
    let mut browser = app.sign_in(case["actor"].as_i64().unwrap()).await;
    let token = browser.authenticity_token().await;
    let accept = match case["format"].as_str().unwrap() {
        "json" => "application/json",
        "turbo_stream" => "text/vnd.turbo-stream.html, text/html",
        _ => "text/html",
    };
    let reply = browser
        .send(
            Req::new(Method::DELETE, case["path"].as_str().unwrap())
                .header("accept", accept)
                .header(campfire_kit::csrf::HEADER, &token),
        )
        .await;
    let deleted = matches!(name, "admin_json" | "creator_html" | "admin_turbo");
    match name {
        "admin_json" => {
            assert_eq!(reply.status, StatusCode::OK, "{name}");
            assert_eq!(reply.json()["deleted"], true);
            assert_eq!(reply.json()["room_id"], id);
        }
        "creator_html" | "admin_turbo" => {
            assert_eq!(reply.status, StatusCode::FOUND, "{name}");
            assert_eq!(reply.location(), Some("http://campfire.test/"), "{name}");
        }
        "forbidden_member" => assert_eq!(reply.status, StatusCode::FORBIDDEN, "{name}"),
        "inaccessible_admin" | "missing" | "wrong_type" => {
            assert_eq!(reply.status, StatusCode::FOUND, "{name}");
            assert_eq!(reply.location(), Some("http://campfire.test/"), "{name}");
        }
        other => panic!("unknown board destroy case {other}"),
    }
    if name != "missing" {
        let marked = app
            .db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_some()))
            .await
            .unwrap();
        assert_eq!(marked, deleted, "{name}");
    }
}

macro_rules! board_route_case {
    ($name:ident) => {
        #[tokio::test]
        async fn $name() {
            board_route(stringify!($name)).await;
        }
    };
}
board_route_case!(admin_json);
board_route_case!(creator_html);
board_route_case!(admin_turbo);
board_route_case!(forbidden_member);
board_route_case!(inaccessible_admin);
board_route_case!(missing);
board_route_case!(wrong_type);

#[tokio::test]
async fn board_destroy_still_requires_session_and_csrf_before_the_reference_error() {
    let app = app().await;
    let id = board(&app, DAVID, &[DAVID, JZ]).await;
    let before = snapshot(&app).await;
    assert_eq!(
        app.anonymous().send(delete(id, ".json")).await.location(),
        Some("http://campfire.test/session/new")
    );
    assert_eq!(
        app.david().send(delete(id, ".json")).await.status,
        StatusCode::UNPROCESSABLE_ENTITY
    );
    assert_eq!(snapshot(&app).await, before);
}

#[tokio::test]
async fn boards_route_deletes_posts_tags_and_work() {
    let app = app().await;
    let id = board(&app, DAVID, &[DAVID, JZ]).await;
    let (thread_id, message_id) = app.db().write(move |tx| {
        let thread = campfire_db::ChannelThread::create(tx, campfire_db::NewChannelThread {
            room_id: id,
            creator_id: DAVID,
            name: Some("Launch post".into()),
            work_status: Some("planned".into()),
            work_owner_id: Some(DAVID),
            tag_names: Some(vec!["Launch".into()]),
            ..Default::default()
        })?;
        let message = campfire_db::Message::create(tx, campfire_db::NewMessage {
            room_id: id,
            thread_id: Some(thread.id),
            board_post_opener: true,
            creator_id: DAVID,
            body: Some("The post".into()),
            ..Default::default()
        })?;
        tx.conn().execute(
            "INSERT INTO board_tag_assignments(room_id,tag,assignee_id,created_by_id,created_at,updated_at) VALUES(?1,'cleanup',?2,?2,?3,?3)",
            rusqlite::params![id, DAVID, tx.now()],
        )?;
        tx.conn().execute(
            "INSERT INTO work_thread_events(channel_thread_id,event_type,actor_id,to_status,created_at,updated_at) VALUES(?1,'status_changed',?2,'planned',?3,?3)",
            rusqlite::params![thread.id, DAVID, tx.now()],
        )?;
        Ok((thread.id, message.id))
    }).await.unwrap();
    fn counts(
        conn: &campfire_db::Connection,
        room_id: i64,
        thread_id: i64,
    ) -> campfire_db::Result<(i64, i64, i64, i64, i64)> {
        let n = |sql: &str, id: i64| conn.query_row(sql, [id], |row| row.get(0));
        Ok((
            n("SELECT count(*) FROM messages WHERE room_id=?", room_id)?,
            n(
                "SELECT count(*) FROM channel_threads WHERE room_id=?",
                room_id,
            )?,
            n(
                "SELECT count(*) FROM thread_tags WHERE channel_thread_id=?",
                thread_id,
            )?,
            n(
                "SELECT count(*) FROM work_thread_events WHERE channel_thread_id=?",
                thread_id,
            )?,
            n(
                "SELECT count(*) FROM board_tag_assignments WHERE room_id=?",
                room_id,
            )?,
        ))
    }
    let before = app
        .db()
        .read(move |conn| counts(conn, id, thread_id))
        .await
        .unwrap();
    assert_eq!(before, (1, 1, 1, 1, 1));
    let reply = app
        .david()
        .write(Req::new(
            Method::DELETE,
            &format!("/rooms/boards/{id}.json"),
        ))
        .await;
    assert_eq!(reply.status, StatusCode::OK, "{}", reply.text());
    assert_eq!(reply.json()["room_id"], id);
    super::directs_rails_cases::pending_destroy(&app, id).await;
    campfire_db::models::room_delete::perform(app.db(), id)
        .await
        .unwrap();
    app.db()
        .read(move |conn| {
            assert!(Room::find_by_id(conn, id)?.is_none());
            assert_eq!(counts(conn, id, thread_id)?, (0, 0, 0, 0, 0));
            let message: i64 = conn.query_row(
                "SELECT count(*) FROM messages WHERE id=?",
                [message_id],
                |row| row.get(0),
            )?;
            assert_eq!(message, 0);
            Ok(())
        })
        .await
        .unwrap();
}

#[tokio::test]
async fn base_board_destroy_durable_worker_cleans_board_rows_and_purges_attachment_files() {
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
        app.david()
            .write(Req::new(Method::DELETE, &format!("/rooms/{id}.json")))
            .await
            .status,
        StatusCode::OK
    );
    super::directs_rails_cases::pending_destroy(&app, id).await;
    // Start a new real durable runner after the request, consuming the committed job.
    let registry = crate::jobs::registry();
    let config = crate::queue::runner_config(&app.booted.app.config);
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
