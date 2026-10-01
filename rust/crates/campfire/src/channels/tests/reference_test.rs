//! reference/test/channels/**, test for test, as black-box specs over a real WebSocket.
//!
//! The Ruby tests stub the connection and inspect the subscription (`assert_has_stream`,
//! `subscription.rejected?`); here the same scenarios go through the wire: a stream is "had" when a
//! broadcast on it arrives, "not had" when one doesn't, and a rejection is the reject frame (or,
//! for a rejection inside an action, which sends nothing, that the subscription stops performing
//! actions). Where a Ruby test mutates the stubbed user or deletes rows without callbacks, the spec
//! does the same with SQL, since a real connection would be closed by the callbacks.
use campfire_db::Clock as _;
use serde_json::json;

use super::support::*;
use crate::channels::room_gid;
use crate::channels::threads::thread_gid;

fn channel(name: &str) -> String {
    identifier(json!({ "channel": name }))
}

fn int(value: i64) -> rusqlite::types::Value {
    rusqlite::types::Value::Integer(value)
}

fn text(value: &str) -> rusqlite::types::Value {
    rusqlite::types::Value::Text(value.to_string())
}

/// Broadcasts a marker on `stream` and asserts `client` receives it on `identifier`.
async fn assert_has_stream(app: &TestApp, client: &mut Client, identifier: &str, stream: &str) {
    assert_eq!(
        app.server.broadcast(stream, &json!({ "on": stream })),
        1,
        "one subscriber on {stream}"
    );
    assert_eq!(
        client.next_text().await,
        delivery(identifier, &json!({ "on": stream }).to_string())
    );
}

/// Broadcasts on `stream` and asserts nobody is streaming it.
fn assert_no_stream(app: &TestApp, stream: &str) {
    assert_eq!(
        app.server.broadcast(stream, &json!({ "on": stream })),
        0,
        "nobody streams {stream}"
    );
}

fn system_now() -> campfire_db::Timestamp {
    campfire_db::Timestamp::from_jiff(jiff::Timestamp::now())
}

async fn deactivate_without_callbacks(app: &TestApp, user: &str) {
    // `user.update!(status: :deactivated)` on the stubbed connection's user: the connection stays.
    app.sql(
        "UPDATE users SET status = 1 WHERE id = ?",
        vec![int(id(user))],
    )
    .await;
}

// activity_channel_test.rb

#[tokio::test]
async fn activity_streams_only_the_subscribers_own_activity() {
    let app = start().await;
    let mut david = app.connect("david").await;
    let activity = channel("ActivityChannel");
    david.confirm(&activity).await;
    assert_has_stream(
        &app,
        &mut david,
        &activity,
        &format!("user_{}_activity", id("david")),
    )
    .await;
    assert_no_stream(&app, &format!("user_{}_activity", id("jason")));
}

#[tokio::test]
async fn activity_rejects_bots() {
    let app = start().await;
    let mut bender = app.connect("bender").await;
    bender.reject(&channel("ActivityChannel")).await;
}

#[tokio::test]
async fn activity_rejects_inactive_users() {
    let app = start().await;
    let session = app.session_for("david", true).await;
    deactivate_without_callbacks(&app, "david").await;
    let mut david = app.connect_with_session(&session).await;
    david.reject(&channel("ActivityChannel")).await;
}

// agents_channel_test.rb

#[tokio::test]
async fn agents_a_signed_in_human_may_subscribe_to_the_agents_stream() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    let agents = channel("AgentsChannel");
    kevin.confirm(&agents).await;
    assert_has_stream(&app, &mut kevin, &agents, "agents:all").await;
}

#[tokio::test]
async fn agents_a_bot_may_not_subscribe() {
    let app = start().await;
    let mut bender = app.connect("bender").await;
    bender.reject(&channel("AgentsChannel")).await;
}

// huddle_notice_channel_test.rb

#[tokio::test]
async fn huddle_notices_stream_only_the_subscribers_own_notices() {
    let app = start().await;
    let mut david = app.connect("david").await;
    let notices = channel("HuddleNoticeChannel");
    david.confirm(&notices).await;
    assert_has_stream(
        &app,
        &mut david,
        &notices,
        &format!("user_{}_huddle_notices", id("david")),
    )
    .await;
    assert_no_stream(&app, &format!("user_{}_huddle_notices", id("jason")));
}

#[tokio::test]
async fn huddle_notices_reject_bots() {
    let app = start().await;
    let mut bender = app.connect("bender").await;
    bender.reject(&channel("HuddleNoticeChannel")).await;
}

#[tokio::test]
async fn huddle_notices_reject_inactive_users() {
    let app = start().await;
    let session = app.session_for("david", true).await;
    deactivate_without_callbacks(&app, "david").await;
    let mut david = app.connect_with_session(&session).await;
    david.reject(&channel("HuddleNoticeChannel")).await;
}

// UnreadThreadsChannel has no Ruby test; it streams like UnreadRoomsChannel.

#[tokio::test]
async fn unread_threads_streams_only_the_subscribers_own_stream() {
    let app = start().await;
    let mut jz = app.connect("jz").await;
    let unread_threads = channel("UnreadThreadsChannel");
    jz.confirm(&unread_threads).await;
    assert_has_stream(
        &app,
        &mut jz,
        &unread_threads,
        &format!("user_{}_unread_threads", id("jz")),
    )
    .await;
    assert_no_stream(&app, &format!("user_{}_unread_threads", id("kevin")));
}

// presence_channel_test.rb

#[tokio::test]
async fn presence_subscribes() {
    let app = start().await;
    let mut david = app.connect("david").await;
    let designers = app.room("designers").await;
    let presence = room_identifier("PresenceChannel", designers.id);
    david.confirm(&presence).await;
    assert_has_stream(
        &app,
        &mut david,
        &presence,
        &format!("presence:{}", room_gid(&designers).to_param()),
    )
    .await;
}

#[tokio::test]
async fn presence_rejects_a_room_the_user_is_not_a_member_of() {
    let app = start().await;
    // `Rooms::Closed.create!(name: "New Room", creator: users(:david))`: no memberships.
    let now = app.clock.now().to_db();
    app.sql(
        "INSERT INTO rooms (name, type, creator_id, created_at, updated_at) VALUES ('New Room', 'Rooms::Closed', ?, ?, ?)",
        vec![int(id("david")), text(&now), text(&now)],
    )
    .await;
    let room_id = app
        .db
        .read(|conn| {
            Ok(conn.query_row("SELECT MAX(id) FROM rooms", [], |row| row.get::<_, i64>(0))?)
        })
        .await
        .unwrap();
    let mut david = app.connect("david").await;
    david
        .reject(&room_identifier("PresenceChannel", room_id))
        .await;
}

#[tokio::test]
async fn presence_rejects_a_room_that_does_not_exist_or_is_missing() {
    let app = start().await;
    let mut david = app.connect("david").await;
    david.reject(&room_identifier("PresenceChannel", -1)).await;
    david.reject(&channel("PresenceChannel")).await;
}

#[tokio::test]
async fn presence_subscribing_marks_the_membership_connected() {
    let app = start().await;
    assert!(
        !app.membership("designers", "david")
            .await
            .unwrap()
            .is_connected(app.clock.now())
    );
    let mut david = app.connect("david").await;
    david
        .confirm(&room_identifier("PresenceChannel", id("designers")))
        .await;
    assert!(
        app.membership("designers", "david")
            .await
            .unwrap()
            .is_connected(app.clock.now())
    );
}

#[tokio::test]
async fn presence_unsubscribing_marks_the_membership_disconnected() {
    let app = start().await;
    let mut david = app.connect("david").await;
    let presence = room_identifier("PresenceChannel", id("designers"));
    david.confirm(&presence).await;
    assert!(
        app.membership("designers", "david")
            .await
            .unwrap()
            .is_connected(app.clock.now())
    );
    david.unsubscribe(&presence).await;
    eventually("revocation cleanup", || async {
        !app.membership("designers", "david")
            .await
            .unwrap()
            .is_connected(app.clock.now())
    })
    .await;
}

#[tokio::test]
async fn presence_refresh_and_unsubscribe_tolerate_a_membership_removed_with_the_room() {
    let app = start().await;
    let mut david = app.connect("david").await;
    let presence = room_identifier("PresenceChannel", id("designers"));
    david.confirm(&presence).await;
    // `membership.delete`: no callbacks, so no disconnect.
    app.sql(
        "DELETE FROM memberships WHERE room_id = ? AND user_id = ?",
        vec![int(id("designers")), int(id("david"))],
    )
    .await;

    david
        .perform(&presence, json!({ "action": "refresh" }))
        .await;
    david.unsubscribe(&presence).await;
    // Neither raised the connection down: it still subscribes, in order.
    david.confirm(&channel("HeartbeatChannel")).await;
}

// unread_rooms_channel_test.rb

#[tokio::test]
async fn unread_rooms_streams_only_the_subscribers_own_unread_stream() {
    let app = start().await;
    let mut jz = app.connect("jz").await;
    let unreads = channel("UnreadRoomsChannel");
    jz.confirm(&unreads).await;
    assert_has_stream(
        &app,
        &mut jz,
        &unreads,
        &format!("user_{}_unreads", id("jz")),
    )
    .await;
    assert_no_stream(&app, "unread_rooms");
}

#[tokio::test]
async fn unread_rooms_an_outsider_is_not_told_about_activity_in_a_room_they_cant_see() {
    let app = start().await;
    let direct = app.room("bender_and_kevin").await;
    assert!(
        app.membership("bender_and_kevin", "jz").await.is_none(),
        "jz must be an outsider for this test to mean anything"
    );
    let mut jz = app.connect("jz").await;
    jz.confirm(&channel("UnreadRoomsChannel")).await;

    let message = app
        .create_message("bender_and_kevin", "kevin", "Private", "outsider")
        .await;
    app.message_create(&direct, &message).await;
    jz.assert_silent().await;
}

#[tokio::test]
async fn unread_rooms_a_member_is_told_about_activity_in_their_own_room() {
    let app = start().await;
    let direct = app.room("bender_and_kevin").await;
    let mut kevin = app.connect("kevin").await;
    let unreads = channel("UnreadRoomsChannel");
    kevin.confirm(&unreads).await;

    let message = app
        .create_message("bender_and_kevin", "bender", "Private", "member")
        .await;
    app.message_create(&direct, &message).await;
    assert_eq!(
        kevin.next_text().await,
        delivery(&unreads, &format!(r#"{{"roomId":{}}}"#, direct.id))
    );
    kevin.assert_silent().await;
}

// typing_notifications_channel_test.rb

fn typing(room_id: i64, thread_id: Option<i64>) -> String {
    match thread_id {
        Some(thread_id) => identifier(
            json!({ "channel": "TypingNotificationsChannel", "room_id": room_id, "thread_id": thread_id }),
        ),
        None => room_identifier("TypingNotificationsChannel", room_id),
    }
}

fn typing_stream(gid_param: &str) -> String {
    format!("typing_notifications:{gid_param}")
}

fn start_payload(user: &str, name: &str) -> String {
    format!(
        r#"{{"action":"start","user":{{"id":{},"name":"{name}"}}}}"#,
        id(user)
    )
}

#[tokio::test]
async fn typing_root_typing_remains_on_the_parent_room_stream() {
    let app = start().await;
    let designers = app.room("designers").await;
    app.create_thread("designers", "jz", "Thread typing").await;
    let mut kevin = app.connect("kevin").await;
    let room_typing = typing(designers.id, None);
    kevin.confirm(&room_typing).await;

    kevin
        .perform(&room_typing, json!({ "action": "start" }))
        .await;
    assert_eq!(
        kevin.next_text().await,
        delivery(&room_typing, &start_payload("kevin", "Kevin"))
    );
    assert_has_stream(
        &app,
        &mut kevin,
        &room_typing,
        &typing_stream(&room_gid(&designers).to_param()),
    )
    .await;
}

#[tokio::test]
async fn typing_thread_typing_uses_a_separate_stream_and_never_broadcasts_into_the_channel() {
    let app = start().await;
    let designers = app.room("designers").await;
    let thread = app.create_thread("designers", "jz", "Thread typing").await;
    let (mut kevin, mut room_reader) = (app.connect("kevin").await, app.connect("jz").await);
    let thread_typing = typing(designers.id, Some(thread));
    let room_typing = typing(designers.id, None);
    kevin.confirm(&thread_typing).await;
    room_reader.confirm(&room_typing).await;

    kevin
        .perform(&thread_typing, json!({ "action": "start" }))
        .await;
    assert_eq!(
        kevin.next_text().await,
        delivery(&thread_typing, &start_payload("kevin", "Kevin"))
    );
    room_reader.assert_silent().await;

    // Kevin streams the thread's broadcasting and not the room's.
    assert_has_stream(
        &app,
        &mut kevin,
        &thread_typing,
        &typing_stream(&thread_gid(thread).to_param()),
    )
    .await;
    assert_eq!(
        app.server
            .broadcast(&typing_stream(&room_gid(&designers).to_param()), &json!({})),
        1,
        "only the room reader"
    );
}

#[tokio::test]
async fn typing_a_thread_cannot_be_subscribed_through_another_room() {
    let app = start().await;
    let thread = app.create_thread("designers", "jz", "Thread typing").await;
    let mut kevin = app.connect("kevin").await;
    kevin.reject(&typing(id("watercooler"), Some(thread))).await;
    // Through a room kevin is in, so only the thread's room decides.
    kevin.reject(&typing(id("hq"), Some(thread))).await;
    kevin.reject(&typing(id("designers"), Some(-1))).await;
}

#[tokio::test]
async fn typing_revoked_membership_stops_typing_and_prevents_reconnection() {
    let app = start().await;
    let designers = app.room("designers").await;
    let thread = app.create_thread("designers", "jz", "Thread typing").await;
    let (mut kevin, mut jz) = (app.connect("kevin").await, app.connect("jz").await);
    let thread_typing = typing(designers.id, Some(thread));
    kevin.confirm(&thread_typing).await;
    jz.confirm(&thread_typing).await;

    // `@room.memberships.revoke_from(@user)` on a stubbed connection, which nothing disconnects.
    app.sql(
        "DELETE FROM memberships WHERE room_id = ? AND user_id = ?",
        vec![int(designers.id), int(id("kevin"))],
    )
    .await;
    kevin
        .perform(&thread_typing, json!({ "action": "start" }))
        .await;
    jz.assert_silent().await;
    kevin.assert_silent().await;
    kevin
        .perform(&thread_typing, json!({ "action": "stop" }))
        .await;
    jz.assert_silent().await;
    kevin.assert_silent().await;

    kevin.unsubscribe(&thread_typing).await;
    kevin.reject(&thread_typing).await;
}

// room_messages_channel_test.rb

fn room_messages(signed: impl Into<serde_json::Value>) -> String {
    identifier(json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed.into() }))
}

fn stock_turbo(signed: impl Into<serde_json::Value>) -> String {
    identifier(json!({ "channel": "Turbo::StreamsChannel", "signed_stream_name": signed.into() }))
}

#[tokio::test]
async fn room_messages_a_member_may_subscribe_to_a_rooms_message_stream() {
    let app = start().await;
    let designers = app.room("designers").await;
    let stream = format!("{}:messages", room_gid(&designers).to_param());
    let subscription =
        room_messages(app.signed_stream_name(&[&room_gid(&designers).to_param(), "messages"]));
    let mut kevin = app.connect("kevin").await;
    kevin.confirm(&subscription).await;
    assert_has_stream(&app, &mut kevin, &subscription, &stream).await;
}

#[tokio::test]
async fn room_messages_a_user_who_was_never_a_member_may_not_subscribe() {
    let app = start().await;
    let designers = app.room("designers").await;
    let mut bender = app.connect("bender").await;
    bender
        .reject(&room_messages(app.signed_stream_name(&[
            &room_gid(&designers).to_param(),
            "messages",
        ])))
        .await;
}

#[tokio::test]
async fn room_messages_a_revoked_member_may_not_resubscribe_with_a_stream_name_harvested_while_a_member()
 {
    let app = start().await;
    let designers = app.room("designers").await;
    let subscription =
        room_messages(app.signed_stream_name(&[&room_gid(&designers).to_param(), "messages"]));
    let mut kevin = app.connect("kevin").await;
    kevin.confirm(&subscription).await;

    let room = designers.clone();
    app.db
        .write(move |tx| room.revoke_from(tx, &[id("kevin")]))
        .await
        .unwrap();
    kevin.until_closed().await;

    let mut kevin = app.connect("kevin").await;
    kevin.reject(&subscription).await;
}

#[tokio::test]
async fn room_messages_an_unsigned_stream_name_is_rejected() {
    let app = start().await;
    let designers = app.room("designers").await;
    let mut kevin = app.connect("kevin").await;
    kevin
        .reject(&room_messages(format!(
            "{}:messages",
            room_gid(&designers).to_param()
        )))
        .await;
}

#[tokio::test]
async fn room_messages_a_missing_stream_name_is_rejected() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    kevin.reject(&channel("RoomMessagesChannel")).await;
}

#[tokio::test]
async fn room_messages_a_validly_signed_stream_name_for_another_room_the_user_isnt_in_is_rejected()
{
    let app = start().await;
    let hq = app.room("hq").await;
    let mut bender = app.connect("bender").await;
    bender
        .reject(&room_messages(
            app.signed_stream_name(&[&room_gid(&hq).to_param(), "messages"]),
        ))
        .await;
}

#[tokio::test]
async fn room_messages_the_stock_turbo_channel_refuses_to_serve_a_room_message_stream() {
    let app = start().await;
    let designers = app.room("designers").await;
    let mut kevin = app.connect("kevin").await;
    kevin
        .reject(&stock_turbo(app.signed_stream_name(&[
            &room_gid(&designers).to_param(),
            "messages",
        ])))
        .await;
}

#[tokio::test]
async fn room_messages_the_stock_turbo_channel_still_serves_the_room_list_stream() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    let rooms = stock_turbo(app.signed_stream_name(&["rooms"]));
    kevin.confirm(&rooms).await;
    assert_has_stream(&app, &mut kevin, &rooms, "rooms").await;
}

#[tokio::test]
async fn thread_messages_a_parent_room_member_may_subscribe_to_a_thread_message_stream() {
    let app = start().await;
    let thread = app
        .create_thread("designers", "jz", "Guarded thread stream")
        .await;
    let gid = thread_gid(thread).to_param();
    let subscription = room_messages(app.signed_stream_name(&[&gid, "messages"]));
    let mut kevin = app.connect("kevin").await;
    kevin.confirm(&subscription).await;
    assert_has_stream(&app, &mut kevin, &subscription, &format!("{gid}:messages")).await;
}

#[tokio::test]
async fn thread_messages_an_outsider_may_not_subscribe_to_a_thread_message_stream() {
    let app = start().await;
    let thread = app
        .create_thread("designers", "jz", "Guarded thread stream")
        .await;
    let mut bender = app.connect("bender").await;
    bender
        .reject(&room_messages(app.signed_stream_name(&[
            &thread_gid(thread).to_param(),
            "messages",
        ])))
        .await;
}

#[tokio::test]
async fn thread_messages_a_revoked_parent_room_member_may_not_resubscribe_to_a_harvested_thread_stream()
 {
    let app = start().await;
    let thread = app
        .create_thread("designers", "jz", "Guarded thread stream")
        .await;
    let subscription =
        room_messages(app.signed_stream_name(&[&thread_gid(thread).to_param(), "messages"]));
    let mut kevin = app.connect("kevin").await;
    kevin.confirm(&subscription).await;

    let designers = app.room("designers").await;
    app.db
        .write(move |tx| designers.revoke_from(tx, &[id("kevin")]))
        .await
        .unwrap();
    kevin.until_closed().await;

    let mut kevin = app.connect("kevin").await;
    kevin.reject(&subscription).await;
}

#[tokio::test]
async fn thread_messages_the_stock_turbo_channel_refuses_a_thread_message_stream() {
    let app = start().await;
    let thread = app
        .create_thread("designers", "jz", "Stock stream guard")
        .await;
    let mut kevin = app.connect("kevin").await;
    let gid = thread_gid(thread).to_param();
    kevin
        .reject(&stock_turbo(app.signed_stream_name(&[&gid, "messages"])))
        .await;
    // `<gid>:threads`, the thread list's stream, is guarded the same way.
    kevin
        .reject(&stock_turbo(app.signed_stream_name(&[
            &room_gid(&app.room("designers").await).to_param(),
            "threads",
        ])))
        .await;
}

// workspace_presence_channel_test.rb

fn workspace_presence() -> String {
    channel("WorkspacePresenceChannel")
}

/// David connected with a fresh verified session, like `sessions(:david_safari)` once verified.
async fn david_with_session(app: &TestApp) -> (Client, campfire_db::Session) {
    let session = app.session_for("david", true).await;
    (app.connect_with_session(&session).await, session)
}

#[tokio::test]
async fn workspace_presence_subscribing_establishes_a_server_owned_presence_lease() {
    let app = start().await;
    let (mut david, session) = david_with_session(&app).await;
    assert!(app.leases().await.is_empty());
    david.confirm(&workspace_presence()).await;

    let leases = app.leases().await;
    assert_eq!(leases.len(), 1);
    assert_eq!(
        (leases[0].user_id, leases[0].session_id),
        (id("david"), session.id)
    );
    let uuid = &leases[0].connection_id;
    assert!(
        uuid.len() == 36 && uuid.chars().all(|c| c.is_ascii_hexdigit() || c == '-'),
        "{uuid}"
    );
}

#[tokio::test]
async fn workspace_presence_unsubscribing_deletes_only_this_connections_lease() {
    let app = start().await;
    let (mut david, session) = david_with_session(&app).await;
    let session_id = session.id;
    let other = app
        .db
        .write(move |tx| {
            campfire_db::WorkspacePresenceLease::establish(tx, id("david"), session_id)
        })
        .await
        .unwrap()
        .unwrap();
    david.confirm(&workspace_presence()).await;
    let leases = app.leases().await;
    let channel_lease = leases.iter().find(|lease| lease.id != other.id).unwrap().id;

    david.unsubscribe(&workspace_presence()).await;
    eventually("workspace presence unsubscribe cleanup", || async {
        app.leases()
            .await
            .iter()
            .all(|lease| lease.id != channel_lease)
    })
    .await;
    assert_eq!(
        app.leases()
            .await
            .iter()
            .map(|lease| lease.id)
            .collect::<Vec<_>>(),
        vec![other.id]
    );
}

#[tokio::test]
async fn workspace_presence_heartbeat_extends_the_lease() {
    let app = start().await;
    let (mut david, _) = david_with_session(&app).await;
    david.confirm(&workspace_presence()).await;
    let initial = app.leases().await[0].clone();

    app.clock.travel(jiff::SignedDuration::from_secs(25));
    david
        .perform(&workspace_presence(), json!({ "action": "heartbeat" }))
        .await;
    eventually("workspace lease expiry refresh", || async { app.leases().await[0].expires_at > initial.expires_at }).await;
    // A heartbeat without `active: true` leaves the activity stamp; one with it moves it.
    assert_eq!(app.leases().await[0].last_active_at, initial.last_active_at);
    david
        .perform(
            &workspace_presence(),
            json!({ "action": "heartbeat", "active": true }),
        )
        .await;
    eventually("workspace lease activity refresh", || async { app.leases().await[0].last_active_at > initial.last_active_at }).await;
}

/// Puts back a deleted session row with its id, so that a subscription that wasn't rejected
/// could establish a lease again.
async fn restore_session(app: &TestApp, session: &campfire_db::Session) {
    let now = app.clock.now().to_db();
    app.sql(
        "INSERT INTO sessions (id, user_id, token, last_active_at, two_factor_verified_at, created_at, updated_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
        vec![int(session.id), int(session.user_id), text(&session.token), text(&now), text(&now), text(&now), text(&now)],
    )
    .await;
}

/// A rejection inside an action sends nothing; afterwards the subscription performs no actions
/// (`processable_action?` is false once `subscription_rejected?`), so with the session back a
/// heartbeat establishes nothing.
async fn assert_rejected_in_action(
    app: &TestApp,
    david: &mut Client,
    session: &campfire_db::Session,
) {
    restore_session(app, session).await;
    david
        .perform(&workspace_presence(), json!({ "action": "heartbeat" }))
        .await;
    // The connection handles frames in order: once this confirms, the heartbeat has run.
    david.confirm(&channel("HeartbeatChannel")).await;
    assert!(
        app.leases().await.is_empty(),
        "a rejected subscription performed the heartbeat"
    );
    david.assert_silent().await;
}

#[tokio::test]
async fn workspace_presence_heartbeat_rejects_and_deletes_presence_after_session_revocation() {
    let app = start().await;
    let (mut david, session) = david_with_session(&app).await;
    david.confirm(&workspace_presence()).await;
    let lease = app.leases().await[0].clone();
    // `Session.delete(@session.id)`: the foreign key cascades to the lease, as in Rails.
    app.sql("DELETE FROM sessions WHERE id = ?", vec![int(session.id)])
        .await;

    david
        .perform(&workspace_presence(), json!({ "action": "heartbeat" }))
        .await;
    david.confirm(&channel("ApplicationCable::Channel")).await;
    assert!(app.leases().await.iter().all(|l| l.id != lease.id));
    assert_rejected_in_action(&app, &mut david, &session).await;
}

#[tokio::test]
async fn workspace_presence_heartbeat_destroys_an_idle_timed_out_administrator_session_and_rejects()
{
    let app = start().await;
    let (mut david, session) = david_with_session(&app).await;
    david.confirm(&workspace_presence()).await;
    let lease = app.leases().await[0].clone();
    let eight_days_ago = app
        .clock
        .now()
        .ago(jiff::SignedDuration::from_hours(8 * 24))
        .to_db();
    app.sql(
        "UPDATE sessions SET last_active_at = ? WHERE id = ?",
        vec![text(&eight_days_ago), int(session.id)],
    )
    .await;

    david
        .perform(&workspace_presence(), json!({ "action": "heartbeat" }))
        .await;
    david.confirm(&channel("ApplicationCable::Channel")).await;
    assert!(!app.session_exists(session.id).await);
    assert!(app.leases().await.iter().all(|l| l.id != lease.id));
    assert_rejected_in_action(&app, &mut david, &session).await;
}

#[tokio::test]
async fn workspace_presence_heartbeat_replaces_a_valid_lease_that_was_pruned() {
    let app = start().await;
    let (mut david, _) = david_with_session(&app).await;
    david.confirm(&workspace_presence()).await;
    let original = app.leases().await[0].clone();
    app.sql(
        "DELETE FROM workspace_presence_leases WHERE id = ?",
        vec![int(original.id)],
    )
    .await;

    david
        .perform(&workspace_presence(), json!({ "action": "heartbeat" }))
        .await;
    eventually("old workspace lease removal", || async { app.leases().await.len() == 1 }).await;
    let replacement = app.leases().await[0].clone();
    assert_ne!(replacement.id, original.id);

    // Not rejected: it keeps performing, and replaces the lease again.
    app.sql(
        "DELETE FROM workspace_presence_leases WHERE id = ?",
        vec![int(replacement.id)],
    )
    .await;
    david
        .perform(&workspace_presence(), json!({ "action": "heartbeat" }))
        .await;
    eventually("workspace resubscription lease replacement", || async { app.leases().await.len() == 1 }).await;
}

/// `current_session: nil` can't happen over the wire (no session, no connection); the nearest is
/// a session revoked after the connection opened.
#[tokio::test]
async fn workspace_presence_rejects_a_subscription_without_a_session() {
    let app = start().await;
    let (mut david, session) = david_with_session(&app).await;
    app.sql("DELETE FROM sessions WHERE id = ?", vec![int(session.id)])
        .await;
    david.reject(&workspace_presence()).await;
    assert!(app.leases().await.is_empty());
}

// application_cable/connection_test.rb

const UNAUTHORIZED: &str = r#"{"type":"disconnect","reason":"unauthorized","reconnect":false}"#;

async fn assert_reject_connection(app: &TestApp, cookie: Option<&str>) {
    let mut client = app.connect_with_cookie(cookie).await;
    assert_eq!(client.until_closed().await, vec![UNAUTHORIZED.to_string()]);
}

/// A session row for `user` that never completed two-step sign-in, like
/// `users(:david).sessions.create!(user_agent: "Test", ip_address: "1.2.3.4")`.
async fn unverified_session(app: &TestApp, user: &str) -> campfire_db::Session {
    app.session_for(user, false).await
}

/// `enroll_two_factor!`: a confirmed credential. The connection never reads the secret.
async fn enroll_two_factor(app: &TestApp, user: &str) {
    let now = app.clock.now().to_db();
    app.sql(
        "INSERT INTO two_factor_credentials (user_id, secret, confirmed_at, created_at, updated_at) VALUES (?, 'test-only', ?, ?, ?)",
        vec![int(id(user)), text(&now), text(&now), text(&now)],
    )
    .await;
}

#[tokio::test]
async fn connection_connects_with_a_valid_session_cookie() {
    let app = start().await;
    let token = "AxJs94fteQ5Autv2VrKsH68c"; // sessions(:david_safari)
    app.sql(
        "UPDATE sessions SET two_factor_verified_at = created_at WHERE token = ?",
        vec![text(token)],
    )
    .await;
    let mut client = app
        .connect_with_cookie(Some(&app.cookie_with_token(token)))
        .await;
    assert_eq!(client.next_text().await, r#"{"type":"welcome"}"#);
    // current_user is david, and current_session david_safari (its lease names both).
    client.confirm(&workspace_presence()).await;
    let session_id = app
        .db
        .read(move |conn| {
            Ok(campfire_db::Session::find_by_token(conn, token)?
                .unwrap()
                .id)
        })
        .await
        .unwrap();
    let lease = app.leases().await[0].clone();
    assert_eq!((lease.user_id, lease.session_id), (id("david"), session_id));
}

#[tokio::test]
async fn connection_rejects_unenrolled_humans_whose_session_never_completed_two_step_sign_in() {
    let app = start().await;
    let session = unverified_session(&app, "david").await;
    assert_reject_connection(&app, Some(&app.cookie_with_token(&session.token))).await;
    // The fixture session never completed it either.
    assert_reject_connection(
        &app,
        Some(&app.cookie_with_token("AxJs94fteQ5Autv2VrKsH68c")),
    )
    .await;
}

#[tokio::test]
async fn connection_rejects_stale_unverified_sessions_of_enrolled_users() {
    let app = start().await;
    enroll_two_factor(&app, "david").await;
    let session = unverified_session(&app, "david").await;
    assert_reject_connection(&app, Some(&app.cookie_with_token(&session.token))).await;
}

#[tokio::test]
async fn connection_connects_enrolled_users_whose_session_completed_two_step_sign_in() {
    let app = start().await;
    enroll_two_factor(&app, "david").await;
    let session = app.session_for("david", true).await;
    let mut david = app.connect_with_session(&session).await;
    david.confirm(&channel("HeartbeatChannel")).await;
}

#[tokio::test]
async fn connection_rejects_a_missing_session_cookie() {
    let app = start().await;
    assert_reject_connection(&app, None).await;
}

#[tokio::test]
async fn connection_rejects_an_invalid_session_cookie() {
    let app = start().await;
    assert_reject_connection(&app, Some(&app.cookie_with_token("-1"))).await;
}

#[tokio::test]
async fn connection_rejects_and_destroys_an_expired_administrator_session() {
    let app = start().await;
    let session = app.session_for("david", true).await;
    // The connection checks expiry on the process clock.
    let eight_days_ago = system_now()
        .ago(jiff::SignedDuration::from_hours(8 * 24))
        .to_db();
    app.sql(
        "UPDATE sessions SET last_active_at = ? WHERE id = ?",
        vec![text(&eight_days_ago), int(session.id)],
    )
    .await;
    assert_reject_connection(&app, Some(&app.cookie_with_token(&session.token))).await;
    assert!(!app.session_exists(session.id).await);
}

#[tokio::test]
async fn connection_rejects_a_revoked_session() {
    let app = start().await;
    let session = app.session_for("david", true).await;
    let revoked = session.clone();
    app.db.write(move |tx| revoked.destroy(tx)).await.unwrap();
    assert_reject_connection(&app, Some(&app.cookie_with_token(&session.token))).await;
}

#[tokio::test]
async fn connection_connects_an_idle_member_session_of_any_age() {
    let app = start().await;
    let session = app.session_for("kevin", true).await;
    let a_year_ago = system_now()
        .ago(jiff::SignedDuration::from_hours(365 * 24))
        .to_db();
    app.sql(
        "UPDATE sessions SET last_active_at = ? WHERE id = ?",
        vec![text(&a_year_ago), int(session.id)],
    )
    .await;
    let mut kevin = app.connect_with_session(&session).await;
    kevin.confirm(&channel("HeartbeatChannel")).await;
    assert!(app.session_exists(session.id).await);
}
