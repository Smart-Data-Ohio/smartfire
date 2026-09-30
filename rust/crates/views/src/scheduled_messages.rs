//! Plain view models for `app/views/scheduled_messages`.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
use jiff::Timestamp;
pub struct Item {
    pub id: i64,
    pub room_name: String,
    pub thread_name: Option<String>,
    pub body: String,
    pub send_at: Timestamp,
    pub sent_at: Option<Timestamp>,
    pub message_path: Option<String>,
}
impl Item {
    pub fn form(&self) -> h::FormWith {
        h::form_with(campfire_routes::scheduled_message(self.id))
            .model("scheduled_message")
            .method("patch")
            .class("scheduled-message__form")
    }
    pub fn time(&self, ctx: &ViewContext, past: bool) -> h::Html {
        crate::time::local_datetime_tag(
            &ctx.time_zone,
            if past {
                self.sent_at.expect("sent row")
            } else {
                self.send_at
            },
            "datetime",
            h::attrs(),
            "",
        )
    }
    pub fn send_field(&self, ctx: &ViewContext) -> h::Html {
        self.form().text_field(
            "send_at",
            None,
            h::attrs()
                .class("input")
                .value(ctx.time_zone.format(self.send_at, "%Y-%m-%dT%H:%M"))
                .type_("datetime-local"),
        )
    }
    pub fn submit(&self) -> h::Html {
        h::legacy_tag(
            "input",
            h::attrs()
                .type_("submit")
                .name("commit")
                .value("Save")
                .class("btn btn--primary btn--small")
                .data("disable_with", "Save"),
        )
    }
    pub fn action(&self, send: bool) -> h::Html {
        h::button_to_form(
            &if send {
                campfire_routes::send_now_scheduled_message(self.id)
            } else {
                campfire_routes::scheduled_message(self.id)
            },
            h::attrs()
                .method(if send { "post" } else { "delete" })
                .class("btn btn--plain btn--small"),
            h::attrs().class("scheduled-message__action-form"),
            if send { "Send now" } else { "Cancel" },
        )
    }
    pub fn excerpt(&self) -> String {
        h::truncate(&self.body, 500, "...")
    }
}
#[derive(Template)]
#[template(path = "scheduled_messages/_item.html")]
pub struct ItemPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub item: &'a Item,
    pub stranded: bool,
}
#[derive(Template)]
#[template(path = "scheduled_messages/_past_item.html")]
pub struct PastPartial<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub item: &'a Item,
}
#[derive(Template)]
#[template(path="scheduled_messages/index.html",blocks=["head","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub upcoming: &'a [Item],
    pub stranded: &'a [Item],
    pub past: &'a [Item],
}
impl Index<'_> {
    pub fn row(&self, item: &Item, stranded: bool) -> h::Html {
        h::raw(
            ItemPartial {
                ctx: self.ctx,
                item,
                stranded,
            }
            .render()
            .expect("scheduled row renders"),
        )
    }
    pub fn past_row(&self, item: &Item) -> h::Html {
        h::raw(
            PastPartial {
                ctx: self.ctx,
                item,
            }
            .render()
            .expect("scheduled history renders"),
        )
    }
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Scheduled messages".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar scheduled-messages")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}

/// Owned control; the room shell mounts this in the Markdown composer send row.
#[derive(Template)]
#[template(path = "scheduled_messages/_composer_button.html")]
pub struct ComposerButton<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub room_id: i64,
    pub thread_id: Option<i64>,
}
impl ComposerButton<'_> {
    pub fn dialog_id(&self) -> String {
        self.thread_id.map_or_else(
            || "schedule-send".into(),
            |id| format!("schedule-send-thread-{id}"),
        )
    }
    pub fn thread_value(&self) -> String {
        self.thread_id.map(|id| id.to_string()).unwrap_or_default()
    }
    pub fn icon(&self) -> h::Html {
        h::image_tag(
            self.ctx,
            "calendar.svg",
            h::attrs().size(17).aria("hidden", "true"),
        )
    }
    pub fn label(&self) -> h::Html {
        h::content_tag_text(
            "label",
            h::attrs().attr("for", format!("{}-custom", self.dialog_id())),
            "Custom time",
        )
    }
}
