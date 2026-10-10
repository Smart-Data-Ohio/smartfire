//! Request-only board-post forms and pages. Presenters supply every database fact.
use crate::helpers::filters;
use crate::{
    ViewContext, helpers as h,
    layouts::Page,
    messages::{MessageItem, UserView, composer::Facts, parts::AgentStep},
};
use askama::Template;
#[derive(Template)]
#[template(path="channel_threads/new.html", blocks=["head","content"])]
pub struct New<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub post: &'a NewPost,
}
impl Page for New<'_> {
    fn page_title(&self) -> Option<String> {
        Some(format!("New post in {}", self.post.room_name))
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl New<'_> {
    fn form(&self) -> h::FormWith {
        h::form_with(format!("/rooms/{}/threads", self.post.room_id))
            .model("thread")
            .class("board-post-form__form")
            .field_errors(self.post.field_errors.clone())
    }
    fn room_link(&self) -> h::Html {
        room_link(self.post.room_id, &self.post.room_name)
    }
    fn status_options(&self) -> h::Html {
        status_options(&self.post.status)
    }
    fn owner_options(&self) -> h::Html {
        owner_options_with_prompt(
            &self.post.humans,
            &self.post.agents,
            self.post.owner_id,
            self.post.owner_id.is_none(),
        )
    }
}
pub fn status_options(selected: &str) -> h::Html {
    h::raw(
        [
            ("planned", "Planned"),
            ("in_progress", "In progress"),
            ("blocked", "Blocked"),
            ("done", "Done"),
        ]
        .into_iter()
        .map(|(value, label)| {
            let mut attrs = h::attrs();
            if selected == value {
                attrs = attrs.attr("selected", "selected");
            }
            h::content_tag_text("option", attrs.value(value), label).0
        })
        .collect::<Vec<_>>()
        .join("\n"),
    )
}
pub fn owner_options(
    humans: &[(String, i64)],
    agents: &[(String, i64)],
    selected: Option<i64>,
) -> h::Html {
    owner_options_with_prompt(humans, agents, selected, true)
}
fn owner_options_with_prompt(
    humans: &[(String, i64)],
    agents: &[(String, i64)],
    selected: Option<i64>,
    prompt: bool,
) -> h::Html {
    let mut html = if prompt {
        h::content_tag_text("option", h::attrs().value(""), "Unassigned").0 + "\n"
    } else {
        String::new()
    };
    for (label, users) in [("Members", humans), ("Agents", agents)] {
        let options = users
            .iter()
            .map(|(name, id)| {
                let mut attrs = h::attrs();
                if Some(*id) == selected {
                    attrs = attrs.attr("selected", "selected");
                }
                h::content_tag_text("option", attrs.value(id.to_string()), name).0
            })
            .collect::<Vec<_>>()
            .join("\n");
        html += &h::content_tag("optgroup", h::attrs().attr("label", label), &options).0;
    }
    h::raw(html)
}
fn room_link(id: i64, name: &str) -> h::Html {
    h::link_to_text(name, &format!("/rooms/{id}"), h::attrs())
}
#[derive(Template)]
#[template(path="channel_threads/board_post.html", blocks=["head","content"])]
pub struct Show<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub post: &'a Post,
}
impl Page for Show<'_> {
    fn page_title(&self) -> Option<String> {
        self.post.name.clone()
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl Show<'_> {
    fn room_link(&self) -> h::Html {
        room_link(self.post.room_id, &self.post.room_name)
    }
    fn path(&self) -> String {
        format!("/rooms/{}/threads/{}", self.post.room_id, self.post.id)
    }
    fn form(&self) -> h::FormWith {
        h::form_with(self.path())
            .method("patch")
            .class("board-post__form")
    }
    fn status_options(&self) -> h::Html {
        status_options(&self.post.status)
    }
    fn owner_options(&self) -> h::Html {
        owner_options(&self.post.humans, &self.post.agents, self.post.owner_id)
    }
    fn message_count(&self) -> String {
        format!(
            "{} {}",
            self.post.count,
            if self.post.count == 1 {
                "message"
            } else {
                "messages"
            }
        )
    }
    fn result_time(&self) -> h::Html {
        self.post
            .result_at
            .map(|at| {
                h::content_tag_text(
                    "time",
                    h::attrs()
                        .attr("datetime", self.ctx.time_zone.iso8601(at))
                        .data("local_time_target", "datetime"),
                    &self.ctx.time_zone.to_fs(at, "long"),
                )
            })
            .unwrap_or_else(h::empty)
    }
    fn with_conversation(
        &self,
        render: impl FnOnce(&super::Conversation<'_>) -> h::Html,
    ) -> h::Html {
        let empty = h::empty();
        render(&super::Conversation {
            ctx: self.ctx,
            thread_id: self.post.id,
            room_updated_at: self.post.room_updated_at,
            anchor: None,
            messages: &self.post.messages,
            user: &self.post.user,
            steps: &self.post.steps,
            composer: &self.post.composer,
            scheduled_control: &empty,
        })
    }
    fn area_open(&self) -> h::Html {
        self.with_conversation(|c| c.area_open())
    }
    fn list_open(&self) -> h::Html {
        self.with_conversation(|c| c.list_open())
    }
    fn pending(&self) -> h::Html {
        self.with_conversation(|c| c.pending_template())
    }
    fn stream(&self) -> h::Html {
        self.with_conversation(|c| c.stream())
    }
    fn composer(&self) -> h::Html {
        let scheduled = crate::scheduled_messages::ComposerButton {
            ctx: self.ctx,
            room_id: self.post.room_id,
            thread_id: Some(self.post.id),
        }
        .render()
        .expect("schedule button");
        h::raw(
            crate::messages::composer::Composer {
                ctx: self.ctx,
                facts: &self.post.composer,
                scheduled_control: &h::raw(scheduled),
            }
            .render()
            .expect("composer"),
        )
    }
    fn membership_button(&self) -> h::Html {
        let (label, path, method) = if self.post.joined {
            ("Leave post", "leave", "delete")
        } else {
            ("Join post", "join", "post")
        };
        h::button_to(
            &format!("{}/{path}", self.path()),
            h::attrs().method(method).class("btn"),
            label,
        )
    }
    fn lifecycle_button(&self, label: &str, status: &str) -> h::Html {
        h::button_to_form_params(
            &self.path(),
            h::attrs().method("patch").class("btn"),
            h::attrs(),
            label,
            &[("thread[status]", status)],
        )
    }
    fn delete_button(&self) -> h::Html {
        h::button_to_form(
            &self.path(),
            h::attrs().method("delete").class("btn btn--negative"),
            h::attrs().data(
                "turbo_confirm",
                "Delete this post and its discussion? This can't be undone.",
            ),
            "Delete post",
        )
    }
    fn handoff_link(&self) -> h::Html {
        h::link_to_text(
            "Hand off to an agent",
            &format!("/threads/{}/work/handoff/new", self.post.id),
            h::attrs().class("btn"),
        )
    }
    fn links(&self) -> h::Html {
        h::raw(
            LinksBox {
                ctx: self.ctx,
                thread_id: self.post.id,
                links: &self.post.links,
                context: "header",
            }
            .render()
            .expect("work links"),
        )
    }
}
#[derive(Template)]
#[template(path = "threads/work/links/_box.html")]
pub struct LinksBox<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub thread_id: i64,
    pub links: &'a Links,
    pub context: &'static str,
}
impl LinksBox<'_> {
    fn path(&self) -> String {
        format!("/threads/{}/work/links", self.thread_id)
    }
    fn link(&self, link: &Link) -> h::Html {
        let attrs = h::attrs().class(format!(
            "work-links__{}",
            match link.kind.as_str() {
                "pull_request" => "pr",
                "event" => "event",
                _ => "drive",
            }
        ));
        let attrs = if link.kind != "event" {
            attrs
                .attr("target", "_blank")
                .attr("rel", "noopener noreferrer")
        } else {
            attrs
        };
        h::link_to_text(&link.label, &link.url, attrs)
    }
    fn remove(&self, link: &Link) -> h::Html {
        h::button_to(
            &format!("{}/{}", self.path(), link.id),
            h::attrs()
                .method("delete")
                .class("work-links__remove")
                .attr("form_class", "work-links__remove-form")
                .aria("label", &link.remove_label),
            "Remove",
        )
    }
    fn event_time(&self, link: &Link) -> h::Html {
        link.event_time
            .map(|at| {
                let zone = crate::time::Zone::for_user(link.event_zone.as_deref());
                h::local_datetime_tag(
                    &self.ctx.time_zone,
                    at,
                    "datetime",
                    h::attrs(),
                    &h::escape(&zone.format(at, "%B %-d, %Y at %-I:%M %p")),
                )
            })
            .unwrap_or_else(h::empty)
    }
    fn event_options(&self) -> h::Html {
        h::raw(
            self.links
                .events
                .iter()
                .map(|(label, id)| {
                    h::content_tag_text("option", h::attrs().value(id.to_string()), label).0
                })
                .collect::<Vec<_>>()
                .join("\n"),
        )
    }
}
pub use campfire_presentation::channel_threads::board::*;

pub struct Post {
    pub id: i64,
    pub room_id: i64,
    pub room_name: String,
    pub room_updated_at: jiff::Timestamp,
    pub name: Option<String>,
    pub lifecycle: String,
    pub count: i64,
    pub status: String,
    pub status_label: String,
    pub owner_label: String,
    pub owner_agent: bool,
    pub owner_id: Option<i64>,
    pub tags: Vec<String>,
    pub run_url: Option<String>,
    pub can_manage: bool,
    pub can_assign: bool,
    pub can_lifecycle: bool,
    pub joined: bool,
    pub humans: Vec<(String, i64)>,
    pub agents: Vec<(String, i64)>,
    pub result: Option<h::Html>,
    pub result_markdown: Option<String>,
    pub result_at: Option<jiff::Timestamp>,
    pub result_by: Option<String>,
    pub user: UserView,
    pub messages: Vec<MessageItem>,
    pub steps: Vec<AgentStep>,
    pub composer: Facts,
    pub history: Vec<History>,
    pub links: Links,
    pub error: Option<String>,
}
