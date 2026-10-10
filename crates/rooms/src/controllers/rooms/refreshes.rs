//! Legacy refresh requests resolve to the SPA conversation.
use campfire_kit::{Ctx, Result};
use crate::concerns::{self, Before, before_actions};
pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let (_, room) = concerns::set_room(c).await?;
    c.redirect_to(&c.url_for(&campfire_routes::room(room.id)))
}
