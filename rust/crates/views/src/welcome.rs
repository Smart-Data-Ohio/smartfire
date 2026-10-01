//! Views for `reference/app/views/welcome`.

use askama::Template;

use crate::ViewContext;
use crate::helpers::{self as h, filters};
use crate::layouts::Page;

/// `welcome/show.html.erb`: shown to users who aren't in any room yet.
#[derive(Template)]
#[template(path = "welcome/show.html", blocks = ["head", "content", "sidebar"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    /// `Current.user.name`.
    pub current_user_name: String,
}

impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        Some("No rooms yet".into())
    }
    fn has_sidebar(&self) -> bool {
        true
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar")
    }
}
