//! Bot configuration facts shared by the API and credential adapters.
use crate::app::AppCtx;
use crate::{concerns, presenters};
use campfire_db::User;
use campfire_kit::{Ctx, Error, Result};
use campfire_presentation::accounts;

pub async fn edit_form(c: &Ctx, bot: &User) -> Result<accounts::BotForm> {
    let viewer_id = concerns::require_current_user(c)?.id;
    let zone = c
        .app()
        .db
        .read(move |conn| {
            Ok(conn.query_row(
                "SELECT time_zone FROM users WHERE id=?",
                [viewer_id],
                |row| row.get::<_, Option<String>>(0),
            )?)
        })
        .await
        .map_err(Error::internal)?;
    let zone = campfire_presentation::time::Zone::for_user(zone.as_deref());
    // `usable?` may mark an unreadable linked token disconnected. The owner
    // service performs that write outside the view reader and never returns a token.
    let id = bot.id;
    let account = c
        .app()
        .db
        .read(move |conn| crate::integrations::github::accounts::Account::for_user(conn, id))
        .await
        .map_err(Error::internal)?;
    let github_usable = if let Some(account) = account {
        c.app()
            .github_accounts
            .usable(account.id)
            .await
            .map_err(Error::internal)?
    } else {
        false
    };
    let (app, base_url, bot) = (c.app().clone(), c.url_for(""), bot.clone());
    c.app()
        .db
        .read(move |conn| {
            presenters::accounts::bot_form(conn, &app, &base_url, &bot, &zone, github_usable)
        })
        .await
        .map_err(Error::internal)
}

pub async fn viewer_zone(c: &Ctx) -> Result<campfire_presentation::time::Zone> {
    let id = concerns::require_current_user(c)?.id;
    let name = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                conn.query_row("SELECT time_zone FROM users WHERE id=?", [id], |r| {
                    r.get::<_, Option<String>>(0)
                })?,
            )
        })
        .await
        .map_err(Error::internal)?;
    Ok(campfire_presentation::time::Zone::for_user(name.as_deref()))
}
