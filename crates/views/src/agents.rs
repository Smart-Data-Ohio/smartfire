//! Human-facing agent directory and broadcast-safe status fragments.
pub mod history;
use crate::{
    ViewContext,
    helpers::{self as h, filters},
    layouts::Page,
};
use askama::Template;
pub trait DirectoryAgentRendering {
    fn since(&self, ctx: &ViewContext, now: &jiff::Timestamp) -> String;
    fn last_seen(&self, ctx: &ViewContext, now: &jiff::Timestamp) -> String;
    fn badge(&self, ctx: &ViewContext<'_>, now: &jiff::Timestamp) -> askama::Result<h::Html>;
}
impl DirectoryAgentRendering for DirectoryAgent {
    fn since(&self, ctx: &ViewContext, now: &jiff::Timestamp) -> String {
        h::time_ago_in_words(
            &ctx.time_zone,
            self.status_changed_at.unwrap_or(self.created_at),
            *now,
        )
    }
    fn last_seen(&self, ctx: &ViewContext, now: &jiff::Timestamp) -> String {
        self.last_seen_at
            .map(|time| {
                format!(
                    "last seen {} ago",
                    h::time_ago_in_words(&ctx.time_zone, time, *now)
                )
            })
            .unwrap_or_else(|| "never".into())
    }
    fn badge(&self, ctx: &ViewContext<'_>, now: &jiff::Timestamp) -> askama::Result<h::Html> {
        Ok(h::raw(
            StatusBadge {
                ctx,
                agent: self,
                now: *now,
            }
            .render()?,
        ))
    }
}

#[derive(Template)]
#[template(path = "agents/_status_badge.html")]
pub struct StatusBadge<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub agent: &'a DirectoryAgent,
    pub now: jiff::Timestamp,
}

#[derive(Template)]
#[template(path = "agents/directory/_agent.html")]
pub struct DirectoryRow<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub agent: &'a DirectoryAgent,
    pub now: jiff::Timestamp,
}

#[derive(Template)]
#[template(path = "agents/directory/index.html", blocks = ["head", "content"])]
pub struct Directory<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub agents: Vec<DirectoryAgent>,
    pub now: jiff::Timestamp,
}
impl Page for Directory<'_> {
    fn page_title(&self) -> Option<String> {
        Some("Agents".into())
    }
    fn body_class(&self) -> Option<&str> {
        Some("sidebar workspace-page")
    }
    fn has_sidebar(&self) -> bool {
        true
    }
}
impl Directory<'_> {
    fn subscription(&self) -> h::Html {
        h::builder_tag(
            "turbo-cable-stream-source",
            h::attrs().attr("channel", "AgentsChannel").attr(
                "signed-stream-name",
                (self.ctx.signed_stream_name)(&["agents:all"]),
            ),
        )
    }
}

#[derive(Template)]
#[template(path = "agent_steps/_thread_steps.html")]
pub struct ThreadSteps {
    pub thread_id: i64,
    pub steps: Vec<crate::messages::parts::AgentStep>,
}
#[derive(Template)]
#[template(path = "users/_agent_profile.html")]
pub struct ProfileDetails<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub profile: &'a Profile,
    pub now: jiff::Timestamp,
}
impl ProfileDetails<'_> {
    fn subscription(&self) -> h::Html {
        h::builder_tag("turbo-cable-stream-source", h::attrs().attr("channel", "AgentsChannel")
            .attr("signed-stream-name", (self.ctx.signed_stream_name)(&["agents:all"])))
    }
    fn rooms(&self) -> h::Html {
        h::raw(self.profile.rooms.iter().map(|(id, name)|
            h::link_to_text(name, &h::routes::room(*id), h::attrs()).0).collect::<Vec<_>>().join(", "))
    }
}
pub use campfire_presentation::agents::*;

use crate::rendering::*;
