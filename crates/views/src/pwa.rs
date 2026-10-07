//! Views for `reference/app/views/pwa`.

use askama::Template;

use crate::ViewContext;
use crate::helpers as h;

/// `pwa/service_worker.js`, served verbatim.
pub const SERVICE_WORKER_JS: &str = include_str!("../templates/pwa/service_worker.js");

/// `pwa/manifest.json.erb`. Preserve ERB's HTML escaping inside JSON, including `&amp;` in
/// the small-logo URL. Byte parity includes that behavior.
#[derive(Template)]
#[template(path = "pwa/manifest.json")]
pub struct Manifest<'a> {
    /// `Current.account&.name`.
    pub account_name: Option<String>,
    /// `fresh_account_logo_path(size: :small)`.
    pub logo_path_small: String,
    /// `fresh_account_logo_path`.
    pub logo_path: String,
    /// `request.base_url`, for `image_url`.
    pub base_url: String,
    /// The start URL and scope share one setting; classic keeps `/`.
    pub root: &'a str,
    pub new_room_url: String,
    pub profile_url: String,
    pub asset_path: &'a dyn Fn(&str) -> String,
}

impl Manifest<'_> {
    /// `image_url(source)`.
    fn image_url(&self, source: &str) -> String {
        format!("{}{}", self.base_url, (self.asset_path)(source))
    }
}

/// `pwa/_install_instructions.html.erb`.
#[derive(Template)]
#[template(path = "pwa/_install_instructions.html")]
pub struct InstallInstructions<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// `pwa/_browser_settings.html.erb`.
#[derive(Template)]
#[template(path = "pwa/_browser_settings.html")]
pub struct BrowserSettings<'a> {
    pub ctx: &'a ViewContext<'a>,
}

/// `pwa/_system_settings.html.erb`.
#[derive(Template)]
#[template(path = "pwa/_system_settings.html")]
pub struct SystemSettings<'a> {
    pub ctx: &'a ViewContext<'a>,
}
