//! Pinned Rails untrusted-name coercions, Unicode validation and group failure paths.
use crate::controllers::presenters::test_support::*;
use axum::http::{Method, StatusCode};
use campfire_db::{CachedStatements, Message, Room, RoomType};
fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!("../../../../../vectors/direct_rename.json")).unwrap()
}
async fn group(app: &TestApp) -> i64 {
    app.db()
        .write(|tx| {
            let room = Room::create(tx, RoomType::Direct, Some("Old"), DAVID)?;
            room.grant_to(tx, &[DAVID, JASON, KEVIN])?;
            Ok(room.id)
        })
        .await
        .unwrap()
}
async fn counts(app: &TestApp) -> (i64, i64) {
    app.db()
        .read(|conn| {
            Ok((
                conn.query_row_cached("SELECT count(*) FROM messages", [], |r| r.get(0))?,
                conn.query_row_cached("SELECT count(*) FROM audit_logs", [], |r| r.get(0))?,
            ))
        })
        .await
        .unwrap()
}
#[tokio::test]
async fn direct_rename_coercions_and_rejections_match_real_rails_requests() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let mut david = app.david();
    let mut cookie = None;
    for row in oracle()["renames"].as_array().unwrap() {
        let id = group(&app).await;
        let before = counts(&app).await;
        if row["status"] == 422 {
            app.db().write(|tx|{tx.conn().execute_batch("CREATE TRIGGER ws8br_reject_invalid_rename_enqueue BEFORE INSERT ON background_jobs BEGIN SELECT RAISE(ABORT, 'invalid rename enqueued a job'); END;")?;Ok(())}).await.unwrap();
        }
        let reply = david
            .write(
                Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                    .header("content-type", "application/json")
                    .header("Accept", "text/html")
                    .body(
                        serde_json::to_vec(&serde_json::json!({"room":{"name":row["input"]}}))
                            .unwrap(),
                    ),
            )
            .await;
        assert_eq!(
            reply.status.as_u16() as u64,
            row["status"].as_u64().unwrap(),
            "{}: {}",
            row["input"],
            reply.text()
        );
        assert_eq!(reply.location(), row["location"].as_str());
        if reply.status == StatusCode::FOUND {
            assert_eq!(super::direct_selection_tests::next_flash(&app, &reply, &mut cookie), row["next_flash"]);
            david.get("/app/").await;
        }
        let renderer = app.db().env().rich_text.clone();
        let (name, notes) = app
            .db()
            .read(move |conn| {
                Ok((
                    Room::find(conn, id)?.name,
                    Message::for_room(conn, id)?
                        .iter()
                        .filter(|m| m.system_note)
                        .map(|m| m.plain_text_body(conn, renderer.as_ref()))
                        .collect::<campfire_db::Result<Vec<_>>>()?,
                ))
            })
            .await
            .unwrap();
        assert_eq!(serde_json::json!(name), row["name"]);
        assert_eq!(serde_json::json!(notes), row["notes"]);
        let after = counts(&app).await;
        assert_eq!(
            serde_json::json!({"messages":after.0-before.0,"audits":after.1-before.1}),
            row["delta"]
        );
        if let Some(_value) = row["error_value"].as_str() {


            assert_eq!(after, before, "invalid rename must not write");
            app.db()
                .write(|tx| {
                    tx.conn()
                        .execute_batch("DROP TRIGGER ws8br_reject_invalid_rename_enqueue")?;
                    Ok(())
                })
                .await
                .unwrap();
        }
    }
}
#[tokio::test]
async fn named_pair_keeps_group_controls_and_cannot_be_deleted_by_a_plain_member() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = app
        .db()
        .write(|tx| {
            let room = Room::create(tx, RoomType::Direct, Some("Still a group"), KEVIN)?;
            room.grant_to(tx, &[DAVID, KEVIN])?;
            Ok(room.id)
        })
        .await
        .unwrap();
    let mut member = app.sign_in(KEVIN).await;
    let page = member
        .send(Req::new(Method::GET, &format!("/rooms/directs/{id}/edit")))
        .await;
    assert_eq!(page.status, StatusCode::FOUND);



    assert_eq!(
        member
            .write(Req::new(Method::DELETE, &format!("/rooms/directs/{id}")))
            .await
            .status,
        StatusCode::FORBIDDEN
    );
    assert_eq!(
        app.db()
            .read(move |conn| Room::find(conn, id)?.user_ids(conn))
            .await
            .unwrap()
            .len(),
        2
    );
}
#[tokio::test]
async fn removed_members_and_wrong_room_types_cannot_submit_invalid_names() {
    let app = TestApp::boot_frozen().await.expect("seed required");
    let id = group(&app).await;
    app.db()
        .write(move |tx| {
            Room::find(tx.conn(), id)?.revoke_from(tx, &[DAVID])?;
            Ok(())
        })
        .await
        .unwrap();
    let before = counts(&app).await;
    for id in [id, HQ, ALL_TALK] {
        let reply = app
            .david()
            .write(
                Req::new(Method::PATCH, &format!("/rooms/directs/{id}"))
                    .header("content-type", "application/json")
                    .body(br#"{"room":{"name":{"nested":"value"}}}"#.to_vec()),
            )
            .await;
        assert_eq!(reply.location(), Some("http://campfire.test/"));
    }
    assert_eq!(counts(&app).await, before);
}
