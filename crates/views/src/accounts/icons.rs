//! `accounts/icons/index` and its icon row.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
#[derive(Clone, Debug)]
pub struct Icon {
    pub id: i64,
    pub name: String,
    pub title: String,
    pub creator_name: String,
}
#[derive(Clone, Debug, Default)]
pub struct Form {
    pub name: Option<String>,
    pub title: Option<String>,
    pub errors: Vec<String>,
    pub invalid_fields: Vec<String>,
}
impl Form {
    fn field(&self, name: &str, tag: h::Html) -> h::Html {
        if self.invalid_fields.iter().any(|f| f == name) {
            h::content_tag(
                "div",
                h::attrs().class("field_with_errors"),
                &tag.to_string(),
            )
        } else {
            tag
        }
    }
}
#[derive(Template)]
#[template(path="accounts/icons/index.html",blocks=["head","nav","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub icons: Vec<Icon>,
    pub icon: Form,
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Workspace icons".into())
    }
}
