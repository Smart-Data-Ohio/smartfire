//! The broadcasts' streams, targets and markup, as delivered to subscribers.
use serde_json::{Value, json};

use super::support::*;
use crate::channels::{room_gid, user_gid};

/// The `<turbo-stream>` a delivery frame carries.
fn turbo_stream(frame: &str) -> String {
    let frame: Value = serde_json::from_str(frame).unwrap();
    frame["message"].as_str().unwrap_or_else(|| panic!("expected a Turbo Stream string: {frame}")).to_string()
}

async fn room_messages(app: &TestApp, client: &mut Client, room: &str) -> String {
    let room = app.room(room).await;
    let signed = app.signed_stream_name(&[&room_gid(&room).to_param(), "messages"]);
    let channel = identifier(json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }));
    client.confirm(&channel).await;
    channel
}

async fn turbo(app: &TestApp, client: &mut Client, streamables: &[&str]) -> String {
    let channel = identifier(json!({ "channel": "Turbo::StreamsChannel", "signed_stream_name": app.signed_stream_name(streamables) }));
    client.confirm(&channel).await;
    channel
}

#[tokio::test]
async fn message_broadcasts() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    let messages = room_messages(&app, &mut kevin, "designers").await;
    let unreads = identifier(json!({ "channel": "UnreadRoomsChannel" }));
    kevin.confirm(&unreads).await;

    let designers = app.room("designers").await;
    let message = app.message("second").await;

    app.message_create(&designers, &message).await;
    kevin.assert_texts(&[
        delivery(&messages, &html_json(&format!(
            r#"<turbo-stream action="append" target="messages_rooms_closed_{}"><template><div id="message_0002">message {}</div></template></turbo-stream>"#,
            designers.id, message.id
        ))),
        delivery(&unreads, &format!(r#"{{"roomId":{}}}"#, designers.id)),
    ]).await;

    app.broadcasts.message_replace(&designers, &message, &FakePartials);
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(
            r#"<turbo-stream maintain_scroll="true" action="replace" target="presentation_message_0002"><template><div>presentation {} & more</div></template></turbo-stream>"#,
            message.id
        )
    );

    app.broadcasts.message_remove(&designers, &message);
    assert_eq!(turbo_stream(&kevin.next_text().await), r#"<turbo-stream action="remove" target="message_0002"></turbo-stream>"#);
    kevin.assert_silent().await;
}

#[tokio::test]
async fn boost_broadcasts() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    room_messages(&app, &mut kevin, "designers").await;
    let designers = app.room("designers").await;
    let message = app.message("first").await;
    let boost = app.boost("first").await;

    app.broadcasts.boost_create(&designers, &message, &boost, &FakePartials);
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(
            r#"<turbo-stream maintain_scroll="true" action="append" target="boosts_message_0001"><template><div>boost {}</div></template></turbo-stream>"#,
            boost.id
        )
    );

    app.broadcasts.boost_remove(&designers, &message, &boost);
    assert_eq!(turbo_stream(&kevin.next_text().await), format!(r#"<turbo-stream action="remove" target="boost_{}"></turbo-stream>"#, boost.id));
}

#[tokio::test]
async fn room_list_broadcasts() {
    let app = start().await;
    let mut jz = app.connect("jz").await;
    turbo(&app, &mut jz, &["rooms"]).await;
    let own_rooms = user_gid(id("jz")).to_param();
    turbo(&app, &mut jz, &[&own_rooms, "rooms"]).await;

    let hq = app.room("hq").await;
    app.broadcasts.open_room_create(&hq, &FakePartials);
    assert_eq!(
        turbo_stream(&jz.next_text().await),
        format!(r#"<turbo-stream action="prepend" target="shared_rooms"><template><li>shared {}</li></template></turbo-stream>"#, hq.id)
    );

    app.broadcasts.open_room_update(&hq, &FakePartials, Some("<h1>header</h1>"));
    assert_eq!(
        turbo_stream(&jz.next_text().await),
        format!(r#"<turbo-stream action="replace" target="list_rooms_open_{0}"><template><li>shared {0}</li></template></turbo-stream>"#, hq.id)
    );
    assert_eq!(
        turbo_stream(&jz.next_text().await),
        format!(r#"<turbo-stream action="replace" target="header_rooms_open_{0}"><template><h1>header</h1></template></turbo-stream>"#, hq.id)
    );

    app.broadcasts.room_remove(&hq);
    assert_eq!(turbo_stream(&jz.next_text().await), format!(r#"<turbo-stream action="remove" target="list_rooms_open_{}"></turbo-stream>"#, hq.id));

    // Closed rooms go to each member's own stream: jz is in designers, not the watercooler.
    let designers = app.room("designers").await;
    let watercooler = app.room("watercooler").await;
    let broadcasts = app.broadcasts.clone();
    let (d, w) = (designers.clone(), watercooler.clone());
    app.db
        .read(move |conn| {
            broadcasts.closed_room_create(conn, &w, &FakePartials)?;
            broadcasts.closed_room_create(conn, &d, &FakePartials)?;
            broadcasts.closed_room_update(conn, &d, &FakePartials, Some("<h1>header</h1>"))
        })
        .await
        .unwrap();
    assert_eq!(
        turbo_stream(&jz.next_text().await),
        format!(r#"<turbo-stream action="prepend" target="shared_rooms"><template><li>shared {}</li></template></turbo-stream>"#, designers.id)
    );
    assert_eq!(
        turbo_stream(&jz.next_text().await),
        format!(r#"<turbo-stream action="replace" target="list_rooms_closed_{0}"><template><li>shared {0}</li></template></turbo-stream>"#, designers.id)
    );
    assert_eq!(
        turbo_stream(&jz.next_text().await),
        format!(r#"<turbo-stream action="replace" target="header_rooms_closed_{0}"><template><h1>header</h1></template></turbo-stream>"#, designers.id)
    );
    jz.assert_silent().await;
}

#[tokio::test]
async fn direct_room_and_involvement_broadcasts() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    let own_rooms = user_gid(id("kevin")).to_param();
    turbo(&app, &mut kevin, &[&own_rooms, "rooms"]).await;

    let direct = app.room("bender_and_kevin").await;
    let broadcasts = app.broadcasts.clone();
    let room = direct.clone();
    app.db.read(move |conn| broadcasts.direct_room_create(conn, &room, &FakePartials)).await.unwrap();
    let membership = app.membership("bender_and_kevin", "kevin").await.unwrap();
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(r#"<turbo-stream action="prepend" target="direct_rooms"><template><li>direct {}</li></template></turbo-stream>"#, membership.id)
    );
    kevin.assert_silent().await;

    use campfire_db::Involvement::*;
    let designers = app.room("designers").await;
    let mut membership = app.membership("designers", "kevin").await.unwrap();

    // Direct rooms only redraw their row on a mute or unmute.
    let mut direct_membership = app.membership("bender_and_kevin", "kevin").await.unwrap();
    app.broadcasts.involvement_change(&direct, &direct_membership, Some(Invisible), &FakePartials);
    kevin.assert_silent().await;
    direct_membership.involvement = Some(Muted);
    app.broadcasts.involvement_change(&direct, &direct_membership, Some(Everything), &FakePartials);
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(r#"<turbo-stream action="replace" target="list_rooms_direct_{}"><template><li>direct {}</li></template></turbo-stream>"#, direct.id, direct_membership.id)
    );

    membership.involvement = Some(Invisible);
    app.broadcasts.involvement_change(&designers, &membership, Some(Mentions), &FakePartials);
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(r#"<turbo-stream action="remove" target="list_rooms_closed_{}"></turbo-stream>"#, designers.id)
    );

    membership.involvement = Some(Everything);
    app.broadcasts.involvement_change(&designers, &membership, Some(Invisible), &FakePartials);
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(r#"<turbo-stream action="prepend" target="shared_rooms"><template><li>row {} unread None</li></template></turbo-stream>"#, designers.id)
    );

    // Neither side muted, or a nil previous involvement (`nil.to_s.inquiry`): nothing to redraw.
    app.broadcasts.involvement_change(&designers, &membership, Some(Mentions), &FakePartials);
    app.broadcasts.involvement_change(&designers, &membership, None, &FakePartials);
    kevin.assert_silent().await;

    // A mute redims the row in place.
    membership.involvement = Some(Muted);
    app.broadcasts.involvement_change(&designers, &membership, Some(Everything), &FakePartials);
    assert_eq!(
        turbo_stream(&kevin.next_text().await),
        format!(r#"<turbo-stream action="replace" target="list_rooms_closed_{}"><template><li>row {} unread Some(false)</li></template></turbo-stream>"#, designers.id, designers.id)
    );
    kevin.assert_silent().await;
}

#[tokio::test]
async fn thread_messages_go_to_the_thread_stream_without_unread_pings() {
    let app = start().await;
    let designers = app.room("designers").await;
    let thread_id = app.create_thread("designers", "jz", "Broadcast thread").await;
    let mut kevin = app.connect("kevin").await;
    let signed = app.signed_stream_name(&[&crate::channels::threads::thread_gid(thread_id).to_param(), "messages"]);
    let thread_channel = identifier(json!({ "channel": "RoomMessagesChannel", "signed_stream_name": signed }));
    kevin.confirm(&thread_channel).await;
    room_messages(&app, &mut kevin, "designers").await;
    let unreads = identifier(json!({ "channel": "UnreadRoomsChannel" }));
    kevin.confirm(&unreads).await;

    let mut message = app.message("second").await;
    message.thread_id = Some(thread_id);
    app.message_create(&designers, &message).await;
    let frame = kevin.next_text().await;
    assert!(frame.starts_with(&format!(r#"{{"identifier":{}"#, campfire_cable::json::encode(&thread_channel))), "{frame}");
    assert_eq!(
        turbo_stream(&frame),
        format!(
            r#"<turbo-stream action="append" target="messages_channel_thread_{thread_id}"><template><div id="message_0002">message {}</div></template></turbo-stream>"#,
            message.id
        )
    );
    app.broadcasts.message_remove(&designers, &message);
    let frame = kevin.next_text().await;
    assert!(frame.starts_with(&format!(r#"{{"identifier":{}"#, campfire_cable::json::encode(&thread_channel))), "{frame}");
    kevin.assert_silent().await;

    // System notes append without unread pings too.
    let mut note = app.message("second").await;
    note.system_note = true;
    app.message_create(&designers, &note).await;
    assert!(turbo_stream(&kevin.next_text().await).contains(r#"target="messages_rooms_closed_"#));
    kevin.assert_silent().await;
}

#[tokio::test]
async fn unread_pings_skip_muted_members_the_message_does_not_mention() {
    let app = start().await;
    let designers = app.room("designers").await;
    let message = app.message("second").await;
    let unreads = identifier(json!({ "channel": "UnreadRoomsChannel" }));
    let mut kevin = app.connect("kevin").await;
    kevin.confirm(&unreads).await;
    let mut jz = app.connect("jz").await;
    jz.confirm(&unreads).await;
    let ping = delivery(&unreads, &format!(r#"{{"roomId":{}}}"#, designers.id));

    app.set_involvement("designers", "kevin", "muted").await;
    let (broadcasts, room, m, rich_text) = (app.broadcasts.clone(), designers.clone(), message.clone(), campfire_db::rich_text::BasicRichText);
    app.db.read(move |conn| broadcasts.unread_room(conn, &room, &m, &rich_text)).await.unwrap();
    assert_eq!(jz.next_text().await, ping);
    kevin.assert_silent().await;

    // Mentioned, a muted member is told.
    app.set_body(&message, &format!("<div>Hey {}</div>", campfire_db::rich_text::mention_attachment_for(id("kevin")))).await;
    let (broadcasts, room, m, rich_text) = (app.broadcasts.clone(), designers.clone(), message.clone(), campfire_db::rich_text::BasicRichText);
    app.db.read(move |conn| broadcasts.unread_room(conn, &room, &m, &rich_text)).await.unwrap();
    assert_eq!(jz.next_text().await, ping);
    assert_eq!(kevin.next_text().await, ping);
}

#[tokio::test]
async fn broadcast_frames_use_active_support_json_escaping() {
    let app = start().await;
    let mut kevin = app.connect("kevin").await;
    let channel = room_messages(&app, &mut kevin, "designers").await;
    let designers = app.room("designers").await;
    let message = app.message("first").await;
    app.broadcasts.message_replace(&designers, &message, &FakePartials);
    assert_eq!(
        kevin.next_text().await,
        delivery(
            &channel,
            &html_json(&format!(
                r#"<turbo-stream maintain_scroll="true" action="replace" target="presentation_message_0001"><template><div>presentation {} & more</div></template></turbo-stream>"#,
                message.id
            ))
        )
    );
}

/// Rails-generated remove and real thread-unread frames must traverse the merged event sink.
#[tokio::test]
async fn ws8_model_frames_reach_subscribers_through_ws7_sink() {
    use campfire_db::{Event, broadcasts::{Broadcast, Streamable}};
    let app = start().await;
    let mut client = app.connect("jason").await;
    let golden: Value = serde_json::from_str(include_str!("../../ws8_runtime_vectors.json")).unwrap();
    let remove = &golden["broadcasts"][0];
    let user_id = remove["user_id"].as_i64().unwrap();
    let own = user_gid(user_id).to_param();
    turbo(&app, &mut client, &[&own, "rooms"]).await;
    let unread = identifier(json!({"channel":"UnreadThreadsChannel"}));
    client.confirm(&unread).await;
    let event = Event::broadcast(&Broadcast::remove(
        vec![Streamable::User(user_id), Streamable::Name("rooms".into())],
        remove["target"].as_str().unwrap().into(),
    ));
    assert!(crate::channels::sink::deliver(&app.server, None, &event));
    assert_eq!(turbo_stream(&client.next_text().await), remove["payload"]);
    let row = &golden["broadcasts"][1];
    let event = Event::broadcast(&Broadcast::Cable {
        stream: row["stream"].as_str().unwrap().into(), payload: row["payload"].clone(),
    });
    assert!(crate::channels::sink::deliver(&app.server, None, &event));
    assert_eq!(client.next_text().await, delivery(&unread, &row["payload"].to_string()));
    client.assert_silent().await;
}

/// A real unread delivery can precede Turbo HTML on an independent subscription.
#[tokio::test]
async fn independent_broadcast_streams_accept_either_arrival_order() {
    let app = start().await;
    let mut client = app.connect("kevin").await;
    let messages = room_messages(&app, &mut client, "designers").await;
    let unreads = identifier(json!({ "channel": "UnreadRoomsChannel" }));
    client.confirm(&unreads).await;
    let room = app.room("designers").await;
    let message = app.message("second").await;
    let expected = [
        delivery(&messages, &html_json(r#"<turbo-stream action="remove" target="message_0002"></turbo-stream>"#)),
        delivery(&unreads, &format!(r#"{{"roomId":{}}}"#, room.id)),
    ];
    for unread_first in [true, false] {
        let unread = || {
            let (broadcasts, room, message) = (app.broadcasts.clone(), room.clone(), message.clone());
            app.db.read(move |conn| broadcasts.unread_room(conn, &room, &message, &campfire_db::rich_text::BasicRichText))
        };
        if unread_first {
            unread().await.unwrap();
            app.broadcasts.message_remove(&room, &message);
        } else {
            app.broadcasts.message_remove(&room, &message);
            unread().await.unwrap();
        }
        client.assert_texts(&expected).await;
        client.assert_silent().await;
    }
}
