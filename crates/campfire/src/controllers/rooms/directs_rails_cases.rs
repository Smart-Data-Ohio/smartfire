//! Individually executed ports of every directs_controller_test.rb declaration.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Message, NewUser, Room, RoomType, User};
const JZ: i64 = 773523953;
const DIRECT_DAVID_KEVIN: i64 = 699448325;
const DESIGNERS: i64 = 654632876;

#[tokio::test]
async fn a_member_can_rename_the_group_and_everyone_sees_the_compact_system_note() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    let reply = app.david().write(Req::new(Method::PATCH, &format!("/rooms/directs/{id}")).form(&[("room[name]", "Weekend Plans")])).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(note(&app, id).await, "renamed the group to Weekend Plans");
}


#[tokio::test]
async fn group_dm_notes_cannot_be_edited_or_deleted() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    app.db().write(move |tx| Room::find(tx.conn(), id)?.rename_direct(tx, "Weekend Plans", DAVID)).await.unwrap();
    let note = app.db().read(move |conn| Ok(Message::for_room(conn, id)?.into_iter().filter(|m| m.system_note).max_by_key(|m| m.id).unwrap())).await.unwrap();
    let note_id = note.id;
    let body_before = app.db().read(move |conn| Message::find(conn, note_id)?.body_html(conn)).await.unwrap();
    let count_before = app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)).await.unwrap();
    let reply = app.david().write(Req::new(Method::PATCH, &format!("/rooms/{id}/messages/{note_id}.json")).form(&[("message[markdown_source]", "Edited")])).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    let reply = app.david().write(Req::new(Method::DELETE, &format!("/rooms/{id}/messages/{note_id}.turbo_stream"))).await;
    assert_eq!(reply.status, StatusCode::FORBIDDEN);
    assert_eq!(app.db().read(move |conn| Message::find(conn, note_id)?.body_html(conn)).await.unwrap(), body_before);
    assert_eq!(app.db().read(|conn| Ok(conn.query_row("SELECT COUNT(*) FROM messages", [], |r| r.get::<_, i64>(0))?)).await.unwrap(), count_before);
}
async fn setup() -> TestApp {
    TestApp::boot_frozen().await.expect("seed required")
}
pub(super) async fn group(app: &TestApp, ids: &[i64], creator: i64) -> i64 {
    let ids = ids.to_vec();
    app.db()
        .write(move |tx| {
            let room = Room::create(tx, RoomType::Direct, None, creator)?;
            room.grant_to(tx, &ids)?;
            Ok(room.id)
        })
        .await
        .unwrap()
}
pub(super) async fn ids(app: &TestApp, id: i64) -> Vec<i64> {
    let mut ids = app
        .db()
        .read(move |conn| Room::find(conn, id)?.user_ids(conn))
        .await
        .unwrap();
    ids.sort();
    ids
}
async fn count(app: &TestApp) -> i64 {
    app.db()
        .read(|conn| {
            Ok(conn.query_row_cached(
                "SELECT count(*) FROM rooms WHERE type='Rooms::Direct'",
                [],
                |r| r.get(0),
            )?)
        })
        .await
        .unwrap()
}
async fn extra_people(app: &TestApp, n: usize) -> Vec<i64> {
    app.db()
        .write(move |tx| {
            (0..n)
                .map(|i| {
                    User::create(
                        tx,
                        NewUser {
                            name: format!("Extra {i}"),
                            email_address: Some(format!("extra{i}@example.test")),
                            ..Default::default()
                        },
                    )
                    .map(|u| u.id)
                })
                .collect()
        })
        .await
        .unwrap()
}
fn create(ids: &[i64]) -> Req {
    Req::new(Method::POST, "/rooms/directs")
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"user_ids":ids})).unwrap())
}
fn add(id: i64, ids: &[i64]) -> Req {
    Req::new(Method::POST, &format!("/rooms/directs/{id}/add_members"))
        .header("content-type", "application/json")
        .body(serde_json::to_vec(&serde_json::json!({"user_ids":ids})).unwrap())
}
fn room_id(reply: &Reply) -> i64 {
    reply
        .location()
        .unwrap()
        .split('?')
        .next()
        .unwrap()
        .rsplit('/')
        .next()
        .unwrap()
        .parse()
        .unwrap()
}
fn root(reply: &Reply) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(reply.location(), Some("http://campfire.test/"));
}
fn edit(reply: &Reply, id: i64) {
    assert_eq!(reply.status, StatusCode::FOUND);
    assert_eq!(
        reply.location(),
        Some(format!("http://campfire.test/rooms/directs/{id}/edit").as_str())
    );
}
pub(super) async fn note(app: &TestApp, id: i64) -> String {
    let renderer = app.db().env().rich_text.clone();
    app.db()
        .read(move |conn| {
            Message::for_room(conn, id)?
                .iter()
                .rev()
                .find(|m| m.system_note)
                .unwrap()
                .plain_text_body(conn, renderer.as_ref())
        })
        .await
        .unwrap()
}
pub(super) async fn pending_destroy(app: &TestApp, id: i64) {
    app.db().read(move|conn| {
        let room=Room::find(conn,id)?;
        assert!(room.deleted_at.is_some()); assert!(room.destroy_enqueued_at.is_some());
        assert!(room.user_ids(conn)?.is_empty());
        let queued:bool=conn.query_row_cached("SELECT EXISTS(SELECT 1 FROM background_jobs WHERE job_class='Room::DestroyJob' AND arguments LIKE ?)",[format!("%{id}%")],|r|r.get(0))?;
        assert!(queued,"the deletion must have a durable job"); Ok(())
    }).await.unwrap();
}
async fn lower_writer_query_limit(app: &TestApp) {
    // Real SQLite rejects any uncapped IN list before execution. The controller's writer
    // performs the User.where query, so this proves the bound at the database boundary.
    app.db()
        .write(|tx| {
            tx.conn()
                .set_limit(rusqlite::limits::Limit::SQLITE_LIMIT_VARIABLE_NUMBER, 11)?;
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn create_case() {
    let app = setup().await;
    let reply = app.david().write(create(&[JZ])).await;
    assert_eq!(reply.status, StatusCode::FOUND);
    let id = room_id(&reply);
    let mut expected = vec![DAVID, JZ];
    expected.sort();
    assert_eq!(ids(&app, id).await, expected);
}
#[tokio::test]
async fn create_only_once_per_user_set() {
    let app = setup().await;
    let before = count(&app).await;
    let mut david = app.david();
    let first = david.write(create(&[JZ])).await;
    let second = david.write(create(&[JZ])).await;
    assert_eq!(first.location(), second.location());
    assert_eq!(count(&app).await - before, 1);
}
#[tokio::test]
async fn create_opens_a_group_dm_for_several_people_and_reuses_it() {
    let app = setup().await;
    let before = count(&app).await;
    let mut david = app.david();
    let reply = david.write(create(&[JASON, KEVIN])).await;
    let id = room_id(&reply);
    let mut expected = vec![DAVID, JASON, KEVIN];
    expected.sort();
    assert_eq!(ids(&app, id).await, expected);
    assert_eq!(count(&app).await - before, 1);
    let second = david.write(create(&[KEVIN, JASON])).await;
    assert_eq!(second.location(), reply.location());
    assert_eq!(count(&app).await - before, 1);
}
#[tokio::test]
async fn create_rejects_more_members_than_the_cap() {
    let app = setup().await;
    let people = extra_people(&app, 10).await;
    let before = count(&app).await;
    let reply = app.david().write(create(&people)).await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/rooms/directs/new")
    );
    assert_eq!(count(&app).await, before);
    let flash = super::direct_selection_tests::next_flash(&app, &reply, &mut None);
    assert!(flash["alert"].as_str().unwrap().contains("at most"));
}
#[tokio::test]
async fn create_with_start_huddle_lands_in_the_room_ready_to_ring() {
    let app = setup().await;
    let reply = app
        .david()
        .write(
            create(&[JASON, KEVIN])
                .header("content-type", "application/json")
                .body(
                    serde_json::to_vec(
                        &serde_json::json!({"user_ids":[JASON,KEVIN],"start_huddle":"1"}),
                    )
                    .unwrap(),
                ),
        )
        .await;
    assert_eq!(reply.status, StatusCode::FOUND);
    assert!(reply.location().unwrap().ends_with("?huddle=start"));
}
#[tokio::test]
async fn create_ignores_deactivated_and_banned_users() {
    let app = setup().await;
    app.db()
        .write(|tx| {
            tx.conn()
                .execute_cached("UPDATE users SET status=1 WHERE id=?", [KEVIN])?;
            tx.conn()
                .execute_cached("UPDATE users SET status=2 WHERE id=?", [JZ])?;
            Ok(())
        })
        .await
        .unwrap();
    let before = count(&app).await;
    let reply = app.david().write(create(&[JASON, KEVIN, JZ])).await;
    assert_eq!(room_id(&reply), DIRECT_DAVID_JASON);
    assert_eq!(count(&app).await, before);
    let mut expected = vec![DAVID, JASON];
    expected.sort();
    assert_eq!(ids(&app, room_id(&reply)).await, expected);
}
#[tokio::test]
async fn create_caps_user_ids_before_querying() {
    let app = setup().await;
    let people = extra_people(&app, 50).await;
    let before = count(&app).await;
    lower_writer_query_limit(&app).await;
    let reply = app.david().write(create(&people)).await;
    assert_eq!(
        reply.location(),
        Some("http://campfire.test/rooms/directs/new")
    );
    assert_eq!(count(&app).await, before);
}
#[tokio::test]
async fn rename_is_rejected_for_one_to_one_dms_and_by_non_members() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    let rename = |id| {
        Req::new(Method::PATCH, &format!("/rooms/directs/{id}")).form(&[("room[name]", "Sneaky")])
    };
    edit(
        &app.david().write(rename(DIRECT_DAVID_JASON)).await,
        DIRECT_DAVID_JASON,
    );
    root(&app.sign_in(JZ).await.write(rename(id)).await);
    app.db()
        .read(move |conn| {
            assert!(Room::find(conn, id)?.name.is_none());
            assert!(Room::find(conn, DIRECT_DAVID_JASON)?.name.is_none());
            Ok(())
        })
        .await
        .unwrap();
}
#[tokio::test]
async fn a_member_can_add_people_up_to_the_cap() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    edit(&app.david().write(add(id, &[JZ])).await, id);
    assert!(ids(&app, id).await.contains(&JZ));
    assert_eq!(note(&app, id).await, "added JZ to the group");
}
#[tokio::test]
async fn adding_members_rejects_one_to_one_dms_the_overflow_and_non_members() {
    let app = setup().await;
    edit(
        &app.david().write(add(DIRECT_DAVID_JASON, &[KEVIN])).await,
        DIRECT_DAVID_JASON,
    );
    assert!(!ids(&app, DIRECT_DAVID_JASON).await.contains(&KEVIN));
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    let people = extra_people(&app, 8).await;
    edit(&app.david().write(add(id, &people[..7])).await, id);
    let reply = app.david().write(add(id, &people[7..])).await;
    edit(&reply, id);
    assert!(
        super::direct_selection_tests::next_flash(&app, &reply, &mut None)["alert"]
            .as_str()
            .unwrap()
            .contains("at most")
    );
    assert!(!ids(&app, id).await.contains(&people[7]));
    root(&app.sign_in(JZ).await.write(add(id, &[KEVIN])).await);
}
#[tokio::test]
async fn adding_members_caps_user_ids_before_querying() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    let people = extra_people(&app, 50).await;
    lower_writer_query_limit(&app).await;
    let reply = app.david().write(add(id, &people)).await;
    edit(&reply, id);
    assert_eq!(ids(&app, id).await.len(), 3);
}
#[tokio::test]
async fn leaving_removes_only_your_membership_and_the_group_keeps_working() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    let mut david = app.david();
    root(
        &david
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{id}/leave"),
            ))
            .await,
    );
    assert!(!ids(&app, id).await.contains(&DAVID));
    assert_eq!(note(&app, id).await, "left the group");
    root(&david.classic_page(&format!("/rooms/{id}")).await);
    assert_eq!(
        app.sign_in(JASON)
            .await
            .classic_page(&format!("/rooms/{id}"))
            .await
            .status,
        StatusCode::FOUND
    );
}
#[tokio::test]
async fn the_last_member_out_destroys_the_group() {
    let app = setup().await.without_job_runner().await;
    let id = group(&app, &[JASON], JASON).await;
    root(
        &app.sign_in(JASON)
            .await
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{id}/leave"),
            ))
            .await,
    );
    pending_destroy(&app, id).await;
}
#[tokio::test]
async fn leave_is_rejected_for_non_members() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    root(
        &app.sign_in(JZ)
            .await
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{id}/leave"),
            ))
            .await,
    );
    assert_eq!(ids(&app, id).await.len(), 3);
}
#[tokio::test]
async fn a_non_member_cannot_read_the_group() {
    let app = setup().await;
    let id = group(&app, &[JASON, KEVIN, JZ], JASON).await;
    root(&app.david().classic_page(&format!("/rooms/{id}")).await);
}
#[tokio::test]
async fn a_removed_member_loses_access_to_the_group() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    app.db()
        .write(move |tx| Room::find(tx.conn(), id)?.leave_direct(tx, DAVID))
        .await
        .unwrap();
    root(&app.david().classic_page(&format!("/rooms/{id}")).await);
}
#[tokio::test]
async fn destroy_only_allowed_for_all_room_users() {
    let app = setup().await.without_job_runner().await;
    root(
        &app.sign_in(KEVIN)
            .await
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{DIRECT_DAVID_KEVIN}"),
            ))
            .await,
    );
    pending_destroy(&app, DIRECT_DAVID_KEVIN).await;
    campfire_db::models::room_delete::perform_with_config(
        app.db(),
        DIRECT_DAVID_KEVIN,
        Default::default(),
    )
    .await
    .unwrap();
    assert!(
        app.db()
            .read(|conn| Room::find_by_id(conn, DIRECT_DAVID_KEVIN))
            .await
            .unwrap()
            .is_none()
    );
}
#[tokio::test]
async fn destroy_cant_reach_a_closed_room_the_member_didnt_create() {
    let app = setup().await;
    let before = count(&app).await;
    root(
        &app.sign_in(KEVIN)
            .await
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{DESIGNERS}"),
            ))
            .await,
    );
    assert_eq!(count(&app).await, before);
    assert!(
        app.db()
            .read(|conn| Ok(Room::find(conn, DESIGNERS)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn destroy_cant_reach_an_open_room_the_member_didnt_create() {
    let app = setup().await;
    root(
        &app.sign_in(KEVIN)
            .await
            .write(Req::new(Method::DELETE, &format!("/rooms/directs/{HQ}")))
            .await,
    );
    assert!(
        app.db()
            .read(|conn| Ok(Room::find(conn, HQ)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn destroy_cant_reach_a_room_the_member_isnt_in_at_all() {
    let app = setup().await;
    root(
        &app.sign_in(JZ)
            .await
            .write(Req::new(
                Method::DELETE,
                &format!("/rooms/directs/{DIRECT_DAVID_KEVIN}"),
            ))
            .await,
    );
    assert!(
        app.db()
            .read(|conn| Ok(Room::find(conn, DIRECT_DAVID_KEVIN)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn a_member_cannot_delete_a_group_dm() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .write(Req::new(Method::DELETE, &format!("/rooms/directs/{id}")))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(ids(&app, id).await.len(), 3);
}
#[tokio::test]
async fn the_non_admin_creator_cannot_delete_a_group_dm_through_the_generic_room_route() {
    let app = setup().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], KEVIN).await;
    assert_eq!(
        app.sign_in(KEVIN)
            .await
            .write(Req::new(Method::DELETE, &format!("/rooms/{id}")))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert!(
        app.db()
            .read(move |conn| Ok(Room::find(conn, id)?.deleted_at.is_none()))
            .await
            .unwrap()
    );
}
#[tokio::test]
async fn an_administrator_can_delete_a_group_dm() {
    let app = setup().await.without_job_runner().await;
    let id = group(&app, &[DAVID, JASON, KEVIN], DAVID).await;
    root(
        &app.david()
            .write(Req::new(Method::DELETE, &format!("/rooms/directs/{id}")))
            .await,
    );
    pending_destroy(&app, id).await;
}
