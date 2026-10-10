//! The join-code enrollment page.

use askama::Template;

use crate::helpers::{self as h, filters};
use crate::layouts::Page;
use crate::sessions::HelpContact;

#[derive(Template)]
#[template(path = "users/new.html", blocks = ["head", "content"])]
pub struct New<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub join_code: String,
    pub description: String,
    pub help_contact: Option<HelpContact>,
}

impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Sign up".into())
    }
}
