//! Rendering helpers for these controllers: pages in the application layout (or turbo-rails'
//! frame layout for Turbo-Frame requests), and partials rendered outside a request for
//! broadcasts (`ApplicationController.render`).

use campfire_db::Account;
use campfire_kit::{Ctx, Format, Result, StatusCode};
use campfire_views::helpers as h;
use campfire_views::layouts::{Application, FrameLayout};
use campfire_views::{Platform, ViewContext};

use crate::app::AppState;
use crate::controllers::presenters::view_context::{Layout, account_summary, find_template};

/// A template that extends `layouts/application` itself (with `blocks = ["head", "content"]`):
/// the full page, or for a Turbo-Frame request its `head` and `content` in turbo-rails' frame
/// layout (`layout -> { "turbo_rails/frame" if turbo_frame_request? }`).
///
/// `framed_page!(c, StatusCode::OK, |ctx| rooms::Show { ctx, show: &show }).await`
#[macro_export]
macro_rules! framed_page {
    ($c:expr, $status:expr, |$ctx:ident| $page:expr) => {
        $crate::controllers::presenters::view_context::page_or_frame(
            $c,
            $status,
            |$ctx| askama::Template::render(&$page),
            |$ctx| {
                let page = $page;
                campfire_views::layouts::frame($ctx, page.as_head(), page.as_content())
            },
        )
    };
}
pub use crate::framed_page;

/// Same shape as [`framed_page`], rendering through the retained shell.
#[macro_export]
macro_rules! retained_page {
    ($c:expr, $status:expr, |$ctx:ident| $page:expr) => {
        $crate::controllers::presenters::view_context::retained_page_or_frame(
            $c,
            $status,
            |$ctx| askama::Template::render(&$page),
            |$ctx| {
                let page = $page;
                campfire_retained::layouts::frame($ctx, page.as_head(), page.as_content())
            },
        )
    };
}
pub use crate::retained_page;

/// A content-only template: Rails wraps it in the application layout, or in turbo-rails'
/// `layouts/turbo_rails/frame` when the request carries a `Turbo-Frame` header.
pub async fn content(c: &mut Ctx, status: StatusCode, render: impl FnOnce(&ViewContext) -> askama::Result<String>) -> Result {
    content_with_page_title(c, status, None, render).await
}

/// A content template that sets Rails' `@page_title` before the application layout renders.
pub async fn titled_content(c: &mut Ctx, status: StatusCode, title: &str, render: impl FnOnce(&ViewContext) -> askama::Result<String>) -> Result {
    content_with_page_title(c, status, Some(title), render).await
}

async fn content_with_page_title(c: &mut Ctx, status: StatusCode, title: Option<&str>, render: impl FnOnce(&ViewContext) -> askama::Result<String>) -> Result {
    use askama::Template;

    find_template(c, &campfire_kit::format::HTML)?;
    let layout = Layout::load(c).await?;
    let frame = c.is_turbo_frame_request();
    let html = layout.render(c, |ctx| {
        let content = h::raw(render(ctx)?);
        if frame { FrameLayout { ctx, head: h::empty(), content }.render() } else {
            let mut application = Application::new(ctx, content);
            application.page_title = title.map(str::to_string);
            application.render()
        }
    })?;
    Ok(if frame { layout.frame(c, status, html) } else { layout.page(c, status, html) })
}

/// A content-only template in the application layout even for Turbo-Frame requests: a controller
/// that declares its own `layout` (MessagesController's `layout false, only: :index`) replaces
/// turbo-rails' frame layout choice.
pub async fn content_in_application_layout(
    c: &mut Ctx,
    status: StatusCode,
    render: impl FnOnce(&ViewContext) -> askama::Result<String>,
) -> Result {
    use askama::Template;

    find_template(c, &campfire_kit::format::HTML)?;
    let layout = Layout::load(c).await?;
    let html = layout.render(c, |ctx| Application::new(ctx, h::raw(render(ctx)?)).render())?;
    Ok(layout.page(c, status, html))
}

/// A template rendered with `layout false` (or a turbo stream), no layout, labelled with the
/// template's format.
pub async fn bare(c: &mut Ctx, status: StatusCode, template: Format, render: impl FnOnce(&ViewContext) -> askama::Result<String>) -> Result {
    find_template(c, template)?;
    let layout = Layout::load(c).await?;
    let html = layout.render(c, render)?;
    Ok(c.render(status, template, html))
}

/// Renders with the `ViewContext` `ApplicationController.render` has: no request, no
/// `Current.user`, no CSRF tokens, and Rails' configured job-renderer route defaults.
/// A scoped event write retains the actor's Time.zone; background work defaults to UTC.
pub fn render_detached<T>(app: &AppState, account: Option<&Account>, render: impl FnOnce(&ViewContext) -> T) -> T {
    render_detached_at(app, account, default_renderer_base_url(app), render)
}

/// [`render_detached`] during a request: URLs get the request's host through
/// `default_url_options` (`SetCurrentRequest`), see [`renderer_base_url`].
pub fn render_detached_at<T>(app: &AppState, account: Option<&Account>, base_url: &str, render: impl FnOnce(&ViewContext) -> T) -> T {
    render_detached_in_zone(app, account, base_url, &renderer_time_zone(), render)
}

/// Nested request partials keep Rails' `Time.zone` without carrying Current.user or
/// session secrets into the detached renderer. Background callers retain the UTC default.
pub fn render_detached_in_zone<T>(app: &AppState, account: Option<&Account>, base_url: &str,
    time_zone: &campfire_views::time::Zone, render: impl FnOnce(&ViewContext) -> T) -> T {
    let asset_path = |path: &str| campfire_assets::asset_path(path);
    let stylesheets = crate::controllers::presenters::view_context::stylesheet_tags();
    let signed_stream_name = |streamables: &[&str]| rails_compat::turbo::signed_stream_name(&app.secrets, streamables);
    let ctx = ViewContext {
        current_user: None,
        account: account_summary(account, false),
        flash_notice: None,
        flash_alert: None,
        platform: Platform::default(),
        vapid_public_key: app.vapid_public_key(),
        asset_path: &asset_path,
        importmap_tags: campfire_assets::javascript_importmap_tags(),
        stylesheet_tags: &stylesheets.html,
        custom_styles: None,
        cable_url: "/cable".into(),
        base_url: base_url.to_string(),
        request_url: format!("{base_url}/"),
        referrer: None,
        last_room_visited_id: None,
        app_version: app.config.app_version.clone(),
        signed_stream_name: &signed_stream_name,
        time_zone: time_zone.clone(),
        chrome: Default::default(),
    };
    // Renders outside a request (broadcasts from jobs) share the fragment cache too.
    campfire_views::fragment_cache::with(&app.fragment_cache, || render(&ctx))
}

pub use campfire_runtime::context::*;

use crate::controllers::presenters::{ view_context::LayoutRendering};
