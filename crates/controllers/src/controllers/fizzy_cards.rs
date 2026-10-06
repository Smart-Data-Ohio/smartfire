//! `Rooms::Fizzy::CardsController`: membership, message room and exact card reference gates.
use crate::{
    app::AppCtx,
    concerns::{self, Before, before_actions, cast_integer, require_current_user},
    controllers::presenters::page::{self, db_error},
    integrations::fizzy::{
        accounts::Account,
        cards::{Cache, Card},
    },
};
use campfire_db::Message;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use rails_compat::ar_encryption::ArEncryption;

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let viewer = require_current_user(c)?.id;
    let room = concerns::set_room(c).await?.1;
    let card_id = c
        .param_str("id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let message_id = c
        .param_str("message_id")
        .and_then(cast_integer)
        .ok_or(Error::NotFound)?;
    let app = c.app().clone();
    let crypto = ArEncryption::new(&app.secrets);
    let (card, cache) = app.db.write(move |tx| {
        let card = Card::find(tx.conn(), card_id)?;
        let message = Message::find(tx.conn(), message_id)?;
        let referenced: bool = tx.conn().query_row("SELECT EXISTS(SELECT 1 FROM fizzy_card_references WHERE message_id=?1 AND fizzy_card_id=?2)",rusqlite::params![message.id,card.id],|r| r.get(0))?;
        if message.room_id != room.id || !referenced { return Err(campfire_db::Error::RecordNotFound("Fizzy::CardReference")); }
        let usable = match Account::for_user(tx.conn(), viewer)? {
            Some(account) => account.usable_token(tx, &crypto)?.is_some(),
            None => false,
        };
        let cache = if usable {
            let cache = Cache::for_viewer(tx, &card, viewer)?;
            cache.request_fetch(tx)?;
            Some(cache)
        } else { None };
        Ok((card, cache))
    }).await.map_err(db_error)?;
    page::bare(c, StatusCode::OK, &format::HTML, move |ctx| {
        Ok(campfire_views::fizzy_cards::Frame {
            account: &card.account_id,
            number: card.number,
            web_url: &card.web_url(),
            id: &format!("card_for_message_{message_id}_fizzy_card_{card_id}"),
            connect: cache.is_none(),
            payload: cache.as_ref().and_then(|c| c.payload.as_ref()),
            error: cache.as_ref().and_then(|c| c.fetch_error.as_deref()),
            zone: &ctx.time_zone,
        }
        .render())
    })
    .await
}
