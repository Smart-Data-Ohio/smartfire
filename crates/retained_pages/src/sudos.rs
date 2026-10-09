//! Sudo confirmation and the auto-submitting continuation form.

use askama::Template;
use serde_json::Value;

use crate::helpers::{self as h, filters};
use crate::layouts::Page;

pub use campfire_views::sudos::replay_fields;

#[derive(Template)]
#[template(path = "sudos/new.html", blocks = ["head", "content"])]
pub struct New<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub password: bool,
    pub totp: bool,
    pub google: bool,
}

impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Confirm it's you".into())
    }
}

impl New<'_> {
    fn totp_options(&self) -> h::Attrs {
        let attrs = h::attrs()
            .required(true)
            .class("input auth-code")
            .attr("autofocus", !self.password)
            .autocomplete("one-time-code")
            .attr("inputmode", "numeric")
            .maxlength(10)
            .placeholder("Enter your authenticator code")
            .attr("aria-label", "Authenticator code");
        h::rejected_field(self.ctx, attrs)
    }
}

#[derive(Template)]
#[template(path = "sudos/continue.html", blocks = ["head", "content"])]
pub struct Continue<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub method: String,
    pub path: String,
    pub params: Value,
}

impl Page for Continue<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Continuing".into())
    }
}
