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
    let presenter = Presenter::new(conn, app, None);
    let pins = MessagePin::ordered_for_room(conn, room.id)?
        .into_iter()
        .map(|pin| {
            let message = Message::find(conn, pin.message_id)?;
            Ok(campfire_views::pins::Pin {
                message_id: message.id,
                pinner_name: User::find(conn, pin.pinner_id)?.name,
                author_name: User::find(conn, message.creator_id)?.name,
                excerpt: campfire_views::helpers::truncate(
                    &presenter.plain_text_body(&message)?,
                    200,
                    "...",
                ),
                created_at: pin.created_at.jiff(),
                message_path: campfire_db::message_pin::message_path(&message),
            })
        })
        .collect::<campfire_db::Result<_>>()?;
    Ok(campfire_views::pins::List {
        room_id: room.id,
        room_param_key: campfire_db::broadcasts::room_param_key(room.room_type),
        pins,
    })
}
