//! The React SPA under `/app` (`crates/spa` embeds the build): the only UI for signed-in people.
//! Always mounted with the JSON API and sync socket.
//!
//! - `GET /app/assets/*`: Vite's content-hashed files, cached as immutable, in the client's best
//!   encoding (brotli, gzip or the file itself). Public, like `/assets`, with the classic pages'
//!   security headers and policy (no nonce: the response is shared by everyone). `no-transform`
//!   keeps the app's gzip middleware (and proxies) off them: the build already chose each file's
//!   encodings, and a font or image is sent as it is.
//! - `GET /app`, `/app/*`: the shell, behind `ApplicationController`'s before-actions, so signing
//!   in (with the return path), two-step enforcement, deactivation and the rest apply as on a
//!   classic page. A file at the dist's root (Vite's `public/`) is served as it is. The setup
//!   screen (`/app/two_factor/setup`) has the retained setup page's before-actions instead, so a
//!   session that must set up two-step sign-in can open it.
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
use campfire_spa::{Boot, BootAccount, BootFlash, BootResponse, BootUser, Favicon, FlashKind};

use crate::app::AppCtx;
use crate::concerns::{self, Authentication, Before};
use crate::controllers::presenters;
use campfire_api_types::{
    FlashMessage, FlashMessageKind, SignInHelpContact, SignInMethods, SignedOut, SignedOutBoot,
};
use campfire_people::controllers::{auth, sessions, two_factor, users};
use campfire_spa::{SignedOutRoute, is_public_page, signed_out_route};

/// The SPA routes. `immutable_cache_control` is the policy for digest-stamped assets.
pub fn routes(immutable_cache_control: &'static str) -> Router<Kit> {
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
        // The join pages' own routes, so the shell reads their code or token as decoded params.
        .route(
            "/app/join/{join_code}",
            axum::routing::get(campfire_kit::action(show)),
        )
        .route(
            "/app/invite/{token}",
            axum::routing::get(campfire_kit::action(show)),
        )
        .route(
            "/api/v1/boot",
            axum::routing::get(campfire_kit::action(boot)),
        )
        .route(
            "/api/v1/session/boot",
            axum::routing::get(campfire_kit::action(signed_out_boot)),
        )
        .route(
            "/api/v1/session",
            axum::routing::post(campfire_kit::unparsed_action(sessions::create_json))
                .delete(campfire_kit::unparsed_action(sessions::destroy_json)),
        )
        .route(
            "/api/v1/session/google",
            axum::routing::post(campfire_kit::action(super::google_sign_in::create_json)),
        )
        .route(
            "/api/v1/session/transfers/{id}",
            axum::routing::put(campfire_kit::action(sessions::transfers::update_json)),
        )
        .route(
            "/api/v1/join/{join_code}",
            axum::routing::get(campfire_kit::action(users::new_json))
                .post(campfire_kit::action(users::create_json)),
        )
        .route(
            "/api/v1/invite/{token}",
            axum::routing::get(campfire_kit::action(users::invite_new_json))
                .post(campfire_kit::action(users::invite_create_json)),
        )
        .route(
            "/api/v1/first_run",
            axum::routing::get(campfire_kit::action(super::first_runs::show_json)).post(
                campfire_kit::action(super::first_runs::create_json)
                    .json_body_parser(defer_json_body),
            ),
        )
        .route(
            "/api/v1/two_factor/challenge",
            axum::routing::get(campfire_kit::action(two_factor::challenge_show_json)).post(
                campfire_kit::unparsed_action(two_factor::challenge_create_json),
            ),
        )
        .route(
            "/api/v1/two_factor/setup",
            axum::routing::get(campfire_kit::action(two_factor::setup_show_json))
                .post(campfire_kit::unparsed_action(two_factor::setup_create_json)),
        )
        .route(
            "/api/v1/sudo",
            axum::routing::get(campfire_kit::action(super::sudos::new_json))
                .post(campfire_kit::unparsed_action(super::sudos::create_json)),
        )
        .route(
            "/api/v1/sudo/google",
            axum::routing::post(campfire_kit::action(super::sudos::google_json)),
        )
        .route(
            "/api/v1/sudo/continue",
            axum::routing::get(campfire_kit::action(super::sudos::continue_json))
                .delete(campfire_kit::action(super::sudos::acknowledge_json)),
        )
}

/// Leaves a JSON body to the action's own DTO decoding (and its body token to the forgery check),
/// so an unreadable one is the contract's 422 rather than the param parser's 400. Multipart forms
/// still parse as usual, files included.
fn defer_json_body(
    _: &axum::http::Method,
    _: &str,
    _: &[u8],
) -> Option<std::result::Result<campfire_kit::ParamMap, campfire_kit::params::ParamError>> {
    Some(Ok(campfire_kit::ParamMap::default()))
}

/// The SPA's two-step setup screen, where the enrollment gate and `/two_factor_setup` send a
/// session that must still set up two-step sign-in. It may open it as a page; a JSON request for
/// it, and the rest of `/app`, stay behind the gate.
const SETUP_SHELL: &str = "/app/two_factor/setup";

/// The shell: the dist's `index.html` with the CSRF meta tags, the CSP nonce and the boot JSON.
pub async fn show(c: &mut Ctx) -> Result {
    // About, Privacy and Terms are for anyone, as their retained pages are, and show everyone
    // the same page: the signed-out shell draws them, skipping the workspace callback chain
    // those pages skip. No session is restored, so a pending two-step enrollment isn't enforced
    // here and a saved return destination stays as it was. They show no flash, so they leave a
    // pending one for the page that does, as the retained pages do.
    if is_public_page(c.request.path()) {
        let (boot, favicon) = load_signed_out_boot(c, false).await?;
        let nonce = c.content_security_policy_nonce();
        let html = campfire_spa::render_signed_out_shell(&boot, nonce.as_deref(), &favicon);
        c.no_store();
        return Ok(c.render_as(StatusCode::OK, "text/html; charset=utf-8", html));
    }
    if let Some(route) = signed_out_route(c.request.path()) {
        let (endpoint, before) = match route {
            SignedOutRoute::SignIn => (
                "sessions#new",
                Before::default().allow_unauthenticated_access(),
            ),
            SignedOutRoute::Transfer => (
                "sessions/transfers#show",
                Before::default().allow_unauthenticated_access(),
            ),
            SignedOutRoute::Challenge => (
                "two_factor/challenges#show",
                Before::default().allow_unauthenticated_access(),
            ),
            // The retained join pages send a signed-in visitor to the root.
            SignedOutRoute::Join | SignedOutRoute::Invite => (
                "users#new",
                Before::default().require_unauthenticated_access(),
            ),
            SignedOutRoute::FirstRun => (
                "first_runs#show",
                Before::default().allow_unauthenticated_access(),
            ),
        };
        auth::before_actions(c, endpoint, before, auth::ResponseMode::Html).await?;
        // As the retained page: once the account exists, first run goes home.
        if route == SignedOutRoute::FirstRun {
            super::first_runs::prevent_repeats(c).await?;
        }
        // Only the retained challenge restores authentication. The public sign-in and transfer
        // forms leave an existing session and its saved return destination alone.
        if route == SignedOutRoute::Challenge && concerns::restore_authentication(c).await? {
            return c.redirect_to(&c.url_for("/"));
        }
        // A wrong join code or a dead invite keeps the retained page's status; the page says why.
        let status = match route {
            SignedOutRoute::Join => users::page_status(c, false).await?,
            SignedOutRoute::Invite => users::page_status(c, true).await?,
            _ => StatusCode::OK,
        };
        let (boot, favicon) = load_signed_out_boot(c, true).await?;
        let nonce = c.content_security_policy_nonce();
        let html = campfire_spa::render_signed_out_shell(&boot, nonce.as_deref(), &favicon);
        c.no_store();
        return Ok(c.render_as(status, "text/html; charset=utf-8", html));
    } else if c.request.path() == SETUP_SHELL && !concerns::sudo::json_request(c)? {
        two_factor::setup_shell_before(c).await?;
    } else {
        concerns::before_actions(c, Before::default()).await?;
    }
    let (mut boot, favicon) = load_boot(c).await?;
    let flash = c.flash();
    boot.flash = flash
        .notice()
        .map(|message| BootFlash {
            kind: FlashKind::Notice,
            message: message.to_owned(),
        })
        .or_else(|| {
            flash.alert().map(|message| BootFlash {
                kind: FlashKind::Alert,
                message: message.to_owned(),
            })
        });
    flash.discard(None);
    let csrf_token = c.authenticity_tokens().global();
    let nonce = c.content_security_policy_nonce();
    let html = campfire_spa::render_shell(&boot, &csrf_token, nonce.as_deref(), &favicon);
    Ok(c.render_as(StatusCode::OK, "text/html; charset=utf-8", html))
}

/// Public authentication inputs and a fresh CSRF token for the signed-out client. Its callers are
/// the pages that show a flash (sign-in and joining, when the shell didn't inline their boot) and
/// the signed-out forms' token refresh, so it hands over a pending flash; the public pages never
/// ask for it.
pub async fn signed_out_boot(c: &mut Ctx) -> Result {
    concerns::before_actions(c, Before::default().allow_unauthenticated_access()).await?;
    let (boot, _) = load_signed_out_boot(c, true).await?;
    auth::json(c, StatusCode::OK, &boot)
}

/// The signed-out boot. With `take_flash`, a redirect's pending notice or alert goes with it and is
/// discarded; without, the flash isn't read and stays for the next page.
async fn load_signed_out_boot(c: &mut Ctx, take_flash: bool) -> Result<(SignedOutBoot, Favicon)> {
    let (account, first_run_pending, help_contact) = c
        .app()
        .db
        .read(|conn| {
            Ok((
                Account::first(conn)?,
                presenters::accounts::no_users(conn)?,
                presenters::accounts::help_contact(conn)?,
            ))
        })
        .await
        .map_err(Error::internal)?;
    let workspace =
        presenters::workspace_branding::public_for_account(c.app(), account.as_ref()).await?;
    let favicon = favicon(account.as_ref(), workspace.logo_url.is_some());
    // A redirect's notice or alert (a refused Google sign-in), shown by this load only.
    let message = if take_flash {
        let flash = c.flash();
        let message = flash
            .notice()
            .map(|message| FlashMessage {
                kind: FlashMessageKind::Notice,
                message: message.to_owned(),
            })
            .or_else(|| {
                flash.alert().map(|message| FlashMessage {
                    kind: FlashMessageKind::Alert,
                    message: message.to_owned(),
                })
            });
        flash.discard(None);
        message
    } else {
        None
    };
    let boot = SignedOutBoot {
        kind: SignedOut::SignedOut,
        workspace,
        sign_in_methods: {
            let provider = c.app().google.sign_in();
            let configured = provider.config.configured();
            let google_domains = if configured {
                provider.config.domains.clone()
            } else {
                Vec::new()
            };
            SignInMethods {
                password: true,
                google: configured,
                google_domains,
            }
        },
        first_run_pending,
        help_contact: help_contact.map(|owner| SignInHelpContact {
            name: owner.name,
            email_address: owner.email_address,
        }),
        version: c.app().config.app_version.clone(),
        csrf_token: c.authenticity_tokens().global(),
        flash: message,
    };
    Ok((boot, favicon))
}

fn favicon(account: Option<&Account>, logo_attached: bool) -> Favicon {
    match account {
        Some(account) if logo_attached => Favicon::Logo {
            icon: super::pwa::manifest_logo_path(Some(account), Some("small")),
            touch_icon: super::pwa::manifest_logo_path(Some(account), Some("touch")),
        },
        _ => Favicon::Mark,
    }
}

/// `GET /api/v1/boot`. Signed out, a JSON request gets an empty 401 and a page navigation the
/// sign-in redirect, as `MembersController` answers.
pub async fn boot(c: &mut Ctx) -> Result {
    let before = Before {
        authentication: Authentication::JsonUnauthorized,
        ..Before::default()
    };
    concerns::before_actions(c, before).await?;
    let (boot, _) = load_boot(c).await?;
    let csrf_token = c.authenticity_tokens().global();
    let body = serde_json::to_string(&BootResponse {
        boot: &boot,
        csrf_token: &csrf_token,
    })
    .map_err(Error::internal)?;
    // It carries the session's token: never kept by a cache.
    c.set_header(header::CACHE_CONTROL, "no-store");
    Ok(c.render_as(StatusCode::OK, "application/json; charset=utf-8", body))
}

async fn load_boot(c: &mut Ctx) -> Result<(Boot, Favicon)> {
    let user = concerns::require_current_user(c)?.clone();
    let app = c.app();
    let service_worker_url = Some("/service-worker.js".into());
    let avatar_url = presenters::avatar_path(&app.secrets, &user);
    let (version, revision) = (
        app.config.app_version.clone(),
        app.config.git_revision.clone(),
    );
    let user_id = user.id;
    let (account, settings, appearance) = app
        .db
        .read(move |conn| {
            let settings = UserStatusSettings::for_ids(conn, &[user_id])?.remove(&user_id);
            Ok((
                Account::first(conn)?,
                settings,
                campfire_db::models::user::profile_settings::appearance(conn, user_id)?,
            ))
        })
        .await
        .map_err(Error::internal)?;
    let branding = match &account {
        Some(account) => Some(presenters::workspace_branding::for_account(app, account).await?),
        None => None,
    };
    let favicon = favicon(
        account.as_ref(),
        branding
            .as_ref()
            .is_some_and(|branding| branding.logo_url.is_some()),
    );
    let boot = Boot {
        user: BootUser {
            id: user.id,
            name: user.display_name().to_owned(),
            avatar_url,
        },
        custom_styles: account
            .as_ref()
            .and_then(|account| account.custom_styles.clone()),
        account: BootAccount {
            upload_limit_bytes: account.as_ref().map_or(
                campfire_db::models::account::DEFAULT_UPLOAD_LIMIT_BYTES,
                |account| account.settings().upload_limit_bytes(),
            ),
            name: account.map(|account| account.name),
            logo_url: branding
                .as_ref()
                .and_then(|branding| branding.logo_url.clone()),
            logo_still_url: branding
                .as_ref()
                .and_then(|branding| branding.logo_still_url.clone()),
            banner_url: branding
                .as_ref()
                .and_then(|branding| branding.banner_url.clone()),
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
    };
    Ok((boot, favicon))
}

/// The embedded file a request under `/app/` names, if there is one.
fn file_response(
    kit: &Kit,
    request: &Request,
    immutable_cache_control: &'static str,
) -> Option<Response> {
    let path = request.uri().path().strip_prefix("/app/")?;
    let accept_encoding = request
        .headers()
        .get(header::ACCEPT_ENCODING)
        .and_then(|v| v.to_str().ok());
    let served = campfire_spa::file(path, accept_encoding)?;
    Some(served_response(kit, served, immutable_cache_control))
}

pub fn served_response(
    kit: &Kit,
    served: campfire_spa::Served,
    immutable_cache_control: &'static str,
) -> Response {
    let worker = served.file.path == "service-worker.js";
    let cache_control = if worker {
        "no-cache"
    } else if served.file.immutable {
        immutable_cache_control
    } else {
        campfire_spa::REVALIDATE_CACHE_CONTROL
    };
    let cache_control = format!("{cache_control}, no-transform");
    let mut response = Response::new(axum::body::Body::from(served.body));
    let headers = response.headers_mut();
    headers.insert(
        header::CONTENT_TYPE,
        HeaderValue::from_static(served.file.content_type),
    );
    headers.insert(header::CONTENT_LENGTH, HeaderValue::from(served.body.len()));
    headers.insert(
        header::CACHE_CONTROL,
        HeaderValue::from_str(&cache_control).expect("a constant policy"),
    );
    if worker {
        headers.insert(
            HeaderName::from_static("service-worker-allowed"),
            HeaderValue::from_static("/"),
        );
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
    response
        .extensions_mut()
        .insert(campfire_kit::deflater::StaticFile);
    response
}
