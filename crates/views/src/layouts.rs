//! Views for `reference/app/views/layouts`.
//!
//! A page template starts with `{% extends "layouts/application.html" %}`, fills the `content`
//! block (and `head`, `nav`, `footer`, `sidebar` for `content_for`), has a `ctx: &ViewContext`
//! field, and implements [`Page`] for what the ERB sets as `@page_title` and `@body_class`.

use askama::Template;

pub use campfire_view_kit::{NotificationSounds, UserPreferences};

use crate::helpers::{self as h, filters};

/// The instance variables a page template hands to the application layout.
pub trait Page {
    /// `@page_title`; the layout falls back to "Smartfire".
    fn page_title(&self) -> Option<String> {
        None
    }

    /// `@body_class`.
    fn body_class(&self) -> Option<&str> {
        None
    }

    fn page_description(&self) -> Option<&str> {
        None
    }

    /// `content_for?(:sidebar)`: whether the page fills the `sidebar` block with something not
    /// blank. The layout only offers the workspace drawer's toggle then.
    fn has_sidebar(&self) -> bool {
        false
    }
}

#[derive(Template)]
#[template(path = "layouts/_huddle.html")]
pub struct Huddle<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
    pub user: &'a crate::CurrentUser,
}

#[derive(Template)]
#[template(path = "layouts/_huddle_invitation.html")]
pub struct HuddleInvitation;

#[derive(Template)]
#[template(path = "layouts/_huddle_join_notice.html")]
pub struct HuddleJoinNotice<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
}

/// The application layout around page parts rendered elsewhere, for templates that don't extend
/// the layout themselves (Rails picks the layout per request). Each part is what the ERB's
/// `content_for` / `yield` would have produced.
#[derive(Template)]
#[template(path = "layouts/application_wrapper.html")]
pub struct Application<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
    /// `@page_title`.
    pub page_title: Option<String>,
    /// `@body_class`.
    pub body_class: Option<String>,
    pub head: h::Html,
    pub nav: h::Html,
    pub content: h::Html,
    pub footer: h::Html,
    pub member_panel: h::Html,
    pub thread_panel: h::Html,
    pub sidebar: h::Html,
}

impl<'a> Application<'a> {
    /// Just a page body, with no title, body class or `content_for` regions.
    pub fn new(ctx: &'a crate::ViewContext<'a>, content: h::Html) -> Self {
        Application {
            ctx,
            page_title: None,
            body_class: None,
            head: h::empty(),
            nav: h::empty(),
            content,
            footer: h::empty(),
            member_panel: h::empty(),
            thread_panel: h::empty(),
            sidebar: h::empty(),
        }
    }
}

impl Page for Application<'_> {
    fn page_title(&self) -> Option<String> {
        self.page_title.clone()
    }

    fn body_class(&self) -> Option<&str> {
        self.body_class.as_deref()
    }

    fn has_sidebar(&self) -> bool {
        !h::is_blank(&self.sidebar.0)
    }
}

/// Minimal auth shell, also usable around detached page content.
#[derive(Template)]
#[template(path = "layouts/auth_wrapper.html")]
pub struct Auth<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
    pub page_title: Option<String>,
    pub head: h::Html,
    pub content: h::Html,
}

impl Page for Auth<'_> {
    fn page_title(&self) -> Option<String> {
        self.page_title.clone()
    }
}

/// turbo-rails' `layouts/turbo_rails/frame.html.erb`, used instead of the application layout
/// whenever a request carries a `Turbo-Frame` header. Pages expose their blocks for it through
/// askama's `blocks = ["head", "content"]` (see [`frame`]).
#[derive(Template)]
#[template(path = "layouts/turbo_rails/frame.html")]
pub struct FrameLayout<'a> {
    pub ctx: &'a crate::ViewContext<'a>,
    /// The page's `:head` content.
    pub head: h::Html,
    /// The page itself.
    pub content: h::Html,
}

/// Renders a page's `head` and `content` blocks in the Turbo-Frame layout:
/// `frame(ctx, page.as_head(), page.as_content())`.
pub fn frame(
    ctx: &crate::ViewContext,
    head: impl Template,
    content: impl Template,
) -> askama::Result<String> {
    FrameLayout {
        ctx,
        head: h::raw(head.render()?),
        content: h::raw(content.render()?),
    }
    .render()
}

/// What the application layout's chrome shows that other domains own. The controller gathers it
/// before rendering (see `PORTING.md`, "Layout view models"); `Default` is a signed-out page in an
/// app with nothing optional configured.
#[derive(Clone, Debug, Default)]
pub struct Chrome {
    /// Rails.env.test?: the layout disables token-driven motion only in tests.
    pub test_environment: bool,
    /// `service_worker_auto_register?`: true outside the test environment (the reference runs in
    /// production), or when the `enable_service_worker` cookie is present.
    pub service_worker_auto_register: bool,
    /// The script chosen for this request, in Turbo-refreshed head metadata, at scope `/`.
    pub service_worker_url: Option<String>,
    /// `Icons.client_icon_names` (the icons domain).
    pub brand_icon_names: Vec<String>,
    /// `Google::Picker.configured?` and its settings (the Google domain).
    pub google_picker: Option<GooglePicker>,
    /// `Huddle.configured?` (the huddles domain): renders the huddle, invitation and join-notice
    /// partials.
    pub huddle_configured: bool,
    /// `global_search_query`: `params[:q].to_s.squish.presence` on the searches controller only.
    pub global_search_query: Option<String>,
    /// `Current.user.searches.ordered.limit(10)` (the searches domain).
    pub recent_searches: Vec<RecentSearch>,
}

#[derive(Clone, Debug)]
pub struct GooglePicker {
    pub client_id: String,
    pub api_key: String,
    pub project_number: String,
}

#[derive(Clone, Debug)]
pub struct RecentSearch {
    pub id: i64,
    pub query: String,
}

/// The public layout receives its stylesheet from the asset owner, without workspace state.
#[derive(Template)]
#[template(path = "layouts/public_wrapper.html")]
pub struct Public {
    pub page_title: Option<String>,
    pub page_description: Option<String>,
    /// `stylesheet_link_tag "public", media: "all"`.
    pub public_stylesheet: h::Html,
    pub content: h::Html,
}

impl Page for Public {
    fn page_title(&self) -> Option<String> {
        self.page_title.clone()
    }
    fn page_description(&self) -> Option<&str> {
        self.page_description.as_deref()
    }
}

#[derive(Template)]
#[template(path = "layouts/mailer.html")]
pub struct Mailer {
    pub content: h::Html,
}

#[derive(Template)]
#[template(path = "layouts/mailer.txt", escape = "none")]
pub struct TextMailer<'a> {
    pub content: &'a str,
}

/// The app adapter supplies domain-owned values; views never perform domain queries.
pub trait ChromeSource {
    fn chrome(&self) -> Chrome;
    fn user_preferences(&self) -> UserPreferences;
}
