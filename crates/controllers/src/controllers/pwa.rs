//! `PwaController` (reference/app/controllers/pwa_controller.rb): the web app manifest and the
//! service worker, at stable URLs.

use askama::Template;
use campfire_db::Account;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::pwa;

use crate::app::AppCtx;
use crate::concerns::{self, Before};
use crate::controllers::presenters;

/// `allow_unauthenticated_access`, `skip_forgery_protection`
fn before() -> Before {
    Before::default().allow_unauthenticated_access().skip_forgery_protection()
}

/// `pwa/service_worker.js`
pub async fn service_worker(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    c.respond_to(&[&format::JS])?;
    Ok(c.render_as(StatusCode::OK, "text/javascript; charset=utf-8", pwa::SERVICE_WORKER_JS))
}

/// `pwa/manifest.json.erb`
pub async fn manifest(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    c.respond_to(&[&format::JSON])?;
    render_manifest(c, "/", false).await
}

/// The installed SPA uses the classic manifest's presentation with its own start URL and scope.
pub async fn spa_manifest(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    render_manifest(c, &campfire_spa::root_path(), true).await
}

async fn render_manifest(c: &mut Ctx, root: &str, spa: bool) -> Result {
    let account = c
        .app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?;
    let asset_path = |path: &str| campfire_assets::asset_path(path);
    let manifest = pwa::Manifest {
        account_name: account.as_ref().map(|account| account.name.clone()),
        logo_path_small: presenters::accounts::fresh_account_logo_path(
            account.as_ref(),
            Some("small"),
        ),
        logo_path: presenters::accounts::fresh_account_logo_path(account.as_ref(), None),
        base_url: c.url_for(""),
        root,
        new_room_url: if spa {
            campfire_spa::screens::spa_url("rooms/opens#new", "/rooms/opens/new", None)
                .unwrap_or_else(|| "/rooms/opens/new".into())
        } else {
            "rooms/opens/new".into()
        },
        profile_url: if spa {
            campfire_spa::screens::spa_url("users/profiles#show", "/users/me/profile", None)
                .unwrap_or_else(|| "/users/me/profile".into())
        } else {
            "/users/me/profile".into()
        },
        asset_path: &asset_path,
    };
    let body = manifest.render().map_err(Error::internal)?;
    let content_type = if spa {
        "application/manifest+json; charset=utf-8"
    } else {
        "application/json; charset=utf-8"
    };
    Ok(c.render_as(StatusCode::OK, content_type, body))
}
