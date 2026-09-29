//! Views for `reference/app/views/pwa`.

use askama::Template;

use crate::ViewContext;
use crate::helpers as h;

/// `pwa/service_worker.js`, served verbatim.
pub const SERVICE_WORKER_JS: &str = include_str!("../templates/pwa/service_worker.js");

/// `pwa/manifest.json.erb`. ERB HTML-escapes the values into the JSON, so an account named `a\b`
/// or `"a"` made the manifest invalid and the logo URL came out as `?size=small&amp;v=...`; the
/// values are JSON strings here.
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
    pub asset_path: &'a dyn Fn(&str) -> String,
}

impl Manifest<'_> {
    /// `image_url(source)`.
    fn image_url(&self, source: &str) -> String {
        format!("{}{}", self.base_url, (self.asset_path)(source))
    }

    /// `value` as a JSON string, quotes included.
    fn json(&self, value: &str) -> askama::filters::Safe<String> {
        askama::filters::Safe(serde_json::to_string(value).expect("a string serializes"))
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
