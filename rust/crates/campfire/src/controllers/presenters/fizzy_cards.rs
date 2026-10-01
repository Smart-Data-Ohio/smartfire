//! HTML adapter only. Shared containers contain no viewer data; payload frames are private.
use crate::{
    app::App,
    channels::broadcasts::{Stream, message_dom_id},
    integrations::fizzy::cards::Card,
};
use campfire_db::{Connection, Message, Room};

pub fn frames(conn: &Connection, message: &Message) -> campfire_db::Result<Vec<String>> {
    Ok(frames_from_cards(message, &Card::for_message(conn, message.id)?))
}
/// Render the same owner frame body from already loaded shared card rows.
pub fn frames_from_cards(message: &Message, cards: &[Card]) -> Vec<String> {
    cards.iter().map(|card| {
        let src = campfire_routes::ROOM_FIZZY_CARD.path_with(&[&message.room_id, &card.id], None, &[("message_id", Some(&message.id.to_string()))]);
        format!("\n  <turbo-frame loading=\"lazy\" class=\"fizzy-card-frame\" id=\"card_for_message_{}_fizzy_card_{}\" src=\"{}\"></turbo-frame>\n",message.id,card.id,campfire_views::helpers::escape(&src))
    }).collect()
}
pub fn container(conn: &Connection, message: &Message) -> campfire_db::Result<String> {
    Ok(format!(
        "<div id=\"{}\" class=\"fizzy-cards\">{}</div>\n",
        campfire_views::helpers::escape(&message_dom_id(message, Some("fizzy_cards"))),
        frames(conn, message)?.concat()
    ))
}
pub fn broadcast_updates(app: &App, card_id: i64) -> anyhow::Result<()> {
    let app2 = app.clone();
    app.db.read_blocking(move |conn| {
        let mut query = conn.prepare("SELECT DISTINCT message_id FROM fizzy_card_references WHERE fizzy_card_id=? ORDER BY message_id")?;
        let ids = query.query_map([card_id], |r| r.get::<_, i64>(0))?.collect::<std::result::Result<Vec<_>, _>>()?;
        for id in ids {
            let message = Message::find(conn, id)?;
            let room = Room::find(conn, message.room_id)?;
            let html = container(conn, &message)?;
            app2.broadcasts.turbo(&Stream::conversation(&room, &message),campfire_cable::turbo::Action::Replace,&message_dom_id(&message,Some("fizzy_cards")),Some(&html),true);
        }
        Ok(())
    })?;
    Ok(())
}
