use axum::http::{Method, StatusCode};
use campfire_api_types as api;
use serde_json::json;

use super::api_tests::{Sync, app, json_body, parse, serve};
use crate::controllers::presenters::test_support::{DAVID, JASON, KEVIN};

fn notified(message_id: i64) -> impl Fn(&api::SyncEvent) -> bool {
    move |event| {
        matches!(&event.payload, api::SyncPayload::MessageNotified(notified) if notified.message_id == message_id)
    }
}

#[tokio::test]
async fn a_group_message_notifies_its_recipient_on_the_user_topic_and_not_its_author() {
    let Some(a) = app(true).await else { panic!("restored default seed required") };
    let (addr, server) = serve(&a).await;
    let mut david = a.sign_in(DAVID).await;
    let mut kevin = a.sign_in(KEVIN).await;
    let mut davids_sync = Sync::connect(addr, &david.cookie_header(), &[]).await;
    davids_sync.welcome().await;
    let mut kevins_sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
    kevins_sync.welcome().await;

    let reply = david
        .write(json_body(Method::POST, "/api/v1/directs", &json!({"userIds": [KEVIN, JASON]})))
        .await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let room = parse::<api::SidebarRow>(&reply).room.id;
    let post = |text: &str| json!({"clientMessageId": text, "markdownSource": text});
    let path = format!("/api/v1/rooms/{room}/messages");

    let reply = david.write(json_body(Method::POST, &path, &post("Got a minute?"))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let davids = parse::<api::MessageDTO>(&reply).id;
    let event = kevins_sync.until(notified(davids), |_| false).await;
    assert_eq!(event.topic, "user");
    assert_eq!(
        event.payload,
        api::SyncPayload::MessageNotified(api::MessageNotified {
            message_id: davids,
            room_id: room,
            thread_id: None,
            ignored: None,
        })
    );

    let reply = kevin.write(json_body(Method::POST, &path, &post("Sure"))).await;
    assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
    let kevins = parse::<api::MessageDTO>(&reply).id;
    davids_sync.until(notified(kevins), notified(davids)).await;
    server.abort();
}

#[tokio::test]
async fn a_private_thread_notification_queued_before_revocation_is_withheld_live() {
    private_thread_notification_after_revocation(false).await;
}

#[tokio::test]
async fn a_private_thread_notification_queued_before_revocation_is_withheld_on_replay() {
    private_thread_notification_after_revocation(true).await;
}

async fn private_thread_notification_after_revocation(replay_only: bool) {
    const DESIGNERS: i64 = 654632876;
    for revoke_room in [false, true] {
        let a = app(true).await.expect("the restored frozen default seed");
        let mut david = a.sign_in(DAVID).await;
        let kevin = a.sign_in(KEVIN).await;
        let thread = a
            .db()
            .write(|tx| {
                let thread = campfire_db::ChannelThread::create(
                    tx,
                    campfire_db::NewChannelThread {
                        room_id: DESIGNERS,
                        creator_id: DAVID,
                        name: Some("Private plans".into()),
                        visibility: campfire_db::channel_thread::ThreadVisibility::Private,
                        ..Default::default()
                    },
                )?;
                thread.add_access_member(tx, DAVID, KEVIN)?;
                let mut kevins = campfire_db::ThreadMembership::join(tx, thread.id, KEVIN)?;
                kevins.update_involvement(tx, campfire_db::ThreadInvolvement::Everything)?;
                Ok(thread.id)
            })
            .await
            .unwrap();
        a.booted.app.broadcasts.settle_sync().await;
        let (addr, server) = serve(&a).await;
        let mut sync = Sync::connect(addr, &kevin.cookie_header(), &[]).await;
        let Some(api::ServerFrame::Welcome { epoch, seq, .. }) = sync.next().await else {
            panic!("welcome")
        };
        let path = format!("/api/v1/threads/{thread}/messages");
        let post = |text: &str| json!({"clientMessageId": text, "markdownSource": text});

        let reply = david.write(json_body(Method::POST, &path, &post("Before"))).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let before = parse::<api::MessageDTO>(&reply).id;
        let event = sync.until(notified(before), |_| false).await;
        assert!(
            matches!(event.payload, api::SyncPayload::MessageNotified(notified) if notified.thread_id == Some(thread)),
            "{event:?}"
        );

        let hold = campfire_api::test_hooks::hold_before_sync_delivery(a.db().path(), KEVIN, |payload| {
            matches!(payload, api::SyncPayload::MessageNotified(_))
        });
        let reply = david.write(json_body(Method::POST, &path, &post("After"))).await;
        assert_eq!(reply.status, StatusCode::CREATED, "{}", reply.text());
        let after = parse::<api::MessageDTO>(&reply).id;
        tokio::time::timeout(std::time::Duration::from_secs(10), hold.reached.wait())
            .await
            .expect("queued notification");
        a.db()
            .write(move |tx| {
                if revoke_room {
                    campfire_db::Room::find(tx.conn(), DESIGNERS)?.revoke_from(tx, &[KEVIN])
                } else {
                    campfire_db::ChannelThread::find(tx.conn(), thread)?.remove_access_member(tx, DAVID, KEVIN)
                }
            })
            .await
            .unwrap();
        hold.release.wait().await;
        a.booted.app.broadcasts.settle_sync().await;
        let marker = |event: &api::SyncEvent| {
            matches!(&event.payload, api::SyncPayload::WorkspaceStylesUpdated(styles) if styles.css.as_deref() == Some("delivery-marker"))
        };
        a.booted.app.cable.sync_publish(campfire_cable::sync::SyncPublication::new(
            campfire_cable::sync::Audience::User(KEVIN),
            serde_json::to_string(&api::SyncPayload::WorkspaceStylesUpdated(api::CustomStyles {
                css: Some("delivery-marker".into()),
            }))
            .unwrap(),
        ));
        if !replay_only {
            loop {
                match sync.next().await {
                    Some(api::ServerFrame::Batch { events }) => {
                        assert!(!events.iter().any(notified(after)), "revoked notification: {events:?}");
                        if events.iter().any(marker) {
                            break;
                        }
                    },
                    Some(api::ServerFrame::Bye { .. }) => break,
                    Some(api::ServerFrame::Ping) => {},
                    other => panic!("expected marker or revocation, got {other:?}"),
                }
            }
        }
        let mut replay = Sync::open(addr, &kevin.cookie_header(), &[], json!({"epoch": epoch, "seq": seq})).await;
        assert!(matches!(replay.next().await, Some(api::ServerFrame::Welcome { resumed: true, .. })));
        replay.until(marker, notified(after)).await;
        server.abort();
    }
}
