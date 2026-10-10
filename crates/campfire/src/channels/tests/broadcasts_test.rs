use super::support::*;
use serde_json::json;
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
