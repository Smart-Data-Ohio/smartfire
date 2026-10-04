//! Individual original sidebar huddle assertions through the real router and producers.
use crate::controllers::presenters::test_support::*;
use axum::http::StatusCode;
use campfire_db::huddle_grant::HuddleGrant;
use campfire_db::{Membership, Room, RoomType, Session, User, UserChanges};
use serde_json::Value;
const JZ: i64 = 773523953;
const ENV: &[(&str, &str)] = &[
    ("LIVEKIT_URL", "wss://huddle.example.test"),
    ("LIVEKIT_INTERNAL_URL", "ws://livekit.example.test:7880"),
    ("LIVEKIT_API_KEY", "fixture-api-key"),
    ("LIVEKIT_API_SECRET", "fixture-api-secret"),
    ("LIVEKIT_GATEWAY_SECRET", "fixture-gateway-secret"),
];
fn oracle() -> Value {
    serde_json::from_str(include_str!(
        "../../../../../vectors/sidebar_originals.json"
    ))
    .unwrap()
}
async fn setup(configured: bool) -> (TestApp, i64, i64) {
    let env = if configured { ENV } else { &ENV[..4] };
    let config = crate::huddle::Config::from_lookup(|name| {
        env.iter()
            .find(|(n, _)| *n == name)
            .map(|(_, v)| v.to_string())
    });
    let app = TestApp::boot_with_huddle_and_clock(
        config,
        std::sync::Arc::new(campfire_kit::FrozenClock::new(SEED_NOW.parse().unwrap())),
    )
    .await
    .expect("CI seed required")
    .without_job_runner()
    .await;
    let (board, group) = app
        .db()
        .write(|tx| {
            tx.conn().execute("DELETE FROM huddle_grants", [])?;
            let board = Room::create_for(tx, RoomType::Board, Some("Launch"), DAVID, &[DAVID, JZ])?;
            let group =
                Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, JASON, KEVIN])?;
            Ok((board.id, group.id))
        })
        .await
        .unwrap();
    assert_eq!(board, oracle()["rooms"]["board"].as_i64().unwrap());
    assert_eq!(group, oracle()["rooms"]["group"].as_i64().unwrap());
    (app, board, group)
}
async fn grant(app: &TestApp, user: i64, room: i64, seen: bool) -> i64 {
    let config = campfire_db::models::room_delete::HuddleConfig {
        api_secret: app.booted.app.config.huddle.api_secret.clone(),
        admin_configured: true,
    };
    app.db()
        .write(move |tx| {
            let session = Session::for_user(tx.conn(), user)?
                .into_iter()
                .next()
                .unwrap();
            let membership = Membership::find_by_room_and_user(tx.conn(), room, user)?.unwrap();
            let mut grant = HuddleGrant::issue(tx, session.id, membership.id, room, &config)?;
            if seen {
                grant.record_seen(tx)?;
            }
            Ok(grant.id)
        })
        .await
        .unwrap()
}
async fn compare_row(app: &TestApp, name: &str) {
    let v = oracle();
    let row = v["rows"]
        .as_array()
        .unwrap()
        .iter()
        .find(|r| r["name"] == name)
        .unwrap();
    let reply = app.david().get("/users/me/sidebar").await;
    assert_eq!(
        reply.status.as_u16(),
        row["response"]["status"].as_u64().unwrap() as u16,
        "{name}: HTTP status"
    );
    let id = row["response"]["room_id"].as_i64().unwrap();
    let kind = if name == "board" {
        "board"
    } else if name == "channel" || name == "quiet_channel" {
        "closed"
    } else {
        "direct"
    };
    super::people_tests::assert_http_fragment(
        &reply.text(),
        row["response"]["html"].as_str().unwrap(),
        if kind == "direct" { "div" } else { "a" },
        "id",
        &format!("list_rooms_{kind}_{id}"),
    );
}
#[tokio::test]
async fn original_channel_live_stack_names_count_and_attributes() {
    let (app, _, _) = setup(true).await;
    grant(&app, DAVID, ALL_TALK, true).await;
    grant(&app, JASON, ALL_TALK, true).await;
    compare_row(&app, "channel").await;
}
#[tokio::test]
async fn original_board_live_stack_names_count_and_attributes() {
    let (app, board, _) = setup(true).await;
    grant(&app, DAVID, board, true).await;
    grant(&app, JZ, board, true).await;
    compare_row(&app, "board").await;
}
#[tokio::test]
async fn original_direct_live_stack_peer_count_and_attributes() {
    let (app, _, _) = setup(true).await;
    grant(&app, JASON, DIRECT_DAVID_JASON, true).await;
    compare_row(&app, "direct").await;
}
#[tokio::test]
async fn original_quiet_rows_have_empty_hidden_stack_targets() {
    let (app, _, _) = setup(true).await;
    compare_row(&app, "quiet_channel").await;
    compare_row(&app, "quiet_direct").await;
}
#[tokio::test]
async fn original_group_direct_name_and_live_stack() {
    let (app, _, group) = setup(true).await;
    grant(&app, JASON, group, true).await;
    compare_row(&app, "group").await;
}
#[tokio::test]
async fn original_cached_direct_rename_then_join_invalidates_only_on_participants() {
    let (app, _, _) = setup(true).await;
    let id = grant(&app, JASON, DIRECT_DAVID_JASON, false).await;
    compare_row(&app, "cache_before").await;
    app.db()
        .write(|tx| {
            User::find(tx.conn(), JASON)?.update(
                tx,
                UserChanges {
                    name: Some("Jordan".into()),
                    ..Default::default()
                },
            )
        })
        .await
        .unwrap();
    compare_row(&app, "cache_rename").await;
    app.db()
        .write(move |tx| {
            HuddleGrant::find_by_id(tx.conn(), id)?
                .unwrap()
                .record_seen(tx)
                .map(|_| ())
        })
        .await
        .unwrap();
    compare_row(&app, "cache_join").await;
}
#[tokio::test]
async fn original_unconfigured_huddle_has_no_stacks_or_presence_controllers() {
    let (app, _, _) = setup(false).await;
    grant(&app, JASON, ALL_TALK, true).await;
    let reply = app.david().get("/users/me/sidebar").await;
    assert_eq!(reply.status, StatusCode::OK);
    let mut dom = campfire_richtext::dom::Dom::new();
    let root = dom.parse_fragment(&reply.text()).unwrap();
    for id in ["shared_rooms", "direct_rooms"] {
        let section = dom
            .descendants(root)
            .into_iter()
            .find(|n| dom.attr(*n, "id") == Some(id))
            .unwrap();
        assert_eq!(
            dom.descendants(section)
                .into_iter()
                .filter(|n| dom
                    .attr(*n, "class")
                    .is_some_and(|c| c.split_whitespace().any(|c| c == "voice-stack")))
                .count(),
            0,
            "{id}: unconfigured stack selector"
        );
    }
    assert_eq!(
        dom.descendants(root)
            .into_iter()
            .filter(|n| dom
                .attr(*n, "data-controller")
                .is_some_and(|c| c.split_whitespace().any(|c| c == "huddle-presence")))
            .count(),
        0,
        "unconfigured presence selector"
    );
}
async fn quiet_rooms(app: &TestApp, more: bool) {
    app.db()
        .write(move |tx| {
            for (kind, number, label) in [
                (RoomType::Closed, if more { 4 } else { 2 }, "quiet"),
                (RoomType::Board, 1, "board"),
                (RoomType::Stage, if more { 4 } else { 2 }, "stage"),
            ] {
                for i in 0..number {
                    Room::create_for(
                        tx,
                        kind,
                        Some(&format!("{more} {label} {i}")),
                        DAVID,
                        &[DAVID],
                    )?;
                }
            }
            for peer in if more { vec![BENDER, KEVIN] } else { vec![JZ] } {
                Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, peer])?;
            }
            Ok(())
        })
        .await
        .unwrap();
}
async fn selects(app: &TestApp) -> Vec<String> {
    let probe = crate::controllers::rooms::query_probe::SqlProbe::start(
        app.db(),
        app.booted.app.config.db_readers,
    )
    .await;
    let reply = app.david().get("/users/me/sidebar").await;
    assert_eq!(reply.status, StatusCode::OK);
    let reads: Vec<_> = probe
        .finish()
        .await
        .into_iter()
        .filter(|r| r.sql.trim_start().starts_with("SELECT"))
        .map(|r| r.sql)
        .collect();
    assert!(
        !reads.is_empty(),
        "the real HTTP SELECT trace must be active"
    );
    reads
}
#[tokio::test]
async fn original_mixed_quiet_sidebar_read_counts_stay_flat_with_one_grants_query() {
    let (app, _, _) = setup(true).await;
    quiet_rooms(&app, false).await;
    app.david().get("/users/me/sidebar").await;
    let before = selects(&app).await;
    quiet_rooms(&app, true).await;
    let after = selects(&app).await;
    assert_eq!(
        before.len(),
        after.len(),
        "original quiet-room request SELECT growth: {before:?} then {after:?}"
    );
    assert_eq!(
        after
            .iter()
            .filter(|q| q.contains("FROM huddle_grants"))
            .count(),
        1,
        "original huddle-grant batch SELECT count"
    );
    assert_eq!(
        oracle()["queries"]["baseline"]["selects"],
        oracle()["queries"]["more"]["selects"]
    );
    assert_eq!(oracle()["queries"]["more"]["grants"], 1);
    println!(
        "Original sidebar reads: Rust {} -> {}; Rails {} -> {}; huddle_grants 1",
        before.len(),
        after.len(),
        oracle()["queries"]["baseline"]["selects"],
        oracle()["queries"]["more"]["selects"]
    );
}
