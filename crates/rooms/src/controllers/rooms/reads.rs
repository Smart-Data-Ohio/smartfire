//! `app/controllers/rooms/reads_controller.rb`: room-scoped read pointers and session broadcasts.
use campfire_db::{Message, Timeline};
use campfire_kit::{Ctx, Error, Result, StatusCode};
use serde::Serialize;
use crate::app::AppCtx;
use crate::concerns::{self, Before, before_actions, cast_integer, require_current_user};
use crate::controllers::presenters::page::db_error;

#[derive(Serialize)]
struct ReadState {
    room_id: i64,
    unread: bool,
    #[serde(skip_serializing_if="Option::is_none")]
    first_unread_message_id: Option<i64>,
}

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (mut membership, room) = concerns::set_room(c).await?;
    c.app().db.write(move |tx| membership.read(tx)).await.map_err(db_error)?;
    crate::channels::broadcasts::read_room(&c.app().cable, require_current_user(c)?.id, room.id);
    c.json(StatusCode::OK, &ReadState {room_id:room.id,unread:false,first_unread_message_id:None})
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (mut membership, room) = concerns::set_room(c).await?;
    let message_id = c.param_str("message_id").and_then(cast_integer).ok_or(Error::NotFound)?;
    let room_id = room.id;
    c.app().db.write(move |tx| {
        let message=Message::find_in(tx.conn(),Timeline::Room(room_id),message_id)?;
        membership.mark_unread_before(tx,&message)
    }).await.map_err(db_error)?;
    c.app().broadcasts.mark_room_unread(require_current_user(c)?.id,room.id);
    c.json(StatusCode::OK, &ReadState {room_id:room.id,unread:true,first_unread_message_id:Some(message_id)})
}
