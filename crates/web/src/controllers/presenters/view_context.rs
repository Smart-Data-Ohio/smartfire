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

use campfire_kit::{Ctx, Error, Result, StatusCode, format};
use campfire_views::helpers::request_forgery::{self, RequestSecrets};
use campfire_views::ViewContext;

use crate::app::AppCtx;
pub trait LayoutRendering {
    fn render(
        &self,
        c: &mut Ctx,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String>;
    fn render_without_secrets(
        &self,
        c: &mut Ctx,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String>;
    fn render_with_secrets(
        &self,
        c: &mut Ctx,
        secrets: Option<RequestSecrets>,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String>;
}
impl LayoutRendering for Layout {

    /// Renders with a `ViewContext` for this request. Only actual template access to
    /// `flash[:notice]` / `flash[:alert]` initializes flash and sweeps it at request end. The
    /// templates get this request's authenticity tokens and CSP nonce (`csrf_meta_tags`, forms,
    /// `csp_meta_tag` and the importmap tags), which puts the CSRF token in the session.
    fn render(
        &self,
        c: &mut Ctx,
        render: impl FnOnce(&ViewContext) -> askama::Result<String>,
    ) -> Result<String> {
        let secrets = RequestSecrets {
            tokens: Box::new(KitTokens(c.authenticity_tokens())),
            csp_nonce: c.content_security_policy_nonce(),
        };
        #[cfg(any(test, feature = "test-support"))]
        let secrets = super::render_secrets::fixed_render_secrets().unwrap_or(secrets);
        self.render_with_secrets(c, Some(secrets), render)
    }

    /// Token-free partials use the viewer's time zone without creating a CSRF session.
    fn render_without_secrets(
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
        let flash = c.peek_flash();
        let flash_notice = flash.notice().map(str::to_string);
        let flash_alert = flash.alert().map(str::to_string);
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
            // A head that names a worker loads the scripts that read it; otherwise the Rails map.
            importmap_tags: if self.chrome.service_worker_url.is_some() {
                campfire_assets::javascript_importmap_tags_selecting_worker()
            } else {
                campfire_assets::javascript_importmap_tags()
            },
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
        let (result, read_flash) = campfire_views::flash::track_reads(|| match secrets {
            Some(secrets) => request_forgery::rendering_with(secrets, || render(&ctx)),
            None => render(&ctx),
        });
        if read_flash {
            // Initializing Flash marks the old values for sweeping, exactly when
            // the ERB layout or content first accesses the lazy Rails flash hash.
            c.flash();
        }
        result.map_err(Error::internal)
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
pub use campfire_runtime::request_context::{RequestContext as Layout, *};
