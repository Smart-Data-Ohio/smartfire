//! `Accounts::BotsController` (reference/app/controllers/accounts/bots_controller.rb).

pub mod keys;

use campfire_db::{User, UserChanges};
use campfire_kit::{Ctx, Error, Param, ParamMap, Result, StatusCode, format, permit_keys};
use campfire_views::accounts;

use crate::app::AppCtx;
use crate::controllers::presenters::page::framed_page;
use crate::concerns::{self, Before, cast_integer};
use crate::controllers::presenters::attachments::{self, Assignment, Record};
use crate::controllers::presenters;

/// `@bots = User.active_bots.ordered`
pub async fn index(c: &mut Ctx) -> Result {
    before(c).await?;
    c.respond_to(&[&format::HTML])?;
    let secrets = c.app().secrets.clone();
    let bots: Vec<_> = c
        .app()
        .db
        .read(move |conn| User::active_bots_ordered(conn)?.iter().map(|bot| presenters::accounts::bot(conn, &secrets, bot)).collect())
        .await
        .map_err(Error::internal)?;
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotsIndex { ctx, bots: bots.clone() }).await
}

pub async fn new(c: &mut Ctx) -> Result {
    before(c).await?;
    c.respond_to(&[&format::HTML])?;
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotsNew { ctx, bot: accounts::BotForm::default() }).await
}

/// `User.create_bot! bot_params`
pub async fn create(c: &mut Ctx) -> Result {
    before(c).await?;
    concerns::sudo::require_sudo_mode(c)?;
    let params = bot_params(c)?;
    // users.name is NOT NULL.
    let name = params.get("name").and_then(Param::to_s).ok_or_else(|| Error::internal(anyhow::anyhow!("NOT NULL constraint failed: users.name")))?;
    // `create_webhook!(url: webhook_url) if webhook_url`: any non-nil value, "" included.
    let webhook_url = params.get("webhook_url").and_then(Param::to_s);
    let avatar = Assignment::from_params(&params, "avatar")?.stage(c.app()).await?;
    c
        .app()
        .db
        .write(move |tx| {
            let bot = User::create_bot(tx, &name, webhook_url.as_deref())?;
            attachments::assign(tx, Record::user(bot.id), "avatar", avatar)
        })
        .await
        .map_err(Error::internal)?;
    redirect_to_bots(c)
}

pub async fn edit(c: &mut Ctx) -> Result {
    before(c).await?;
    let bot = set_bot(c).await?;
    c.respond_to(&[&format::HTML])?;
    let (storage, base_url, bot_id) = (c.app().storage.clone(), c.url_for(""), bot.id);
    let form = c.app().db.read(move |conn| presenters::accounts::bot_form(conn, &storage, &base_url, &bot)).await.map_err(Error::internal)?;
    framed_page!(c, StatusCode::OK, |ctx| accounts::BotsEdit { ctx, bot_id, bot: form.clone() }).await
}

/// `@bot.update_bot! bot_params`: the webhook first, then the bot, in one transaction.
pub async fn update(c: &mut Ctx) -> Result {
    before(c).await?;
    let mut bot = set_bot(c).await?;
    // Rails gates an actual change, not the always-submitted edit field's presence.
    if let Some(user) = c.params.get("user").and_then(Param::as_hash)
        && let Some(url) = user.get("webhook_url")
    {
        let submitted = url.to_s().unwrap_or_default();
        let bot_id = bot.id;
        let existing = c.app().db.read(move |conn| campfire_db::Webhook::find_by_user(conn, bot_id)).await.map_err(Error::internal)?;
        if submitted.trim_matches(|ch: char| ch == '\0' || ch.is_ascii_whitespace()) != existing.as_ref().and_then(|webhook| webhook.url.as_deref()).unwrap_or("") {
            concerns::sudo::require_sudo_mode(c)?;
        }
    }
    let params = bot_params(c)?;
    let changes = UserChanges { name: params.get("name").and_then(Param::to_s), ..UserChanges::default() };
    let webhook_submitted = params.contains_key("webhook_url");
    let webhook_url = params.get("webhook_url").and_then(Param::to_s);
    let avatar = Assignment::from_params(&params, "avatar")?.stage(c.app()).await?;
    c
        .app()
        .db
        .write(move |tx| {
            if webhook_submitted {
                bot.update_bot(tx, changes, webhook_url.as_deref())?;
            } else {
                bot.update(tx, changes)?;
            }
            attachments::assign(tx, Record::user(bot.id), "avatar", avatar)
        })
        .await
        .map_err(Error::internal)?;
    redirect_to_bots(c)
}

/// `@bot.deactivate`
pub async fn destroy(c: &mut Ctx) -> Result {
    before(c).await?;
    let mut bot = set_bot(c).await?;
    c.app().db.write(move |tx| bot.deactivate(tx)).await.map_err(Error::internal)?;
    redirect_to_bots(c)
}

/// ApplicationController's chain, then `ensure_can_administer`.
async fn before(c: &mut Ctx) -> Result<()> {
    concerns::before_actions(c, Before::default()).await?;
    concerns::ensure_can_administer(c)
}

/// `User.active_bots.find(params[:id])`
async fn set_bot(c: &Ctx) -> Result<User> {
    find_active_bot(c, "id").await
}

pub(crate) async fn find_active_bot(c: &Ctx, key: &str) -> Result<User> {
    let id = c.param_str(key).and_then(cast_integer).ok_or(Error::NotFound)?;
    c.app()
        .db
        .read(move |conn| match User::find_active_bot(conn, id) {
            Ok(bot) => Ok(Some(bot)),
            Err(campfire_db::Error::RecordNotFound(_)) => Ok(None),
            Err(error) => Err(error),
        })
        .await
        .map_err(Error::internal)?
        .ok_or(Error::NotFound)
}

/// `params.require(:user).permit(:name, :avatar, :webhook_url)`
fn bot_params(c: &Ctx) -> Result<ParamMap> {
    Ok(c.params.require("user")?.permit(&permit_keys(&["name", "avatar", "webhook_url"])))
}

fn redirect_to_bots(c: &mut Ctx) -> Result {
    let location = c.url_for(&campfire_routes::account_bots());
    c.redirect_to(&location)
}
