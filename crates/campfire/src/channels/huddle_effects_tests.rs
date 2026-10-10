//! JSON stage publications keep post-commit and rollback behavior.
use crate::controllers::presenters::test_support::{DAVID, JASON, TestApp, seed_clock};
use crate::controllers::spa::api_tests::{Sync, serve};
use campfire_api_types as api;
use campfire_db::{Membership, Room, RoomType};

async fn app() -> Option<TestApp> {
    TestApp::boot_seed_with_env("default", seed_clock(), &[
        ("SPA_ENABLED", "1"), ("LIVEKIT_URL", "wss://public.example.test"),
        ("LIVEKIT_INTERNAL_URL", "http://internal.example.test:7880"),
        ("LIVEKIT_API_KEY", "key"), ("LIVEKIT_API_SECRET", "secret"),
        ("LIVEKIT_GATEWAY_SECRET", "gateway"),
    ]).await
}

#[tokio::test]
async fn stage_stream_commit_reaches_the_socket_and_rollback_stays_silent() {
    let Some(a) = app().await else { return };
    let a = a.without_job_runner().await;
    let (room, host) = a.db().write(|tx| {
        let room = Room::create_for(tx, RoomType::Stage, Some("Town Hall"), DAVID, &[DAVID, JASON])?;
        let host = Membership::find_by_room_and_user(tx.conn(), room.id, DAVID)?.unwrap().id;
        Ok((room, host))
    }).await.unwrap();
    let id = room.id;
    let (addr, server) = serve(&a).await;
    let browser = a.sign_in(JASON).await;
    let mut socket = Sync::connect(addr, &browser.cookie_header(), &[format!("room:{id}")]).await;
    socket.welcome().await;
    let rolled_back = a.db().write(move |tx| {
        campfire_db::models::stream::Stream::create(tx, id, host, DAVID, "1080p15", None)?;
        Err::<(), _>(campfire_db::Error::Other("rollback".into()))
    }).await;
    assert!(rolled_back.is_err());
    a.db().write(move |tx| { tx.emit_after_commit(campfire_db::Event::broadcast(&campfire_db::models::huddle_effects::StageRoster { room_id: id })); Ok(()) }).await.unwrap();
    let event = socket.until(|event| matches!(&event.payload, api::SyncPayload::StageUpdated(stage) if stage.room_id == id), |event| matches!(&event.payload, api::SyncPayload::StageUpdated(stage) if stage.live.is_some())).await;
    let api::SyncPayload::StageUpdated(stage) = event.payload else { unreachable!() };
    assert!(stage.live.is_none());
    a.db().write(move |tx| campfire_db::models::stream::Stream::create(tx, id, host, DAVID, "1080p15", None)).await.unwrap();
    socket.until(|event| matches!(&event.payload, api::SyncPayload::StageUpdated(stage) if stage.room_id == id && stage.live.is_some()), |_| false).await;
    server.abort();
}

#[tokio::test]
async fn last_host_departure_delivers_a_json_system_note() {
    let Some(a) = app().await else { return };
    let a = a.without_job_runner().await;
    let room = a.db().write(|tx| Room::create_for(tx, RoomType::Stage, Some("Town Hall"), DAVID, &[DAVID, JASON])).await.unwrap();
    let id = room.id;
    let (addr, server) = serve(&a).await;
    let browser = a.sign_in(JASON).await;
    let mut socket = Sync::connect(addr, &browser.cookie_header(), &[format!("room:{id}")]).await;
    socket.welcome().await;
    a.db().write(move |tx| Membership::find_by_room_and_user(tx.conn(), id, DAVID)?.unwrap().destroy(tx)).await.unwrap();
    let event = socket.until(|event| matches!(&event.payload, api::SyncPayload::MessageCreated(message) if message.room_id == id && message.system_note), |_| false).await;
    let api::SyncPayload::MessageCreated(message) = event.payload else { unreachable!() };
    assert!(message.body_html.contains("The stage ended because the last host left."));
    assert_eq!(message.creator_id, DAVID);
    server.abort();
}
