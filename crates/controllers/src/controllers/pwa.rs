//! `PwaController` (reference/app/controllers/pwa_controller.rb): the web app manifest and the
//! service worker, at stable URLs.

use askama::Template;
use axum::{
    Router,
    extract::{Request, State},
};
use campfire_db::Account;
use campfire_kit::Kit;
use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_spa::pwa;

use crate::app::AppCtx;
use crate::concerns::{self, Before};

/// `allow_unauthenticated_access`, `skip_forgery_protection`
fn before() -> Before {
    Before::default()
        .allow_unauthenticated_access()
        .skip_forgery_protection()
}

/// `pwa/service_worker.js`
pub async fn service_worker(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    c.respond_to(&[&format::JS])?;
    let served = pwa::file("service-worker.js", None)
        .expect("PWA fallback worker");
    c.set_header("cache-control", "no-cache, no-transform");
    c.set_header("service-worker-allowed", "/");
    Ok(c.render_as(StatusCode::OK, served.file.content_type, served.body))
}

/// `pwa/manifest.json.erb`
pub async fn manifest(c: &mut Ctx) -> Result {
    concerns::before_actions(c, before()).await?;
    c.respond_to(&[&format::JSON])?;
    // Anonymous access skips session restoration; shortcuts still honor an existing session.
    concerns::restore_authentication(c).await?;
    render_manifest(c).await
}

pub async fn favicon(c: &mut Ctx) -> Result {
    c.use_live_response();
    concerns::before_actions(c, before().skip_allow_browser()).await?;
    let variation = campfire_storage::Variation::resize_to_limit(192, 192, Some("png"));
    campfire_people::controllers::accounts::logos::send_logo(c, variation, |c| {
        let mark = campfire_spa::favicon::file("favicon.ico").expect("the mark's icon");
        Ok(c.render_as(StatusCode::OK, mark.file.content_type, mark.body))
    })
    .await
}

/// Compatibility for a manifest linked by an older SPA bundle; install identity stays at `/`.
pub async fn spa_manifest(c: &mut Ctx) -> Result {
    // The /app manifest from #331 never shipped in a release, so no installed PWA carries
    // its implicit /app identity. Only that makes redirecting to the root identity safe.
    c.redirect_to("/webmanifest.json")
}

pub fn routes(immutable_cache_control: &'static str) -> Router<Kit> {
    let files = move |State(kit): State<Kit>, request: Request| async move {
        let path = request
            .uri()
            .path()
            .strip_prefix("/pwa/")
            .or_else(|| request.uri().path().strip_prefix("/app/"))
            .unwrap_or_else(|| request.uri().path().trim_start_matches('/'));
        let encoding = request
            .headers()
            .get("accept-encoding")
            .and_then(|value| value.to_str().ok());
        match pwa::file(path, encoding).or_else(|| campfire_spa::favicon::file(path)) {
            Some(served) => {
                let mut response =
                    super::spa::served_response(&kit, served, immutable_cache_control);
                if path == "offline.html"
                    && campfire_spa::file(path, None).is_none()
                {
                    // The original standalone shell has inline retry/reconnect scripts and,
                    // like ActionDispatch::Static, needs no request's nonce policy.
                    response.headers_mut().remove("content-security-policy");
                }
                response
            }
            None => campfire_kit::adapter::not_found(State(kit), request).await,
        }
    };
    Router::new()
        .route("/offline.html", axum::routing::get(files))
        .route("/favicon.svg", axum::routing::get(files))
        .route("/favicon.png", axum::routing::get(files))
        .route("/apple-touch-icon.png", axum::routing::get(files))
        .route("/favicon.ico", axum::routing::get(campfire_kit::action(favicon)))
        .route("/pwa/assets/{*path}", axum::routing::get(files))
        .route("/app/offline.html", axum::routing::get(files))
        .route("/app/service-worker.js", axum::routing::get(files))
        .route(
            "/app/manifest.webmanifest",
            axum::routing::get(campfire_kit::action(spa_manifest)),
        )
}

async fn render_manifest(c: &mut Ctx) -> Result {
    let account = c
        .app()
        .db
        .read(Account::first)
        .await
        .map_err(Error::internal)?;
    let manifest = pwa::Manifest {
        account_name: account.as_ref().map(|account| account.name.clone()),
        logo_path_small: manifest_logo_path(account.as_ref(), Some("small")),
        logo_path: manifest_logo_path(account.as_ref(), None),
        base_url: c.url_for(""),
        root: "/",
        new_room_url: "/app/rooms/new/open".into(),
        profile_url: "/app/settings".into(),
        asset_path: &pwa::asset_path,
    };
    let body = manifest.render().map_err(Error::internal)?;
    // Keep the historical content type and implicit manifest id (resolved from start_url).
    c.set_header("cache-control", "private, no-cache");
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}

pub(crate) fn manifest_logo_path(account: Option<&Account>, size: Option<&str>) -> String {
    let version = account.map(|account| account.updated_at.jiff().strftime("%Y%m%d%H%M%S").to_string());
    campfire_routes::fresh_account_logo(version.as_deref(), size)
}
