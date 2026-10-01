//! Plain models for `app/views/saved_items`.
use crate::helpers::filters;
use crate::{ViewContext, helpers as h, layouts::Page};
use askama::Template;
use jiff::Timestamp;

pub struct Item {
    pub id: i64,
    pub status: String,
    pub created_at: Timestamp,
    pub remind_at: Option<Timestamp>,
    pub reminded_at: Option<Timestamp>,
    pub room_name: String,
    pub author_name: String,
    pub body: String,
    pub message_path: String,
}
impl Item {
    pub fn done(&self) -> bool {
        self.status == "done"
    }
    pub fn status_class(&self) -> String {
        self.status.replace('_', "-")
    }
    pub fn created(&self, ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
            self.created_at,
            "time",
            h::attrs().class("saved-item__time"),
            "",
        )
    }
    pub fn reminder(&self, ctx: &ViewContext) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
            self.reminded_at.or(self.remind_at).expect("has reminder"),
            "time",
            h::attrs(),
            "",
        )
    }
    fn path(&self, filter: &str) -> String {
        format!("{}?status={filter}", campfire_routes::saved_item(self.id))
    }
    pub fn status_button(&self, filter: &str) -> h::Html {
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
    pub fn remove_button(&self, filter: &str) -> h::Html {
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
