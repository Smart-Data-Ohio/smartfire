//! A person's profile banner by their avatar token: served as the workspace banner is, the
//! original of an animated image, its first frame for `still=1`, or a bounded resize. A token
//! for someone no longer in the database finds nothing, as their avatar's does.

use campfire_db::User;
use campfire_kit::{Ctx, Error, Result};

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::accounts::banners::send_image;
use crate::controllers::presenters;

pub async fn show(c: &mut Ctx) -> Result {
    c.use_live_response();
    concerns::before_actions(c, Before::default()).await?;
    let token = c.param_str("user_id").unwrap_or_default().to_string();
    let id = presenters::accounts::user_id_from_avatar_token(&c.app().secrets, &token, c.now())
        .ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| User::find_by_id(conn, id))
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)?;
    send_image(c, "User", id, "banner", format!("user-{id}-banner")).await
}
