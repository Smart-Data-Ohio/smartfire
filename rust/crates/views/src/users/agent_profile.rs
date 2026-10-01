//! Display facts from WS11's agent readers; no queries or credentials in these templates.
use super::*;
#[derive(Clone, Debug)]
pub struct AgentProfile {
    pub id: i64,
    pub kind_description: String,
    pub provider_runtime: Option<String>,
    pub description: Option<String>,
    pub status: String,
    pub status_note: Option<String>,
    pub status_since: jiff::Timestamp,
    pub last_seen_at: Option<jiff::Timestamp>,
    pub suspended: bool,
    pub rooms: Vec<(i64, String)>,
    pub hidden_rooms: usize,
    pub grants_summary: String,
    pub activity_summary: Option<String>,
    pub budget_usage: Option<String>,
    pub now: jiff::Timestamp,
}
#[derive(Template)]
#[template(path = "users/agents/_status_badge.html")]
pub struct AgentProfileBadge<'a> {
    pub ctx: &'a ViewContext<'a>,
    pub agent: &'a AgentProfile,
}
impl AgentProfileBadge<'_> {
    fn since(&self) -> String {
        h::time_ago_in_words(&self.ctx.time_zone, self.agent.status_since, self.agent.now)
    }
    fn title(&self) -> &str {
        match self.agent.status.as_str() {
            "working" => "Working",
            "waiting" => "Waiting",
            "failed" => "Failed",
            _ => "Idle",
        }
    }
}
impl Show<'_> {
    pub(super) fn agent_subscription(&self) -> h::Html {
        h::builder_tag(
            "turbo-cable-stream-source",
            h::attrs().attr("channel", "AgentsChannel").attr(
                "signed-stream-name",
                (self.ctx.signed_stream_name)(&["agents:all"]),
            ),
        )
    }
    pub(super) fn agent_badge(&self, agent: &AgentProfile) -> h::Html {
        h::raw(
            AgentProfileBadge {
                ctx: self.ctx,
                agent,
            }
            .render()
            .expect("agent profile badge"),
        )
    }
    pub(super) fn agent_seen(&self, agent: &AgentProfile) -> String {
        agent
            .last_seen_at
            .map(|t| {
                format!(
                    "last seen {} ago",
                    h::time_ago_in_words(&self.ctx.time_zone, t, agent.now)
                )
            })
            .unwrap_or_else(|| "never".into())
    }
    pub(super) fn agent_rooms(&self, agent: &AgentProfile) -> h::Html {
        h::raw(
            agent
                .rooms
                .iter()
                .map(|(id, name)| h::link_to_text(name, &h::routes::room(*id), h::attrs()).0)
                .collect::<Vec<_>>()
                .join(", "),
        )
    }
}
