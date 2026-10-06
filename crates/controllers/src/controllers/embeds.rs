//! Signed link-preview images (`app/controllers/embeds/images_controller.rb`).
use crate::app::AppCtx;
use crate::concerns::{Before, before_actions};
use crate::integrations::{image_proxy, net::Network};
use campfire_kit::{Ctx, ExpiresIn, Result, SendOptions, StatusCode};

pub async fn show(c: &mut Ctx) -> Result {
    before_actions(c, Before::default()).await?;
    let signed = c.param_str("signed").unwrap_or("");
    let Some(url) = image_proxy::verified_url(&c.app().secrets, signed, c.now()) else {
        return Ok(c.head(StatusCode::NOT_FOUND));
    };
    let net = c.current::<Network>().cloned().unwrap_or_else(Network::system);
    match image_proxy::fetch(&net, &url).await {
        Ok(image) => {
            c.expires_in(3600, ExpiresIn::default());
            Ok(c.send_data(image.body, SendOptions::inline(&image.content_type)))
        }
        Err(error) => Ok(c.head(if error.denied() { StatusCode::NOT_FOUND } else { StatusCode::BAD_GATEWAY })),
    }
}
