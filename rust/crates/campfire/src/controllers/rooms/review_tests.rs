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

fn recheck_oracle() -> serde_json::Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/huddle_review_recheck.json"
    ))
    .unwrap()
}

#[tokio::test]
async fn review_dm_peers_do_not_grow_total_sidebar_selects() {
    use campfire_db::{NewUser, User};
    let test = TestApp::boot_with_huddle(configured())
        .await
        .expect("seed")
        .without_job_runner()
        .await;
    let mut browser = test.david();
    assert_eq!(
        browser.get("/users/me/sidebar").await.status,
        StatusCode::OK
    );
    let mut counts = Vec::new();
    for added in [false, true] {
        if added {
            test.db()
                .write(|tx| {
                    for i in 0..5 {
                        let peer = User::create(
                            tx,
                            NewUser {
                                name: format!("Review peer {i}"),
                                email_address: Some(format!("ws13-review-peer-{i}@example.test")),
                                ..Default::default()
                            },
                        )?;
                        Room::create_for(
                            tx,
                            RoomType::Direct,
                            Some(&format!("Review group {i}")),
                            DAVID,
                            &[DAVID, KEVIN, peer.id],
                        )?;
                    }
                    Ok(())
                })
                .await
                .unwrap();
        }
        let probe = SqlProbe::start(test.db(), test.booted.app.config.db_readers, None).await;
        let response = browser.get("/users/me/sidebar").await;
        let statements = probe.finish().await;
        assert_eq!(response.status, StatusCode::OK);
        if added {
            assert!(response.text().contains("Review group 4"));
        }
        counts.push(
            statements
                .iter()
                .filter(|s| s.sql.trim_start().to_uppercase().starts_with("SELECT"))
                .count(),
        );
    }
    let rails = &recheck_oracle()["direct_sidebar"];
    println!(
        "DM sidebar total SELECTs: Rust {} -> {}; Rails {} -> {} after five new peers/group DMs",
        counts[0], counts[1], rails["before_selects"], rails["after_selects"]
    );
    assert_eq!(
        counts[1] as i64 - counts[0] as i64,
        rails["after_selects"].as_i64().unwrap() - rails["before_selects"].as_i64().unwrap(),
        "total query growth must match Rails, including user reads (Astra full-request measurement: 20 -> 20; independent recorded sidebar-frame measurement below)"
    );
}

#[tokio::test]
async fn review_board_navigation_matches_rails_with_huddles_configured_or_not() {
    let oracle = recheck_oracle();
    for config in [crate::huddle::Config::default(), configured()] {
        let test = TestApp::boot_with_huddle(config).await.expect("seed");
        let room_id = oracle["board"]["room_id"].as_i64().unwrap();
        let response = test.david().get(&format!("/rooms/{room_id}")).await;
        assert_eq!(
            response.status.as_u16(),
            oracle["board"]["status"].as_u64().unwrap() as u16
        );
        let text = response.text();
        // Compare the complete nav contributed by the actual HTTP page, including
        // the overflow menu and notification frame; only shared asset digests vary.
        let start = text.find("  <div class=\"room-header__identity").unwrap();
        let end = start + text[start..].find("<div id=\"global-search\"").unwrap();
        let region = &text[start..end];
        let close = region.rfind("  </div>\n").unwrap() + "  </div>\n".len();
        let actual = &region[..close];
        let expected = oracle["board"]["nav_html"].as_str().unwrap();
        assert!(
            crate::app::asset_goldens::compare("board-navigation", actual, expected),
            "Board must retain Rails' separate navigation"
        );
    }
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
