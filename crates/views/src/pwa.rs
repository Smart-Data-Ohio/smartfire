//! Classic installation instructions. The installed PWA assets live in `campfire_spa::pwa`.

use askama::Template;
use crate::ViewContext;
use crate::helpers as h;

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
