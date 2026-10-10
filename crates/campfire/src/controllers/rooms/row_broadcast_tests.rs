//! Sidebar publications carry room/header refreshes and OOO facts over the JSON socket.
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN};
use crate::controllers::spa::api_tests::{Sync, app, serve};
use campfire_api_types as api;
use campfire_db::{Room, RoomType};

#[tokio::test]
async fn direct_rename_refreshes_each_members_sidebar_and_header() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let room = a.db().write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, KEVIN, JASON])).await.unwrap();
    let (addr, server) = serve(&a).await;
    let david = a.sign_in(DAVID).await;
    let kevin = a.sign_in(KEVIN).await;
    let mut owner = Sync::connect(addr, &david.cookie_header(), &[]).await;
    let mut member = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    owner.welcome().await;
    member.welcome().await;
    let id = room.id;
    a.db().write(move |tx| Room::find(tx.conn(), id)?.rename_direct(tx, "Friday <&>", DAVID)).await.unwrap();
    for socket in [&mut owner, &mut member] {
        let event = socket.until(|event| matches!(&event.payload, api::SyncPayload::SidebarRowUpserted(row) if row.room.id == id && row.refresh_room == Some(true)), |_| false).await;
        let api::SyncPayload::SidebarRowUpserted(row) = event.payload else { unreachable!() };
        assert_eq!(row.display_name, "Friday <&>");
        assert_eq!(row.room.name, None);
    }
    server.abort();
}

#[tokio::test]
async fn ooo_presence_carries_the_date_and_note_then_clears_them() {
    let Some(a) = app(true).await else { return };
    let a = a.without_job_runner().await;
    let room = a.db().write(|tx| Room::create_for(tx, RoomType::Direct, None, DAVID, &[DAVID, KEVIN])).await.unwrap();
    let id = room.id;
    let (addr, server) = serve(&a).await;
    let david = a.sign_in(DAVID).await;
    let mut socket = Sync::connect(addr, &david.cookie_header(), &[]).await;
    socket.welcome().await;
    for (command, active) in [("/ooo tomorrow Trip <&>", true), ("/ooo off", false)] {
        a.db().write(move |tx| {
            let result = campfire_db::slash_commands::dispatch(tx, &campfire_db::slash_commands::Context { user_id: KEVIN, room_id: id, thread_id: None, huddles_configured: false }, command)?;
            assert_eq!(result.kind, "ephemeral");
            Ok(())
        }).await.unwrap();
        let event = socket.until(|event| matches!(&event.payload, api::SyncPayload::Presence(presence) if presence.user_id == KEVIN && presence.status_text.as_ref().is_some_and(|text| text.contains("Trip <&>")) == active), |_| false).await;
        let api::SyncPayload::Presence(presence) = event.payload else { unreachable!() };
        if active {
            let text = presence.status_text.unwrap();
            assert!(text.contains("Mar"), "{text}");
        } else { assert_eq!(presence.status_text, None); }
    }
    server.abort();
}
