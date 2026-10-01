use super::call_channel_tests::configured;
use crate::controllers::presenters::{
    sql_probe::SqlProbe,
    test_support::{DAVID, JASON, KEVIN, Req, TestApp},
};
use axum::http::{Method, StatusCode};
use campfire_db::{Room, RoomType};

fn oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/huddle_review_fixes.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn review_audit_failure_preserves_committed_call_room_deletion() {
    let test = TestApp::boot()
        .await
        .expect("seed")
        .without_job_runner()
        .await;
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Review stage"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )
        })
        .await
        .unwrap();
    test.db().write(|tx| {
        tx.conn().execute_batch("CREATE TRIGGER ws13_reject_room_audit BEFORE INSERT ON audit_logs WHEN NEW.action='room.destroy' BEGIN SELECT RAISE(ABORT,'WS13 rejected audit'); END")?;
        Ok(())
    }).await.unwrap();
    let response = test
        .david()
        .write(
            Req::new(Method::DELETE, &format!("/rooms/{}", room.id))
                .header("accept", "application/json"),
        )
        .await;
    assert_eq!(
        response.status.as_u16(),
        oracle()["audit_failure"]["status"].as_u64().unwrap() as u16
    );
    let state = test.db().read(move |conn| Ok(serde_json::json!({"deleted": Room::find(conn,room.id)?.deleted(), "memberships": Room::find(conn,room.id)?.user_ids(conn)?.len()}))).await.unwrap();
    assert_eq!(state["deleted"], oracle()["audit_failure"]["deleted"]);
    assert_eq!(
        state["memberships"],
        oracle()["audit_failure"]["memberships"]
    );
}

#[tokio::test]
async fn review_unconfigured_stage_keeps_navigation_dialog_and_roster() {
    let test = TestApp::boot().await.expect("seed");
    let room = test
        .db()
        .write(|tx| {
            Room::create_for(
                tx,
                RoomType::Stage,
                Some("Review stage"),
                DAVID,
                &[DAVID, JASON, KEVIN],
            )
        })
        .await
        .unwrap();
    let response = test.david().get(&format!("/rooms/{}", room.id)).await;
    let text = response.text();
    let facts = serde_json::json!({"status": response.status.as_u16(), "show_stage": text.contains("aria-label=\"Show stage\""), "dialog": text.contains(&format!("id=\"stage_panel_dialog_rooms_stage_{}\"",room.id)), "roster": text.contains(&format!("id=\"stage_roster_rooms_stage_{}\"",room.id)), "media_launcher": text.contains("data-controller=\"huddle-launcher\"")});
    assert_eq!(facts, oracle()["unconfigured_stage"]);
}

#[tokio::test]
async fn review_configured_navigation_keeps_every_room_header_identity() {
    let test = TestApp::boot_with_huddle(configured()).await.expect("seed");
    for (i, kind) in [
        RoomType::Open,
        RoomType::Closed,
        RoomType::Direct,
        RoomType::Voice,
        RoomType::Stage,
        RoomType::Board,
    ]
    .into_iter()
    .enumerate()
    {
        let room = test
            .db()
            .write(move |tx| {
                Room::create_for(
                    tx,
                    kind,
                    Some("Review identity"),
                    DAVID,
                    &[DAVID, JASON, KEVIN],
                )
            })
            .await
            .unwrap();
        let response = test.david().get(&format!("/rooms/{}", room.id)).await;
        assert_eq!(response.status, StatusCode::OK);
        let expected = &oracle()["identities"][i];
        assert!(
            response.text().contains(&format!(
                "<span class=\"room-header__kind\">{}</span>",
                expected["label"].as_str().unwrap()
            )),
            "{kind:?}: {}",
            response.text()
        );
        assert!(
            response.text().contains(&format!(
                "id=\"{}\"",
                expected["header_id"]
                    .as_str()
                    .unwrap()
                    .replace(":id", &room.id.to_string())
            )),
            "{kind:?}"
        );
    }
}

#[tokio::test]
async fn review_sidebar_batches_call_reads_for_one_hundred_quiet_rooms() {
    let test = TestApp::boot_with_huddle(configured())
        .await
        .expect("seed")
        .without_job_runner()
        .await;
    test.db().write(|tx| {
        for i in 0..100 {
            let id = 9800+i;
            tx.conn().execute("INSERT INTO rooms(id,type,name,creator_id,created_at,updated_at) VALUES (?,'Rooms::Closed',?, ?,?,?)",rusqlite::params![id,format!("Quiet {i}"),DAVID,tx.now(),tx.now()])?;
            tx.conn().execute("INSERT INTO memberships(room_id,user_id,created_at,updated_at) VALUES (?,?,?,?)",rusqlite::params![id,DAVID,tx.now(),tx.now()])?;
        }
        Ok(())
    }).await.unwrap();
    let mut browser = test.david();
    let probe = SqlProbe::start(test.db(), test.booted.app.config.db_readers, None).await;
    let response = browser.get("/users/me/sidebar").await;
    let statements = probe.finish().await;
    assert_eq!(response.status, StatusCode::OK);
    assert!(response.text().contains("Quiet 99"));
    let counts = serde_json::json!({"participant_queries":statements.iter().filter(|s|s.sql.contains("FROM huddle_grants")||s.sql.contains("JOIN huddle_grants")).count(), "stream_queries":statements.iter().filter(|s|s.sql.contains("FROM streams")).count()});
    for key in ["participant_queries", "stream_queries"] {
        assert!(
            counts[key].as_u64().unwrap() <= oracle()["quiet_sidebar"][key].as_u64().unwrap(),
            "{counts:?}"
        );
    }
}
