//! Activity inbox rendering. Source access and approval decisions remain with domain callers.
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
#[derive(Clone, Debug, serde::Deserialize)]
pub struct Item {
    pub id: i64,
    pub state: String,
    pub event_label: String,
    pub created_at: Option<jiff::Timestamp>,
    pub approval: Option<crate::agents::history::Approval>,
    pub title: String,
    pub author: Option<String>,
    pub body: String,
}
pub const DESTINATIONS: [(&str, &str, &str); 5] = [
    ("/work", "check.svg", "Work threads"),
    ("/saved", "bookmark.svg", "Saved"),
    ("/scheduled_messages", "calendar.svg", "Scheduled"),
    ("/agents", "bot.svg", "Agents"),
    ("/users", "everyone.svg", "People"),
];
pub const TYPES: [(&str, &str); 9] = [
    ("all", "All"),
    ("mentions", "Mentions and replies"),
    ("threads", "Threads and work"),
    ("events", "Events"),
    ("agents", "Agents"),
    ("github", "GitHub"),
    ("huddles", "Huddles"),
    ("reminders", "Reminders"),
    ("security", "Security"),
];
pub fn path(filter: &str, kind: &str, before: Option<i64>, stream: bool) -> String {
    use h::url::{Param, with_query};
    let mut params = vec![
        ("status", Param::One(filter.into())),
        ("type", Param::One(kind.into())),
    ];
    if let Some(before) = before {
        params.push(("before", Param::One(before.to_string())));
    }
    with_query(
        if stream {
            "/activity.turbo_stream"
        } else {
            "/activity"
        },
        params,
    )
}
#[derive(Template)]
#[template(path="activity_items/index.html", blocks=["head","nav","content"])]
pub struct Inbox<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub items: &'a [Item],
    pub filter: &'a str,
    pub type_filter: &'a str,
    pub before: Option<i64>,
    pub next_cursor: Option<i64>,
    pub unread_count: usize,
    pub now: jiff::Timestamp,
}
impl Page for Inbox<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Activity inbox".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar activity-inbox")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl Inbox<'_> {
    fn active_filter(&self, value: &str) -> bool {
        self.filter == value
    }
    fn active_type(&self, value: &str) -> bool {
        self.type_filter == value
    }
    fn stream_path(&self) -> String {
        path(self.filter, self.type_filter, self.before, true)
    }
    fn list(&self) -> askama::Result<h::Html> {
        List {
            ctx: self.ctx,
            items: self.items,
            filter: self.filter,
            type_filter: self.type_filter,
            now: self.now,
        }
        .render()
        .map(h::raw)
    }
    fn pagination(&self) -> h::Html {
        self.next_cursor
            .map(|id| {
                h::raw(format!(
                    "  {}\n",
                    h::link_to_text(
                        "Older activity",
                        &path(self.filter, self.type_filter, Some(id), false),
                        h::attrs().class("btn btn--plain activity-inbox__older"),
                    )
                    .0
                ))
            })
            .unwrap_or_else(h::empty)
    }
    fn filter_link(&self, filter: &str, label: &str, type_filter: &str, active: bool) -> h::Html {
        h::link_to_text(
            label,
            &path(filter, type_filter, None, false),
            h::attrs()
                .class(if active {
                    "btn activity-inbox__filter is-active"
                } else {
                    "btn activity-inbox__filter"
                })
                .attr_opt("aria-current", active.then_some("page")),
        )
    }
}
#[derive(Template)]
#[template(path = "activity_items/_list.html")]
pub struct List<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub items: &'a [Item],
    pub filter: &'a str,
    pub type_filter: &'a str,
    pub now: jiff::Timestamp,
}
impl List<'_> {
    fn item(&self, item: &Item) -> askama::Result<h::Html> {
        ItemView {
            ctx: self.ctx,
            item,
            filter: self.filter,
            type_filter: self.type_filter,
            now: self.now,
        }
        .render()
        .map(|html| {
            h::raw(if item.approval.is_some() {
                format!("  {html}\n\n")
            } else {
                format!("{html}\n")
            })
        })
    }
}
#[derive(Template)]
#[template(path = "activity_items/_item.html")]
struct ItemView<'a> {
    ctx: &'a ViewContext<'a>,
    item: &'a Item,
    filter: &'a str,
    type_filter: &'a str,
    now: jiff::Timestamp,
}
impl ItemView<'_> {
    fn open_action(&self) -> h::Html {
        let form = self.action("open", "Open", None);
        h::raw(format!(
            "{}{}",
            if self.item.approval.is_some() {
                ""
            } else {
                "  "
            },
            form
        ))
    }
    fn timestamp(&self) -> h::Html {
        self.item
            .created_at
            .map(|at| {
                h::local_datetime_tag(
                    &self.ctx.time_zone,
                    at,
                    "time",
                    h::attrs().class("activity-item__time"),
                    "",
                )
            })
            .unwrap_or_else(h::empty)
    }
    fn card(&self, approval: &crate::agents::history::Approval) -> askama::Result<h::Html> {
        crate::agents::history::ApprovalCard {
            ctx: self.ctx,
            approval,
            now: self.now,
        }
        .render()
        .map(h::raw)
    }
    fn action(&self, action: &str, label: &str, state: Option<&str>) -> h::Html {
        let attrs = h::attrs()
            .method(if action == "open" { "post" } else { "patch" })
            .class(if action == "open" {
                "btn btn--primary"
            } else {
                "btn btn--plain"
            });
        let mut form = h::button_to_form(
            &format!("/activity/{}/{action}", self.item.id),
            attrs,
            h::attrs().class("activity-item__action-form"),
            label,
        );
        if let Some(state) = state {
            let fields = [
                ("state", state),
                ("status", self.filter),
                ("type", self.type_filter),
            ]
            .into_iter()
            .map(|(k, v)| h::legacy_tag("input", h::attrs().type_("hidden").name(k).value(v)).0)
            .collect::<String>();
            form.0 = form.0.replace("</form>", &format!("{fields}</form>"));
        }
        form
    }
}
