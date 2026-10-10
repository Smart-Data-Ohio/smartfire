//! Retained request facts, preferences and scoped CSRF/flash support, independent of the
//! classic application Layout.

use campfire_db::{Account, User};
use campfire_kit::{Ctx, Error, Response, Result, StatusCode, format};
use campfire_presentation::layouts::UserPreferences;
use campfire_presentation::time::Zone;
use campfire_presentation::{AccountSummary, CurrentUser, Platform};
use campfire_view_kit::helpers::request_forgery::{self, RequestSecrets};

use crate::app::AppCtx;
use crate::concerns;

/// Facts read by retained authentication and public pages.
#[derive(Debug, Clone)]
pub struct RequestContext {
    pub current_user: Option<CurrentUser>,
    pub account: AccountSummary,
    pub custom_styles: Option<String>,
    pub platform: Platform,
    pub app_version: String,
    pub chrome: campfire_retained::Chrome,
}

/// Reuse the zone already carried by the authenticated row; no association reload.
pub async fn time_zone(c: &Ctx) -> Result<Zone> {
    if let Some(loaded) = c.current::<crate::presenters::layout_preferences::LoadedPreferences>()
        && concerns::current_user(c).is_some_and(|user| user.id == loaded.user_id)
    {
        return Ok(loaded.time_zone());
    }
    let Some(user) = concerns::current_user(c) else {
        return Ok(Zone::utc());
    };
    // Write/error paths have no authentication preload. Only SetTimeZone's
    // scalar is needed here; associations belong to the eventual layout load.
    let id = user.id;
    let zone: Option<String> = c
        .app()
        .db
        .read(move |conn| {
            Ok(
                conn.query_row("SELECT time_zone FROM users WHERE id=?", [id], |row| {
                    row.get(0)
                })?,
            )
        })
        .await
        .map_err(Error::internal)?;
    Ok(Zone::for_user(zone.as_deref()))
}
impl RequestContext {
    pub async fn load(c: &Ctx) -> Result<Self> {
        let app = c.app();
        let secrets = app.secrets.clone();
        let user = concerns::current_user(c).cloned();
        let user_id = user.as_ref().map(|user| user.id);
        let now = c.now();
        let loaded = c
            .current::<crate::presenters::layout_preferences::LoadedPreferences>()
            .filter(|loaded| Some(loaded.user_id) == user_id)
            .cloned();
        let (account, has_logo, preferences) = app
            .db
            .read(move |conn| {
                let account = Account::first(conn)?;
                let has_logo = match &account {
                    Some(account) => crate::presenters::attachments::attached_blob(
                        conn, "Account", account.id, "logo",
                    )?
                    .is_some(),
                    None => false,
                };
                let preferences = match user_id {
                    Some(user_id) => match loaded {
                        Some(loaded) => loaded.load(conn)?,
                        None => user_preferences(conn, user_id, now)?,
                    },
                    None => UserPreferences::default(),
                };
                Ok((account, has_logo, preferences))
            })
            .await
            .map_err(Error::internal)?;
        let time_zone = Zone::for_user(preferences.time_zone.as_deref());
        let current_user = user.as_ref().map(|user| CurrentUser {
            preferences,
            ..current_user(&secrets, user)
        });
        let test_environment = app.config.environment == "test";
        let service_worker_auto_register = !test_environment
            || c.cookies
                .get("enable_service_worker")
                .is_some_and(|value| !campfire_richtext::ruby::is_blank(value));
        let service_worker_url = if app.config.spa_enabled {
            Some(concerns::service_worker_url(
                concerns::effective_ui(c).await?,
            ))
        } else {
            None
        };
        let mut summary = account_summary(account.as_ref(), has_logo);
        summary.logo_url = crate::presenters::accounts::fresh_account_logo_path_in_zone(
            account.as_ref(),
            None,
            &time_zone,
        );
        Ok(Self {
            current_user,
            account: summary,
            custom_styles: account.and_then(|account| account.custom_styles),
            platform: crate::presenters::accounts::platform(c),
            app_version: app.config.app_version.clone(),
            chrome: campfire_retained::Chrome {
                service_worker_auto_register,
                service_worker_url,
            },
        })
    }
}

impl RequestContext {
    /// Render retained HTML with the request's CSRF/CSP secrets and lazy flash sweep.
    pub fn render_retained(
        &self,
        c: &mut Ctx,
        render: impl FnOnce(&campfire_retained::Context) -> askama::Result<String>,
    ) -> Result<String> {
        let secrets = RequestSecrets {
            tokens: Box::new(KitTokens(c.authenticity_tokens())),
            csp_nonce: c.content_security_policy_nonce(),
        };
        #[cfg(any(test, feature = "test-support"))]
        let secrets = crate::request_secrets::fixed_render_secrets().unwrap_or(secrets);
        let flash = c.peek_flash();
        let asset_path = |path: &str| campfire_static_assets::asset_path(path);
        let ctx = campfire_retained::Context {
            current_user: self.current_user.clone(),
            account: self.account.clone(),
            flash_notice: flash.notice().map(str::to_string),
            flash_alert: flash.alert().map(str::to_string),
            custom_styles: self.custom_styles.clone(),
            platform: self.platform.clone(),
            app_version: self.app_version.clone(),
            base_url: c.url_for(""),
            asset_path: &asset_path,
            chrome: campfire_retained::Chrome {
                service_worker_auto_register: self.chrome.service_worker_auto_register,
                service_worker_url: self.chrome.service_worker_url.clone(),
            },
        };
        let (result, read_flash) = campfire_view_kit::flash::track_reads(|| {
            request_forgery::rendering_with(secrets, || render(&ctx))
        });
        if read_flash {
            c.flash();
        }
        result.map_err(Error::internal)
    }
}

/// The kit's tokens for the views' `form_authenticity_token`.
pub struct KitTokens(pub campfire_kit::csrf::AuthenticityTokens);

impl request_forgery::AuthenticityTokens for KitTokens {
    fn global(&self) -> String {
        self.0.global()
    }

    fn for_form(&self, action: &str, method: &str) -> String {
        self.0.for_form(action, method)
    }
}

/// `Current.user` as the layout's meta tags and helpers see it.
pub fn current_user(secrets: &rails_compat::Secrets, user: &User) -> CurrentUser {
    CurrentUser {
        id: user.id,
        name: user.name.clone(),
        administrator: user.can_administer(None, false),
        bot: user.is_bot(),
        avatar_url: crate::presenters::avatar_path(secrets, user),
        preferences: UserPreferences::default(),
    }
}

/// Read persisted user preferences and the flagged read-only owner projections used by the
/// actual request layout. No caller-supplied expected display facts are needed.
pub fn user_preferences(
    conn: &campfire_db::Connection,
    user_id: i64,
    now: jiff::Timestamp,
) -> campfire_db::Result<UserPreferences> {
    crate::presenters::layout_preferences::for_user(conn, user_id, now)
}

/// `Current.account` for the layout: its name, `fresh_account_logo_path` and whether a logo is
/// attached.
pub fn account_summary(account: Option<&Account>, has_logo: bool) -> AccountSummary {
    AccountSummary {
        name: account
            .map(|account| account.name.clone())
            .unwrap_or_default(),
        logo_url: crate::presenters::accounts::fresh_account_logo_path(account, None),
        has_logo,
    }
}

/// The implicit render's template lookup (`default_render`): an action whose only template is
/// `<action>.html.erb` can't answer a request that doesn't accept HTML, which is
/// `ActionController::UnknownFormat` (406), and the response carries the template's format
/// whatever the `Accept` header preferred.
pub fn find_template(c: &mut Ctx, template: campfire_kit::Format) -> Result<()> {
    c.respond_to(&[template]).map(|_| ())
}

/// [`retained_page_or_frame`] for a retained page. Full documents still send the stylesheet preload
/// `Link` header; the context itself does not carry those tags or an import map.
pub async fn retained_page_or_frame(
    c: &mut Ctx,
    status: StatusCode,
    full: impl FnOnce(&campfire_retained::Context) -> askama::Result<String>,
    frame: impl FnOnce(&campfire_retained::Context) -> askama::Result<String>,
) -> Result {
    find_template(c, &format::HTML)?;
    retained_page_or_frame_in_any_format(c, status, full, frame).await
}

pub async fn retained_page_or_frame_in_any_format(
    c: &mut Ctx,
    status: StatusCode,
    full: impl FnOnce(&campfire_retained::Context) -> askama::Result<String>,
    frame: impl FnOnce(&campfire_retained::Context) -> askama::Result<String>,
) -> Result {
    let layout = RequestContext::load(c).await?;
    if c.is_turbo_frame_request() {
        let html = layout.render_retained(c, frame)?;
        Ok(layout.frame(c, status, html))
    } else {
        let html = layout.render_retained(c, full)?;
        Ok(layout.page(c, status, html))
    }
}

/// A retained document even for a Turbo-Frame request (controllers that force their own layout).
pub async fn retained_document(
    c: &mut Ctx,
    status: StatusCode,
    full: impl FnOnce(&campfire_retained::Context) -> askama::Result<String>,
) -> Result {
    let layout = RequestContext::load(c).await?;
    let html = layout.render_retained(c, full)?;
    Ok(layout.page(c, status, html))
}
impl RequestContext {
    pub fn page(&self, c: &mut Ctx, status: StatusCode, html: String) -> Response {
        let links = &campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")])
            .preload_links;
        let existing = c
            .headers
            .get("link")
            .and_then(|v| v.to_str().ok())
            .unwrap_or("")
            .to_string();
        c.set_header(
            "link",
            &campfire_assets::append_preload_links(&existing, links),
        );
        c.render(status, &format::HTML, html)
    }
    pub fn frame(&self, c: &mut Ctx, status: StatusCode, html: String) -> Response {
        c.render(status, &format::HTML, html)
    }
}
