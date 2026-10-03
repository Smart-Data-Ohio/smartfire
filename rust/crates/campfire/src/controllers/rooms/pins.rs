//! `app/controllers/rooms/pins_controller.rb`.
use crate::app::{App, AppCtx};
use crate::concerns::{Before, before_actions};
use crate::controllers::message_features;
use crate::controllers::presenters::{Presenter, page};
use askama::Template;
use campfire_db::{Message, MessagePin, Room, User};
use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn index(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let room = message_features::room(c).await?;
    c.start_action();
    c.respond_to(&[&format::HTML])?;
    let app = c.app().clone();
    let list = c
        .app()
        .db
        .read(move |conn| list(conn, &app, &room))
        .await
        .map_err(page::db_error)?;
    page::content(c, StatusCode::OK, |ctx| {
        campfire_views::pins::Index { ctx, list: &list }.render()
    })
    .await
}

pub(crate) fn list(
    conn: &campfire_db::Connection,
    app: &App,
    room: &Room,
) -> campfire_db::Result<campfire_views::pins::List> {
    let records = MessagePin::ordered_for_room(conn, room.id)?;
    if records.is_empty() {
        return Ok(campfire_views::pins::List { room_id: room.id,
            room_param_key: campfire_db::broadcasts::room_param_key(room.room_type), pins: vec![] });
    }
    let ids: Vec<_> = records.iter().map(|pin|pin.message_id).collect();
    let messages = Message::for_ids(conn,&ids)?;
    let users: std::collections::HashMap<_,_> = User::where_ids(conn,
        &records.iter().map(|pin|pin.pinner_id).chain(messages.iter().map(|m|m.creator_id)).collect::<Vec<_>>())?
        .into_iter().map(|u|(u.id,u)).collect();
    let presenter = Presenter::new(conn, app, None).preload_plain_text(&messages)?;
    let messages: std::collections::HashMap<_,_> = messages.into_iter().map(|m|(m.id,m)).collect();
    let pins = records
        .into_iter()
        .map(|pin| {
            let message = messages.get(&pin.message_id).ok_or(campfire_db::Error::RecordNotFound("Message"))?;
            Ok(campfire_views::pins::Pin {
                message_id: message.id,
                pinner_name: users.get(&pin.pinner_id).ok_or(campfire_db::Error::RecordNotFound("User"))?.name.clone(),
                author_name: users.get(&message.creator_id).ok_or(campfire_db::Error::RecordNotFound("User"))?.name.clone(),
                excerpt: campfire_views::helpers::truncate(
                    &presenter.plain_text_body(message)?,
                    200,
                    "...",
                ),
                created_at: pin.created_at.jiff(),
                message_path: campfire_db::message_pin::message_path(message),
            })
        })
        .collect::<campfire_db::Result<_>>()?;
    Ok(campfire_views::pins::List {
        room_id: room.id,
        room_param_key: campfire_db::broadcasts::room_param_key(room.room_type),
        pins,
    })
}
