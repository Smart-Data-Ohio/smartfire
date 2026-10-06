//! Admin-only read-only health page; all data comes from the integration domain.
use crate::{
    app::AppCtx,
    concerns::{self, Before},
    controllers::presenters::page::framed_page,
};
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)?;
    c.respond_to(&[&format::HTML])?;
    let now = campfire_db::Timestamp::from_jiff(c.now());
    let snapshot = c
        .app()
        .db
        .read(move |conn| {
            crate::integrations::health::snapshot(conn, now, |key| std::env::var(key).ok())
        })
        .await
        .map_err(Error::internal)?;
    framed_page!(c, StatusCode::OK, |ctx| {
        campfire_views::integration_health::Show {
            ctx,
            snapshot: snapshot.clone(),
        }
    })
    .await
}
