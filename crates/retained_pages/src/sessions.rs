//! Sign-in, the session-transfer handoff, and the unsupported-browser rejection.

use askama::Template;

use crate::helpers::{self as h, filters};
use crate::layouts::Page;

pub use campfire_views::accounts::HelpContact;
pub use campfire_views::sessions::ALLOW_BROWSER_VERSIONS;

#[derive(Template)]
#[template(path = "sessions/new.html", blocks = ["head", "content"])]
pub struct New<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub email_address: Option<String>,
    pub help_contact: Option<HelpContact>,
    pub google_sign_in_domains: Vec<String>,
}

impl New<'_> {
    fn google_sign_in(&self) -> h::Html {
        if self.google_sign_in_domains.is_empty() {
            h::empty()
        } else {
            h::raw(
                GoogleSignIn {
                    domains: self.google_sign_in_domains.clone(),
                }
                .render()
                .unwrap(),
            )
        }
    }
}

#[derive(Template)]
#[template(path = "sessions/_google_sign_in.html")]
pub struct GoogleSignIn {
    pub domains: Vec<String>,
}

impl GoogleSignIn {
    fn domain_sentence(&self) -> String {
        h::to_sentence(
            &self
                .domains
                .iter()
                .map(|domain| format!("@{domain}"))
                .collect::<Vec<_>>(),
            " and ",
        )
    }
}

impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Sign in".into())
    }
}

/// Unsupported browsers get this page on the retained shell, not the classic application layout.
#[derive(Template)]
#[template(path = "sessions/incompatible_browser.html", blocks = ["head", "content"])]
pub struct IncompatibleBrowser<'a> {
    pub ctx: &'a crate::Context<'a>,
}

impl Page for IncompatibleBrowser<'_> {
    fn page_title(&self) -> Option<String> {
        Some(
            if self.ctx.platform.apple_messages {
                "Smartfire"
            } else {
                "Unsupported browser"
            }
            .into(),
        )
    }
}

#[derive(Template)]
#[template(path = "sessions/transfers/show.html", blocks = ["head", "content"])]
pub struct TransferShow<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub action: String,
}

impl Page for TransferShow<'_> {}
