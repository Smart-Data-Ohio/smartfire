//! First-run account setup.

use askama::Template;

use crate::helpers::{self as h, filters};
use crate::layouts::Page;

#[derive(Template)]
#[template(path = "first_runs/show.html", blocks = ["head", "content"])]
pub struct Show<'a> {
    pub ctx: &'a crate::Context<'a>,
}

impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Set up Smartfire".into())
    }
}
