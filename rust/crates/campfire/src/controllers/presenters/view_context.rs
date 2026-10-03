//! Builds the `campfire_views::ViewContext` every page renders with: what the application
//! layout and the helpers read from `Current`, `request`, `flash`, the session and the config
//! (`reference/app/views/layouts/application.html.erb` and `app/helpers`).
//!
//! Gather the per-request data with [`Layout::load`] (it reads the database), then render inside
//! [`Layout::render`], which lends the templates a `ViewContext` for this request:
//!
//! ```ignore
//! let layout = Layout::load(c).await?;
//! let html = layout.render(c, |ctx| sessions::New { ctx, email_address, help_contact }.render())?;
//! Ok(layout.page(c, StatusCode::OK, html))
//! ```

use std::sync::LazyLock;

use campfire_db::{Account, User};
use campfire_kit::{Ctx, Error, Response, Result, StatusCode, format};
use campfire_views::helpers::request_forgery::{self, RequestSecrets};
use campfire_views::layouts::{Chrome, UserPreferences};
use campfire_views::time::Zone;
use campfire_views::{AccountSummary, CurrentUser, Platform, ViewContext};

use crate::app::AppCtx;
use crate::concerns;

/// Everything the layout needs, loaded before rendering.
#[derive(Clone)]
pub(crate) struct RenderedSettings(pub campfire_db::UserStatusSettings);

#[derive(Debug, Clone)]
pub struct Layout {
    pub current_user: Option<CurrentUser>,
    pub account: AccountSummary,
    pub custom_styles: Option<String>,
    pub platform: Platform,
    pub last_room_visited_id: Option<i64>,
    pub vapid_public_key: Option<String>,
    pub app_version: String,
    /// `Time.zone` for the request (`SetTimeZone`).
    pub time_zone: Zone,
    /// The layout's chrome from other domains. Only what this crate can answer yet is filled in;
    /// see `campfire_views::layouts::Chrome` for the owners.
    pub chrome: Chrome,
}

/// These templates never call `link_back_to_last_room_visited`. Leave its input
/// unloaded, as Rails does; templates that use it retain the normal room lookup.
#[derive(Clone)]
struct UnusedRoomBackLink;
pub(crate) fn omit_unused_room_back_link(c: &mut Ctx) { c.set_current(UnusedRoomBackLink); }

/// Reuse the zone already carried by the authenticated row; no association reload.
pub async fn time_zone(c: &Ctx) -> Result<Zone> {
    if let Some(loaded) = c.current::<super::layout_preferences::LoadedPreferences>()
        && concerns::current_user(c).is_some_and(|user| user.id == loaded.user_id)
    {
        return Ok(loaded.time_zone());
    }
    Ok(Layout::load(c).await?.time_zone)
}

impl Layout {
    /// `Current.account`, `Current.user`, `last_room_visited` and the platform. With no account
    /// yet (first run) the account summary is blank: the pages that reference the account raise
    /// in Rails then, the others don't read it.
    pub async fn load(c: &Ctx) -> Result<Self> {
        let app = c.app();
        let secrets = app.secrets.clone();
        let user = concerns::current_user(c).cloned();
        let user_id = user.as_ref().map(|user| user.id);
        let now = c.now();
        let loaded = c.current::<super::layout_preferences::LoadedPreferences>().filter(|loaded| Some(loaded.user_id) == user_id).cloned();
        let (account, has_logo, mut preferences, mut chrome) = app
            .db
            .read(move |conn| {
                let account = Account::first(conn)?;
                let has_logo = match &account {
                    Some(account) => {
                        super::attachments::attached_blob(conn, "Account", account.id, "logo")?
                            .is_some()
                    }
                    None => false,
                };
                let preferences = match user_id {
                    Some(user_id) => match loaded {
                        Some(loaded) => loaded.load(conn)?,
                        None => user_preferences(conn, user_id, now)?,
                    },
                    None => UserPreferences::default(),
                };
                Ok((account, has_logo, preferences, chrome(conn, user_id)?))
            })
            .await
            .map_err(Error::internal)?;
        let last_room_visited_id = if user.is_some() && c.current::<UnusedRoomBackLink>().is_none() {
            concerns::last_room_visited(c).await?.map(|room| room.id)
        } else {
            None
        };

        let time_zone = Zone::for_user(preferences.time_zone.as_deref());
        if let Some(RenderedSettings(settings)) = c.current::<RenderedSettings>()
            && Some(settings.user.id) == user_id
        {
            apply_settings_preferences(
                &mut preferences,
                settings,
                campfire_db::Timestamp::from_jiff(now),
            );
        }
        let current_user = user.as_ref().map(|user| CurrentUser {
            preferences,
            ..current_user(&secrets, user)
        });
        chrome.google_picker = user.as_ref().and(app.config.google_picker.clone());
        chrome.huddle_configured = app.config.huddle.configured();
        chrome.global_search_query = if c.request.path().starts_with("/searches") {
            crate::controllers::searches::display_query(c)
        } else {
            None
        };
        let mut summary = account_summary(account.as_ref(), has_logo);
        summary.logo_url = super::accounts::fresh_account_logo_path_in_zone(
            account.as_ref(), None, &time_zone,
        );
        Ok(Self {
            current_user,
            account: summary,
            custom_styles: account.and_then(|account| account.custom_styles),
            platform: super::accounts::platform(c),
            last_room_visited_id,
            vapid_public_key: app.vapid_public_key(),
            app_version: app.config.app_version.clone(),
            time_zone,
            chrome,
        })
    }

    /// Renders with a `ViewContext` for this request. The flash is read (and so swept at the end
    /// of the request) the way the layout's `flash[:notice]` / `flash[:alert]` read it. The
    /// templates get this request's authenticity tokens and CSP nonce (`csrf_meta_tags`, forms,
    /// `csp_meta_tag` and the importmap tags), which puts the CSRF token in the session.
    pub fn render(
        &self,
        c: &mut Ctx,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String> {
        let secrets = RequestSecrets {
            tokens: Box::new(KitTokens(c.authenticity_tokens())),
            csp_nonce: c.content_security_policy_nonce(),
        };
        #[cfg(test)]
        let secrets = super::test_support::fixed_render_secrets().unwrap_or(secrets);
        self.render_with_secrets(c, Some(secrets), render)
    }

    /// Token-free partials use the viewer's time zone without creating a CSRF session.
    pub fn render_without_secrets(
        &self,
        c: &mut Ctx,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String> {
        self.render_with_secrets(c, None, render)
    }

    fn render_with_secrets(
        &self,
        c: &mut Ctx,
        secrets: Option<RequestSecrets>,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String> {
        let flash_notice = c.flash().notice().map(str::to_string);
        let flash_alert = c.flash().alert().map(str::to_string);
        let base_url = c.url_for("");
        let request_url = c.request.url();
        let referrer = c.request.referer().map(str::to_string);
        let stylesheets = stylesheet_tags();

        let asset_path = |path: &str| campfire_assets::asset_path(path);
        let app_secrets = c.app().secrets.clone();
        let signed_stream_name = move |streamables: &[&str]| {
            rails_compat::turbo::signed_stream_name(&app_secrets, streamables)
        };
        let ctx = ViewContext {
            current_user: self.current_user.clone(),
            account: self.account.clone(),
            flash_notice,
            flash_alert,
            platform: self.platform.clone(),
            vapid_public_key: self.vapid_public_key.clone(),
            asset_path: &asset_path,
            importmap_tags: campfire_assets::javascript_importmap_tags(),
            stylesheet_tags: &stylesheets.html,
            custom_styles: self.custom_styles.clone(),
            cable_url: "/cable".into(),
            base_url,
            request_url,
            referrer,
            last_room_visited_id: self.last_room_visited_id,
            app_version: self.app_version.clone(),
            signed_stream_name: &signed_stream_name,
            time_zone: self.time_zone.clone(),
            chrome: self.chrome.clone(),
        };
        match secrets {
            Some(secrets) => request_forgery::rendering_with(secrets, || render(&ctx)),
            None => render(&ctx),
        }
        .map_err(Error::internal)
    }

    /// A page rendered in the application layout: `text/html`, plus the `Link` preload header
    /// `stylesheet_link_tag` adds (`config.action_view.preload_links_header`).
    pub fn page(&self, c: &mut Ctx, status: StatusCode, html: String) -> Response {
        let links = &stylesheet_tags().preload_links;
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

    /// A page rendered in turbo-rails' frame layout (no stylesheets, so no `Link` header).
    pub fn frame(&self, c: &mut Ctx, status: StatusCode, html: String) -> Response {
        c.render(status, &format::HTML, html)
    }
}

/// The kit's tokens for the views' `form_authenticity_token`.
struct KitTokens(campfire_kit::csrf::AuthenticityTokens);

impl request_forgery::AuthenticityTokens for KitTokens {
    fn global(&self) -> String {
        self.0.global()
    }

    fn for_form(&self, action: &str, method: &str) -> String {
        self.0.for_form(action, method)
    }
}

/// The layout's `stylesheet_link_tag :all, "data-turbo-track": "reload"`: the assets are fixed at
/// build time, so it renders once per process.
pub fn stylesheet_tags() -> &'static campfire_assets::StylesheetTags {
    static TAGS: LazyLock<campfire_assets::StylesheetTags> = LazyLock::new(|| {
        campfire_assets::stylesheet_link_tag_all(&[("data-turbo-track", "reload")])
    });
    &TAGS
}

/// `Current.user` as the layout's meta tags and helpers see it.
pub fn current_user(secrets: &rails_compat::Secrets, user: &User) -> CurrentUser {
    CurrentUser {
        id: user.id,
        name: user.name.clone(),
        administrator: user.can_administer(None, false),
        bot: user.is_bot(),
        avatar_url: super::avatar_path(secrets, user),
        preferences: UserPreferences::default(),
    }
}

/// Read persisted user preferences and the flagged read-only owner projections used by the
/// actual request layout. No caller-supplied expected display facts are needed.
pub(crate) fn user_preferences(
    conn: &campfire_db::Connection,
    user_id: i64,
    now: jiff::Timestamp,
) -> campfire_db::Result<UserPreferences> {
    super::layout_preferences::for_user(conn, user_id, now)
}

/// Icons.client_icon_names and the viewer's ten ordered recent searches. WS14g's Picker,
/// WS13's huddle configuration and WS8b-m's searches-controller query remain flagged inputs.
pub(crate) fn chrome(
    conn: &campfire_db::Connection,
    user_id: Option<i64>,
) -> campfire_db::Result<Chrome> {
    let mut chrome = Chrome {
        service_worker_auto_register: true,
        brand_icon_names: super::client_icon_names(conn)?,
        ..Chrome::default()
    };
    if let Some(id) = user_id {
        chrome.recent_searches = campfire_db::Search::ordered_for_user(conn, id)?
            .into_iter()
            .take(10)
            .map(|search| campfire_views::layouts::RecentSearch {
                id: search.id,
                query: search.query,
            })
            .collect();
    }
    Ok(chrome)
}

/// `Current.account` for the layout: its name, `fresh_account_logo_path` and whether a logo is
/// attached.
pub fn account_summary(account: Option<&Account>, has_logo: bool) -> AccountSummary {
    AccountSummary {
        name: account
            .map(|account| account.name.clone())
            .unwrap_or_default(),
        logo_url: super::accounts::fresh_account_logo_path(account, None),
        has_logo,
    }
}

/// Renders a page in the application layout without the implicit render's template lookup: an
/// explicit `render template:` answers HTML whatever the request's format.
pub async fn page_in_any_format(
    c: &mut Ctx,
    status: StatusCode,
    full: impl FnOnce(&ViewContext) -> askama::Result<String>,
) -> Result {
    let layout = Layout::load(c).await?;
    let html = layout.render(c, full)?;
    Ok(layout.page(c, status, html))
}

/// Renders a page in the application layout, or, for templates that expose their `head`/`content`
/// blocks, turbo-rails' frame layout for a Turbo-Frame request
/// (`layout -> { "turbo_rails/frame" if turbo_frame_request? }`).
pub async fn page_or_frame(
    c: &mut Ctx,
    status: StatusCode,
    full: impl FnOnce(&ViewContext) -> askama::Result<String>,
    frame: impl FnOnce(&ViewContext) -> askama::Result<String>,
) -> Result {
    find_template(c, &format::HTML)?;
    page_or_frame_in_any_format(c, status, full, frame).await
}

/// [`page_or_frame`] without the template lookup (see [`page_in_any_format`]).
pub async fn page_or_frame_in_any_format(
    c: &mut Ctx,
    status: StatusCode,
    full: impl FnOnce(&ViewContext) -> askama::Result<String>,
    frame: impl FnOnce(&ViewContext) -> askama::Result<String>,
) -> Result {
    let layout = Layout::load(c).await?;
    if c.is_turbo_frame_request() {
        let html = layout.render(c, frame)?;
        Ok(layout.frame(c, status, html))
    } else {
        let html = layout.render(c, full)?;
        Ok(layout.page(c, status, html))
    }
}

/// The implicit render's template lookup (`default_render`): an action whose only template is
/// `<action>.html.erb` can't answer a request that doesn't accept HTML, which is
/// `ActionController::UnknownFormat` (406), and the response carries the template's format
/// whatever the `Accept` header preferred.
pub fn find_template(c: &mut Ctx, template: campfire_kit::Format) -> Result<()> {
    c.respond_to(&[template]).map(|_| ())
}

fn apply_settings_preferences(
    preferences: &mut UserPreferences,
    settings: &campfire_db::UserStatusSettings,
    now: campfire_db::Timestamp,
) {
    preferences.theme = Some(settings.theme.clone());
    preferences.text_size = Some(settings.text_size.clone());
    preferences.time_zone = settings.time_zone.clone();
    preferences.time_zone_explicit = settings.time_zone_explicit;
    let mut sounds = campfire_views::layouts::NotificationSounds {
        muted: settings.manual_dnd_active(now) || settings.presence_setting == "dnd",
        quiet_hours: settings
            .quiet_hours_enabled
            .then(|| {
                settings
                    .quiet_hours_start_minute
                    .zip(settings.quiet_hours_end_minute)
            })
            .flatten(),
        ..Default::default()
    };
    if settings.meeting_dnd_enabled && settings.meeting_status_enabled {
        sounds.meeting_quiet = settings
            .meeting_cache
            .as_ref()
            .map(|cache| cache.quiet_window_epochs(now))
            .unwrap_or_default();
    }
    if !settings.ooo_notify_enabled {
        if settings.manual_ooo_active(now) {
            sounds
                .ooo_quiet
                .push((0, settings.ooo_until.unwrap().jiff().as_second()));
        }
        if settings.ooo_calendar_enabled {
            sounds.ooo_quiet.extend(
                settings
                    .meeting_cache
                    .as_ref()
                    .map(|cache| cache.ooo_window_epochs(now))
                    .unwrap_or_default(),
            );
        }
    }
    preferences.notification_sounds = sounds;
}

#[cfg(test)]
pub(crate) fn user_preferences_at(conn: &campfire_db::Connection, user_id: i64, now: campfire_db::Timestamp) -> campfire_db::Result<UserPreferences> {
    user_preferences(conn,user_id,now.jiff())
}
