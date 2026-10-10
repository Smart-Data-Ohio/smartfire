//! The React SPA under `/app` (`crates/spa` embeds the build): the only UI for signed-in people.
//! Always mounted in a running app (`Config::spa_enabled`); only classic page tests leave it off.
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
//!
//! Every classic page the SPA has (`campfire_spa::screens`) redirects a signed-in navigation here
//! (`concerns::redirect_to_spa`). There's no way back: the old `POST /app/ui_preference` switch is
//! gone, and a stored choice of the classic UI is ignored.

use axum::Router;
use axum::extract::{Request, State};
use axum::handler::Handler as _;
use axum::http::{HeaderName, HeaderValue, header};
use axum::response::Response;
use campfire_db::{Account, UserStatusSettings};
use campfire_kit::{Ctx, Error, Kit, Result, StatusCode};
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
        .route("/app", axum::routing::get(campfire_kit::action(show)))
        .route("/app/", axum::routing::get(campfire_kit::action(show)))
        .route("/app/{*path}", axum::routing::get(page))
        .route("/api/v1/boot", axum::routing::get(campfire_kit::action(boot)))
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
    let service_worker_url = Some("/service-worker.js".into());
    let avatar_url = presenters::avatar_path(&app.secrets, &user);
    let (version, revision) = (app.config.app_version.clone(), app.config.git_revision.clone());
    let user_id = user.id;
    let (account, settings, appearance) = app
        .db
        .read(move |conn| {
            let settings = UserStatusSettings::for_ids(conn, &[user_id])?.remove(&user_id);
            Ok((Account::first(conn)?, settings, campfire_db::models::user::profile_settings::appearance(conn, user_id)?))
        })
        .await
        .map_err(Error::internal)?;
    let branding = match &account {
        Some(account) => Some(presenters::workspace_branding::for_account(app, account).await?),
        None => None,
    };
    Ok(Boot {
        user: BootUser { id: user.id, name: user.display_name().to_owned(), avatar_url },
        custom_styles: account.as_ref().and_then(|account| account.custom_styles.clone()),
        account: BootAccount {
            upload_limit_bytes: account.as_ref().map_or(
                campfire_db::models::account::DEFAULT_UPLOAD_LIMIT_BYTES,
                |account| account.settings().upload_limit_bytes(),
            ),
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
        appearance_preferences: appearance.appearance_preferences,
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
