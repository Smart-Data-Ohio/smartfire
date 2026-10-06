//! Work index and human handoff form. Domain policies are gathered before rendering.
use crate::channel_threads::board::{Links, LinksBox};
use crate::{ViewContext, helpers as h, helpers::filters, layouts::Page};
use askama::Template;

pub struct Row {
    pub id: i64,
    pub name: String,
    pub path: String,
    pub status: String,
    pub status_label: String,
    pub room_name: String,
    pub owner_label: String,
    pub agent: bool,
    pub count: i64,
    pub updated_at: jiff::Timestamp,
    pub links: Links,
}
#[derive(Template)]
#[template(path="work_threads/index.html", blocks=["head","content"])]
pub struct Index<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub state: &'a str,
    pub rows: &'a [Row],
}
impl Page for Index<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Work".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar work-workspace")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl Index<'_> {
    fn filter(&self, label: &str, state: &str) -> h::Html {
        h::link_to_text(
            label,
            &format!("/work?state={state}"),
            h::attrs().class(if self.state == state {
                "btn work-threads__filter active"
            } else {
                "btn work-threads__filter"
            }),
        )
    }
    fn row(&self, row: &Row) -> h::Html {
        h::raw(
            Thread { ctx: self.ctx, row }
                .render()
                .expect("work list row"),
        )
    }
}
#[derive(Template)]
#[template(path = "work_threads/_thread.html")]
struct Thread<'a> {
    ctx: &'a ViewContext<'a>,
    row: &'a Row,
}
impl Thread<'_> {
    fn updated_time(&self) -> h::Html {
        h::content_tag_text(
            "time",
            h::attrs()
                .attr("datetime", self.ctx.time_zone.iso8601(self.row.updated_at))
                .data("local_time_target", "datetime"),
            &self.ctx.time_zone.to_fs(self.row.updated_at, "long"),
        )
    }
    fn links(&self) -> h::Html {
        h::raw(
            LinksBox {
                ctx: self.ctx,
                thread_id: self.row.id,
                links: &self.row.links,
                context: "row",
            }
            .render()
            .expect("work row links"),
        )
    }
}
pub struct Handoff {
    pub id: i64,
    pub room_id: i64,
    pub name: String,
    pub receivers: Vec<(String, i64)>,
    pub error: Option<String>,
}
#[derive(Template)]
#[template(path="threads/work/handoffs/new.html",blocks=["head","content"])]
pub struct NewHandoff<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub handoff: &'a Handoff,
}
impl Page for NewHandoff<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("Hand off {}", self.handoff.name))
    }
}
impl NewHandoff<'_> {
    fn back(&self) -> h::Html {
        h::link_to_text(
            &format!("Back to {}", self.handoff.name),
            &format!(
                "/rooms/{}/threads/{}",
                self.handoff.room_id, self.handoff.id
            ),
            h::attrs().class("btn"),
        )
    }
    fn form(&self) -> h::FormWith {
        h::form_with(format!("/threads/{}/work/handoff", self.handoff.id)).class("board-post__form")
    }
    fn options(&self) -> h::Html {
        h::raw(
            self.handoff
                .receivers
                .iter()
                .map(|(name, id)| {
                    h::content_tag_text("option", h::attrs().value(id.to_string()), name).0
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}

#[derive(Template)]
#[template(path = "threads/work/links/index.html")]
pub struct LinksIndex<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub thread_id: i64,
    pub links: &'a Links,
}
impl LinksIndex<'_> {
    fn links(&self) -> h::Html {
        h::raw(
            LinksBox {
                ctx: self.ctx,
                thread_id: self.thread_id,
                links: self.links,
                context: "panel",
            }
            .render()
            .expect("panel links"),
        )
    }
}
#[derive(Template)]
#[template(path = "threads/work/links/create.turbo_stream.html")]
pub struct LinksChange<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub thread_id: i64,
    pub links: &'a Links,
}
impl LinksChange<'_> {
    fn stream(&self, context: &'static str) -> h::Html {
        let content = LinksBox {
            ctx: self.ctx,
            thread_id: self.thread_id,
            links: self.links,
            context,
        }
        .render()
        .expect("changed links");
        let template = if content.is_empty() {
            "\n".into()
        } else {
            format!("\n  {content}\n")
        };
        h::content_tag(
            "turbo-stream",
            h::attrs().attr("action", "replace").attr(
                "target",
                format!("work-thread-links-{context}-{}", self.thread_id),
            ),
            &format!("<template>{template}</template>"),
        )
    }
}
#[derive(Template)]
#[template(path = "threads/work/links/invalid.turbo_stream.html")]
pub struct LinksInvalid<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub thread_id: i64,
    pub error: &'a str,
}
impl LinksInvalid<'_> {
    fn stream(&self, context: &'static str) -> h::Html {
        let id = format!("work-thread-links-{context}-status-{}", self.thread_id);
        let status = format!(
            "<p class=\"work-links__status work-links__status--error\"\n   id=\"{id}\" role=\"status\">{}</p>\n",
            h::escape(self.error)
        );
        h::content_tag(
            "turbo-stream",
            h::attrs().attr("action", "replace").attr("target", id),
            &format!("<template>\n  {status}\n</template>"),
        )
    }
}
