//! Ports of reference/test/channels/**, plus the channels the reference doesn't test directly.
use campfire_db::Clock as _;
use serde_json::json;

use super::support::*;
use crate::channels::room_gid;

// ApplicationCable::Connection

#[tokio::test]
async fn connects_with_a_valid_session_cookie() {
    let app = start().await;
    let mut client = app.connect("david").await;
    client.confirm(&identifier(json!({ "channel": "HeartbeatChannel" }))).await;
}

#[tokio::test]
async fn rejects_a_connection_without_or_with_a_bad_session_cookie() {
    let app = start().await;
    let unauthorized = r#"{"type":"disconnect","reason":"unauthorized","reconnect":false}"#;

    for cookie in [None, Some(app.cookie_with_token("-1")), Some("session_token=forged--0000".to_string())] {
        let mut client = app.connect_with_cookie(cookie.as_deref()).await;
        assert_eq!(client.until_closed().await, vec![unauthorized.to_string()]);
    }
}

// HeartbeatChannel and ApplicationCable::Channel

#[tokio::test]
async fn heartbeat_and_base_channels_confirm() {
    let app = start().await;
    let mut client = app.connect("jz").await;
    client.confirm(&identifier(json!({ "channel": "HeartbeatChannel" }))).await;
    client.confirm(&identifier(json!({ "channel": "ApplicationCable::Channel" }))).await;
    client.assert_silent().await;
}

// RoomChannel

#[tokio::test]
async fn room_channel_streams_for_member_rooms_only() {
    let app = start().await;
    let mut client = app.connect("kevin").await;
    let designers = app.room("designers").await;

    let member = room_identifier("RoomChannel", designers.id);
    client.confirm(&member).await;
    client.reject(&room_identifier("RoomChannel", id("watercooler"))).await;
    client.reject(&room_identifier("RoomChannel", -1)).await;
    client.reject(&identifier(json!({ "channel": "RoomChannel" }))).await;
    // Params are cast like Active Record casts an id.
    let by_string = identifier(json!({ "channel": "RoomChannel", "room_id": designers.id.to_string() }));
    client.confirm(&by_string).await;

    // Both subscriptions stream the room; a connection polls its subscriptions in no set order.
    let stream = format!("room:{}", room_gid(&designers).to_param());
    app.server.broadcast(&stream, &json!({ "hello": 1 }));
    let mut received = vec![client.next_text().await, client.next_text().await];
    received.sort();
    let mut expected = vec![delivery(&member, r#"{"hello":1}"#), delivery(&by_string, r#"{"hello":1}"#)];
    expected.sort();
    assert_eq!(received, expected);
    client.assert_silent().await;
}

// PresenceChannel (reference/test/channels/presence_channel_test.rb)

#[tokio::test]
async fn presence_subscribes_and_marks_the_membership_connected() {
    let app = start().await;
    let mut client = app.connect("david").await;
    let reads = identifier(json!({ "channel": "ReadRoomsChannel" }));
    client.confirm(&reads).await;

    let membership = app.membership("designers", "david").await.unwrap();
    assert!(!membership.is_connected(app.clock.now()));
    // A member's unread room gets cleared by `present`.
    app.db.write(move |tx| tx.conn().execute("UPDATE memberships SET unread_at = '2024-01-01 00:00:00' WHERE id = ?", [membership.id]).map_err(Into::into)).await.unwrap();

    let presence = room_identifier("PresenceChannel", id("designers"));
    client.confirm(&presence).await;
    assert_eq!(client.next_text().await, delivery(&reads, &format!(r#"{{"room_id":{}}}"#, id("designers"))));

    let membership = app.membership("designers", "david").await.unwrap();
    assert!(membership.is_connected(app.clock.now()));
    assert_eq!(membership.connections, 1);
    assert_eq!(membership.unread_at, None);

    client.unsubscribe(&presence).await;
    eventually("room unsubscribe to clear connected membership", || async { !app.membership("designers", "david").await.unwrap().is_connected(app.clock.now()) }).await;
    let membership = app.membership("designers", "david").await.unwrap();
    assert_eq!((membership.connections, membership.connected_at), (0, None));
}

#[tokio::test]
async fn presence_counts_connections_and_refreshes() {
    let app = start().await;
    let presence = room_identifier("PresenceChannel", id("designers"));
    let mut first = app.connect("david").await;
    let mut second = app.connect("david").await;
    first.confirm(&presence).await;
    second.confirm(&presence).await;
    assert_eq!(app.membership("designers", "david").await.unwrap().connections, 2);

    // `refresh` reconnects a membership that timed out.
    app.clock.travel(jiff::SignedDuration::from_secs(61));
    assert!(!app.membership("designers", "david").await.unwrap().is_connected(app.clock.now()));
    first.perform(&presence, json!({ "action": "refresh" })).await;
    eventually("room action to connect membership", || async { app.membership("designers", "david").await.unwrap().is_connected(app.clock.now()) }).await;
    assert_eq!(app.membership("designers", "david").await.unwrap().connections, 1);

    second.unsubscribe(&presence).await;
    eventually("room disconnect to clear connected_at", || async { app.membership("designers", "david").await.unwrap().connected_at.is_none() }).await;
}

#[tokio::test]
async fn presence_rejects_rooms_the_user_is_not_in_without_touching_memberships() {
    let app = start().await;
    let mut client = app.connect("david").await;
    let before = app.membership("bender_and_kevin", "kevin").await.unwrap();

    client.reject(&room_identifier("PresenceChannel", id("bender_and_kevin"))).await;
    client.reject(&room_identifier("PresenceChannel", -1)).await;
    client.assert_silent().await;
    assert_eq!(app.membership("bender_and_kevin", "kevin").await.unwrap(), before);
}

// ReadRoomsChannel and UnreadRoomsChannel (reference/test/channels/unread_rooms_channel_test.rb)

#[tokio::test]
async fn unread_rooms_streams_only_the_subscribers_own_stream() {
    let app = start().await;
    let direct = app.room("bender_and_kevin").await;
    let unreads = identifier(json!({ "channel": "UnreadRoomsChannel" }));

    let mut outsider = app.connect("jz").await;
    let mut member = app.connect("kevin").await;
    outsider.confirm(&unreads).await;
    member.confirm(&unreads).await;

    let message = app.message("first").await;
    app.unread_room(&direct, &message).await;

    assert_eq!(member.next_text().await, delivery(&unreads, &format!(r#"{{"roomId":{}}}"#, direct.id)));
    member.assert_silent().await;
    outsider.assert_silent().await;
}

#[tokio::test]
async fn read_rooms_streams_the_users_reads() {
    let app = start().await;
    let reads = identifier(json!({ "channel": "ReadRoomsChannel" }));
    let mut client = app.connect("jason").await;
    client.confirm(&reads).await;
    crate::channels::broadcasts::read_room(&app.server, id("jason"), 7);
    crate::channels::broadcasts::read_room(&app.server, id("david"), 8);
    assert_eq!(client.next_text().await, delivery(&reads, r#"{"room_id":7}"#));
    client.assert_silent().await;
}

// TypingNotificationsChannel

#[tokio::test]
async fn typing_notifications_broadcast_start_and_stop_to_the_room() {
    let app = start().await;
    let typing = room_identifier("TypingNotificationsChannel", id("designers"));
    let mut typist = app.connect("jz").await;
    let mut reader = app.connect("kevin").await;
    typist.confirm(&typing).await;
    reader.confirm(&typing).await;

    typist.perform(&typing, json!({ "action": "start" })).await;
    let start = format!(r#"{{"action":"start","user":{{"id":{},"name":"JZ"}}}}"#, id("jz"));
    assert_eq!(reader.next_text().await, delivery(&typing, &start));
    assert_eq!(typist.next_text().await, delivery(&typing, &start));

    app.db.write(|tx| {
        let mut user = campfire_db::User::find(tx.conn(), id("jz"))?;
        user.update(tx, campfire_db::UserChanges {
            nickname: Some(Some("Jay".into())), ..Default::default()
        })
    }).await.unwrap();
    typist.perform(&typing, json!({ "action": "stop" })).await;
    let stop = format!(r#"{{"action":"stop","user":{{"id":{},"name":"Jay"}}}}"#, id("jz"));
    assert_eq!(reader.next_text().await, delivery(&typing, &stop));

    // Not an action: nothing is sent.
    typist.perform(&typing, json!({ "action": "dance" })).await;
    reader.assert_silent().await;

    let designers = app.room("designers").await;
    let stream = format!("typing_notifications:{}", room_gid(&designers).to_param());
    assert_eq!(typing_stream_name(&designers), stream);
}

fn typing_stream_name(room: &campfire_db::Room) -> String {
    campfire_cable::naming::broadcasting_for("TypingNotificationsChannel", &[&room_gid(room).to_param()])
}

/// A subscription whose `subscribed` failed has no room: typing there is an error, not a panic
/// that takes the whole connection down.
#[tokio::test]
async fn typing_on_a_failed_subscription_leaves_the_connection_up() {
    let app = start().await;
    let rename = |from: &'static str, to: &'static str| {
        app.db.write(move |tx| tx.conn().execute_batch(&format!("ALTER TABLE {from} RENAME TO {to}")).map_err(Into::into))
    };
    let mut client = app.connect("jz").await;
    let typing = room_identifier("TypingNotificationsChannel", id("designers"));

    rename("rooms", "rooms_away").await.unwrap();
    client.subscribe(&typing).await;
    client.confirm(&identifier(json!({ "channel": "HeartbeatChannel" }))).await;
    rename("rooms_away", "rooms").await.unwrap();

    client.perform(&typing, json!({ "action": "start" })).await;
    client.confirm(&identifier(json!({ "channel": "ApplicationCable::Channel" }))).await;
}

#[tokio::test]
async fn typing_notifications_reject_non_members() {
    let app = start().await;
    let mut client = app.connect("jz").await;
    client.reject(&room_identifier("TypingNotificationsChannel", id("watercooler"))).await;
}











// Preserve the golden's second receiver, but bound further public callbacks.

#[tokio::test]
async fn performing_subscribed_bounds_stream_receivers() {
    let app = start().await;
    let unreads = identifier(json!({ "channel": "UnreadRoomsChannel" }));
    let mut client = app.connect("kevin").await;
    client.confirm(&unreads).await;
    for _ in 0..32 {
        client.perform(&unreads, json!({ "action": "subscribed" })).await;
    }
    client.assert_silent().await;

    let broadcasts = app.broadcasts.clone();
    let room = app.room("bender_and_kevin").await;
    let message = app.message("first").await;
    app.db.read(move |conn| broadcasts.unread_room(conn, &room, &message, &campfire_db::rich_text::BasicRichText)).await.unwrap();
    let expected = delivery(&unreads, &format!(r#"{{"roomId":{}}}"#, id("bender_and_kevin")));
    assert_eq!(client.next_text().await, expected);
    assert_eq!(client.next_text().await, expected);
    client.assert_silent().await;
}
