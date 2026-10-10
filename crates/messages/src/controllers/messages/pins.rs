//! `app/controllers/messages/pins_controller.rb`.
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions, require_current_user};
use crate::controllers::message_features as features;
use crate::controllers::presenters::page::db_error;
use campfire_db::MessagePin;
use campfire_kit::{Ctx, Result, StatusCode, format};

pub async fn create(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let message = features::reachable_message(c).await?;
    c.start_action();
    let pinner = require_current_user(c)?.id;
    let room_id = message.room_id;
    let pinned = c
        .app()
        .db
        .write(
            move |tx| MessagePin::pin(tx, &message, pinner),
        )
        .await;
    // Preserve the cap outcome alongside the count; a cap refusal has no writes or frames.
    if let Err(cap) = pinned.map_err(db_error)? {
        return match c.respond_to(&[&format::HTML, &format::TURBO_STREAM, &format::JSON])? {
            f if *f == format::JSON => c.json(
                StatusCode::UNPROCESSABLE_ENTITY,
                &serde_json::json!({"error": cap.0}),
            ),
            f if *f == format::TURBO_STREAM => Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY)),
            _ => features::redirect(
                c,
                &campfire_routes::room(room_id),
                None,
                Some(cap.0),
                true,
                None,
            ),
        };
    }
    pin_response(c, room_id, true).await
}

pub async fn destroy(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let message = features::reachable_message(c).await?;
    c.start_action();
    let room_id = message.room_id;
    c.app()
        .db
        .write(
            move |tx| {
                if let Some(pin) = MessagePin::find_by_message(tx.conn(), message.id)? {
                    pin.unpin(tx)?;
                }
                Ok(())
            },
        )
        .await
        .map_err(db_error)?;
    pin_response(c, room_id, false).await
}

async fn pin_response(c: &mut Ctx, room_id: i64, pinned: bool) -> Result {
    match c.respond_to(&[&format::HTML, &format::TURBO_STREAM, &format::JSON])? {
        f if *f == format::JSON => {
            let count = c
                .app()
                .db
                .read(move |conn| MessagePin::count_for_room(conn, room_id))
                .await
                .map_err(db_error)?;
            c.json(
                if pinned {
                    StatusCode::CREATED
                } else {
                    StatusCode::OK
                },
                &serde_json::json!({"pinned": pinned, "pin_count": count}),
            )
        }
        f if *f == format::TURBO_STREAM && pinned => Ok(c.head(StatusCode::OK)),
        f if *f == format::TURBO_STREAM => features::redirect(
            c,
            &c.url_for(&campfire_routes::room_pins(room_id)),
            None,
            None,
            false,
            Some(StatusCode::SEE_OTHER),
        ),
        _ => features::redirect(
            c,
            &campfire_routes::room(room_id),
            Some(
                if pinned {
                    "Message pinned"
                } else {
                    "Message unpinned"
                }
                .into(),
            ),
            None,
            true,
            None,
        ),
    }
}
