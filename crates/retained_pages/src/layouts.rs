//! The auth and public shells, and turbo-rails' frame layout for a frame request.

use askama::Template;

use crate::helpers::{self as h, Html};

pub trait Page {
    fn page_title(&self) -> Option<String> {
        None
    }

    fn page_description(&self) -> Option<&str> {
        None
    }
}

#[derive(Template)]
#[template(path = "layouts/auth_wrapper.html")]
pub struct Auth<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub page_title: Option<String>,
    pub head: Html,
    pub content: Html,
}

impl Page for Auth<'_> {
    fn page_title(&self) -> Option<String> {
        self.page_title.clone()
    }
}

#[derive(Template)]
#[template(path = "layouts/turbo_rails/frame.html")]
pub struct FrameLayout<'a> {
    pub ctx: &'a crate::Context<'a>,
    pub head: Html,
    pub content: Html,
}

pub fn frame(
    ctx: &crate::Context,
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

#[derive(Template)]
#[template(path = "layouts/public_wrapper.html")]
pub struct Public {
    pub page_title: Option<String>,
    pub page_description: Option<String>,
    pub public_stylesheet: Html,
    pub content: Html,
}

impl Page for Public {
    fn page_title(&self) -> Option<String> {
        self.page_title.clone()
    }

    fn page_description(&self) -> Option<&str> {
        self.page_description.as_deref()
    }
}
