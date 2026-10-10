//! Plain models for `app/views/saved_items`.
use crate::helpers::filters;
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;
pub trait ItemRendering {
    fn created(&self, _ctx: &ViewContext) -> h::Html;
    fn reminder(&self, _ctx: &ViewContext) -> h::Html;
    fn status_button(&self, filter: &str) -> h::Html;
    fn remove_button(&self, filter: &str) -> h::Html;
}
impl ItemRendering for Item {
    fn created(&self, _ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag_iso(
            &self.created_at,
            "time",
            h::attrs().class("saved-item__time"),
            "",
        )
    }
    fn reminder(&self, _ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag_iso(
            self.reminded_at.as_deref().or(self.remind_at.as_deref()).expect("has reminder"),
            "time",
            h::attrs(),
            "",
        )
    }
    fn status_button(&self, filter: &str) -> h::Html {
        h::button_to_form_params(
            &self.path(filter),
            h::attrs().method("patch").class("btn btn--plain"),
            h::attrs().class("saved-item__action-form"),
            if self.done() { "Reopen" } else { "Mark done" },
            &[(
                "saved_item[status]",
                if self.done() { "in_progress" } else { "done" },
            )],
        )
    }
    fn remove_button(&self, filter: &str) -> h::Html {
        h::button_to_form(
            &self.path(filter),
            h::attrs().method("delete").class("btn btn--plain"),
            h::attrs().class("saved-item__action-form"),
            "Remove",
        )
    }
}

#[derive(Template)]
#[template(path = "saved_items/_item.html")]
pub struct ItemPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub item: &'a Item,
    pub status_filter: &'a str,
}
#[derive(Template)]
#[template(path = "saved_items/index.html", blocks = ["head", "content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub items: &'a [Item],
    pub status_filter: &'a str,
}
impl Index<'_> {
    pub fn filter_link(&self, filter: &str, label: &str) -> h::Html {
        let active = self.status_filter == filter;
        h::link_to_text(
            label,
            &format!("{}?status={filter}", campfire_routes::saved_items()),
            h::attrs()
                .class(if active {
                    "btn saved-items__filter is-active"
                } else {
                    "btn saved-items__filter"
                })
                .attr_opt("aria-current", active.then_some("page")),
        )
    }
    pub fn item(&self, item: &Item) -> h::Html {
        h::raw(
            ItemPartial {
                ctx: self.ctx,
                item,
                status_filter: self.status_filter,
            }
            .render()
            .expect("saved item renders"),
        )
    }
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Saved for later".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar saved-items")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
pub use campfire_presentation::saved_items::*;
