//! The React SPA under `/app` (`crates/spa` embeds the build), mounted only with `SPA_ENABLED`;
//! without it these paths are unknown, as they always were.
//!
//! - `GET /app/assets/*`: Vite's content-hashed files, cached as immutable, in the client's best
//!   encoding (brotli, gzip or the file itself). Public, like `/assets`, with the classic pages'
//!   security headers and policy (no nonce: the response is shared by everyone). `no-transform`
//!   keeps the app's gzip middleware (and proxies) off them: the build already chose each file's
//!   encodings, and a font or image is sent as it is.
//! - `GET /app`, `/app/*`: the shell, behind `ApplicationController`'s before-actions, so signing
//!   in (with the return path), two-step enforcement, deactivation and the rest apply as on a
//!   classic page. A file at the dist's root (Vite's `public/`) is served as it is.
//! - `GET /api/v1/boot`: the shell's boot JSON and CSRF token, for the Vite dev server (whose own
//!   `index.html` has neither) and for the SPA to refresh its token without a reload.
//! - `POST /app/ui_preference`: the person's UI (`ui=next` or `classic`), from the classic
//!   profile's "Try the new Smartfire" and the SPA's "Switch to classic". While it says `next`,
//!   classic pages the SPA has ported redirect there (`concerns::redirect_to_spa`).

use axum::Router;
use axum::extract::{Request, State};
use axum::handler::Handler as _;
use axum::http::{HeaderName, HeaderValue, header};
use axum::response::Response;
use campfire_db::models::user::ui_preference::{self, UiPreference};
use campfire_db::{Account, UserStatusSettings};
use campfire_kit::{Ctx, Error, Kit, Redirect, Result, StatusCode};
use campfire_spa::{Boot, BootAccount, BootFlash, BootResponse, BootUser, FlashKind};

use crate::app::AppCtx;
use crate::concerns::{self, Authentication, Before};
use crate::controllers::presenters;

/// The SPA's routes when `enabled`, none otherwise. `immutable_cache_control` is the policy the
/// app gives digest-stamped `/assets`.
pub fn routes(enabled: bool, immutable_cache_control: &'static str) -> Router<Kit> {
    if !enabled {
        return Router::new();
    }
    let assets = move |State(kit): State<Kit>, request: Request| async move {
        match file_response(&kit, &request, immutable_cache_control) {
            Some(response) => response,
            None => campfire_kit::adapter::not_found(State(kit), request).await,
        }
    };
    let page = move |State(kit): State<Kit>, request: Request| async move {
        match file_response(&kit, &request, immutable_cache_control) {
            Some(response) => response,
            None => campfire_kit::action(show).call(request, kit).await,
        }
    };
    Router::new()
        .route("/app/assets/{*path}", axum::routing::get(assets))
        .route("/app/service-worker.js", axum::routing::get(assets))
        .route("/app/offline.html", axum::routing::get(assets))
        .route("/app", axum::routing::get(campfire_kit::action(show)))
        .route("/app/", axum::routing::get(campfire_kit::action(show)))
        .route("/app/manifest.webmanifest", axum::routing::get(campfire_kit::action(super::pwa::spa_manifest)))
        .route("/app/{*path}", axum::routing::get(page))
        .route("/api/v1/boot", axum::routing::get(campfire_kit::action(boot)))
        .route("/app/ui_preference", axum::routing::post(campfire_kit::action(update_ui_preference)))
}

/// `POST /app/ui_preference`, a form post: `ui` (`next` or `classic`) becomes the person's UI.
/// `next` goes to `/app/`. `classic` goes back to `return_to`: an SPA path becomes its classic
/// page (`/app/r/5` is `/rooms/5`; one with none is `/`), a classic path stays as it is, and
/// anything but a local path is `/`. Any other `ui` is a 422.
pub async fn update_ui_preference(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let Some(preference) = c.param_str("ui").and_then(UiPreference::parse) else {
        return Ok(c.head(StatusCode::UNPROCESSABLE_ENTITY));
    };
    let user_id = concerns::require_current_user(c)?.id;
    c.app().db.write(move |tx| ui_preference::store(tx, user_id, preference)).await.map_err(Error::internal)?;
    let location = match preference {
        UiPreference::Next => format!("{}/", campfire_spa::PREFIX),
        UiPreference::Classic => classic_return_path(c.param_str("return_to")),
    };
    let location = c.url_for(&location);
    c.redirect_to_with(&location, Redirect { status: Some(StatusCode::SEE_OTHER), ..Redirect::default() })
}

/// Where "Switch to classic" lands for `return_to` (see [`update_ui_preference`]).
pub fn classic_return_path(return_to: Option<&str>) -> String {
    let Some(return_to) = return_to.filter(|path| local_path(path)) else {
        return "/".into();
    };
    let (path, query) = match return_to.split_once('?') {
        Some((path, query)) => (path, Some(query)),
        None => (return_to, None),
    };
    let under_spa = path == campfire_spa::PREFIX || path.starts_with(&format!("{}/", campfire_spa::PREFIX));
    if under_spa {
        campfire_spa::screens::classic_url(path, query).unwrap_or_else(|| "/".into())
    } else {
        return_to.to_string()
    }
}

/// A path on this host: one leading `/`, no backslash (browsers read it as a slash, so `/\evil`
/// names another host) and no control character.
fn local_path(path: &str) -> bool {
    path.starts_with('/') && !path.starts_with("//") && !path.bytes().any(|b| b.is_ascii_control() || b == b'\\')
}

/// The shell: the dist's `index.html` with the CSRF meta tags, the CSP nonce and the boot JSON.
pub async fn show(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default()).await?;
    let mut boot = load_boot(c).await?;
    let flash = c.flash();
    boot.flash = flash.notice().map(|message| BootFlash { kind: FlashKind::Notice, message: message.to_owned() })
        .or_else(|| flash.alert().map(|message| BootFlash { kind: FlashKind::Alert, message: message.to_owned() }));
    flash.discard(None);
    let csrf_token = c.authenticity_tokens().global();
    let nonce = c.content_security_policy_nonce();
    let html = campfire_spa::render_shell(&boot, &csrf_token, nonce.as_deref());
    Ok(c.render_as(StatusCode::OK, "text/html; charset=utf-8", html))
}

/// `GET /api/v1/boot`. Signed out, a JSON request gets an empty 401 and a page navigation the
/// sign-in redirect, as `MembersController` answers.
pub async fn boot(c: &mut Ctx) -> Result {
    let before = Before { authentication: Authentication::JsonUnauthorized, ..Before::default() };
    concerns::before_actions(c, before).await?;
    let boot = load_boot(c).await?;
    let csrf_token = c.authenticity_tokens().global();
    let body = serde_json::to_string(&BootResponse { boot: &boot, csrf_token: &csrf_token }).map_err(Error::internal)?;
    // It carries the session's token: never kept by a cache.
    c.set_header(header::CACHE_CONTROL, "no-store");
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}

async fn load_boot(c: &mut Ctx) -> Result<Boot> {
    let user = concerns::require_current_user(c)?.clone();
    let app = c.app();
    let service_worker_url = match concerns::effective_ui(c).await? {
        ui @ UiPreference::Next => Some(concerns::service_worker_url(ui)),
        UiPreference::Classic => None,
    };
    let avatar_url = presenters::avatar_path(&app.secrets, &user);
    let (version, revision) = (app.config.app_version.clone(), app.config.git_revision.clone());
    let user_id = user.id;
    let (account, settings) = app
        .db
        .read(move |conn| {
            let settings = UserStatusSettings::for_ids(conn, &[user_id])?.remove(&user_id);
            Ok((Account::first(conn)?, settings))
        })
        .await
        .map_err(Error::internal)?;
    let branding = match &account {
        Some(account) => Some(presenters::workspace_branding::for_account(app, account).await?),
        None => None,
    };
    Ok(Boot {
        user: BootUser { id: user.id, name: user.name, avatar_url },
        account: BootAccount {
            name: account.map(|account| account.name),
            logo_url: branding.as_ref().and_then(|branding| branding.logo_url.clone()),
            logo_still_url: branding.as_ref().and_then(|branding| branding.logo_still_url.clone()),
            banner_url: branding.as_ref().and_then(|branding| branding.banner_url.clone()),
            banner_still_url: branding.and_then(|branding| branding.banner_still_url),
        },
        theme: campfire_spa::theme(settings.as_ref().map(|s| s.theme.as_str())),
        text_size: campfire_spa::text_size(settings.as_ref().map(|s| s.text_size.as_str())),
        cable_url: campfire_cable::protocol::DEFAULT_MOUNT_PATH.to_string(),
        service_worker_url,
        version,
        revision,
        flash: None,
    })
}

/// The embedded file a request under `/app/` names, if there is one.
fn file_response(kit: &Kit, request: &Request, immutable_cache_control: &'static str) -> Option<Response> {
    let path = request.uri().path().strip_prefix("/app/")?;
    let accept_encoding = request.headers().get(header::ACCEPT_ENCODING).and_then(|v| v.to_str().ok());
    let served = campfire_spa::file(path, accept_encoding)?;
    Some(served_response(kit, served, immutable_cache_control))
}

pub fn served_response(kit: &Kit, served: campfire_spa::Served, immutable_cache_control: &'static str) -> Response {
    let worker = served.file.path == "service-worker.js";
    let cache_control = if worker { "no-cache" } else if served.file.immutable { immutable_cache_control } else { campfire_spa::REVALIDATE_CACHE_CONTROL };
    let cache_control = format!("{cache_control}, no-transform");
    let mut response = Response::new(axum::body::Body::from(served.body));
    let headers = response.headers_mut();
    headers.insert(header::CONTENT_TYPE, HeaderValue::from_static(served.file.content_type));
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(served.body.len()));
    headers.insert(header::CACHE_CONTROL, HeaderValue::from_str(&cache_control).expect("a constant policy"));
    if worker {
        headers.insert(HeaderName::from_static("service-worker-allowed"), HeaderValue::from_static("/"));
    }
    if let Some(encoding) = served.content_encoding {
        headers.insert(header::CONTENT_ENCODING, HeaderValue::from_static(encoding));
    }
    if served.varies() {
        headers.insert(header::VARY, HeaderValue::from_static("accept-encoding"));
    }
    for (name, value) in &kit.config().default_headers {
        headers.insert(name.clone(), value.clone());
    }
    if let Some(policy) = &kit.config().content_security_policy
        && let Ok(value) = HeaderValue::from_str(&policy.build(None))
    {
        headers.insert(HeaderName::from_static(policy.header_name()), value);
    }
    response.extensions_mut().insert(campfire_kit::deflater::StaticFile);
    response
}
